#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{BattleEvent, BattleState};
    use crate::battle::tests::common::{create_test_player, predictable_rng, TestPokemonBuilder};
    use crate::player::{PlayerAction, StatType, TeamCondition};
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use schema::Move;

    // --- Unit Test for BattlePlayer Logic ---

    #[test]
    fn test_mist_ticking_unit() {
        // Arrange
        let pokemon = TestPokemonBuilder::new(Species::Alakazam, 10)
            .with_moves(vec![Move::Splash])
            .build();
        let mut player = create_test_player("p1", "Player 1", vec![pokemon]);
        player.add_team_condition(TeamCondition::Mist, 2);

        // Act & Assert: First tick (2 -> 1)
        player.tick_team_conditions();
        assert_eq!(
            player.get_team_condition_turns(&TeamCondition::Mist),
            Some(1)
        );

        // Act & Assert: Second tick (1 -> 0, removed)
        player.tick_team_conditions();
        assert_eq!(player.get_team_condition_turns(&TeamCondition::Mist), None);
        assert!(!player.has_team_condition(&TeamCondition::Mist));
    }

    // --- Integration Tests for Battle Engine Logic ---

    #[rstest]
    #[case("single-stage reduction (Growl)", Move::Growl)]
    #[case("multi-stage reduction (Screech)", Move::Screech)]
    fn test_mist_blocks_stat_reduction(#[case] desc: &str, #[case] debuff_move: Move) {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Alakazam, 10)
            .with_moves(vec![debuff_move])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Machamp, 10)
            .with_moves(vec![Move::Splash])
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        let mut player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);
        player2.add_team_condition(TeamCondition::Mist, 3);

        let initial_stat_stages = player2.stat_stages.clone();
        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message(&format!(
            "Events for test_mist_blocks_stat_reduction [{}]:",
            desc
        ));

        assert_eq!(
            battle_state.players[1].stat_stages, initial_stat_stages,
            "Mist should prevent all stat reductions"
        );
        let blocked_event_found = event_bus
            .events()
            .iter()
            .any(|e| matches!(e, BattleEvent::StatChangeBlocked { .. }));
        let stage_changed_event_found = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::StatStageChanged {
                    target: Species::Machamp,
                    ..
                }
            )
        });

        assert!(
            blocked_event_found,
            "A StatChangeBlocked event should have been emitted"
        );
        assert!(
            !stage_changed_event_found,
            "No StatStageChanged event should be emitted for the target when blocked"
        );
    }

    #[test]
    fn test_mist_allows_self_targeting_stat_increases() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Alakazam, 10)
            .with_moves(vec![Move::SwordsDance])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Machamp, 10)
            .with_moves(vec![Move::Splash])
            .build();

        let mut player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        player1.add_team_condition(TeamCondition::Mist, 3); // User has Mist
        let player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);

        let mut battle_state = BattleState::new("test".to_string(), player1, player2);
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus
            .print_debug_with_message("Events for test_mist_allows_self_targeting_stat_increases:");

        assert_eq!(
            battle_state.players[0].get_stat_stage(StatType::Atk),
            2,
            "Attack should be raised by Swords Dance"
        );
        let blocked_event_found = event_bus
            .events()
            .iter()
            .any(|e| matches!(e, BattleEvent::StatChangeBlocked { .. }));
        assert!(
            !blocked_event_found,
            "Mist should not block self-targeting stat increases"
        );
    }

    #[test]
    fn test_mist_does_not_block_damaging_moves() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Alakazam, 10)
            .with_moves(vec![Move::Tackle])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Machamp, 10)
            .with_moves(vec![Move::Splash])
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        let mut player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);
        player2.add_team_condition(TeamCondition::Mist, 3);

        let mut battle_state = BattleState::new("test".to_string(), player1, player2);
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_mist_does_not_block_damaging_moves:");

        let blocked_event_found = event_bus
            .events()
            .iter()
            .any(|e| matches!(e, BattleEvent::StatChangeBlocked { .. }));
        let damage_event_found = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::DamageDealt {
                    target: Species::Machamp,
                    ..
                }
            )
        });

        assert!(
            !blocked_event_found,
            "Mist should not block moves that only deal damage"
        );
        assert!(
            damage_event_found,
            "A damage-only move should still hit a target protected by Mist"
        );
    }

    #[test]
    fn test_mist_expires_normally() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Alakazam, 10)
            .with_moves(vec![Move::Growl])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Machamp, 10)
            .with_moves(vec![Move::Splash])
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        let mut player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);
        player2.add_team_condition(TeamCondition::Mist, 1); // Only 1 turn remaining

        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        // Act - Turn 1 (Mist is active and should block Growl, then expire)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        let bus1 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 1
        bus1.print_debug_with_message("Events for test_mist_expires_normally (Turn 1):");
        assert!(
            !battle_state.players[1].has_team_condition(&TeamCondition::Mist),
            "Mist should have expired"
        );
        assert!(bus1.events().iter().any(|e| matches!(
            e,
            BattleEvent::TeamConditionExpired {
                condition: TeamCondition::Mist,
                ..
            }
        )));

        // Act - Turn 2 (Mist is gone, Growl should work)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        let bus2 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 2
        bus2.print_debug_with_message("Events for test_mist_expires_normally (Turn 2):");
        assert_eq!(
            battle_state.players[1].get_stat_stage(StatType::Atk),
            -1,
            "Growl should now lower Attack"
        );
        let blocked_event_found = bus2
            .events()
            .iter()
            .any(|e| matches!(e, BattleEvent::StatChangeBlocked { .. }));
        assert!(
            !blocked_event_found,
            "Stat reduction should not be blocked after Mist expires"
        );
    }
}
