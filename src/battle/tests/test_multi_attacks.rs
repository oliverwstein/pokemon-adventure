#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleEvent, BattleState, GameState, TurnRng};
    use crate::battle::turn_orchestrator::resolve_turn;
    use crate::moves::Move;
    use crate::player::{BattlePlayer, PlayerAction};
    use crate::pokemon::{MoveInstance, PokemonInst};
    use crate::species::Species;
    use std::collections::HashMap;

    // Helper function to create a Pokemon with specific stats and moves
    fn create_test_pokemon(
        species: Species,
        moves: Vec<Move>,
        hp: u16,
        attack: u16,
    ) -> PokemonInst {
        let mut pokemon_moves = [const { None }; 4];
        for (i, mv) in moves.into_iter().enumerate() {
            if i < 4 {
                pokemon_moves[i] = Some(MoveInstance { move_: mv, pp: 30 });
            }
        }

        {
            let mut pokemon = PokemonInst::new_for_test(
                species,
                10, 0,
                0, // Will be set below
                [15; 6],
                [0; 6],
                [hp, attack, 80, 80, 80, 80],
                pokemon_moves,
                None,
            );
            pokemon.set_hp(hp);
            pokemon
        }
    }

    // Helper function to create a player
    fn create_test_player(pokemon: PokemonInst) -> BattlePlayer {
        BattlePlayer {
            player_id: "test_player".to_string(),
            player_name: "TestPlayer".to_string(),
            team: [Some(pokemon), None, None, None, None, None],
            active_pokemon_index: 0,
            stat_stages: HashMap::new(),
            team_conditions: HashMap::new(),
            active_pokemon_conditions: HashMap::new(),
            last_move: None,
            ante: 200,
        }
    }

    #[test]
    fn test_probabilistic_multi_hit_logic() {
        // SETUP
        // Initialize global data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        // We assume Fury Swipes is defined in its .ron file as:
        // MultiHit(guaranteed_hits: 2, continuation_chance: 50)
        let attacker = create_test_pokemon(Species::Meowth, vec![Move::FurySwipes], 100, 80);
        // Defender needs enough HP to survive a few hits
        let defender = create_test_pokemon(Species::Pidgey, vec![Move::Tackle], 100, 80);

        let player1 = create_test_player(attacker);
        let player2 = create_test_player(defender);

        let mut battle_state = BattleState::new("multi_hit_test".to_string(), player1, player2);

        // Manually set actions to ensure Fury Swipes is used
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // EXECUTION
        // Craft a specific RNG sequence to force 3 hits and then stop.
        // Fury Swipes has guaranteed_hits: 2, continuation_chance: 50
        let test_rng = TurnRng::new_for_test(vec![
            // Attacker's turn (Meowth)
            // Hit 1 (Guaranteed):
            50, // Accuracy roll (hit)
            90, // Crit roll (no crit)
            95, // Damage variance roll
            // Hit 2 (Guaranteed):
            50, // Accuracy roll (hit)
            90, // Crit roll (no crit)
            92, // Damage variance roll
            // Continuation roll for Hit 3 (needs <= 50):
            40, // SUCCESS! Queue another hit.
            // Hit 3 (Probabilistic):
            50, // Accuracy roll (hit)
            90, // Crit roll (no crit)
            90, // Damage variance roll
            // Continuation roll for Hit 4 (needs <= 50):
            60, // FAIL! Stop the sequence.
            // Defender's turn (Pidgey) - needs rolls even if we don't care about the outcome
            50, 90, 90, 75, 80, 85, 55, 60, 70, 45,
        ]);

        let event_bus = resolve_turn(&mut battle_state, test_rng);
        let events = event_bus.events();

        // --- ADDED LOGGING ---
        println!(
            "Generated {} events in probabilistic multi-hit test:",
            events.len()
        );
        for event in events {
            println!("  {:?}", event);
        }
        // ---------------------

        // ASSERTIONS
        let hit_events: Vec<_> = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    BattleEvent::MoveHit {
                        move_used: Move::FurySwipes,
                        ..
                    }
                )
            })
            .collect();
        let damage_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, BattleEvent::DamageDealt { .. }))
            .collect();

        // We forced exactly 3 hits.
        assert_eq!(
            hit_events.len(),
            3,
            "Fury Swipes should have hit exactly 3 times"
        );
        // The defender's move also deals damage, so we expect 3 + 1 = 4 damage events.
        assert_eq!(
            damage_events.len(),
            4,
            "Should be 4 total damage events in the turn"
        );
        assert!(
            matches!(battle_state.game_state, GameState::WaitingForBothActions),
            "Game should be ready for the next turn"
        );
    }

    #[test]
    fn test_multi_hit_stops_on_faint() {
        // GOAL: Verify that a multi-hit sequence terminates immediately when the target faints,
        // preventing subsequent guaranteed hits from executing.

        // SETUP
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        // Attacker uses Fury Swipes (assume 2 guaranteed hits).
        let attacker = create_test_pokemon(Species::Meowth, vec![Move::FurySwipes], 100, 80);
        // Defender has VERY low HP to guarantee it faints on the first hit.
        let defender = create_test_pokemon(Species::Pidgey, vec![Move::Tackle], 10, 80); // <-- CRITICAL CHANGE HERE

        let player1 = create_test_player(attacker);
        let mut player2 = create_test_player(defender);
        // Give player 2 a backup so the battle doesn't end.
        player2.team[1] = Some(create_test_pokemon(Species::Rattata, vec![], 100, 80));

        let mut battle_state =
            BattleState::new("multi_hit_faint_test".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // EXECUTION
        // RNG ensures the first hit connects and deals damage.
        let test_rng = TurnRng::new_for_test(vec![50, 90, 100, 50, 90, 90]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);
        let events = event_bus.events();

        println!("--- Events for test_multi_hit_stops_on_faint ---");
        for event in events {
            println!("  {:?}", event);
        }

        // ASSERTIONS
        let fury_swipes_hits = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    BattleEvent::MoveHit {
                        move_used: Move::FurySwipes,
                        ..
                    }
                )
            })
            .count();

        // This assertion will now pass, as the faint on the first hit will prevent
        // the second guaranteed hit from ever starting.
        assert_eq!(
            fury_swipes_hits, 1,
            "The multi-hit sequence should be stopped by the faint, resulting in exactly one hit event."
        );

        let faint_events = events
            .iter()
            .filter(|e| matches!(e, BattleEvent::PokemonFainted { .. }))
            .count();
        assert_eq!(faint_events, 1, "The defender should have fainted");
        assert!(
            matches!(
                battle_state.game_state,
                GameState::WaitingForPlayer2Replacement
            ),
            "Game should be waiting for replacement"
        );

        // We can also assert that only one damage event occurred in total, since the
        // fainted defender's turn is skipped.
        let damage_events = events
            .iter()
            .filter(|e| matches!(e, BattleEvent::DamageDealt { .. }))
            .count();
        assert_eq!(
            damage_events, 1,
            "Only the first, fatal hit should have dealt damage."
        );
    }
}
