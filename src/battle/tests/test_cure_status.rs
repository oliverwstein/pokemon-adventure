#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::BattleEvent;
    use crate::battle::tests::common::{create_test_battle, predictable_rng, TestPokemonBuilder};
    use crate::player::PlayerAction;
    use crate::pokemon::StatusCondition;
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use schema::Move;

    #[rstest]
    #[case(
        "user's Agility cures user's Paralysis",
        Move::Agility,
        Some(StatusCondition::Paralysis),
        0, // Status is on Player 1 (user)
        true // Expect the status to be cured
    )]
    #[case(
        "user's Screech cures target's Sleep",
        Move::Screech,
        Some(StatusCondition::Sleep(2)),
        1, // Status is on Player 2 (target)
        true // Expect the status to be cured
    )]
    #[case(
        "user's Agility does not cure user's Burn",
        Move::Agility,
        Some(StatusCondition::Burn),
        0, // Status is on Player 1 (user)
        false // Expect no cure
    )]
    #[case(
        "user's Agility does nothing if user has no status",
        Move::Agility,
        None, // No initial status
        0,
        false // Expect no cure
    )]
    #[case(
        "user's Agility does not cure user's Poison",
        Move::Agility,
        Some(StatusCondition::Poison(1)),
        0, // Status is on Player 1 (user)
        false // Expect no cure
    )]
    fn test_cure_status_outcomes(
        #[case] desc: &str,
        #[case] user_move: Move,
        #[case] initial_status: Option<StatusCondition>,
        #[case] status_target_idx: usize,
        #[case] expect_cure: bool,
    ) {
        // Arrange
        let mut p1_builder =
            TestPokemonBuilder::new(Species::Alakazam, 10).with_moves(vec![user_move]);
        let mut p2_builder =
            TestPokemonBuilder::new(Species::Snorlax, 10).with_moves(vec![Move::Growl]);

        if let Some(status) = initial_status {
            if status_target_idx == 0 {
                p1_builder = p1_builder.with_status(status);
            } else {
                p2_builder = p2_builder.with_status(status);
            }
        }

        let mut battle_state = create_test_battle(p1_builder.build(), p2_builder.build());

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus
            .print_debug_with_message(&format!("Events for test_cure_status_outcomes [{}]:", desc));

        let final_status = battle_state.players[status_target_idx]
            .active_pokemon()
            .unwrap()
            .status;
        let status_cured = event_bus
            .events()
            .iter()
            .any(|e| matches!(e, BattleEvent::PokemonStatusRemoved { .. }));

        if expect_cure {
            assert_eq!(final_status, None, "Status should have been cured to None");
            assert!(
                status_cured,
                "A PokemonStatusRemoved event should have been emitted"
            );
        } else {
            // If we didn't expect a cure, the status should still be present (or None if it started as None).
            // Note: End-of-turn effects might change the status, e.g., Poison(1) -> Poison(2).
            // We check `is_some()` or `is_none()` which is more robust.
            assert_eq!(
                final_status.is_some(),
                initial_status.is_some(),
                "Status presence should not have changed unexpectedly"
            );
            assert!(
                !status_cured,
                "A PokemonStatusRemoved event should NOT have been emitted"
            );
        }
    }
}
