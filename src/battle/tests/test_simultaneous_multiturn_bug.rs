#[cfg(test)]
mod tests {
    use crate::battle::conditions::PokemonCondition;
    use crate::battle::state::{BattleState, TurnRng};
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
        // This test verifies that the "End-of-Turn Injection" model correctly handles
        // a scenario that would have caused a deadlock in the old architecture:
        // both players using a forced multi-turn move at the same time.

        println!("=== Testing Simultaneous Multi-Turn Move Resolution ===");

        // --- SETUP ---
        let player1 = BattlePlayer::new(
            "testuser".to_string(),
            "testuser".to_string(),
            vec![create_test_pokemon(Species::Venusaur, vec![Move::SolarBeam])], // move_index: 0
        );

        let mut player2 = BattlePlayer::new(
            "ai_opponent".to_string(),
            "AI charizard_team".to_string(),
            vec![create_test_pokemon(Species::Charizard, vec![Move::Fly])], // move_index: 0
        );
        player2.player_type = PlayerType::NPC;

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);
        
        // --- TURN 1: Both players initiate multi-turn moves ---
        println!("\n--- TURN 1: SolarBeam vs Fly ---");
        
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // SolarBeam
        let npc_actions = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions {
            battle_state.action_queue[player_index] = Some(action);
        }

        assert!(ready_for_turn_resolution(&battle_state));

        // Execute Turn 1
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let _ = resolve_turn(&mut battle_state, test_rng);

        // Verify both conditions were applied
        assert!(battle_state.players[0].has_condition(&PokemonCondition::Charging));
        assert!(battle_state.players[1].has_condition(&PokemonCondition::InAir));

        println!("After Turn 1:");
        println!("  Player 1 conditions: {:?}", battle_state.players[0].active_pokemon_conditions);
        println!("  Player 2 conditions: {:?}", battle_state.players[1].active_pokemon_conditions);
        println!("  Action queue after finalize_turn: {:?}", battle_state.action_queue);

        // --- TURN 2: Verify the new architecture works ---
        println!("\n--- TURN 2: Verifying Forced Action Injection ---");
        
        // NEW ASSERTION: The core of the fix. After Turn 1, `finalize_turn` should have
        // pre-filled the action queue for BOTH players for the upcoming Turn 2.
        assert!(
            battle_state.action_queue[0].is_some() && battle_state.action_queue[1].is_some(),
            "Action queue should be PRE-FILLED with both players' forced moves."
        );
        assert_eq!(battle_state.action_queue[0], Some(PlayerAction::UseMove { move_index: 0 }));
        assert_eq!(battle_state.action_queue[1], Some(PlayerAction::UseMove { move_index: 0 }));

        // Calling collect_npc_actions should now do nothing, as both slots are full.
        let npc_actions_turn2 = collect_npc_actions(&battle_state);
        assert!(npc_actions_turn2.is_empty(), "AI should not select an action when its queue slot is already filled.");
        
        // The battle is now ready for resolution because the queue is full.
        let ready = ready_for_turn_resolution(&battle_state);
        assert!(ready, "Battle should be ready for resolution with a full action queue.");

        // Resolve Turn 2
        let test_rng_2 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_2 = resolve_turn(&mut battle_state, test_rng_2);
        
        println!("Turn 2 resolved successfully! Events:");
        for event in event_bus_2.events() {
            println!("  {:?}", event);
        }
        
        // Verify that forced moves executed and conditions were cleared
        assert!(!battle_state.players[0].has_condition(&PokemonCondition::Charging), "Charging should be cleared after forced SolarBeam");
        assert!(!battle_state.players[1].has_condition(&PokemonCondition::InAir), "InAir should be cleared after forced Fly");
        
        println!("\n=== SIMULTANEOUS FORCED MOVE SCENARIO PASSES ===");
    }

}