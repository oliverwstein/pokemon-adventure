#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{BattleEvent, TurnRng};
    use crate::battle::tests::common::{create_test_battle, predictable_rng, TestPokemonBuilder};
    use crate::player::PlayerAction;
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use schema::Move;

    #[rstest]
    #[case("level 25", 25)]
    #[case("level 10", 10)]
    fn test_pay_day_increases_ante_based_on_level(#[case] desc: &str, #[case] attacker_level: u8) {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Alakazam, attacker_level)
            .with_moves(vec![Move::PayDay])
            .build();
        let defender = TestPokemonBuilder::new(Species::Machamp, 30)
            .with_moves(vec![Move::Splash])
            .build();
        let mut battle_state = create_test_battle(attacker, defender);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Pay Day
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message(&format!(
            "Events for test_pay_day_increases_ante_based_on_level [{}]:",
            desc
        ));

        let expected_ante = attacker_level as u32 * 2;
        assert_eq!(
            battle_state.players[1].get_ante(),
            expected_ante,
            "Player 2's ante should be 2x the attacker's level"
        );
        assert_eq!(
            battle_state.players[0].get_ante(),
            0,
            "Player 1's ante should remain unchanged"
        );

        // Find and verify the specific event
        let ante_event = event_bus.events().iter().find_map(|event| match event {
            BattleEvent::AnteIncreased {
                player_index,
                amount,
                new_total,
            } => Some((*player_index, *amount, *new_total)),
            _ => None,
        });

        assert!(
            ante_event.is_some(),
            "Should have emitted an AnteIncreased event"
        );
        let (player_index, amount, new_total) = ante_event.unwrap();
        assert_eq!(player_index, 1);
        assert_eq!(amount, expected_ante);
        assert_eq!(new_total, expected_ante);
    }

    #[test]
    fn test_pay_day_activates_at_100_percent_chance() {
        // Arrange: This test confirms that Pay Day's effect, which has a 100% activation
        // chance in its data file, will always trigger, even on the highest possible RNG roll.
        let attacker = TestPokemonBuilder::new(Species::Alakazam, 20)
            .with_moves(vec![Move::PayDay])
            .build();
        let defender = TestPokemonBuilder::new(Species::Machamp, 30)
            .with_moves(vec![Move::Splash])
            .build();
        let mut battle_state = create_test_battle(attacker, defender);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Pay Day
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        // Use an RNG roll of 100. For an effect with `chance: 100`, this should still succeed.
        let test_rng =
            TurnRng::new_for_test(vec![50, 100, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);

        // Act
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Assert
        event_bus
            .print_debug_with_message("Events for test_pay_day_activates_at_100_percent_chance:");
        let expected_ante = 20u32 * 2;
        assert_eq!(
            battle_state.players[1].get_ante(),
            expected_ante,
            "Pay Day with 100% chance should activate even with an RNG roll of 100"
        );
    }

    #[test]
    fn test_ante_accumulation() {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Alakazam, 15)
            .with_moves(vec![Move::PayDay])
            .build();
        let defender = TestPokemonBuilder::new(Species::Machamp, 25)
            .with_moves(vec![Move::Splash])
            .build();
        let mut battle_state = create_test_battle(attacker, defender);

        let expected_per_use = 15u32 * 2;

        // Act - Turn 1
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        let bus1 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 1
        bus1.print_debug_with_message("Events for test_ante_accumulation (Turn 1):");
        assert_eq!(battle_state.players[1].get_ante(), expected_per_use);

        // Act - Turn 2
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        let bus2 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 2
        bus2.print_debug_with_message("Events for test_ante_accumulation (Turn 2):");
        assert_eq!(
            battle_state.players[1].get_ante(),
            expected_per_use * 2,
            "Ante should accumulate across multiple turns"
        );
    }
}
