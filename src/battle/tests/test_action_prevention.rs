#[cfg(test)]
mod tests {
    use crate::battle::action_stack::{ActionStack, BattleAction};
    use crate::battle::conditions::PokemonCondition;
    use crate::battle::engine::execute_battle_action;
    use crate::battle::state::{ActionFailureReason, BattleEvent, EventBus, TurnRng};
    use crate::battle::tests::common::{create_test_battle, create_test_player, TestPokemonBuilder};
    use crate::moves::Move;
    use crate::pokemon::StatusCondition;
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[test]
    fn test_sleep_prevents_action() {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Tackle])
            .with_status(StatusCondition::Sleep(2))
            .build();
        let defender = TestPokemonBuilder::new(Species::Charmander, 25).build();
        let mut battle_state = create_test_battle(attacker, defender);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![128]);
        let mut action_stack = ActionStack::new();

        // Act
        execute_battle_action(
            BattleAction::AttackHit { attacker_index: 0, defender_index: 1, move_used: Move::Tackle, hit_number: 0 },
            &mut battle_state, &mut action_stack, &mut bus, &mut rng,
        );

        // Assert
        bus.print_debug_with_message("Events for test_sleep_prevents_action:");
        assert_eq!(bus.len(), 1);
        assert!(matches!(
            bus.events()[0],
            BattleEvent::ActionFailed { reason: ActionFailureReason::IsAsleep }
        ));
    }

    #[rstest]
    #[case("prevents action (roll 24)", 24, true)]
    #[case("allows action (roll 25)", 25, false)]
    fn test_paralysis_outcomes(#[case] desc: &str, #[case] rng_val: u8, #[case] should_fail: bool) {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Tackle])
            .with_status(StatusCondition::Paralysis)
            .build();
        let defender = TestPokemonBuilder::new(Species::Charmander, 25).build();
        let mut battle_state = create_test_battle(attacker, defender);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![rng_val, 75, 60, 80, 90, 85]);
        let mut action_stack = ActionStack::new();

        // Act
        execute_battle_action(
            BattleAction::AttackHit { attacker_index: 0, defender_index: 1, move_used: Move::Tackle, hit_number: 0 },
            &mut battle_state, &mut action_stack, &mut bus, &mut rng,
        );

        // Assert
        bus.print_debug_with_message(&format!("Events for test_paralysis_outcomes [{}]:", desc));
        if should_fail {
            assert!(matches!(bus.events()[0], BattleEvent::ActionFailed { reason: ActionFailureReason::IsParalyzed }));
        } else {
            assert!(matches!(bus.events()[0], BattleEvent::MoveUsed { .. }));
        }
    }

    #[rstest]
    #[case("prevents action (roll 49)", 49, true)]
    #[case("allows action (roll 50)", 50, false)]
    fn test_confusion_outcomes(#[case] desc: &str, #[case] rng_val: u8, #[case] should_fail: bool) {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Ember])
            .build();
        let defender = TestPokemonBuilder::new(Species::Charmander, 25).build();

        let mut player1 = create_test_player("p1", "Player 1", vec![attacker]);
        player1.add_condition(PokemonCondition::Confused { turns_remaining: 2 });
        let player2 = create_test_player("p2", "Player 2", vec![defender]);
        let mut battle_state = crate::battle::state::BattleState::new("test".to_string(), player1, player2);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![rng_val, 75, 90, 80, 85, 70]);
        let mut action_stack = ActionStack::new();

        // Act
        execute_battle_action(
            BattleAction::AttackHit { attacker_index: 0, defender_index: 1, move_used: Move::Ember, hit_number: 0 },
            &mut battle_state, &mut action_stack, &mut bus, &mut rng,
        );

        // If confused, a self-attack is pushed to the stack. We must execute it to see all events.
        if should_fail {
            if let Some(next_action) = action_stack.pop_front() {
                execute_battle_action(next_action, &mut battle_state, &mut action_stack, &mut bus, &mut rng);
            } else {
                panic!("Expected confusion to add self-attack action to stack");
            }
        }

        // Assert
        bus.print_debug_with_message(&format!("Events for test_confusion_outcomes [{}]:", desc));
        if should_fail {
            assert!(matches!(bus.events()[0], BattleEvent::ActionFailed { reason: ActionFailureReason::IsConfused }));
            assert!(bus.events().iter().any(|e| matches!(e, BattleEvent::MoveHit { move_used: Move::HittingItself, .. })));
        } else {
            assert!(matches!(bus.events()[0], BattleEvent::MoveUsed { move_used: Move::Ember, .. }));
            assert!(!bus.events().iter().any(|e| matches!(e, BattleEvent::ActionFailed { .. })));
        }
    }

    #[test]
    fn test_exhausted_prevents_action() {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Tackle])
            .build();
        let defender = TestPokemonBuilder::new(Species::Charmander, 25).build();
        
        let mut player1 = create_test_player("p1", "Player 1", vec![attacker]);
        player1.add_condition(PokemonCondition::Exhausted { turns_remaining: 1 });
        let player2 = create_test_player("p2", "Player 2", vec![defender]);
        let mut battle_state = crate::battle::state::BattleState::new("test".to_string(), player1, player2);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75, 85, 60]);
        let mut action_stack = ActionStack::new();

        // Act
        execute_battle_action(
            BattleAction::AttackHit { attacker_index: 0, defender_index: 1, move_used: Move::Tackle, hit_number: 0 },
            &mut battle_state, &mut action_stack, &mut bus, &mut rng,
        );

        // Assert
        bus.print_debug_with_message("Events for test_exhausted_prevents_action:");
        assert_eq!(bus.len(), 1);
        assert!(matches!(
            bus.events()[0],
            BattleEvent::ActionFailed { reason: ActionFailureReason::IsExhausted }
        ));
    }

    #[test]
    fn test_multiple_conditions_priority() {
        // Arrange: Sleep condition should take priority over Flinched condition.
        let attacker = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Tackle])
            .with_status(StatusCondition::Sleep(2))
            .build();
        let defender = TestPokemonBuilder::new(Species::Charmander, 25).build();

        let mut player1 = create_test_player("p1", "Player 1", vec![attacker]);
        player1.add_condition(PokemonCondition::Flinched);
        let player2 = create_test_player("p2", "Player 2", vec![defender]);
        let mut battle_state = crate::battle::state::BattleState::new("test".to_string(), player1, player2);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75]);
        let mut action_stack = ActionStack::new();

        // Act
        execute_battle_action(
            BattleAction::AttackHit { attacker_index: 0, defender_index: 1, move_used: Move::Tackle, hit_number: 0 },
            &mut battle_state, &mut action_stack, &mut bus, &mut rng,
        );

        // Assert
        bus.print_debug_with_message("Events for test_multiple_conditions_priority:");
        assert_eq!(bus.len(), 1);
        assert!(matches!(
            bus.events()[0],
            BattleEvent::ActionFailed { reason: ActionFailureReason::IsAsleep }
        ));
    }

    #[rstest]
    #[case("prevents action with disabled move", Move::Tackle, true)]
    #[case("allows action with different move", Move::Ember, false)]
    fn test_disabled_move_outcomes(#[case] desc: &str, #[case] move_to_use: Move, #[case] should_fail: bool) {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Tackle, Move::Ember])
            .build();
        let defender = TestPokemonBuilder::new(Species::Charmander, 25).build();

        let mut player1 = create_test_player("p1", "Player 1", vec![attacker]);
        player1.add_condition(PokemonCondition::Disabled { pokemon_move: Move::Tackle, turns_remaining: 2 });
        let player2 = create_test_player("p2", "Player 2", vec![defender]);
        let mut battle_state = crate::battle::state::BattleState::new("test".to_string(), player1, player2);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75, 60, 80, 50, 40, 30, 20, 10]);
        let mut action_stack = ActionStack::new();

        // Act
        execute_battle_action(
            BattleAction::AttackHit { attacker_index: 0, defender_index: 1, move_used: move_to_use, hit_number: 0 },
            &mut battle_state, &mut action_stack, &mut bus, &mut rng,
        );

        // Assert
        bus.print_debug_with_message(&format!("Events for test_disabled_move_outcomes [{}]:", desc));
        if should_fail {
            assert_eq!(bus.len(), 1);
            assert!(matches!(bus.events()[0], BattleEvent::ActionFailed { reason: ActionFailureReason::MoveFailedToExecute }));
        } else {
            assert!(bus.len() >= 1);
            assert!(matches!(bus.events()[0], BattleEvent::MoveUsed { .. }));
            assert!(!bus.events().iter().any(|e| matches!(e, BattleEvent::ActionFailed { .. })));
        }
    }

    #[test]
    fn test_no_preventing_conditions_allows_action() {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Tackle])
            .build();
        let defender = TestPokemonBuilder::new(Species::Charmander, 25).build();
        let mut battle_state = create_test_battle(attacker, defender);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75, 60, 80, 50, 40, 30, 20, 10]);
        let mut action_stack = ActionStack::new();

        // Act
        execute_battle_action(
            BattleAction::AttackHit { attacker_index: 0, defender_index: 1, move_used: Move::Tackle, hit_number: 0 },
            &mut battle_state, &mut action_stack, &mut bus, &mut rng,
        );

        // Assert
        bus.print_debug_with_message("Events for test_no_preventing_conditions_allows_action:");
        let events = bus.events();
        assert!(events.len() >= 1);
        assert!(matches!(events[0], BattleEvent::MoveUsed { .. }));
        assert!(!events.iter().any(|e| matches!(e, BattleEvent::ActionFailed { .. })));
    }
}