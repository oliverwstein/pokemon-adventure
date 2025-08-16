#[cfg(test)]
mod tests {
    use crate::battle::state::{ActionFailureReason, BattleEvent, BattleState, TurnRng};
    use crate::battle::engine::{resolve_turn};
    use crate::moves::Move;
    use crate::player::{BattlePlayer, PlayerAction};
    use crate::pokemon::{MoveInstance, PokemonInst, StatusCondition};
    use crate::species::Species;

    fn create_test_pokemon(species: Species, moves: Vec<Move>) -> PokemonInst {
        let mut pokemon_moves = [const { None }; 4];
        for (i, mv) in moves.into_iter().enumerate() {
            if i < 4 {
                pokemon_moves[i] = Some(MoveInstance { move_: mv, pp: 20 });
            }
        }

        let mut pokemon = PokemonInst::new_for_test(
            species,
            10,
            0,
            0, // Will be set below
            [15; 6],
            [0; 6],
            [100, 80, 80, 80, 80, 80],
            pokemon_moves,
            None,
        );
        pokemon.set_hp_to_max();
        pokemon
    }

    #[test]
    fn test_nightmare_effect_works_on_sleeping_target() {
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Hypno, vec![Move::DreamEater])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Rest])],
        );

        // Make Player 2's Pokemon asleep
        player2.active_pokemon_mut().unwrap().status = Some(StatusCondition::Sleep(2));

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Dream Eater (has Nightmare effect), Player 2 uses Rest
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Dream Eater
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Rest

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Nightmare effect works on sleeping target test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Dream Eater should succeed because target is asleep
        let failed_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::ActionFailed {
                        reason: ActionFailureReason::MoveFailedToExecute
                    }
                )
            })
            .collect();
        assert!(
            failed_events.is_empty(),
            "Dream Eater should succeed when target is asleep"
        );

        // Should have move used event for Dream Eater
        let move_used_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::MoveUsed {
                        pokemon: Species::Hypno,
                        move_used: Move::DreamEater,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            !move_used_events.is_empty(),
            "Dream Eater should be used successfully"
        );
    }

    #[test]
    fn test_nightmare_effect_fails_on_awake_target() {
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Hypno, vec![Move::DreamEater])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Tackle])], // Awake Pokemon
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Dream Eater (has Nightmare effect), Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Dream Eater
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Nightmare effect fails on awake target test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Dream Eater should fail because target is not asleep
        let failed_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::ActionFailed {
                        reason: ActionFailureReason::MoveFailedToExecute
                    }
                )
            })
            .collect();
        assert!(
            !failed_events.is_empty(),
            "Dream Eater should fail when target is not asleep"
        );

        // Should NOT have move used event for Dream Eater
        let move_used_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::MoveUsed {
                        pokemon: Species::Hypno,
                        move_used: Move::DreamEater,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            move_used_events.is_empty(),
            "Dream Eater should not be used when it fails"
        );

        // Player 2 should not take any damage since Dream Eater failed
        let initial_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();
        assert_eq!(
            initial_hp, 100,
            "Target should not take damage when Dream Eater fails"
        );
    }

    #[test]
    fn test_nightmare_effect_with_other_statuses() {
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Hypno, vec![Move::DreamEater])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Tackle])],
        );

        // Make Player 2's Pokemon paralyzed (not asleep)
        player2.active_pokemon_mut().unwrap().status = Some(StatusCondition::Paralysis);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Dream Eater, Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Dream Eater
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Nightmare effect with other statuses test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Dream Eater should fail because target is paralyzed, not asleep
        let failed_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::ActionFailed {
                        reason: ActionFailureReason::MoveFailedToExecute
                    }
                )
            })
            .collect();
        assert!(
            !failed_events.is_empty(),
            "Dream Eater should fail when target is paralyzed (not asleep)"
        );
    }
}
