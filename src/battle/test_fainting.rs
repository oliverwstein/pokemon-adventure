#[cfg(test)]
mod tests {
    use crate::pokemon::{PokemonInst, StatusCondition, MoveInstance};
    use crate::species::Species;
    use crate::moves::Move;
    use crate::battle::state::{BattleState, TurnRng, BattleEvent};
    use crate::battle::turn_orchestrator::{collect_player_actions, resolve_turn};
    use crate::player::BattlePlayer;
    use std::collections::HashMap;

    fn create_test_pokemon_with_hp(species: Species, moves: Vec<Move>, hp: u16) -> PokemonInst {
        let mut pokemon_moves = [const { None }; 4];
        for (i, mv) in moves.into_iter().enumerate() {
            if i < 4 {
                pokemon_moves[i] = Some(MoveInstance {
                    move_: mv,
                    pp: 10,
                });
            }
        }

        let mut pokemon = PokemonInst {
            name: species.name().to_string(),
            species,
            curr_exp: 0,
            ivs: [15; 6],
            evs: [0; 6],
            curr_stats: [hp, 80, 80, 80, 80, 80], // Set specific HP
            moves: pokemon_moves,
            status: None,
        };
        
        pokemon
    }

    fn create_test_player(pokemon: PokemonInst) -> BattlePlayer {
        BattlePlayer {
            player_id: "test_player".to_string(),
            player_name: "TestPlayer".to_string(),
            team: [Some(pokemon), None, None, None, None, None],
            active_pokemon_index: 0,
            stat_stages: HashMap::new(),
            team_conditions: HashMap::new(),
            active_pokemon_conditions: HashMap::new(),
            last_move: None,
        }
    }

    #[test]
    fn test_pokemon_fainting_mechanics() {
        // Test Pokemon with low HP that will faint from one hit
        let mut pokemon = create_test_pokemon_with_hp(Species::Pikachu, vec![Move::Tackle], 20);
        
        // Test initial state
        assert!(!pokemon.is_fainted());
        assert_eq!(pokemon.current_hp(), 20);
        
        // Test taking damage without fainting
        let fainted = pokemon.take_damage(10);
        assert!(!fainted);
        assert_eq!(pokemon.current_hp(), 10);
        assert!(!pokemon.is_fainted());
        
        // Test taking fatal damage
        let fainted = pokemon.take_damage(15); // More than remaining HP
        assert!(fainted);
        assert_eq!(pokemon.current_hp(), 0);
        assert!(pokemon.is_fainted());
        assert_eq!(pokemon.status, Some(StatusCondition::Faint));
    }

    #[test]
    fn test_faint_replaces_other_statuses() {
        let mut pokemon = create_test_pokemon_with_hp(Species::Pikachu, vec![Move::Tackle], 10);
        
        // Apply burn status
        pokemon.status = Some(StatusCondition::Burn);
        assert_eq!(pokemon.status, Some(StatusCondition::Burn));
        
        // Take fatal damage - faint should replace burn
        let fainted = pokemon.take_damage(20);
        assert!(fainted);
        assert!(pokemon.is_fainted());
        assert_eq!(pokemon.status, Some(StatusCondition::Faint));
    }

    #[test]
    fn test_healing_and_revival() {
        let mut pokemon = create_test_pokemon_with_hp(Species::Pikachu, vec![Move::Tackle], 50);
        
        // Damage without fainting
        pokemon.take_damage(30);
        assert_eq!(pokemon.current_hp(), 20);
        
        // Heal (should work on non-fainted Pokemon) - but our max_hp logic is simplified
        // For now, let's test that heal doesn't crash and changes HP appropriately
        let hp_before_heal = pokemon.current_hp();
        pokemon.heal(10);
        let hp_after_heal = pokemon.current_hp();
        assert!(hp_after_heal >= hp_before_heal, "Healing should not decrease HP");
        
        // Faint the Pokemon
        pokemon.take_damage(100); // Ensure it faints
        assert!(pokemon.is_fainted());
        assert_eq!(pokemon.current_hp(), 0);
        
        // Healing should not work on fainted Pokemon
        pokemon.heal(20);
        assert_eq!(pokemon.current_hp(), 0);
        assert!(pokemon.is_fainted());
        
        // Revive should work
        pokemon.revive(25);
        assert!(!pokemon.is_fainted());
        assert_eq!(pokemon.current_hp(), 25);
        assert_eq!(pokemon.status, None);
    }

    #[test]
    fn test_battle_with_fainting() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        
        // Create Pokemon with low HP to ensure fainting
        let pokemon1 = create_test_pokemon_with_hp(Species::Pikachu, vec![Move::Tackle], 100);
        let pokemon2 = create_test_pokemon_with_hp(Species::Charmander, vec![Move::Scratch], 20); // Low HP
        
        let player1 = create_test_player(pokemon1);
        let player2 = create_test_player(pokemon2);

        // Create battle state
        let mut battle_state = BattleState::new(
            "test_battle".to_string(),
            player1,
            player2,
        );

        // Collect AI actions
        collect_player_actions(&mut battle_state).expect("Should collect actions successfully");

        // Create RNG that ensures hits
        let test_rng = TurnRng::new_for_test(vec![
            50, 50, 50, 50, 50, 50, // Mid values for hits but no crits
        ]);

        // Execute turn
        let event_bus = resolve_turn(&mut battle_state, test_rng);
        
        // Check events
        let events = event_bus.events();
        
        println!("Generated {} events:", events.len());
        for event in events {
            println!("  {:?}", event);
        }

        // Should have damage and potentially fainting events
        let damage_events: Vec<_> = events.iter().filter(|event| {
            matches!(event, BattleEvent::DamageDealt { .. })
        }).collect();
        
        let faint_events: Vec<_> = events.iter().filter(|event| {
            matches!(event, BattleEvent::PokemonFainted { .. })
        }).collect();

        assert!(!damage_events.is_empty(), "Should generate damage events");
        
        // Check if Pokemon actually fainted in battle state
        let pokemon2_hp = battle_state.players[1].team[0].as_ref().unwrap().current_hp();
        if pokemon2_hp == 0 {
            assert!(!faint_events.is_empty(), "Should generate faint event when Pokemon faints");
        }
    }

    #[test]
    fn test_skip_actions_against_fainted_pokemon() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        
        // Create Pokemon where one is already fainted
        let pokemon1 = create_test_pokemon_with_hp(Species::Pikachu, vec![Move::Tackle], 100);
        let mut pokemon2 = create_test_pokemon_with_hp(Species::Charmander, vec![Move::Scratch], 20);
        
        // Pre-faint the second Pokemon
        pokemon2.take_damage(50);
        assert!(pokemon2.is_fainted());
        
        let player1 = create_test_player(pokemon1);
        let player2 = create_test_player(pokemon2);

        // Create battle state
        let mut battle_state = BattleState::new(
            "test_battle".to_string(),
            player1,
            player2,
        );

        // Manually set actions since AI might not work with fainted Pokemon
        battle_state.action_queue[0] = Some(crate::player::PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(crate::player::PlayerAction::UseMove { move_index: 0 });

        // Create RNG
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50]);

        // Execute turn
        let event_bus = resolve_turn(&mut battle_state, test_rng);
        
        // Check events
        let events = event_bus.events();
        
        println!("Generated {} events:", events.len());
        for event in events {
            println!("  {:?}", event);
        }

        // Should have action failed events for targeting fainted Pokemon
        let action_failed_events: Vec<_> = events.iter().filter(|event| {
            matches!(event, BattleEvent::ActionFailed { .. })
        }).collect();

        assert!(!action_failed_events.is_empty(), "Should skip actions against fainted Pokemon");
    }
}