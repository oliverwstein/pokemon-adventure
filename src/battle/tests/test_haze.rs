#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleEvent, BattleState, TurnRng};
    use crate::battle::turn_orchestrator::resolve_turn;
    use crate::moves::Move;
    use crate::player::{BattlePlayer, PlayerAction, StatType};
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
    fn test_haze_clears_all_stat_changes_both_players() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Koffing, vec![Move::Haze])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Tackle])],
        );

        // Set up stat stage modifications for both players
        player1.set_stat_stage(StatType::Attack, 2);
        player1.set_stat_stage(StatType::Defense, -1);
        player1.set_stat_stage(StatType::Speed, 3);

        player2.set_stat_stage(StatType::Attack, -2);
        player2.set_stat_stage(StatType::SpecialAttack, 1);
        player2.set_stat_stage(StatType::Accuracy, -3);

        // Verify initial stat stages
        assert_eq!(player1.get_stat_stage(StatType::Attack), 2);
        assert_eq!(player1.get_stat_stage(StatType::Defense), -1);
        assert_eq!(player1.get_stat_stage(StatType::Speed), 3);
        assert_eq!(player2.get_stat_stage(StatType::Attack), -2);
        assert_eq!(player2.get_stat_stage(StatType::SpecialAttack), 1);
        assert_eq!(player2.get_stat_stage(StatType::Accuracy), -3);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Haze, Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Haze
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Haze effect clears all stat changes test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Verify all stat stages are now 0 for both players
        assert_eq!(battle_state.players[0].get_stat_stage(StatType::Attack), 0);
        assert_eq!(battle_state.players[0].get_stat_stage(StatType::Defense), 0);
        assert_eq!(battle_state.players[0].get_stat_stage(StatType::Speed), 0);
        assert_eq!(battle_state.players[1].get_stat_stage(StatType::Attack), 0);
        assert_eq!(
            battle_state.players[1].get_stat_stage(StatType::SpecialAttack),
            0
        );
        assert_eq!(
            battle_state.players[1].get_stat_stage(StatType::Accuracy),
            0
        );

        // Should have StatStageChanged events for each cleared stat
        let stat_change_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| matches!(event, BattleEvent::StatStageChanged { new_stage: 0, .. }))
            .collect();

        // Should have 6 events (3 for player 1 + 3 for player 2)
        assert_eq!(
            stat_change_events.len(),
            6,
            "Should have 6 StatStageChanged events clearing stats to 0"
        );

        // Verify specific stat stage change events
        let player1_attack_reset = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::StatStageChanged {
                    target: Species::Koffing,
                    stat: StatType::Attack,
                    old_stage: 2,
                    new_stage: 0
                }
            )
        });
        assert!(
            player1_attack_reset,
            "Player 1 attack should be reset from +2 to 0"
        );

        let player2_accuracy_reset = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::StatStageChanged {
                    target: Species::Snorlax,
                    stat: StatType::Accuracy,
                    old_stage: -3,
                    new_stage: 0
                }
            )
        });
        assert!(
            player2_accuracy_reset,
            "Player 2 accuracy should be reset from -3 to 0"
        );
    }

    #[test]
    fn test_haze_no_effect_when_no_stat_changes() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Koffing, vec![Move::Haze])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Tackle])],
        );

        // Both players have no stat changes (all stats at stage 0)
        assert_eq!(player1.get_stat_stage(StatType::Attack), 0);
        assert_eq!(player2.get_stat_stage(StatType::Attack), 0);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Haze, Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Haze
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Haze with no stat changes test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Should have used Haze
        let haze_used_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::MoveUsed {
                        pokemon: Species::Koffing,
                        move_used: Move::Haze,
                        ..
                    }
                )
            })
            .collect();
        assert!(!haze_used_events.is_empty(), "Haze should be used");

        // Should NOT have any StatStageChanged events since no stats were changed
        let stat_change_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| matches!(event, BattleEvent::StatStageChanged { .. }))
            .collect();
        assert!(
            stat_change_events.is_empty(),
            "Should not have StatStageChanged events when no stats need clearing"
        );
    }

    #[test]
    fn test_haze_chance_based_effect() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Koffing, vec![Move::Haze])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Tackle])],
        );

        // Set up stat stage modification for testing
        player1.set_stat_stage(StatType::Attack, 1);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Haze with a very low chance (should fail)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Haze
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        // Use RNG that will cause Haze to fail (roll 99, but Haze has 100% chance so it will succeed)
        // Let's test that it still works with high rolls
        let test_rng = TurnRng::new_for_test(vec![100, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Haze chance-based test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // With normal Haze (100% chance), should still clear stats even with high roll
        assert_eq!(battle_state.players[0].get_stat_stage(StatType::Attack), 0);

        // Should have StatStageChanged event
        let stat_change_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| matches!(event, BattleEvent::StatStageChanged { new_stage: 0, .. }))
            .collect();
        assert!(
            !stat_change_events.is_empty(),
            "Should have cleared the stat change"
        );
    }
}
