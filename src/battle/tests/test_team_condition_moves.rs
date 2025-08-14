#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleState, TurnRng};
    use crate::battle::engine::resolve_turn;
    use crate::moves::Move;
    use crate::player::{BattlePlayer, PlayerAction, TeamCondition};
    use crate::pokemon::{MoveInstance, PokemonInst};
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
    fn test_reflect_move_applies_reflect_condition() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Reflect])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])],
        );

        // Initially no team conditions
        assert!(!player1.has_team_condition(&TeamCondition::Reflect));

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Reflect, Player 2 uses Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Reflect
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let _ = resolve_turn(&mut battle_state, test_rng);

        // Player 1 should now have Reflect condition
        assert!(
            battle_state.players[0].has_team_condition(&TeamCondition::Reflect),
            "Reflect move should apply Reflect team condition"
        );
        assert_eq!(
            battle_state.players[0].get_team_condition_turns(&TeamCondition::Reflect),
            Some(4),
            "Reflect should have 4 turns remaining after first turn"
        );

        // Player 2 should not have Reflect
        assert!(!battle_state.players[1].has_team_condition(&TeamCondition::Reflect));
    }

    #[test]
    fn test_light_screen_move_applies_light_screen_condition() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Alakazam,
                vec![Move::LightScreen],
            )],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])],
        );

        // Initially no team conditions
        assert!(!player1.has_team_condition(&TeamCondition::LightScreen));

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Light Screen, Player 2 uses Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Light Screen
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let _ = resolve_turn(&mut battle_state, test_rng);

        // Player 1 should now have Light Screen condition
        assert!(
            battle_state.players[0].has_team_condition(&TeamCondition::LightScreen),
            "Light Screen move should apply Light Screen team condition"
        );
        assert_eq!(
            battle_state.players[0].get_team_condition_turns(&TeamCondition::LightScreen),
            Some(4),
            "Light Screen should have 4 turns remaining after first turn"
        );

        // Player 2 should not have Light Screen
        assert!(!battle_state.players[1].has_team_condition(&TeamCondition::LightScreen));
    }

    #[test]
    fn test_mist_move_applies_mist_condition() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Mist])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])],
        );

        // Initially no team conditions
        assert!(!player1.has_team_condition(&TeamCondition::Mist));

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Mist, Player 2 uses Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Mist
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let _ = resolve_turn(&mut battle_state, test_rng);

        // Player 1 should now have Mist condition
        assert!(
            battle_state.players[0].has_team_condition(&TeamCondition::Mist),
            "Mist move should apply Mist team condition"
        );
        assert_eq!(
            battle_state.players[0].get_team_condition_turns(&TeamCondition::Mist),
            Some(4),
            "Mist should have 4 turns remaining after first turn"
        );

        // Player 2 should not have Mist
        assert!(!battle_state.players[1].has_team_condition(&TeamCondition::Mist));
    }

    #[test]
    fn test_team_conditions_work_immediately() {
        // Test that team conditions become active immediately after being applied
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        // Player 1 sets up Mist, Player 2 tries to use Growl (stat reduction)
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Mist])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Growl])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Turn 1: Player 1 uses Mist, Player 2 uses Growl
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Mist
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Growl

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Player 1 should have Mist
        assert!(battle_state.players[0].has_team_condition(&TeamCondition::Mist));

        // Growl should have been blocked by Mist (Mist protects the team that used it)
        let blocked_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, crate::battle::state::BattleEvent::StatChangeBlocked { reason, .. } if reason.contains("Mist")))
            .collect();
        assert!(
            !blocked_events.is_empty(),
            "Mist should immediately protect against stat reductions"
        );
    }

    #[test]
    fn test_using_team_condition_when_already_active() {
        // Test behavior when using a move to set up a condition that's already active
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Reflect])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Turn 1: Set up Reflect
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Reflect
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng1 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let _ = resolve_turn(&mut battle_state, test_rng1);

        assert!(battle_state.players[0].has_team_condition(&TeamCondition::Reflect));
        assert_eq!(
            battle_state.players[0].get_team_condition_turns(&TeamCondition::Reflect),
            Some(4)
        );

        // Turn 2: Use Reflect again (should reset the turn counter)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Reflect again
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng2 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let _ = resolve_turn(&mut battle_state, test_rng2);

        // Should still have Reflect, and turn counter should be back to 4 (refreshed and decremented)
        assert!(battle_state.players[0].has_team_condition(&TeamCondition::Reflect));
        assert_eq!(
            battle_state.players[0].get_team_condition_turns(&TeamCondition::Reflect),
            Some(4),
            "Using Reflect again should refresh the turn counter"
        );
    }
}
