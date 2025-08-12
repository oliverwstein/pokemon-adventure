#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleEvent, BattleState, TurnRng};
    use crate::battle::turn_orchestrator::resolve_turn;
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
            10,0,
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
    fn test_cure_status_user_paralysis() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Agility])], // Agility has CureStatus(User, Paralysis)
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Tackle])],
        );

        // Make Player 1's Pokemon paralyzed
        player1.active_pokemon_mut().unwrap().status = Some(StatusCondition::Paralysis);
        
        // Verify initial status
        assert_eq!(player1.active_pokemon().unwrap().status, Some(StatusCondition::Paralysis));

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Agility (cures own paralysis), Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Agility
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("CureStatus User Paralysis test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Player 1's Pokemon should no longer be paralyzed
        assert_eq!(battle_state.players[0].active_pokemon().unwrap().status, None);

        // Should have PokemonStatusRemoved event
        let status_removed_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::PokemonStatusRemoved { 
                target: Species::Alakazam, 
                status: StatusCondition::Paralysis 
            }))
            .collect();
        assert!(!status_removed_events.is_empty(), "Should have PokemonStatusRemoved event for Paralysis");

        // Should also have used Agility
        let agility_used_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::MoveUsed { pokemon: Species::Alakazam, move_used: Move::Agility, .. }))
            .collect();
        assert!(!agility_used_events.is_empty(), "Agility should be used");
    }

    #[test]
    fn test_cure_status_target_sleep() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Meowth, vec![Move::Screech])], // Screech has CureStatus(Target, Sleep)
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Tackle])],
        );

        // Make Player 2's Pokemon asleep
        player2.active_pokemon_mut().unwrap().status = Some(StatusCondition::Sleep(2));
        
        // Verify initial status
        assert_eq!(player2.active_pokemon().unwrap().status, Some(StatusCondition::Sleep(2)));

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Screech (cures target's sleep), Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Screech
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("CureStatus Target Sleep test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Player 2's Pokemon should no longer be asleep
        assert_eq!(battle_state.players[1].active_pokemon().unwrap().status, None);

        // Should have PokemonStatusRemoved event
        let status_removed_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::PokemonStatusRemoved { 
                target: Species::Snorlax, 
                status: StatusCondition::Sleep(_) 
            }))
            .collect();
        assert!(!status_removed_events.is_empty(), "Should have PokemonStatusRemoved event for Sleep");
    }

    #[test]
    fn test_cure_status_no_matching_condition() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Agility])], // Agility cures Paralysis
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Tackle])],
        );

        // Make Player 1's Pokemon burned (not paralyzed, so Agility shouldn't cure it)
        player1.active_pokemon_mut().unwrap().status = Some(StatusCondition::Burn);
        
        // Verify initial status
        assert_eq!(player1.active_pokemon().unwrap().status, Some(StatusCondition::Burn));

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Agility, Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Agility
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("CureStatus no matching condition test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Player 1's Pokemon should still be burned (Agility only cures Paralysis)
        assert_eq!(battle_state.players[0].active_pokemon().unwrap().status, Some(StatusCondition::Burn));

        // Should NOT have PokemonStatusRemoved event since no matching status was cured
        let status_removed_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::PokemonStatusRemoved { .. }))
            .collect();
        assert!(status_removed_events.is_empty(), "Should not have PokemonStatusRemoved event when no matching status");

        // Should still have used Agility though (and gotten speed boost)
        let agility_used_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::MoveUsed { pokemon: Species::Alakazam, move_used: Move::Agility, .. }))
            .collect();
        assert!(!agility_used_events.is_empty(), "Agility should be used");
    }

    #[test]
    fn test_cure_status_no_status_condition() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Agility])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Tackle])],
        );

        // Player 1's Pokemon has no status condition
        assert_eq!(player1.active_pokemon().unwrap().status, None);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Agility, Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Agility
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("CureStatus no status condition test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Player 1's Pokemon should still have no status
        assert_eq!(battle_state.players[0].active_pokemon().unwrap().status, None);

        // Should NOT have PokemonStatusRemoved event since there was no status to cure
        let status_removed_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::PokemonStatusRemoved { .. }))
            .collect();
        assert!(status_removed_events.is_empty(), "Should not have PokemonStatusRemoved event when no status exists");

        // Should still have used Agility and gotten speed boost
        let agility_used_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::MoveUsed { pokemon: Species::Alakazam, move_used: Move::Agility, .. }))
            .collect();
        assert!(!agility_used_events.is_empty(), "Agility should be used");
    }

    #[test]
    fn test_cure_status_poison() {
        // Initialize move data  
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        // Create a custom test to verify poison curing
        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Agility])], // We'll pretend this cures poison for testing
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Tackle])],
        );

        // Make Player 1's Pokemon poisoned
        player1.active_pokemon_mut().unwrap().status = Some(StatusCondition::Poison(1));
        
        // Verify initial status
        assert_eq!(player1.active_pokemon().unwrap().status, Some(StatusCondition::Poison(1)));

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // For this test, we'd need a move that cures poison, but Agility only cures paralysis
        // This test demonstrates the matching logic works for different status types
        // The Pokemon should remain poisoned since Agility doesn't cure poison

        // Player 1 uses Agility, Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Agility
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("CureStatus poison (should not cure) test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Player 1's Pokemon should still be poisoned (Agility doesn't cure poison)
        // The poison will have ticked during end-of-turn, so it will be Poison(2) now
        assert!(matches!(battle_state.players[0].active_pokemon().unwrap().status, Some(StatusCondition::Poison(_))));

        // Should NOT have status removed events
        let status_removed_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::PokemonStatusRemoved { .. }))
            .collect();
        assert!(status_removed_events.is_empty(), "Should not cure poison with a move that only cures paralysis");
    }
}