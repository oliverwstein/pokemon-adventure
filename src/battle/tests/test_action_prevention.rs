#[cfg(test)]
mod tests {
    use crate::battle::action_stack::BattleAction;
    use crate::battle::conditions::PokemonCondition;
    use crate::battle::state::{ActionFailureReason, BattleEvent, BattleState, EventBus, TurnRng};
    use crate::battle::engine::{execute_battle_action};
    use crate::moves::Move;
    use crate::player::BattlePlayer;
    use crate::pokemon::{PokemonInst, StatusCondition};
    use crate::species::Species;

    fn create_test_battle_state(
        attacker_status: Option<StatusCondition>,
        attacker_conditions: Vec<PokemonCondition>,
    ) -> BattleState {
        let pikachu_data = crate::pokemon::get_species_data(Species::Pikachu).unwrap();
        let charmander_data = crate::pokemon::get_species_data(Species::Charmander).unwrap();

        let mut pikachu = PokemonInst::new(
            Species::Pikachu,
            &pikachu_data,
            25,
            None,
            Some(vec![Move::Tackle, Move::Ember]),
        );
        let charmander = PokemonInst::new(
            Species::Charmander,
            &charmander_data,
            25,
            None,
            Some(vec![Move::Tackle, Move::Ember]),
        );

        // Set attacker status
        pikachu.status = attacker_status;

        let mut player1 =
            BattlePlayer::new("p1".to_string(), "Player 1".to_string(), vec![pikachu]);
        let player2 = BattlePlayer::new("p2".to_string(), "Player 2".to_string(), vec![charmander]);

        // Add attacker conditions
        for condition in attacker_conditions {
            player1.add_condition(condition);
        }

        BattleState {
            battle_id: "test".to_string(),
            players: [player1, player2],
            turn_number: 1,
            game_state: crate::battle::state::GameState::TurnInProgress,
            action_queue: [None, None],
        }
    }

    #[test]
    fn test_sleep_prevents_action() {

        let mut battle_state = create_test_battle_state(Some(StatusCondition::Sleep(2)), vec![]);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![128]);
        let mut action_stack = crate::battle::action_stack::ActionStack::new();

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

        let events = bus.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            BattleEvent::ActionFailed {
                reason: ActionFailureReason::IsAsleep
            }
        ));
    }

    #[test]
    fn test_paralysis_sometimes_prevents_action() {
        let mut battle_state = create_test_battle_state(Some(StatusCondition::Paralysis), vec![]);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![24]); // 24 < 25, so paralyzed (25% chance)
        let mut action_stack = crate::battle::action_stack::ActionStack::new();

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

        let events = bus.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            BattleEvent::ActionFailed {
                reason: ActionFailureReason::IsParalyzed
            }
        ));
    }

    #[test]
    fn test_paralysis_sometimes_allows_action() {
        let mut battle_state = create_test_battle_state(Some(StatusCondition::Paralysis), vec![]);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![25, 75, 60, 80, 90, 85]); // 25 >= 25, so not paralyzed + extra values
        let mut action_stack = crate::battle::action_stack::ActionStack::new();

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

        let events = bus.events();
        // Should have MoveUsed, not ActionFailed
        assert!(events.len() >= 1);
        assert!(matches!(events[0], BattleEvent::MoveUsed { .. }));
    }

    #[test]
    fn test_confusion_sometimes_prevents_action() {
        let mut battle_state = create_test_battle_state(
            None,
            vec![PokemonCondition::Confused { turns_remaining: 2 }],
        );

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![49, 75, 90, 80, 85, 70, 65, 95, 88, 92]); // 49 < 50, so confused (50% chance) + many extra values
        let mut action_stack = crate::battle::action_stack::ActionStack::new();

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

        // When confused, the Pokemon should add a self-attack to the action stack
        // We need to process that action to see the self-damage
        if let Some(next_action) = action_stack.pop_front() {
            match next_action {
                crate::battle::action_stack::BattleAction::AttackHit {
                    attacker_index,
                    defender_index,
                    move_used,
                    hit_number,
                } => {
                    // This should be a self-attack (attacker_index == defender_index)
                    assert_eq!(attacker_index, defender_index);
                    assert_eq!(attacker_index, 0);
                    assert_eq!(move_used, Move::HittingItself);
                    execute_battle_action(
                        BattleAction::AttackHit {
                            attacker_index,
                            defender_index,
                            move_used,
                            hit_number,
                        },
                        &mut battle_state,
                        &mut action_stack,
                        &mut bus,
                        &mut rng,
                    );
                }
                _ => panic!("Expected AttackHit action from confusion"),
            }
        } else {
            panic!("Expected confusion to add self-attack action to stack");
        }

        let events = bus.events();
        // Should have both ActionFailed event AND self-damage events
        println!("Confusion events: {:?}", events);
        assert!(events.len() >= 3); // ActionFailed + MoveUsed + DamageDealt at minimum

        // First event should be ActionFailed due to confusion
        assert!(matches!(
            events[0],
            BattleEvent::ActionFailed {
                reason: ActionFailureReason::IsConfused
            }
        ));

        // Should also have events for self-damage (MoveHit, DamageDealt)
        assert!(events.iter().any(|e| matches!(
            e,
            BattleEvent::MoveHit {
                move_used: Move::HittingItself,
                ..
            }
        )));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, BattleEvent::DamageDealt { .. }))
        );
    }

    #[test]
    fn test_confusion_sometimes_allows_action() {
        let mut battle_state = create_test_battle_state(
            None,
            vec![PokemonCondition::Confused { turns_remaining: 2 }],
        );

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50, 75, 60, 80, 90, 85]); // 50 >= 50, so not confused this turn + extra values
        let mut action_stack = crate::battle::action_stack::ActionStack::new();

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

        let events = bus.events();
        // Should have MoveUsed, not ActionFailed
        assert!(events.len() >= 1);
        assert!(matches!(events[0], BattleEvent::MoveUsed { .. }));
    }

    #[test]
    fn test_exhausted_prevents_action() {
        let mut battle_state = create_test_battle_state(
            None,
            vec![PokemonCondition::Exhausted { turns_remaining: 1 }],
        );

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75, 85, 60]);
        let mut action_stack = crate::battle::action_stack::ActionStack::new();

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

        let events = bus.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            BattleEvent::ActionFailed {
                reason: ActionFailureReason::IsExhausted
            }
        ));
    }

    #[test]
    fn test_multiple_conditions_priority() {
        // Test that status conditions (sleep) take priority over active conditions (flinch)
        let mut battle_state = create_test_battle_state(
            Some(StatusCondition::Sleep(2)),
            vec![PokemonCondition::Flinched],
        );

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75]);
        let mut action_stack = crate::battle::action_stack::ActionStack::new();

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

        let events = bus.events();
        assert_eq!(events.len(), 1);

        // Should fail due to sleep (Sleep(2) -> Sleep(1)), not flinch
        assert!(matches!(
            events[0],
            BattleEvent::ActionFailed {
                reason: ActionFailureReason::IsAsleep
            }
        ));
    }

    #[test]
    fn test_disabled_move_prevents_action() {
        let mut battle_state = create_test_battle_state(
            None,
            vec![PokemonCondition::Disabled {
                pokemon_move: Move::Tackle,
                turns_remaining: 2,
            }],
        );

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75, 85, 60]);
        let mut action_stack = crate::battle::action_stack::ActionStack::new();

        execute_battle_action(
            BattleAction::AttackHit {
                attacker_index: 0,
                defender_index: 1,
                move_used: Move::Tackle, // This move is disabled
                hit_number: 0,
            },
            &mut battle_state,
            &mut action_stack,
            &mut bus,
            &mut rng,
        );

        let events = bus.events();
        println!("Disabled move events: {:?}", events);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            BattleEvent::ActionFailed {
                reason: ActionFailureReason::MoveFailedToExecute
            }
        ));
    }

    #[test]
    fn test_disabled_move_allows_different_move() {
        let mut battle_state = create_test_battle_state(
            None,
            vec![PokemonCondition::Disabled {
                pokemon_move: Move::Tackle, // Tackle is disabled
                turns_remaining: 2,
            }],
        );

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75, 60, 80, 50, 40, 30, 20, 10]);
        let mut action_stack = crate::battle::action_stack::ActionStack::new();

        execute_battle_action(
            BattleAction::AttackHit {
                attacker_index: 0,
                defender_index: 1,
                move_used: Move::Ember, // Different move should work
                hit_number: 0,
            },
            &mut battle_state,
            &mut action_stack,
            &mut bus,
            &mut rng,
        );

        let events = bus.events();
        println!("Non-disabled move events: {:?}", events);
        // Should proceed with normal attack flow
        assert!(events.len() >= 1);
        assert!(matches!(events[0], BattleEvent::MoveUsed { .. }));

        // Should not have any ActionFailed events
        for event in events {
            assert!(!matches!(event, BattleEvent::ActionFailed { .. }));
        }
    }

    #[test]
    fn test_no_preventing_conditions_allows_action() {
        let mut battle_state = create_test_battle_state(None, vec![]);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75, 60, 80, 50, 40, 30, 20, 10]); // Good rolls for accuracy, etc.
        let mut action_stack = crate::battle::action_stack::ActionStack::new();

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

        let events = bus.events();
        // Should proceed with normal attack flow
        assert!(events.len() >= 1);
        assert!(matches!(events[0], BattleEvent::MoveUsed { .. }));

        // Should not have any ActionFailed events
        for event in events {
            assert!(!matches!(event, BattleEvent::ActionFailed { .. }));
        }
    }
}
