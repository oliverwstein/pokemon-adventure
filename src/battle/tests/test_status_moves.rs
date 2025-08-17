#[cfg(test)]
mod tests {
    use crate::battle::action_stack::ActionStack;
    use crate::battle::state::{BattleEvent, BattleState, EventBus, TurnRng};
    use crate::battle::engine::{execute_attack_hit};
    use crate::moves::Move;
    use crate::player::{BattlePlayer, StatType};
    use crate::pokemon::PokemonInst;
    use crate::species::Species;

    fn create_test_pokemon(species: Species, moves: Vec<Move>) -> PokemonInst {
        PokemonInst::new_for_test(
            species,
            10,
            0,
            100, // Set current HP directly to max
            [15, 15, 15, 15, 15, 15],
            [0, 0, 0, 0, 0, 0],
            [100, 80, 70, 60, 60, 90], // Max HP same as current for simplicity
            [
                moves.get(0).map(|&m| crate::pokemon::MoveInstance::new(m)),
                moves.get(1).map(|&m| crate::pokemon::MoveInstance::new(m)),
                moves.get(2).map(|&m| crate::pokemon::MoveInstance::new(m)),
                moves.get(3).map(|&m| crate::pokemon::MoveInstance::new(m)),
            ],
            None,
        )
    }

    fn create_test_player(pokemon: PokemonInst) -> BattlePlayer {
        BattlePlayer::new(
            "test_player".to_string(),
            "TestPlayer".to_string(),
            vec![pokemon],
        )
    }

    fn create_test_battle_state() -> BattleState {
        let pokemon1 = create_test_pokemon(Species::Meowth, vec![Move::SwordsDance, Move::Tackle]);
        let pokemon2 = create_test_pokemon(Species::Pidgey, vec![Move::Tackle]);

        let player1 = create_test_player(pokemon1);
        let player2 = create_test_player(pokemon2);

        BattleState::new("test_battle".to_string(), player1, player2)
    }

    #[test]
    fn test_status_move_swords_dance() {
        let mut battle_state = create_test_battle_state();
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50, 60, 70]); // Good accuracy roll
        let mut action_stack = ActionStack::new();

        // Check initial attack stat stage
        let initial_attack_stage = battle_state.players[0].get_stat_stage(StatType::Attack);
        assert_eq!(initial_attack_stage, 0);

        // Execute Swords Dance (Status move that raises Attack by 2 stages)
        execute_attack_hit(
            0,
            1,
            Move::SwordsDance,
            0,
            &mut action_stack,
            &mut bus,
            &mut rng,
            &mut battle_state,
        );

        // Check that attack stage was increased
        let new_attack_stage = battle_state.players[0].get_stat_stage(StatType::Attack);
        assert_eq!(new_attack_stage, 2);

        // Verify the events generated
        let events = bus.events();

        // Should have MoveUsed, MoveHit, and StatStageChanged events
        assert!(events.iter().any(|e| matches!(
            e,
            BattleEvent::MoveUsed {
                move_used: Move::SwordsDance,
                ..
            }
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            BattleEvent::MoveHit {
                move_used: Move::SwordsDance,
                ..
            }
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            BattleEvent::StatStageChanged {
                stat: StatType::Attack,
                old_stage: 0,
                new_stage: 2,
                ..
            }
        )));

        println!("Swords Dance test events:");
        for event in events {
            println!("  {:?}", event);
        }
    }

    #[test]
    fn test_status_move_harden() {
        let pokemon1 = create_test_pokemon(Species::Metapod, vec![Move::Harden, Move::Tackle]);
        let pokemon2 = create_test_pokemon(Species::Pidgey, vec![Move::Tackle]);

        let player1 = create_test_player(pokemon1);
        let player2 = create_test_player(pokemon2);
        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50, 60, 70]); // Good accuracy roll
        let mut action_stack = ActionStack::new();

        // Check initial defense stat stage
        let initial_defense_stage = battle_state.players[0].get_stat_stage(StatType::Defense);
        assert_eq!(initial_defense_stage, 0);

        // Execute Harden (Status move that raises Defense by 1 stage)
        execute_attack_hit(
            0,
            1,
            Move::Harden,
            0,
            &mut action_stack,
            &mut bus,
            &mut rng,
            &mut battle_state,
        );

        // Check that defense stage was increased
        let new_defense_stage = battle_state.players[0].get_stat_stage(StatType::Defense);
        assert_eq!(new_defense_stage, 1);

        // Verify the events generated
        let events = bus.events();

        // Should have MoveUsed, MoveHit, and StatStageChanged events
        assert!(events.iter().any(|e| matches!(
            e,
            BattleEvent::MoveUsed {
                move_used: Move::Harden,
                ..
            }
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            BattleEvent::MoveHit {
                move_used: Move::Harden,
                ..
            }
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            BattleEvent::StatStageChanged {
                stat: StatType::Defense,
                old_stage: 0,
                new_stage: 1,
                ..
            }
        )));

        println!("Harden test events:");
        for event in events {
            println!("  {:?}", event);
        }
    }

    #[test]
    fn test_other_category_status_move_thunder_wave() {
        let pokemon1 = create_test_pokemon(Species::Pikachu, vec![Move::ThunderWave, Move::Tackle]);
        let pokemon2 = create_test_pokemon(Species::Pidgey, vec![Move::Tackle]);

        let player1 = create_test_player(pokemon1);
        let player2 = create_test_player(pokemon2);
        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50, 60, 70]); // Good accuracy and effect roll
        let mut action_stack = ActionStack::new();

        // Check that defender is not paralyzed initially
        let defender_status = &battle_state.players[1].team[0].as_ref().unwrap().status;
        assert!(defender_status.is_none());

        // Execute Thunder Wave (Other category move that paralyzes the target)
        execute_attack_hit(
            0,
            1,
            Move::ThunderWave,
            0,
            &mut action_stack,
            &mut bus,
            &mut rng,
            &mut battle_state,
        );

        // Check that defender is now paralyzed
        let defender_status = &battle_state.players[1].team[0].as_ref().unwrap().status;
        assert!(matches!(
            defender_status,
            Some(crate::pokemon::StatusCondition::Paralysis)
        ));

        // Verify the events generated
        let events = bus.events();

        // Should have MoveUsed, MoveHit, and PokemonStatusApplied events
        assert!(events.iter().any(|e| matches!(
            e,
            BattleEvent::MoveUsed {
                move_used: Move::ThunderWave,
                ..
            }
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            BattleEvent::MoveHit {
                move_used: Move::ThunderWave,
                ..
            }
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            BattleEvent::PokemonStatusApplied {
                status: crate::pokemon::StatusCondition::Paralysis,
                ..
            }
        )));

        println!("Thunder Wave test events:");
        for event in events {
            println!("  {:?}", event);
        }
    }
}
