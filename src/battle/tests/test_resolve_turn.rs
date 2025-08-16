#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleState, GameState, TurnRng, BattleEvent};
    use crate::battle::engine::{resolve_turn};
    use crate::moves::Move;
    use crate::player::{BattlePlayer, PlayerAction};
    use crate::pokemon::{MoveInstance, PokemonInst};
    use crate::species::Species;

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
        let player_team = vec![pokemon];

        // Step 2: Use the constructor to create the player.
        // This will create a default player with ante = 0. We declare it `mut` to change it.
        let mut player = BattlePlayer::new(
            "test_player".to_string(),
            "TestPlayer".to_string(),
            player_team,
        );

        // Step 3: Modify any fields that differ from the default constructor values.
        player.ante = 200;
        player
    }

    #[test]
    fn test_resolve_turn_basic_speed_order() {
        
        // Create a faster Pikachu and a slower Charmander to test speed-based turn order.
        let pikachu = create_test_pokemon(Species::Pikachu, vec![Move::Tackle]); // Faster
        let charmander = create_test_pokemon(Species::Charmander, vec![Move::Scratch]); // Slower

        let player1 = create_test_player(pikachu);
        let player2 = create_test_player(charmander);
        
        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Manually set actions to ensure predictability
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Pikachu uses Tackle
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Charmander uses Scratch

        // Verify actions were collected
        assert!(battle_state.action_queue[0].is_some());
        assert!(battle_state.action_queue[1].is_some());

        let test_rng = TurnRng::new_for_test(vec![
            50, 90, 90, // RNG for Pikachu's Tackle (hit, no crit, damage)
            50, 90, 90, // RNG for Charmander's Scratch (hit, no crit, damage)
            50, 50, 50, 50, // Extra values for any other checks
        ]);

        let event_bus = resolve_turn(&mut battle_state, test_rng);
        let events = event_bus.events();
        
        // --- Verify Turn Order from Events ---
        let move_used_events: Vec<_> = events.iter().filter_map(|e| match e {
            BattleEvent::MoveUsed { player_index, .. } => Some(player_index),
            _ => None
        }).collect();

        // The first MoveUsed event should be from player 0 (the faster Pikachu)
        assert_eq!(move_used_events.get(0), Some(&&0), "Faster Pokémon should act first.");
        // The second MoveUsed event should be from player 1 (the slower Charmander)
        assert_eq!(move_used_events.get(1), Some(&&1), "Slower Pokémon should act second.");

        // --- Verify Final State ---
        assert!(!events.is_empty(), "Turn should generate events");
        assert_eq!(battle_state.turn_number, 2, "Turn number should increment");
        assert_eq!(battle_state.game_state, GameState::WaitingForActions);
        assert!(battle_state.action_queue[0].is_none() && battle_state.action_queue[1].is_none(), "Action queue should be cleared");

        println!("Generated {} events:", events.len());
        for event in events {
            println!("  {:?}", event);
        }
    }

}
