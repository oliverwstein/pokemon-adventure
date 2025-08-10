#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleState, TurnRng, GameState, BattleEvent};
    use crate::battle::turn_orchestrator::{collect_player_actions, resolve_turn};
    use crate::player::{BattlePlayer, PlayerAction};
    use crate::pokemon::{PokemonInst, MoveInstance};
    use crate::species::Species;
    use crate::moves::Move;
    use std::collections::HashMap;

    fn create_test_pokemon(species: Species, moves: Vec<Move>) -> PokemonInst {
        let mut pokemon_moves = [const { None }; 4];
        for (i, mv) in moves.into_iter().enumerate() {
            if i < 4 {
                pokemon_moves[i] = Some(MoveInstance {
                    move_: mv,
                    pp: 10,
                });
            }
        }

        PokemonInst {
            name: species.name().to_string(),
            species,
            curr_exp: 0,
            ivs: [15; 6],
            evs: [0; 6],
            curr_stats: [100, 80, 80, 80, 80, 80],
            moves: pokemon_moves,
            status: None,
        }
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
    fn test_critical_hits_in_battle() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        
        // Create two test Pokemon
        let pokemon1 = create_test_pokemon(Species::Pikachu, vec![Move::Tackle]);
        let pokemon2 = create_test_pokemon(Species::Charmander, vec![Move::Scratch]);
        
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

        // Create RNG with values that will guarantee critical hits
        let test_rng = TurnRng::new_for_test(vec![
            2, 1, 3, 2, 1, 3, // Low values to ensure critical hits
        ]);

        // Execute turn
        let event_bus = resolve_turn(&mut battle_state, test_rng);
        
        // Check events
        let events = event_bus.events();
        let critical_hit_events: Vec<_> = events.iter().filter(|event| {
            matches!(event, BattleEvent::CriticalHit { .. })
        }).collect();

        println!("Generated {} events:", events.len());
        for event in events {
            println!("  {:?}", event);
        }

        // Should have at least one critical hit with these low RNG values
        assert!(!critical_hit_events.is_empty(), "Should generate at least one critical hit event");
    }

    #[test]
    fn test_no_critical_hits_guaranteed_miss() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        
        // Create two test Pokemon
        let pokemon1 = create_test_pokemon(Species::Pikachu, vec![Move::Tackle]);
        let pokemon2 = create_test_pokemon(Species::Charmander, vec![Move::Scratch]);
        
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

        // Create RNG with high values that will miss all attacks
        let test_rng = TurnRng::new_for_test(vec![
            99, 99, 99, 99, 99, 99, // High values to ensure misses
        ]);

        // Execute turn
        let event_bus = resolve_turn(&mut battle_state, test_rng);
        
        // Check events
        let events = event_bus.events();
        let critical_hit_events: Vec<_> = events.iter().filter(|event| {
            matches!(event, BattleEvent::CriticalHit { .. })
        }).collect();
        let miss_events: Vec<_> = events.iter().filter(|event| {
            matches!(event, BattleEvent::MoveMissed { .. })
        }).collect();

        println!("Generated {} events:", events.len());
        for event in events {
            println!("  {:?}", event);
        }

        // Should have no critical hits when moves miss
        assert!(critical_hit_events.is_empty(), "Should not generate critical hit events when moves miss");
        assert!(!miss_events.is_empty(), "Should generate miss events");
    }
}