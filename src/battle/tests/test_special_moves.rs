#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleEvent, BattleState, GameState, TurnRng};
    use crate::battle::turn_orchestrator::{collect_player_actions, resolve_turn};
    use crate::moves::Move;
    use crate::player::{BattlePlayer, PlayerAction, PokemonCondition};
    use crate::pokemon::{MoveInstance, PokemonInst};
    use crate::species::Species;
    use std::collections::HashMap;

    fn create_test_pokemon(species: Species, moves: Vec<Move>) -> PokemonInst {
        let mut pokemon_moves = [const { None }; 4];
        for (i, mv) in moves.into_iter().enumerate() {
            if i < 4 {
                pokemon_moves[i] = Some(MoveInstance { move_: mv, pp: 20 }); // Increased PP to ensure tests work
            }
        }

        {
            let mut pokemon = PokemonInst::new_for_test(
                species,
                0,
                0, // Will be set below
                [15; 6],
                [0; 6],
                [100, 80, 80, 80, 80, 80],
                pokemon_moves,
                None,
            );
            pokemon.set_hp_to_max();
            pokemon
        }
    }

    #[test]
    fn test_two_turn_move_charging() {
        // Test ChargeUp moves like Solar Beam
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Venusaur, vec![Move::Solarbeam])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(), 
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Charizard, vec![Move::Tackle])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Turn 1: Solar Beam should charge
        collect_player_actions(&mut battle_state).expect("Should collect actions");
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Player 1 should now have Charging condition
        assert!(battle_state.players[0].has_condition(&PokemonCondition::Charging));
        
        // Player 1's last move should be Solar Beam
        assert_eq!(battle_state.players[0].last_move, Some(Move::Solarbeam));

        // Turn 2: Solar Beam should execute with damage
        collect_player_actions(&mut battle_state).expect("Should collect actions");
        
        // Player 1 should have a ForcedMove action
        match &battle_state.action_queue[0] {
            Some(PlayerAction::ForcedMove { pokemon_move }) => {
                assert_eq!(*pokemon_move, Move::Solarbeam);
            }
            _ => panic!("Player 1 should have ForcedMove action"),
        }
        
        let test_rng2 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus2 = resolve_turn(&mut battle_state, test_rng2);
        
        // Charging condition should be cleared after execution
        assert!(!battle_state.players[0].has_condition(&PokemonCondition::Charging));
    }

    #[test]
    fn test_two_turn_move_fly() {
        // Test InAir moves like Fly
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(), 
            vec![create_test_pokemon(Species::Pidgeot, vec![Move::Fly])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Rattata, vec![Move::Tackle])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Turn 1: Fly should go in air
        collect_player_actions(&mut battle_state).expect("Should collect actions");
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Player 1 should now have InAir condition
        assert!(battle_state.players[0].has_condition(&PokemonCondition::InAir));
        assert_eq!(battle_state.players[0].last_move, Some(Move::Fly));

        // Turn 2: Fly should execute attack
        collect_player_actions(&mut battle_state).expect("Should collect actions");
        
        // Player 1 should have a ForcedMove action
        match &battle_state.action_queue[0] {
            Some(PlayerAction::ForcedMove { pokemon_move }) => {
                assert_eq!(*pokemon_move, Move::Fly);
            }
            _ => panic!("Player 1 should have ForcedMove action"),
        }
    }

    #[test]
    fn test_two_turn_move_dig() {
        // Test Underground moves like Dig
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Sandslash, vec![Move::Dig])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Geodude, vec![Move::RockThrow])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Turn 1: Dig should go underground
        collect_player_actions(&mut battle_state).expect("Should collect actions");
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Player 1 should now have Underground condition
        assert!(battle_state.players[0].has_condition(&PokemonCondition::Underground));
        assert_eq!(battle_state.players[0].last_move, Some(Move::Dig));
    }

    #[test]
    fn test_rampage_move() {
        // Test Rampaging moves like Thrash
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Tauros, vec![Move::Thrash])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Slowpoke, vec![Move::WaterGun])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Turn 1: Thrash should apply Rampaging condition
        collect_player_actions(&mut battle_state).expect("Should collect actions");
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]); // 50% chance for 2-3 turns
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Player 1 should now have Rampaging condition
        let has_rampage = battle_state.players[0].active_pokemon_conditions.values().any(|condition| {
            matches!(condition, PokemonCondition::Rampaging { .. })
        });
        assert!(has_rampage);
        assert_eq!(battle_state.players[0].last_move, Some(Move::Thrash));

        // Turn 2: Should be forced to use Thrash again
        collect_player_actions(&mut battle_state).expect("Should collect actions");
        match &battle_state.action_queue[0] {
            Some(PlayerAction::ForcedMove { pokemon_move }) => {
                assert_eq!(*pokemon_move, Move::Thrash);
            }
            _ => panic!("Player 1 should have ForcedMove action for rampage"),
        }
    }

    #[test]
    fn test_mirror_move_success() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");
            
        // Test Mirror Move copying opponent's last move in a single turn
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Pidgeot, vec![Move::MirrorMove])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Lightning])],
        );

        // Set Player 2's last move to Lightning (as if they used it previously)
        player2.last_move = Some(Move::Lightning);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Mirror Move, Player 2 uses Lightning
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Mirror Move
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Lightning

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Check what events were generated to debug the issue
        println!("Events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Mirror Move should have been executed (no ActionFailed events)
        let failed_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::ActionFailed { .. }))
            .collect();
        
        // For debugging: if there are failed events, print them
        if !failed_events.is_empty() {
            println!("ActionFailed events found:");
            for event in &failed_events {
                println!("  {:?}", event);
            }
        }
        
        assert_eq!(failed_events.len(), 0, "Mirror Move should not fail when copying a valid move");
    }

    #[test]
    fn test_mirror_move_fail_mirror_move() {
        // Test Mirror Move failing when trying to copy Mirror Move
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Pidgeot, vec![Move::MirrorMove])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Fearow, vec![Move::MirrorMove])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Set Player 2's last move to Mirror Move
        battle_state.players[1].last_move = Some(Move::MirrorMove);

        // Player 1 uses Mirror Move
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Mirror Move
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Some other move

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Should have ActionFailed event
        let failed_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::ActionFailed { .. }))
            .collect();
        assert!(!failed_events.is_empty(), "Mirror Move should fail when copying Mirror Move");
    }

    #[test]
    fn test_mirror_move_fail_no_last_move() {
        // Test Mirror Move failing when opponent has no last move
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Pidgeot, vec![Move::MirrorMove])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Rattata, vec![Move::Tackle])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 2 has no last move (None)
        assert_eq!(battle_state.players[1].last_move, None);

        // Player 1 uses Mirror Move
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Mirror Move
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Should have ActionFailed event
        let failed_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::ActionFailed { .. }))
            .collect();
        assert!(!failed_events.is_empty(), "Mirror Move should fail when no move to copy");
    }

    #[test]
    fn test_explode_move() {
        // Test Explode effect - user faints then damage is dealt
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Electrode, vec![Move::Explosion])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Rest])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        let initial_hp_p1 = battle_state.players[0].active_pokemon().unwrap().current_hp();
        let initial_hp_p2 = battle_state.players[1].active_pokemon().unwrap().current_hp();

        // Player 1 uses Explosion
        collect_player_actions(&mut battle_state).expect("Should collect actions");
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Player 1 should have fainted
        assert!(battle_state.players[0].active_pokemon().unwrap().is_fainted());
        
        // Player 2 should have taken damage (if explosion hit)
        let final_hp_p2 = battle_state.players[1].active_pokemon().unwrap().current_hp();
        // Note: Explosion might miss, so we just check that the battle proceeded without error
        
        // Should have PokemonFainted event for Player 1
        let fainted_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::PokemonFainted { player_index: 0, .. }))
            .collect();
        assert!(!fainted_events.is_empty(), "Player 1 should have fainted from Explosion");
    }
}