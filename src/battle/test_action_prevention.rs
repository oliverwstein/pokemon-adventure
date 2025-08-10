#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleState, EventBus, TurnRng, BattleEvent, ActionFailureReason};
    use crate::battle::turn_orchestrator::execute_attack_hit;
    use crate::pokemon::{PokemonInst, StatusCondition, initialize_species_data};
    use crate::move_data::initialize_move_data;
    use crate::player::{BattlePlayer, PokemonCondition};
    use crate::species::Species;
    use crate::moves::Move;
    use std::path::Path;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init_test_data() {
        INIT.call_once(|| {
            let data_path = Path::new("data");
            initialize_move_data(data_path).expect("Failed to initialize move data");
            initialize_species_data(data_path).expect("Failed to initialize species data");
        });
    }

    fn create_test_battle_state(
        attacker_status: Option<StatusCondition>,
        attacker_conditions: Vec<PokemonCondition>,
    ) -> BattleState {
        let pikachu_data = crate::pokemon::get_species_data(Species::Pikachu).unwrap();
        let charmander_data = crate::pokemon::get_species_data(Species::Charmander).unwrap();
        
        let mut pikachu = PokemonInst::new(Species::Pikachu, &pikachu_data, 25, None, None);
        let charmander = PokemonInst::new(Species::Charmander, &charmander_data, 25, None, None);
        
        // Set attacker status
        pikachu.status = attacker_status;
        
        let mut player1 = BattlePlayer::new("p1".to_string(), "Player 1".to_string(), vec![pikachu]);
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
        init_test_data();
        
        let mut battle_state = create_test_battle_state(
            Some(StatusCondition::Sleep(2)),
            vec![]
        );
        
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![128]);
        let mut action_stack = crate::battle::turn_orchestrator::ActionStack::new();
        
        execute_attack_hit(0, 1, Move::Tackle, 0, &mut action_stack, &mut bus, &mut rng, &mut battle_state);
        
        let events = bus.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], BattleEvent::ActionFailed { reason: ActionFailureReason::IsAsleep }));
    }

    #[test]
    fn test_paralysis_sometimes_prevents_action() {
        init_test_data();
        
        let mut battle_state = create_test_battle_state(
            Some(StatusCondition::Paralysis),
            vec![]
        );
        
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![24]); // 24 < 25, so paralyzed (25% chance)
        let mut action_stack = crate::battle::turn_orchestrator::ActionStack::new();
        
        execute_attack_hit(0, 1, Move::Tackle, 0, &mut action_stack, &mut bus, &mut rng, &mut battle_state);
        
        let events = bus.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], BattleEvent::ActionFailed { reason: ActionFailureReason::IsParalyzed }));
    }

    #[test]
    fn test_paralysis_sometimes_allows_action() {
        init_test_data();
        
        let mut battle_state = create_test_battle_state(
            Some(StatusCondition::Paralysis),
            vec![]
        );
        
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![25, 75, 60, 80, 90, 85]); // 25 >= 25, so not paralyzed + extra values
        let mut action_stack = crate::battle::turn_orchestrator::ActionStack::new();
        
        execute_attack_hit(0, 1, Move::Tackle, 0, &mut action_stack, &mut bus, &mut rng, &mut battle_state);
        
        let events = bus.events();
        // Should have MoveUsed, not ActionFailed
        assert!(events.len() >= 1);
        assert!(matches!(events[0], BattleEvent::MoveUsed { .. }));
    }

    #[test]
    fn test_confusion_sometimes_prevents_action() {
        init_test_data();
        
        let mut battle_state = create_test_battle_state(
            None,
            vec![PokemonCondition::Confused { turns_remaining: 2 }]
        );
        
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![49, 75, 90, 80]); // 49 < 50, so confused (50% chance) + extra values
        let mut action_stack = crate::battle::turn_orchestrator::ActionStack::new();
        
        execute_attack_hit(0, 1, Move::Tackle, 0, &mut action_stack, &mut bus, &mut rng, &mut battle_state);
        
        let events = bus.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], BattleEvent::ActionFailed { reason: ActionFailureReason::IsConfused }));
    }

    #[test] 
    fn test_confusion_sometimes_allows_action() {
        init_test_data();
        
        let mut battle_state = create_test_battle_state(
            None,
            vec![PokemonCondition::Confused { turns_remaining: 2 }]
        );
        
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50, 75, 60, 80, 90, 85]); // 50 >= 50, so not confused this turn + extra values
        let mut action_stack = crate::battle::turn_orchestrator::ActionStack::new();
        
        execute_attack_hit(0, 1, Move::Tackle, 0, &mut action_stack, &mut bus, &mut rng, &mut battle_state);
        
        let events = bus.events();
        // Should have MoveUsed, not ActionFailed
        assert!(events.len() >= 1);
        assert!(matches!(events[0], BattleEvent::MoveUsed { .. }));
    }

    #[test]
    fn test_exhausted_prevents_action() {
        init_test_data();
        
        let mut battle_state = create_test_battle_state(
            None,
            vec![PokemonCondition::Exhausted { turns_remaining: 1 }]
        );
        
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75, 85, 60]);
        let mut action_stack = crate::battle::turn_orchestrator::ActionStack::new();
        
        execute_attack_hit(0, 1, Move::Tackle, 0, &mut action_stack, &mut bus, &mut rng, &mut battle_state);
        
        let events = bus.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], BattleEvent::ActionFailed { reason: ActionFailureReason::IsExhausted }));
    }

    #[test]
    fn test_multiple_conditions_priority() {
        init_test_data();
        
        // Test that status conditions (sleep) take priority over active conditions (flinch)
        let mut battle_state = create_test_battle_state(
            Some(StatusCondition::Sleep(1)),
            vec![PokemonCondition::Flinched]
        );
        
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75]);
        let mut action_stack = crate::battle::turn_orchestrator::ActionStack::new();
        
        execute_attack_hit(0, 1, Move::Tackle, 0, &mut action_stack, &mut bus, &mut rng, &mut battle_state);
        
        let events = bus.events();
        assert_eq!(events.len(), 1);
        // Should fail due to sleep, not flinch
        assert!(matches!(events[0], BattleEvent::ActionFailed { reason: ActionFailureReason::IsAsleep }));
    }

    #[test]
    fn test_no_preventing_conditions_allows_action() {
        init_test_data();
        
        let mut battle_state = create_test_battle_state(None, vec![]);
        
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![75, 60, 80]); // Good rolls for accuracy, etc.
        let mut action_stack = crate::battle::turn_orchestrator::ActionStack::new();
        
        execute_attack_hit(0, 1, Move::Tackle, 0, &mut action_stack, &mut bus, &mut rng, &mut battle_state);
        
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