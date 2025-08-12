#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleState, TurnRng};
    use crate::battle::turn_orchestrator::{execute_end_turn_phase, resolve_turn};
    use crate::moves::Move;
    use crate::player::{BattlePlayer, PlayerAction, TeamCondition};
    use crate::pokemon::{MoveInstance, PokemonInst};
    use crate::species::Species;
    use crate::battle::state::EventBus;

    fn create_test_pokemon(species: Species, moves: Vec<Move>) -> PokemonInst {
        let mut pokemon_moves = [const { None }; 4];
        for (i, mv) in moves.into_iter().enumerate() {
            if i < 4 {
                pokemon_moves[i] = Some(MoveInstance { move_: mv, pp: 20 });
            }
        }

        let mut pokemon = PokemonInst::new_for_test(
            species,
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
    fn test_reflect_expires_after_turns() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Tackle])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Splash])],
        );

        // Add Reflect with 3 turns
        player2.add_team_condition(TeamCondition::Reflect, 3);
        assert_eq!(player2.get_team_condition_turns(&TeamCondition::Reflect), Some(3));
        assert!(player2.has_team_condition(&TeamCondition::Reflect));

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Turn 1: Should have Reflect (3 -> 2 turns)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let _ = resolve_turn(&mut battle_state, test_rng);

        assert_eq!(battle_state.players[1].get_team_condition_turns(&TeamCondition::Reflect), Some(2));
        assert!(battle_state.players[1].has_team_condition(&TeamCondition::Reflect));

        // Turn 2: Should still have Reflect (2 -> 1 turns)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let _ = resolve_turn(&mut battle_state, test_rng);

        assert_eq!(battle_state.players[1].get_team_condition_turns(&TeamCondition::Reflect), Some(1));
        assert!(battle_state.players[1].has_team_condition(&TeamCondition::Reflect));

        // Turn 3: Should expire Reflect (1 -> 0, removed)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let _ = resolve_turn(&mut battle_state, test_rng);

        assert_eq!(battle_state.players[1].get_team_condition_turns(&TeamCondition::Reflect), None);
        assert!(!battle_state.players[1].has_team_condition(&TeamCondition::Reflect));
    }

    #[test]
    fn test_light_screen_expires_after_turns() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Confusion])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])],
        );

        // Add Light Screen with 2 turns
        player2.add_team_condition(TeamCondition::LightScreen, 2);
        assert_eq!(player2.get_team_condition_turns(&TeamCondition::LightScreen), Some(2));
        assert!(player2.has_team_condition(&TeamCondition::LightScreen));

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Turn 1: Should have Light Screen (2 -> 1 turns)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let _ = resolve_turn(&mut battle_state, test_rng);

        assert_eq!(battle_state.players[1].get_team_condition_turns(&TeamCondition::LightScreen), Some(1));
        assert!(battle_state.players[1].has_team_condition(&TeamCondition::LightScreen));

        // Turn 2: Should expire Light Screen (1 -> 0, removed)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let _ = resolve_turn(&mut battle_state, test_rng);

        assert_eq!(battle_state.players[1].get_team_condition_turns(&TeamCondition::LightScreen), None);
        assert!(!battle_state.players[1].has_team_condition(&TeamCondition::LightScreen));
    }

    #[test]
    fn test_multiple_team_conditions_expire_independently() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Confusion])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])],
        );

        // Add both Reflect (3 turns) and Light Screen (1 turn)
        player2.add_team_condition(TeamCondition::Reflect, 3);
        player2.add_team_condition(TeamCondition::LightScreen, 1);
        
        assert!(player2.has_team_condition(&TeamCondition::Reflect));
        assert!(player2.has_team_condition(&TeamCondition::LightScreen));

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Turn 1: Light Screen should expire (1 -> 0), Reflect should remain (3 -> 2)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let _ = resolve_turn(&mut battle_state, test_rng);

        assert_eq!(battle_state.players[1].get_team_condition_turns(&TeamCondition::Reflect), Some(2));
        assert!(battle_state.players[1].has_team_condition(&TeamCondition::Reflect));
        
        assert_eq!(battle_state.players[1].get_team_condition_turns(&TeamCondition::LightScreen), None);
        assert!(!battle_state.players[1].has_team_condition(&TeamCondition::LightScreen));
    }

    #[test]
    fn test_team_condition_effectiveness_changes_on_expiry() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Tackle])], // Physical attack
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Splash])],
        );

        // Add Reflect with only 1 turn
        player2.add_team_condition(TeamCondition::Reflect, 1);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1.clone(), player2);

        // Turn 1: Attack with Reflect active - should get reduced damage
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        
        let test_rng1 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus1 = resolve_turn(&mut battle_state, test_rng1);

        // Get damage from first turn
        let turn1_damage = event_bus1.events().iter()
            .find_map(|event| match event {
                crate::battle::state::BattleEvent::DamageDealt { target: Species::Alakazam, damage, .. } => Some(*damage),
                _ => None,
            });

        // Reflect should be expired now
        assert!(!battle_state.players[1].has_team_condition(&TeamCondition::Reflect));

        // Turn 2: Attack without Reflect - should get full damage
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        
        let test_rng2 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus2 = resolve_turn(&mut battle_state, test_rng2);

        // Get damage from second turn
        let turn2_damage = event_bus2.events().iter()
            .find_map(|event| match event {
                crate::battle::state::BattleEvent::DamageDealt { target: Species::Alakazam, damage, .. } => Some(*damage),
                _ => None,
            });

        // Compare damages - turn 2 should deal more damage than turn 1
        if let (Some(damage1), Some(damage2)) = (turn1_damage, turn2_damage) {
            println!("Turn 1 damage (with Reflect): {}", damage1);
            println!("Turn 2 damage (without Reflect): {}", damage2);
            assert!(damage2 > damage1, "Damage without Reflect ({}) should be greater than damage with Reflect ({})", damage2, damage1);
            
            // Should be roughly double damage
            let expected_turn2_damage = damage1 * 2;
            let damage_difference = if damage2 > expected_turn2_damage { 
                damage2 - expected_turn2_damage 
            } else { 
                expected_turn2_damage - damage2 
            };
            assert!(damage_difference <= 2, "Turn 2 damage should be roughly double turn 1 damage. Expected: ~{}, Actual: {}", expected_turn2_damage, damage2);
        } else {
            panic!("Should have damage events in both turns");
        }
    }

    #[test]
    fn test_team_condition_ticking_in_isolation() {
        let mut player = BattlePlayer::new(
            "test".to_string(),
            "Test".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Splash])],
        );

        // Add conditions with different turn counts
        player.add_team_condition(TeamCondition::Reflect, 3);
        player.add_team_condition(TeamCondition::LightScreen, 1);

        // Verify initial state
        assert_eq!(player.get_team_condition_turns(&TeamCondition::Reflect), Some(3));
        assert_eq!(player.get_team_condition_turns(&TeamCondition::LightScreen), Some(1));

        // First tick
        player.tick_team_conditions();
        
        assert_eq!(player.get_team_condition_turns(&TeamCondition::Reflect), Some(2));
        assert_eq!(player.get_team_condition_turns(&TeamCondition::LightScreen), None); // Should be removed
        assert!(player.has_team_condition(&TeamCondition::Reflect));
        assert!(!player.has_team_condition(&TeamCondition::LightScreen));

        // Second tick
        player.tick_team_conditions();
        
        assert_eq!(player.get_team_condition_turns(&TeamCondition::Reflect), Some(1));
        assert!(player.has_team_condition(&TeamCondition::Reflect));

        // Third tick
        player.tick_team_conditions();
        
        assert_eq!(player.get_team_condition_turns(&TeamCondition::Reflect), None); // Should be removed
        assert!(!player.has_team_condition(&TeamCondition::Reflect));
    }
}