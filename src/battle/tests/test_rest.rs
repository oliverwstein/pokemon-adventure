#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleEvent, BattleState, TurnRng};
    use crate::battle::engine::resolve_turn;
    use crate::moves::Move;
    use crate::player::{BattlePlayer, PlayerAction, PokemonCondition};
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
    fn test_rest_full_heal() {
        // Initialize move data
        
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Rest])], // Snorlax with Rest
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Splash])],
        );

        // Damage the Pokemon to low HP
        let attacker_pokemon = player1.active_pokemon_mut().unwrap();
        let max_hp = attacker_pokemon.max_hp();
        let damage_taken = max_hp / 2; // Take 50% damage
        attacker_pokemon.take_damage(damage_taken);
        let damaged_hp = attacker_pokemon.current_hp();
        
        assert_eq!(damaged_hp, max_hp - damage_taken, "Pokemon should be at 50% HP");

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Rest, Player 2 uses Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Rest
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        println!("Rest full heal test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        let final_hp = battle_state.players[0].active_pokemon().unwrap().current_hp();
        let final_status = battle_state.players[0].active_pokemon().unwrap().status;

        // Should be fully healed
        assert_eq!(final_hp, max_hp, "Pokemon should be at full HP after Rest");

        // Should be asleep for 2 turns
        assert!(matches!(final_status, Some(StatusCondition::Sleep(2))), "Pokemon should be asleep for 2 turns");

        // Should have healing event
        let heal_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::PokemonHealed { target: Species::Snorlax, amount, new_hp } if *amount == damage_taken && *new_hp == max_hp))
            .collect();
        assert!(!heal_events.is_empty(), "Should have PokemonHealed event");

        // Should have sleep status change event
        let sleep_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::PokemonStatusChanged { target: Species::Snorlax, new_status: Some(StatusCondition::Sleep(2)) }))
            .collect();
        assert!(!sleep_events.is_empty(), "Should have PokemonStatusChanged event for Sleep");
    }

    #[test]
    fn test_rest_no_heal_at_full_hp() {
        // Initialize move data
        
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Rest])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Splash])],
        );

        // Pokemon is already at full HP
        let max_hp = player1.active_pokemon().unwrap().max_hp();
        let initial_hp = player1.active_pokemon().unwrap().current_hp();
        assert_eq!(initial_hp, max_hp, "Pokemon should start at full HP");

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Rest, Player 2 uses Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Rest
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        println!("Rest no heal at full HP test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        let final_hp = battle_state.players[0].active_pokemon().unwrap().current_hp();
        let final_status = battle_state.players[0].active_pokemon().unwrap().status;

        // Should still be at full HP
        assert_eq!(final_hp, max_hp, "Pokemon should remain at full HP");

        // Should be asleep for 2 turns
        assert!(matches!(final_status, Some(StatusCondition::Sleep(2))), "Pokemon should be asleep for 2 turns");

        // Should NOT have healing event (no healing needed)
        let heal_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::PokemonHealed { .. }))
            .collect();
        assert!(heal_events.is_empty(), "Should not have PokemonHealed event when already at full HP");

        // Should still have sleep status change event
        let sleep_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::PokemonStatusChanged { target: Species::Snorlax, new_status: Some(StatusCondition::Sleep(2)) }))
            .collect();
        assert!(!sleep_events.is_empty(), "Should have PokemonStatusChanged event for Sleep");
    }

    #[test]
    fn test_rest_clears_all_active_conditions() {
        // Initialize move data
        
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Rest])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Splash])],
        );

        // Add some active conditions to the Pokemon
        player1.add_condition(PokemonCondition::Confused { turns_remaining: 2 });
        player1.add_condition(PokemonCondition::Substitute { hp: 25 });
        player1.add_condition(PokemonCondition::Teleported);

        // Verify conditions are present
        assert!(player1.has_condition(&PokemonCondition::Confused { turns_remaining: 2 }));
        assert!(player1.has_condition(&PokemonCondition::Substitute { hp: 25 }));
        assert!(player1.has_condition(&PokemonCondition::Teleported));

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Rest, Player 2 uses Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Rest
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        println!("Rest clears conditions test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // All active conditions should be cleared
        assert!(battle_state.players[0].active_pokemon_conditions.is_empty(), "All active conditions should be cleared");

        // Should have status removal events
        let status_removed_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::PokemonStatusRemoved { target: Species::Snorlax, .. }))
            .collect();
        assert_eq!(status_removed_events.len(), 3, "Should have 3 PokemonStatusRemoved events for the 3 conditions");

        // Should be asleep
        let final_status = battle_state.players[0].active_pokemon().unwrap().status;
        assert!(matches!(final_status, Some(StatusCondition::Sleep(2))), "Pokemon should be asleep for 2 turns");
    }

    #[test]
    fn test_rest_clears_existing_status_condition() {
        // Initialize move data
        
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Rest])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Splash])],
        );

        // Give Pokemon a different status condition (Burn)
        player1.active_pokemon_mut().unwrap().status = Some(StatusCondition::Burn);
        assert!(matches!(player1.active_pokemon().unwrap().status, Some(StatusCondition::Burn)));

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Rest, Player 2 uses Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Rest
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        println!("Rest clears existing status test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Should be asleep (not burned)
        let final_status = battle_state.players[0].active_pokemon().unwrap().status;
        assert!(matches!(final_status, Some(StatusCondition::Sleep(2))), "Pokemon should be asleep, not burned");

        // Should have status change event showing Sleep replacing Burn
        let status_change_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::PokemonStatusChanged { target: Species::Snorlax, new_status: Some(StatusCondition::Sleep(2)) }))
            .collect();
        assert!(!status_change_events.is_empty(), "Should have PokemonStatusChanged event for Sleep");
    }

    #[test]
    fn test_rest_with_damage_and_conditions_combined() {
        // Initialize move data
        
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Rest])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Splash])],
        );

        // Damage the Pokemon and add conditions
        let attacker_pokemon = player1.active_pokemon_mut().unwrap();
        let max_hp = attacker_pokemon.max_hp();
        let damage_taken = max_hp * 3 / 4; // Take 75% damage
        attacker_pokemon.take_damage(damage_taken);
        attacker_pokemon.status = Some(StatusCondition::Poison);

        player1.add_condition(PokemonCondition::Confused { turns_remaining: 3 });
        player1.add_condition(PokemonCondition::Enraged);

        let damaged_hp = player1.active_pokemon().unwrap().current_hp();
        assert_eq!(damaged_hp, max_hp - damage_taken, "Pokemon should be at 25% HP");
        assert!(matches!(player1.active_pokemon().unwrap().status, Some(StatusCondition::Poison)));

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Rest, Player 2 uses Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Rest
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        println!("Rest combined effects test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        let final_hp = battle_state.players[0].active_pokemon().unwrap().current_hp();
        let final_status = battle_state.players[0].active_pokemon().unwrap().status;

        // Should be fully healed
        assert_eq!(final_hp, max_hp, "Pokemon should be at full HP after Rest");

        // Should be asleep (not poisoned)
        assert!(matches!(final_status, Some(StatusCondition::Sleep(2))), "Pokemon should be asleep, not poisoned");

        // All active conditions should be cleared
        assert!(battle_state.players[0].active_pokemon_conditions.is_empty(), "All active conditions should be cleared");

        // Should have healing event
        let heal_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::PokemonHealed { target: Species::Snorlax, amount, new_hp } if *amount == damage_taken && *new_hp == max_hp))
            .collect();
        assert!(!heal_events.is_empty(), "Should have PokemonHealed event");

        // Should have status removal events for active conditions
        let status_removed_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::PokemonStatusRemoved { target: Species::Snorlax, .. }))
            .collect();
        assert_eq!(status_removed_events.len(), 2, "Should have 2 PokemonStatusRemoved events for the 2 active conditions");
    }

    #[test]
    fn test_rest_prevents_action_when_asleep() {
        // Test that Pokemon cannot use moves when asleep after Rest
        
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Rest, Move::Tackle])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Splash])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Turn 1: Player 1 uses Rest
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Rest
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng1 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let _ = resolve_turn(&mut battle_state, test_rng1);

        // Verify Pokemon is asleep
        let status_after_rest = battle_state.players[0].active_pokemon().unwrap().status;
        assert!(matches!(status_after_rest, Some(StatusCondition::Sleep(2))), "Pokemon should be asleep for 2 turns");

        // Turn 2: Try to use Tackle while asleep (should fail)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 1 }); // Tackle
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng2 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus2 = resolve_turn(&mut battle_state, test_rng2);

        // Should have ActionFailed event due to sleep
        let action_failed_events: Vec<_> = event_bus2.events().iter()
            .filter(|event| matches!(event, BattleEvent::ActionFailed { .. }))
            .collect();
        assert!(!action_failed_events.is_empty(), "Should have ActionFailed event when trying to move while asleep");

        // Should NOT have damage dealt to opponent
        let damage_events: Vec<_> = event_bus2.events().iter()
            .filter(|event| matches!(event, BattleEvent::DamageDealt { target: Species::Pikachu, .. }))
            .collect();
        assert!(damage_events.is_empty(), "Should not deal damage when asleep");
    }
}