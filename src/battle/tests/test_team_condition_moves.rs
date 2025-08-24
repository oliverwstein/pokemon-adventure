#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::BattleEvent;
    use crate::battle::tests::common::{create_test_battle, predictable_rng, TestPokemonBuilder};
    use crate::moves::Move;
    use crate::player::{PlayerAction, TeamCondition};
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case("Reflect", Move::Reflect, TeamCondition::Reflect)]
    #[case("LightScreen", Move::LightScreen, TeamCondition::LightScreen)]
    #[case("Mist", Move::Mist, TeamCondition::Mist)]
    fn test_team_condition_moves_apply_correct_condition(
        #[case] desc: &str,
        #[case] move_to_use: Move,
        #[case] expected_condition: TeamCondition,
    ) {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Alakazam, 10)
            .with_moves(vec![move_to_use])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Machamp, 10)
            .with_moves(vec![Move::Splash])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message(&format!("Events for {} test:", desc));

        assert!(
            battle_state.players[0].has_team_condition(&expected_condition),
            "The correct team condition should be applied to the user"
        );
        assert!(
            !battle_state.players[1].has_team_condition(&expected_condition),
            "The team condition should not be applied to the opponent"
        );

        let event_found = event_bus.events().iter().any(|e| {
            matches!(e, BattleEvent::TeamConditionApplied { player_index: 0, condition } if *condition == expected_condition)
        });
        assert!(
            event_found,
            "The correct TeamConditionApplied event should have been emitted"
        );
    }

    #[test]
    fn test_team_conditions_work_immediately() {
        // Arrange: Player 1 uses Mist, which should immediately protect it from Player 2's Growl in the same turn.
        let p1_pokemon = TestPokemonBuilder::new(Species::Alakazam, 25)
            .with_moves(vec![Move::Mist])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Machamp, 10)
            .with_moves(vec![Move::Growl])
            .build(); // Slower
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Mist
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Growl

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_team_conditions_work_immediately:");

        let stat_blocked_event = event_bus
            .events()
            .iter()
            .any(|e| matches!(e, BattleEvent::StatChangeBlocked { .. }));
        let stat_changed_event = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::StatStageChanged {
                    target: Species::Alakazam,
                    ..
                }
            )
        });

        assert!(
            stat_blocked_event,
            "Mist should have blocked Growl's stat reduction"
        );
        assert!(
            !stat_changed_event,
            "The user's stats should not have changed"
        );
    }

    #[test]
    fn test_using_team_condition_move_refreshes_duration() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Alakazam, 10)
            .with_moves(vec![Move::Reflect])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Machamp, 10)
            .with_moves(vec![Move::Splash])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        // Act - Turn 1: Set up Reflect. Duration should be 5 turns, so it becomes 4 after the turn tick.
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        let bus1 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 1
        bus1.print_debug_with_message("Events for refreshing condition [Turn 1]:");
        assert_eq!(
            battle_state.players[0].get_team_condition_turns(&TeamCondition::Reflect),
            Some(4)
        );

        // Act - Turn 2: Use Reflect again. It should be reapplied for 5 turns, then ticked to 4.
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        let bus2 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 2
        bus2.print_debug_with_message("Events for refreshing condition [Turn 2]:");
        assert_eq!(
            battle_state.players[0].get_team_condition_turns(&TeamCondition::Reflect),
            Some(4),
            "Using Reflect again should refresh its duration"
        );
    }
}
