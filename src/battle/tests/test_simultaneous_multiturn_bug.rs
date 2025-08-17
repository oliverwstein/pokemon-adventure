#[cfg(test)]
mod tests {
    use crate::battle::conditions::PokemonCondition;
    use crate::battle::state::{BattleState, TurnRng, GameState};
    use crate::battle::engine::{collect_npc_actions, resolve_turn, ready_for_turn_resolution};
    use crate::moves::Move;
    use crate::player::{BattlePlayer, PlayerAction, PlayerType};
    use crate::pokemon::{MoveInstance, PokemonInst};
    use crate::species::Species;

    fn create_test_pokemon(species: Species, moves: Vec<Move>) -> PokemonInst {
        let mut pokemon_moves = [const { None }; 4];
        for (i, mv) in moves.into_iter().enumerate() {
            if i < 4 {
                pokemon_moves[i] = Some(MoveInstance { move_: mv, pp: 20 });
            }
        }

        {
            let mut pokemon = PokemonInst::new_for_test(
                species,
                10,
                0,
                0,
                [15; 6],
                [0; 6],
                [100, 80, 80, 80, 80, 80],
                pokemon_moves,
                None,
            );
            pokemon.set_hp_to_max();
            pokemon
        }
    }

    #[test]
    fn test_simultaneous_multiturn_bug_reproduction() {
        // Reproduce the exact bug reported: SolarBeam (charging) + Fly (semi-invulnerable)
        // Both players use multi-turn moves simultaneously causing action queue deadlock

        println!("=== Testing Simultaneous Multi-Turn Move Bug ===");

        // Create battle with Venusaur (SolarBeam) vs Charizard (Fly)
        let player1 = BattlePlayer::new(
            "testuser".to_string(),
            "testuser".to_string(),
            vec![create_test_pokemon(Species::Venusaur, vec![Move::SolarBeam])],
        );

        let mut player2 = BattlePlayer::new(
            "ai_opponent".to_string(),
            "AI charizard_team".to_string(),
            vec![create_test_pokemon(Species::Charizard, vec![Move::Fly])],
        );
        player2.player_type = PlayerType::NPC; // Set as NPC for collect_npc_actions

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        println!("Initial state: turn {}, game_state: {:?}", battle_state.turn_number, battle_state.game_state);
        println!("Action queue: {:?}", battle_state.action_queue);

        // === TURN 1: Both players use multi-turn moves ===
        println!("\n--- TURN 1: SolarBeam vs Fly ---");
        
        // Player 1 (human) uses SolarBeam, Player 2 (NPC) will use Fly via collect_npc_actions
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // SolarBeam
        
        let npc_actions = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions {
            battle_state.action_queue[player_index] = Some(action);
        }

        println!("Turn 1 action queue after collection: {:?}", battle_state.action_queue);
        assert!(battle_state.action_queue[0].is_some(), "Player 1 should have SolarBeam queued");
        assert!(battle_state.action_queue[1].is_some(), "Player 2 should have Fly queued");

        // Execute Turn 1
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        println!("Turn 1 events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Verify both conditions were applied
        assert!(
            battle_state.players[0].has_condition(&PokemonCondition::Charging),
            "Player 1 should be Charging after SolarBeam"
        );
        assert!(
            battle_state.players[1].has_condition(&PokemonCondition::InAir),
            "Player 2 should be InAir after Fly"
        );

        println!("After Turn 1:");
        println!("  Player 1 conditions: {:?}", battle_state.players[0].active_pokemon_conditions);
        println!("  Player 2 conditions: {:?}", battle_state.players[1].active_pokemon_conditions);
        println!("  Turn number: {}", battle_state.turn_number);
        println!("  Game state: {:?}", battle_state.game_state);
        println!("  Action queue: {:?}", battle_state.action_queue);

        // === TURN 2: The Bug Scenario ===
        println!("\n--- TURN 2: Bug Reproduction ---");
        
        // This is where the bug occurs - both players have forced moves but action queue isn't populated
        
        // Check current state
        assert_eq!(battle_state.game_state, GameState::WaitingForActions, "Should be waiting for actions");
        assert_eq!(battle_state.action_queue, [None, None], "Action queue should be empty at start of turn");

        // Try to collect NPC actions - this is where the bug manifests
        let npc_actions_turn2 = collect_npc_actions(&battle_state);
        println!("NPC actions collected for turn 2: {:?}", npc_actions_turn2);
        
        for (player_index, action) in npc_actions_turn2 {
            battle_state.action_queue[player_index] = Some(action);
        }

        println!("Action queue after NPC collection: {:?}", battle_state.action_queue);

        // *** THE BUG: Both action slots should be filled, but they're not ***
        // collect_npc_actions skips both players because they have forced moves
        
        // Check if battle is ready for turn resolution
        let ready = ready_for_turn_resolution(&battle_state);
        println!("Ready for turn resolution: {}", ready);

        // *** FIX CONFIRMED: ready_for_turn_resolution now returns true because forced moves are detected ***
        assert!(ready, "FIX CONFIRMED: Battle ready for resolution despite empty action queue (forced moves)");

        // Demonstrate that turn resolution now works
        println!("\n--- Attempting Turn Resolution (should succeed) ---");
        
        if ready {
            let test_rng_2 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
            let event_bus_2 = resolve_turn(&mut battle_state, test_rng_2);
            println!("Turn 2 resolved successfully!");
            for event in event_bus_2.events() {
                println!("  {:?}", event);
            }
            
            // Verify that forced moves executed and conditions were cleared
            assert!(!battle_state.players[0].has_condition(&PokemonCondition::Charging),
                    "Charging should be cleared after forced SolarBeam");
            assert!(!battle_state.players[1].has_condition(&PokemonCondition::InAir),
                    "InAir should be cleared after forced Fly");
        } else {
            panic!("Turn resolution should be ready but isn't");
        }

        // Verify the fix worked
        assert_ne!(battle_state.game_state, GameState::WaitingForActions,
                  "FIX: Game state should have progressed beyond WaitingForActions");
        
        println!("\n=== BUG FIX SUCCESSFUL ===");
        println!("Simultaneous multi-turn moves now resolve correctly");
        println!("Battle proceeds normally despite empty initial action queue");
    }


    #[test]
    fn test_single_player_forced_move_works() {
        // Verify that single-player forced moves work correctly (should pass)
        // This confirms the issue is specifically with simultaneous forced moves
        
        println!("=== Testing Single Player Forced Move (Control Test) ===");

        let player1 = BattlePlayer::new(
            "testuser".to_string(),
            "testuser".to_string(),
            vec![create_test_pokemon(Species::Venusaur, vec![Move::SolarBeam])],
        );

        let mut player2 = BattlePlayer::new(
            "ai_opponent".to_string(),
            "AI opponent".to_string(),
            vec![create_test_pokemon(Species::Charizard, vec![Move::Tackle])], // Normal move, not multi-turn
        );
        player2.player_type = PlayerType::NPC;

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Turn 1: Only Player 1 uses multi-turn move
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // SolarBeam
        let npc_actions = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions {
            battle_state.action_queue[player_index] = Some(action);
        }
        
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        resolve_turn(&mut battle_state, test_rng);

        // Verify only Player 1 has forced move
        assert!(battle_state.players[0].has_condition(&PokemonCondition::Charging));
        assert!(!battle_state.players[1].has_condition(&PokemonCondition::InAir));

        println!("After Turn 1: Only Player 1 has Charging condition");

        // Turn 2: This should work correctly
        let npc_actions_turn2 = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions_turn2 {
            battle_state.action_queue[player_index] = Some(action);
        }

        println!("Turn 2 action queue: {:?}", battle_state.action_queue);
        
        // Should have NPC action but not Player 1 action (forced move)
        assert!(battle_state.action_queue[0].is_none(), "Player 1 should have empty queue (forced)");
        assert!(battle_state.action_queue[1].is_some(), "Player 2 should have queued action");

        // Should be ready for resolution
        assert!(ready_for_turn_resolution(&battle_state), "Should be ready despite Player 1 empty queue");

        // Should resolve successfully
        let test_rng_2 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_2 = resolve_turn(&mut battle_state, test_rng_2);

        println!("Turn 2 events:");
        for event in event_bus_2.events() {
            println!("  {:?}", event);
        }

        // Charging condition should be cleared
        assert!(!battle_state.players[0].has_condition(&PokemonCondition::Charging),
                "Charging should be cleared after forced SolarBeam");

        println!("\n=== SINGLE PLAYER FORCED MOVE WORKS CORRECTLY ===");
        println!("This confirms the bug is specific to simultaneous forced moves");
    }
}