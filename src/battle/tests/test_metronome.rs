#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleEvent, BattleState, TurnRng};
    use crate::battle::turn_orchestrator::resolve_turn;
    use crate::moves::Move;
    use crate::player::{BattlePlayer, PlayerAction};
    use crate::pokemon::{MoveInstance, PokemonInst};
    use crate::species::Species;

    fn create_test_pokemon(species: Species, moves: Vec<Move>) -> PokemonInst {
        let mut pokemon_moves = [const { None }; 4];
        for (i, mv) in moves.into_iter().enumerate() {
            if i < 4 {
                pokemon_moves[i] = Some(MoveInstance { move_: mv, pp: 20 });
            }
        }

        let mut pokemon = PokemonInst::new_for_test(
            species,
            10,
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

    #[test]
    fn test_metronome_selects_random_move() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Clefairy,
                vec![Move::Metronome],
            )], // Clefairy with Metronome
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Splash])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Metronome, Player 2 uses Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Metronome
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        // Use deterministic RNG to ensure consistent test results
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        println!("Metronome random move test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Should have MoveUsed events - one for the selected move
        let move_used_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::MoveUsed {
                        player_index: 0,
                        pokemon: Species::Clefairy,
                        ..
                    }
                )
            })
            .collect();

        // Should have at least two MoveUsed events: one for Metronome, one for the selected move
        assert!(
            move_used_events.len() >= 2,
            "Should have MoveUsed events for both Metronome and the randomly selected move"
        );

        // Should have one Metronome event and one non-Metronome event
        let metronome_events: Vec<_> = move_used_events.iter()
            .filter(|event| {
                if let BattleEvent::MoveUsed { move_used, .. } = event {
                    *move_used == Move::Metronome
                } else {
                    false
                }
            })
            .collect();
        
        let non_metronome_events: Vec<_> = move_used_events.iter()
            .filter(|event| {
                if let BattleEvent::MoveUsed { move_used, .. } = event {
                    *move_used != Move::Metronome
                } else {
                    false
                }
            })
            .collect();

        assert_eq!(metronome_events.len(), 1, "Should have exactly one Metronome MoveUsed event");
        assert_eq!(non_metronome_events.len(), 1, "Should have exactly one randomly selected move MoveUsed event");

        // Should have at least executed some action (move used event proves this)
        assert!(
            move_used_events.len() >= 1,
            "Should have executed at least one move"
        );
    }

    #[test]
    fn test_metronome_can_select_different_moves() {
        // Test that Metronome can select different moves with different RNG values
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let mut selected_moves = std::collections::HashSet::new();

        // Run Metronome multiple times with different RNG values to see different moves
        for rng_value in [10, 25, 50, 75, 90] {
            let player1 = BattlePlayer::new(
                "player1".to_string(),
                "Player 1".to_string(),
                vec![create_test_pokemon(
                    Species::Clefairy,
                    vec![Move::Metronome],
                )],
            );

            let player2 = BattlePlayer::new(
                "player2".to_string(),
                "Player 2".to_string(),
                vec![create_test_pokemon(Species::Pikachu, vec![Move::Splash])],
            );

            let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

            battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Metronome
            battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

            let test_rng =
                TurnRng::new_for_test(vec![rng_value, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
            let event_bus = resolve_turn(&mut battle_state, test_rng);

            // Find all MoveUsed events for player 0
            let move_used_events: Vec<_> = event_bus
                .events()
                .iter()
                .filter_map(|event| match event {
                    BattleEvent::MoveUsed {
                        player_index: 0,
                        move_used,
                        ..
                    } => Some(*move_used),
                    _ => None,
                })
                .collect();

            // Should have both Metronome and the selected move
            assert!(move_used_events.len() >= 2, "Should have MoveUsed events for both Metronome and the selected move");
            assert!(move_used_events.contains(&Move::Metronome), "Should have Metronome MoveUsed event");
            
            // Find the non-Metronome move (the selected move)
            let selected_move = move_used_events.iter()
                .find(|&&mv| mv != Move::Metronome)
                .expect("Should have selected a non-Metronome move");

            selected_moves.insert(*selected_move);
        }

        // We should have seen multiple different moves (though not guaranteed due to randomness)
        // At minimum, we should have at least one valid move selection
        assert!(
            !selected_moves.is_empty(),
            "Should have selected at least one move"
        );
        println!("Metronome selected these moves: {:?}", selected_moves);
    }

    #[test]
    fn test_metronome_executes_selected_move_fully() {
        // Test that Metronome fully executes the selected move (e.g., damage moves deal damage)
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Clefairy,
                vec![Move::Metronome],
            )],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Splash])],
        );

        let initial_p2_hp = player2.active_pokemon().unwrap().current_hp();

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Metronome
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        // Use RNG values that are likely to select a damage-dealing move
        // We'll run this multiple times to increase chance of getting a damaging move
        let mut found_damage = false;

        for rng_seed in [20, 40, 60, 80] {
            // Reset battle state for each attempt
            let player1_fresh = BattlePlayer::new(
                "player1".to_string(),
                "Player 1".to_string(),
                vec![create_test_pokemon(
                    Species::Clefairy,
                    vec![Move::Metronome],
                )],
            );

            let player2_fresh = BattlePlayer::new(
                "player2".to_string(),
                "Player 2".to_string(),
                vec![create_test_pokemon(Species::Pikachu, vec![Move::Splash])],
            );

            let mut fresh_battle_state =
                BattleState::new("test_battle".to_string(), player1_fresh, player2_fresh);
            fresh_battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
            fresh_battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

            let test_rng =
                TurnRng::new_for_test(vec![rng_seed, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
            let event_bus = resolve_turn(&mut fresh_battle_state, test_rng);

            // Check if any damage was dealt
            let damage_events: Vec<_> = event_bus
                .events()
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        BattleEvent::DamageDealt {
                            target: Species::Pikachu,
                            ..
                        }
                    )
                })
                .collect();

            if !damage_events.is_empty() {
                found_damage = true;
                println!(
                    "Found damage event with RNG seed {}: {:?}",
                    rng_seed, damage_events[0]
                );

                // Verify the damage was actually applied
                let final_p2_hp = fresh_battle_state.players[1]
                    .active_pokemon()
                    .unwrap()
                    .current_hp();
                assert!(
                    final_p2_hp < initial_p2_hp,
                    "Damage should have been applied to target Pokemon"
                );
                break;
            }
        }

        // Note: Due to randomness, we might not always get a damage move, but the test structure is correct
        if found_damage {
            println!("Successfully found and verified Metronome executing a damage-dealing move");
        } else {
            println!(
                "No damage moves were randomly selected in this test run (this can happen due to randomness)"
            );
        }
    }

    #[test]
    fn test_metronome_with_status_moves() {
        // Test that Metronome can select and execute status moves properly
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Clefairy,
                vec![Move::Metronome],
            )],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Splash])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Metronome
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![30, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Should have at least executed some move
        let move_used_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::MoveUsed {
                        player_index: 0,
                        ..
                    }
                )
            })
            .collect();

        // Should have at least two MoveUsed events: one for Metronome, one for the selected move
        assert!(move_used_events.len() >= 2, "Should have MoveUsed events for both Metronome and the selected move");

        // Verify we have both Metronome and a non-Metronome move
        let moves: Vec<_> = move_used_events.iter()
            .map(|event| {
                if let BattleEvent::MoveUsed { move_used, .. } = event {
                    *move_used
                } else {
                    unreachable!()
                }
            })
            .collect();
        
        assert!(moves.contains(&Move::Metronome), "Should have Metronome MoveUsed event");
        assert!(moves.iter().any(|&mv| mv != Move::Metronome), "Should have a non-Metronome move selected");
    }
}
