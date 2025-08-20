#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{BattleEvent, BattleState};
    use crate::battle::tests::common::{
        TestPokemonBuilder, create_test_battle, create_test_player, predictable_rng,
    };
    use crate::moves::Move;
    use crate::player::{PlayerAction, StatType};
    use crate::species::Species;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_haze_clears_all_stat_changes_both_players() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Koffing, 10)
            .with_moves(vec![Move::Haze])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Snorlax, 10)
            .with_moves(vec![Move::Tackle])
            .build();

        // Set initial stat stages for both players
        let mut player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        player1.set_stat_stage(StatType::Attack, 2);
        player1.set_stat_stage(StatType::Defense, -1);
        player1.set_stat_stage(StatType::Speed, 3);

        let mut player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);
        player2.set_stat_stage(StatType::Attack, -2);
        player2.set_stat_stage(StatType::SpecialAttack, 1);
        player2.set_stat_stage(StatType::Accuracy, -3);

        let mut battle_state = BattleState::new("test".to_string(), player1, player2);
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Haze
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus
            .print_debug_with_message("Events for test_haze_clears_all_stat_changes_both_players:");

        // Both players' stat stages should be empty (reset to 0)
        assert!(
            battle_state.players[0].stat_stages.is_empty(),
            "Player 1's stat changes should be cleared"
        );
        assert!(
            battle_state.players[1].stat_stages.is_empty(),
            "Player 2's stat changes should be cleared"
        );

        // Verify that events were generated for each stat that was reset
        let stat_reset_events = event_bus
            .events()
            .iter()
            .filter(
                |e| matches!(e, BattleEvent::StatStageChanged { new_stage, .. } if *new_stage == 0),
            )
            .count();
        assert_eq!(
            stat_reset_events, 6,
            "Should be exactly 6 stat reset events"
        );
    }

    #[test]
    fn test_haze_no_effect_when_no_stat_changes() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Koffing, 10)
            .with_moves(vec![Move::Haze])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Snorlax, 10)
            .with_moves(vec![Move::Tackle])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Haze
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_haze_no_effect_when_no_stat_changes:");

        let haze_used = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::MoveUsed {
                    move_used: Move::Haze,
                    ..
                }
            )
        });
        assert!(
            haze_used,
            "Haze should still be used even if there are no stat changes"
        );

        let stat_change_events = event_bus
            .events()
            .iter()
            .any(|e| matches!(e, BattleEvent::StatStageChanged { .. }));
        assert!(
            !stat_change_events,
            "Should not be any StatStageChanged events when no stats were modified"
        );
    }

    #[test]
    fn test_haze_activates_at_100_percent_chance() {
        // Arrange: Verify that Haze, with a 100% activation chance, works even on the highest RNG roll.
        let p1_pokemon = TestPokemonBuilder::new(Species::Koffing, 10)
            .with_moves(vec![Move::Haze])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Snorlax, 10)
            .with_moves(vec![Move::Tackle])
            .build();

        let mut player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        player1.set_stat_stage(StatType::Attack, 1);
        let player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);

        let mut battle_state = BattleState::new("test".to_string(), player1, player2);
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        // The first RNG value (100) is for Haze's effect activation check. It should pass.
        let event_bus = resolve_turn(
            &mut battle_state,
            crate::battle::state::TurnRng::new_for_test(vec![100; 20]),
        );

        // Assert
        event_bus.print_debug_with_message("Events for test_haze_activates_at_100_percent_chance:");
        assert!(
            battle_state.players[0].stat_stages.is_empty(),
            "Player 1's stat changes should be cleared"
        );
        let stat_reset_event_found = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::StatStageChanged {
                    target: Species::Koffing,
                    stat: StatType::Attack,
                    old_stage: 1,
                    new_stage: 0
                }
            )
        });
        assert!(
            stat_reset_event_found,
            "A StatStageChanged event should confirm the stat reset"
        );
    }
}
