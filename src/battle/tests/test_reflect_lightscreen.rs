#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleEvent, BattleState, TurnRng};
    use crate::battle::turn_orchestrator::resolve_turn;
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
    fn test_reflect_reduces_physical_damage() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        // Test 1: Without Reflect (baseline)
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Tackle])], // Physical attacker
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Splash])], // Defensive target
        );

        let mut battle_state_baseline = BattleState::new(
            "test_battle_baseline".to_string(),
            player1.clone(),
            player2.clone(),
        );

        battle_state_baseline.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        battle_state_baseline.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash (no damage)

        let test_rng_baseline =
            TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_baseline = resolve_turn(&mut battle_state_baseline, test_rng_baseline);

        // Get baseline damage
        let baseline_damage = event_bus_baseline
            .events()
            .iter()
            .find_map(|event| match event {
                BattleEvent::DamageDealt {
                    target: Species::Alakazam,
                    damage,
                    ..
                } => Some(*damage),
                _ => None,
            })
            .expect("Should have damage event in baseline test");

        // Test 2: With Reflect
        let mut player2_reflect = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Splash])], // Same defensive target
        );

        player2_reflect.add_team_condition(TeamCondition::Reflect, 5);

        let mut battle_state_reflect =
            BattleState::new("test_battle_reflect".to_string(), player1, player2_reflect);

        battle_state_reflect.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        battle_state_reflect.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash (no damage)

        let test_rng_reflect =
            TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_reflect = resolve_turn(&mut battle_state_reflect, test_rng_reflect);

        // Get Reflect damage
        let reflect_damage = event_bus_reflect
            .events()
            .iter()
            .find_map(|event| match event {
                BattleEvent::DamageDealt {
                    target: Species::Alakazam,
                    damage,
                    ..
                } => Some(*damage),
                _ => None,
            })
            .expect("Should have damage event in reflect test");

        // Print results for clarity
        println!("Reflect test results:");
        println!("  Baseline damage (no Reflect): {}", baseline_damage);
        println!("  Reflect damage: {}", reflect_damage);
        println!(
            "  Damage reduction: {}%",
            (100 * (baseline_damage - reflect_damage)) / baseline_damage
        );

        // Reflect should reduce damage by approximately 50%
        assert!(
            reflect_damage < baseline_damage,
            "Reflect should reduce damage"
        );

        // Should be roughly half damage (allowing some rounding variance)
        let expected_reflect_damage = baseline_damage / 2;
        let damage_difference = if reflect_damage > expected_reflect_damage {
            reflect_damage - expected_reflect_damage
        } else {
            expected_reflect_damage - reflect_damage
        };

        assert!(
            damage_difference <= 2,
            "Reflect damage should be approximately half of baseline damage. Expected: ~{}, Actual: {}",
            expected_reflect_damage,
            reflect_damage
        );
    }

    #[test]
    fn test_light_screen_reduces_special_damage() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        // Test 1: Without Light Screen (baseline)
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Alakazam,
                vec![Move::Confusion],
            )], // Special attacker
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])], // Defensive target
        );

        let mut battle_state_baseline = BattleState::new(
            "test_battle_baseline".to_string(),
            player1.clone(),
            player2.clone(),
        );

        battle_state_baseline.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Confusion
        battle_state_baseline.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash (no damage)

        let test_rng_baseline =
            TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_baseline = resolve_turn(&mut battle_state_baseline, test_rng_baseline);

        // Get baseline damage
        let baseline_damage = event_bus_baseline
            .events()
            .iter()
            .find_map(|event| match event {
                BattleEvent::DamageDealt {
                    target: Species::Machamp,
                    damage,
                    ..
                } => Some(*damage),
                _ => None,
            })
            .expect("Should have damage event in baseline test");

        // Test 2: With Light Screen
        let mut player2_lightscreen = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])], // Same defensive target
        );

        player2_lightscreen.add_team_condition(TeamCondition::LightScreen, 5);

        let mut battle_state_lightscreen = BattleState::new(
            "test_battle_lightscreen".to_string(),
            player1,
            player2_lightscreen,
        );

        battle_state_lightscreen.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Confusion
        battle_state_lightscreen.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash (no damage)

        let test_rng_lightscreen =
            TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_lightscreen =
            resolve_turn(&mut battle_state_lightscreen, test_rng_lightscreen);

        // Get Light Screen damage
        let lightscreen_damage = event_bus_lightscreen
            .events()
            .iter()
            .find_map(|event| match event {
                BattleEvent::DamageDealt {
                    target: Species::Machamp,
                    damage,
                    ..
                } => Some(*damage),
                _ => None,
            })
            .expect("Should have damage event in light screen test");

        // Print results for clarity
        println!("Light Screen test results:");
        println!("  Baseline damage (no Light Screen): {}", baseline_damage);
        println!("  Light Screen damage: {}", lightscreen_damage);
        println!(
            "  Damage reduction: {}%",
            (100 * (baseline_damage - lightscreen_damage)) / baseline_damage
        );

        // Light Screen should reduce damage by approximately 50%
        assert!(
            lightscreen_damage < baseline_damage,
            "Light Screen should reduce damage"
        );

        // Should be roughly half damage (allowing some rounding variance)
        let expected_lightscreen_damage = baseline_damage / 2;
        let damage_difference = if lightscreen_damage > expected_lightscreen_damage {
            lightscreen_damage - expected_lightscreen_damage
        } else {
            expected_lightscreen_damage - lightscreen_damage
        };

        assert!(
            damage_difference <= 2,
            "Light Screen damage should be approximately half of baseline damage. Expected: ~{}, Actual: {}",
            expected_lightscreen_damage,
            lightscreen_damage
        );
    }

    #[test]
    fn test_reflect_does_not_reduce_special_damage() {
        // Test that Reflect has no effect on special moves - damage should be the same with or without Reflect
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        // Test 1: Without Reflect
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Alakazam,
                vec![Move::Confusion],
            )], // Special move
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])],
        );

        let mut battle_state_no_reflect =
            BattleState::new("test_battle".to_string(), player1.clone(), player2.clone());
        battle_state_no_reflect.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Confusion
        battle_state_no_reflect.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng_no_reflect =
            TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_no_reflect = resolve_turn(&mut battle_state_no_reflect, test_rng_no_reflect);

        let damage_without_reflect = event_bus_no_reflect
            .events()
            .iter()
            .find_map(|event| match event {
                BattleEvent::DamageDealt {
                    target: Species::Machamp,
                    damage,
                    ..
                } => Some(*damage),
                _ => None,
            })
            .expect("Should have damage event without reflect");

        // Test 2: With Reflect (should have no effect on special damage)
        let mut player2_reflect = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])],
        );
        player2_reflect.add_team_condition(TeamCondition::Reflect, 5);

        let mut battle_state_reflect =
            BattleState::new("test_battle".to_string(), player1, player2_reflect);
        battle_state_reflect.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Confusion
        battle_state_reflect.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng_reflect =
            TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_reflect = resolve_turn(&mut battle_state_reflect, test_rng_reflect);

        let damage_with_reflect = event_bus_reflect
            .events()
            .iter()
            .find_map(|event| match event {
                BattleEvent::DamageDealt {
                    target: Species::Machamp,
                    damage,
                    ..
                } => Some(*damage),
                _ => None,
            })
            .expect("Should have damage event with reflect");

        println!("Reflect vs Special damage test:");
        println!("  Damage without Reflect: {}", damage_without_reflect);
        println!("  Damage with Reflect: {}", damage_with_reflect);

        // Damage should be identical - Reflect doesn't affect special moves
        assert_eq!(
            damage_without_reflect, damage_with_reflect,
            "Reflect should not affect special damage"
        );
    }

    #[test]
    fn test_light_screen_does_not_reduce_physical_damage() {
        // Test that Light Screen has no effect on physical moves - damage should be the same with or without Light Screen
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        // Test 1: Without Light Screen
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Tackle])], // Physical move
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Splash])],
        );

        let mut battle_state_no_lightscreen =
            BattleState::new("test_battle".to_string(), player1.clone(), player2.clone());
        battle_state_no_lightscreen.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        battle_state_no_lightscreen.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng_no_lightscreen =
            TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_no_lightscreen =
            resolve_turn(&mut battle_state_no_lightscreen, test_rng_no_lightscreen);

        let damage_without_lightscreen = event_bus_no_lightscreen
            .events()
            .iter()
            .find_map(|event| match event {
                BattleEvent::DamageDealt {
                    target: Species::Alakazam,
                    damage,
                    ..
                } => Some(*damage),
                _ => None,
            })
            .expect("Should have damage event without light screen");

        // Test 2: With Light Screen (should have no effect on physical damage)
        let mut player2_lightscreen = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Splash])],
        );
        player2_lightscreen.add_team_condition(TeamCondition::LightScreen, 5);

        let mut battle_state_lightscreen =
            BattleState::new("test_battle".to_string(), player1, player2_lightscreen);
        battle_state_lightscreen.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        battle_state_lightscreen.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng_lightscreen =
            TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_lightscreen =
            resolve_turn(&mut battle_state_lightscreen, test_rng_lightscreen);

        let damage_with_lightscreen = event_bus_lightscreen
            .events()
            .iter()
            .find_map(|event| match event {
                BattleEvent::DamageDealt {
                    target: Species::Alakazam,
                    damage,
                    ..
                } => Some(*damage),
                _ => None,
            })
            .expect("Should have damage event with light screen");

        println!("Light Screen vs Physical damage test:");
        println!(
            "  Damage without Light Screen: {}",
            damage_without_lightscreen
        );
        println!("  Damage with Light Screen: {}", damage_with_lightscreen);

        // Damage should be identical - Light Screen doesn't affect physical moves
        assert_eq!(
            damage_without_lightscreen, damage_with_lightscreen,
            "Light Screen should not affect physical damage"
        );
    }
}
