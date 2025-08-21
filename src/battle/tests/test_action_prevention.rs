#[cfg(test)]
mod tests {
    use crate::battle::action_stack::{ActionStack, BattleAction};
    use crate::battle::conditions::{PokemonCondition, PokemonConditionType};
    use crate::battle::engine::execute_battle_action;
    use crate::battle::state::{ActionFailureReason, BattleEvent, EventBus, TurnRng};
    use crate::battle::tests::common::{
        TestPokemonBuilder, create_test_battle, create_test_player,
    };
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
            BattleAction::AttackHit {
                attacker_index: 0,
                defender_index: 1,
                move_used: Move::Tackle,
                hit_number: 0,
            },
            &mut battle_state,
            &mut action_stack,
            &mut bus,
            &mut rng,
        );

        // Assert
        bus.print_debug_with_message("Events for test_sleep_prevents_action:");
        assert_eq!(bus.len(), 1);
        assert!(matches!(
            bus.events()[0],
            BattleEvent::ActionFailed {
                reason: ActionFailureReason::IsAsleep { .. }
            }
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
            BattleAction::AttackHit {
                attacker_index: 0,
                defender_index: 1,
                move_used: Move::Tackle,
                hit_number: 0,
            },
            &mut battle_state,
            &mut action_stack,
            &mut bus,
            &mut rng,
        );

        // Assert
        bus.print_debug_with_message(&format!("Events for test_paralysis_outcomes [{}]:", desc));
        if should_fail {
            assert!(matches!(
                bus.events()[0],
                BattleEvent::ActionFailed {
                    reason: ActionFailureReason::IsParalyzed { .. }
                }
            ));
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
        let mut battle_state =
            crate::battle::state::BattleState::new("test".to_string(), player1, player2);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![rng_val, 75, 90, 80, 85, 70]);
        let mut action_stack = ActionStack::new();

        // Act
        execute_battle_action(
            BattleAction::AttackHit {
                attacker_index: 0,
                defender_index: 1,
                move_used: Move::Ember,
                hit_number: 0,
            },
            &mut battle_state,
            &mut action_stack,
            &mut bus,
            &mut rng,
        );

        // If confused, a self-attack is pushed to the stack. We must execute it to see all events.
        if should_fail {
            if let Some(next_action) = action_stack.pop_front() {
                execute_battle_action(
                    next_action,
                    &mut battle_state,
                    &mut action_stack,
                    &mut bus,
                    &mut rng,
                );
            } else {
                panic!("Expected confusion to add self-attack action to stack");
            }
        }

        // Assert
        bus.print_debug_with_message(&format!("Events for test_confusion_outcomes [{}]:", desc));
        if should_fail {
            assert!(matches!(
                bus.events()[0],
                BattleEvent::ActionFailed {
                    reason: ActionFailureReason::IsConfused { .. }
                }
            ));
            assert!(bus.events().iter().any(|e| matches!(
                e,
                BattleEvent::MoveHit {
                    move_used: Move::HittingItself,
                    ..
                }
            )));
        } else {
            assert!(matches!(
                bus.events()[0],
                BattleEvent::MoveUsed {
                    move_used: Move::Ember,
                    ..
                }
            ));
            assert!(
                !bus.events()
                    .iter()
                    .any(|e| matches!(e, BattleEvent::ActionFailed { .. }))
            );
        }
    }

    #[test]
    fn test_confusion_expires_no_self_hit() {
        // Test that when confusion expires (turns_remaining = 0), there's no self-hit check
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Ember])
            .build();
        let defender = TestPokemonBuilder::new(Species::Charmander, 25).build();

        let mut player1 = create_test_player("p1", "Player 1", vec![attacker]);
        player1.add_condition(PokemonCondition::Confused { turns_remaining: 0 }); // Confusion should expire when trying to act
        let player2 = create_test_player("p2", "Player 2", vec![defender]);
        let mut battle_state =
            crate::battle::state::BattleState::new("test".to_string(), player1, player2);

        let mut bus = EventBus::new();
        let mut action_stack = ActionStack::new();
        let mut rng = TurnRng::new_for_test(vec![25, 50, 75, 90]); // Provide enough RNG values for the test

        // Act
        execute_battle_action(
            BattleAction::AttackHit {
                attacker_index: 0,
                defender_index: 1,
                move_used: Move::Ember,
                hit_number: 0,
            },
            &mut battle_state,
            &mut action_stack,
            &mut bus,
            &mut rng,
        );

        // Assert
        bus.print_debug_with_message("Events for test_confusion_expires_no_self_hit:");

        // Should see the move being used (confusion expired), not action failed
        assert!(bus.events().iter().any(|e| matches!(
            e,
            BattleEvent::MoveUsed {
                move_used: Move::Ember,
                ..
            }
        )));
        assert!(!bus.events().iter().any(|e| matches!(
            e,
            BattleEvent::ActionFailed {
                reason: ActionFailureReason::IsConfused { .. }
            }
        )));
        assert!(!bus.events().iter().any(|e| matches!(
            e,
            BattleEvent::MoveHit {
                move_used: Move::HittingItself,
                ..
            }
        )));

        // Confusion should be removed from the player
        assert!(!battle_state.players[0].has_condition_type(PokemonConditionType::Confused));

        // Should see a condition expired event
        assert!(bus.events().iter().any(|e| matches!(
            e,
            BattleEvent::ConditionExpired {
                condition: PokemonCondition::Confused { .. },
                ..
            }
        )));
    }

    #[test]
    fn test_confusion_mechanics_timing() {
        // Test the revised confusion mechanics:
        // 1. End of turn decrements but never goes below 0
        // 2. When Pokemon tries to act with 0 turns, confusion is removed without self-hit
        // 3. When Pokemon tries to act with >0 turns, confusion check occurs but no decrement

        let attacker = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Ember])
            .build();
        let defender = TestPokemonBuilder::new(Species::Charmander, 25).build();

        let mut player1 = create_test_player("p1", "Player 1", vec![attacker]);
        player1.add_condition(PokemonCondition::Confused { turns_remaining: 1 }); // Will become 0 after end-of-turn
        let player2 = create_test_player("p2", "Player 2", vec![defender]);
        let mut battle_state =
            crate::battle::state::BattleState::new("test".to_string(), player1, player2);

        // Set up actions for a full turn
        battle_state.action_queue[0] = Some(crate::player::PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(crate::player::PlayerAction::UseMove { move_index: 0 });

        // Execute a full turn resolution which includes end-of-turn processing
        let event_bus = crate::battle::engine::resolve_turn(
            &mut battle_state,
            TurnRng::new_for_test(vec![75, 50, 90, 85, 50, 90, 85]),
        ); // Provide extra RNG values
        event_bus.print_debug_with_message("Events for confusion expiration test (Turn 1):");
        // After end-of-turn processing, confusion should have decremented from 1 to 0
        let confusion = battle_state.players[0]
            .active_pokemon_conditions
            .get(&PokemonConditionType::Confused);
        assert!(matches!(
            confusion,
            Some(PokemonCondition::Confused { turns_remaining: 0 })
        ));

        // Clear the action queue and test what happens when Pokemon tries to act with 0 turns remaining
        battle_state.action_queue = [None, None];
        battle_state.action_queue[0] = Some(crate::player::PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(crate::player::PlayerAction::UseMove { move_index: 0 });

        let event_bus2 = crate::battle::engine::resolve_turn(
            &mut battle_state,
            TurnRng::new_for_test(vec![25, 50, 90, 85, 50, 90, 85]),
        ); // Even with low roll, no self-hit should occur

        event_bus2.print_debug_with_message("Events for confusion expiration test (Turn 2):");

        // Confusion should now be completely removed
        assert!(!battle_state.players[0].has_condition_type(PokemonConditionType::Confused));

        // Should see the move being used (confusion expired), not action failed
        assert!(event_bus2.events().iter().any(|e| matches!(
            e,
            BattleEvent::MoveUsed {
                move_used: Move::Ember,
                ..
            }
        )));
        assert!(!event_bus2.events().iter().any(|e| matches!(
            e,
            BattleEvent::ActionFailed {
                reason: ActionFailureReason::IsConfused { .. }
            }
        )));
        assert!(!event_bus2.events().iter().any(|e| matches!(
            e,
            BattleEvent::MoveHit {
                move_used: Move::HittingItself,
                ..
            }
        )));
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
        let mut battle_state =
            crate::battle::state::BattleState::new("test".to_string(), player1, player2);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75, 85, 60]);
        let mut action_stack = ActionStack::new();

        // Act
        execute_battle_action(
            BattleAction::AttackHit {
                attacker_index: 0,
                defender_index: 1,
                move_used: Move::Tackle,
                hit_number: 0,
            },
            &mut battle_state,
            &mut action_stack,
            &mut bus,
            &mut rng,
        );

        // Assert
        bus.print_debug_with_message("Events for test_exhausted_prevents_action:");
        assert_eq!(bus.len(), 1);
        assert!(matches!(
            bus.events()[0],
            BattleEvent::ActionFailed {
                reason: ActionFailureReason::IsExhausted { .. }
            }
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
        let mut battle_state =
            crate::battle::state::BattleState::new("test".to_string(), player1, player2);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75]);
        let mut action_stack = ActionStack::new();

        // Act
        execute_battle_action(
            BattleAction::AttackHit {
                attacker_index: 0,
                defender_index: 1,
                move_used: Move::Tackle,
                hit_number: 0,
            },
            &mut battle_state,
            &mut action_stack,
            &mut bus,
            &mut rng,
        );

        // Assert
        bus.print_debug_with_message("Events for test_multiple_conditions_priority:");
        assert_eq!(bus.len(), 1);
        assert!(matches!(
            bus.events()[0],
            BattleEvent::ActionFailed {
                reason: ActionFailureReason::IsAsleep { .. }
            }
        ));
    }

    #[rstest]
    #[case("prevents action with disabled move", Move::Tackle, true)]
    #[case("allows action with different move", Move::Ember, false)]
    fn test_disabled_move_outcomes(
        #[case] desc: &str,
        #[case] move_to_use: Move,
        #[case] should_fail: bool,
    ) {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Tackle, Move::Ember])
            .build();
        let defender = TestPokemonBuilder::new(Species::Charmander, 25).build();

        let mut player1 = create_test_player("p1", "Player 1", vec![attacker]);
        player1.add_condition(PokemonCondition::Disabled {
            pokemon_move: Move::Tackle,
            turns_remaining: 2,
        });
        let player2 = create_test_player("p2", "Player 2", vec![defender]);
        let mut battle_state =
            crate::battle::state::BattleState::new("test".to_string(), player1, player2);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75, 60, 80, 50, 40, 30, 20, 10]);
        let mut action_stack = ActionStack::new();

        // Act
        execute_battle_action(
            BattleAction::AttackHit {
                attacker_index: 0,
                defender_index: 1,
                move_used: move_to_use,
                hit_number: 0,
            },
            &mut battle_state,
            &mut action_stack,
            &mut bus,
            &mut rng,
        );

        // Assert
        bus.print_debug_with_message(&format!(
            "Events for test_disabled_move_outcomes [{}]:",
            desc
        ));
        if should_fail {
            assert_eq!(bus.len(), 1);
            assert!(matches!(
                bus.events()[0],
                BattleEvent::ActionFailed {
                    reason: ActionFailureReason::MoveFailedToExecute { .. }
                }
            ));
        } else {
            assert!(bus.len() >= 1);
            assert!(matches!(bus.events()[0], BattleEvent::MoveUsed { .. }));
            assert!(
                !bus.events()
                    .iter()
                    .any(|e| matches!(e, BattleEvent::ActionFailed { .. }))
            );
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
            BattleAction::AttackHit {
                attacker_index: 0,
                defender_index: 1,
                move_used: Move::Tackle,
                hit_number: 0,
            },
            &mut battle_state,
            &mut action_stack,
            &mut bus,
            &mut rng,
        );

        // Assert
        bus.print_debug_with_message("Events for test_no_preventing_conditions_allows_action:");
        let events = bus.events();
        assert!(events.len() >= 1);
        assert!(matches!(events[0], BattleEvent::MoveUsed { .. }));
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, BattleEvent::ActionFailed { .. }))
        );
    }
}
