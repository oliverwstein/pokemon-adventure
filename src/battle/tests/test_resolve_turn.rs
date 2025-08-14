#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleState, GameState, TurnRng};
    use crate::battle::turn_orchestrator::{collect_player_actions, resolve_turn};
    use crate::moves::Move;
    use crate::player::{BattlePlayer, PlayerAction};
    use crate::pokemon::{MoveInstance, PokemonInst};
    use crate::species::Species;
    use std::collections::HashMap;

    fn create_test_pokemon(species: Species, moves: Vec<Move>) -> PokemonInst {
        let mut pokemon_moves = [const { None }; 4];
        for (i, mv) in moves.into_iter().enumerate() {
            if i < 4 {
                pokemon_moves[i] = Some(MoveInstance {
                    move_: mv,
                    pp: 10, // Give each move some PP
                });
            }
        }

        PokemonInst::new_for_test(
            species,
            10,
            0,
            100,                       // Set current HP directly
            [15; 6],                   // Decent IVs
            [0; 6],                    // No EVs for simplicity
            [100, 80, 80, 80, 80, 80], // HP, Att, Def, SpAtt, SpDef, Speed
            pokemon_moves,
            None,
        )
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
            ante: 200,
        }
    }

    #[test]
    fn test_resolve_turn_basic() {
        // Initialize move data (required for get_move_data to work)
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");
        // Create two test Pokemon with basic moves
        let pokemon1 =
            create_test_pokemon(Species::Pikachu, vec![Move::Tackle, Move::ThunderPunch]);
        let pokemon2 = create_test_pokemon(Species::Charmander, vec![Move::Scratch, Move::Ember]);

        let player1 = create_test_player(pokemon1);
        let player2 = create_test_player(pokemon2);

        // Create battle state
        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Collect AI actions
        collect_player_actions(&mut battle_state).expect("Should collect actions successfully");

        // Verify actions were collected
        assert!(battle_state.action_queue[0].is_some());
        assert!(battle_state.action_queue[1].is_some());

        // Test action ordering - both are using moves, so order should be determined by speed
        let action_order = crate::battle::turn_orchestrator::determine_action_order(&battle_state);
        println!("Action order: {:?}", action_order);

        // Both Pokemon have same stats in our test, so order could be either way
        // But the order should be consistent and have both players
        assert_eq!(
            action_order.len(),
            2,
            "Should have both players in action order"
        );
        assert!(action_order.contains(&0), "Should contain player 0");
        assert!(action_order.contains(&1), "Should contain player 1");

        // Create deterministic RNG for testing
        let test_rng = TurnRng::new_for_test(vec![
            95, 95, 95, 95, 50, 50, 50, 50, 50, 50, // Various rolls for the turn
        ]);

        // Execute turn
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Check that events were generated
        let events = event_bus.events();
        assert!(!events.is_empty(), "Turn should generate events");

        // Check that turn number incremented
        assert_eq!(battle_state.turn_number, 2, "Turn number should increment");

        // Check that game state returned to waiting
        assert_eq!(battle_state.game_state, GameState::WaitingForActions);

        // Check that action queue was cleared
        assert!(battle_state.action_queue[0].is_none());
        assert!(battle_state.action_queue[1].is_none());

        println!("Generated {} events:", events.len());
        for event in events {
            println!("  {:?}", event);
        }
    }
}
