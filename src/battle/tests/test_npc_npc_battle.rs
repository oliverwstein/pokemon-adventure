#[cfg(test)]
mod tests {
    use crate::battle::engine::{collect_npc_actions, ready_for_turn_resolution, resolve_turn};
    use crate::battle::state::{BattleState, GameState, TurnRng};
    use crate::player::PlayerType;
    use crate::teams::create_battle_player_from_team;
    use pretty_assertions::assert_ne;

    #[test]
    fn test_full_npc_battle_completes_without_crashing() {
        // Arrange: Set up a full battle using prefab teams.
        // This serves as a high-level integration test for the entire engine.
        let mut player1 = create_battle_player_from_team(
            "demo_venusaur",
            "npc_trainer_1".to_string(),
            "AI Trainer Red".to_string(),
        )
        .expect("Failed to create Player 1");
        player1.player_type = PlayerType::NPC;

        let mut player2 = create_battle_player_from_team(
            "demo_charizard",
            "npc_trainer_2".to_string(),
            "AI Trainer Blue".to_string(),
        )
        .expect("Failed to create Player 2");
        player2.player_type = PlayerType::NPC;

        let mut battle_state = BattleState::new("full_battle_test".to_string(), player1, player2);
        let mut turn_limit = 100; // Safety break to prevent infinite loops in tests

        // Act: Run the battle loop until a winner is decided or the turn limit is reached.
        while !matches!(
            battle_state.game_state,
            GameState::Player1Win | GameState::Player2Win | GameState::Draw
        ) && turn_limit > 0
        {
            // Collect actions for any players that need them (NPCs, or players who need to switch).
            let npc_actions = collect_npc_actions(&battle_state);
            for (player_index, action) in npc_actions {
                battle_state.action_queue[player_index] = Some(action);
            }

            // If the battle is ready, resolve the turn.
            if ready_for_turn_resolution(&battle_state) {
                let rng = TurnRng::new_random();
                let event_bus = resolve_turn(&mut battle_state, rng);

                // As per our testing standard, log the events for this turn for clarity.
                event_bus.print_debug_with_message(&format!(
                    "--- Events for Turn {} ---",
                    battle_state.turn_number - 1
                ));
            }

            turn_limit -= 1;
        }

        // Assert
        println!("\n--- Battle Finished ---");
        println!("Final Game State: {:?}", battle_state.game_state);
        println!("Total Turns: {}", battle_state.turn_number);

        // The primary assertion is that the battle reached a valid conclusion.
        // It didn't get stuck in a state like WaitingForActions indefinitely.
        assert_ne!(
            battle_state.game_state,
            GameState::WaitingForActions,
            "Battle should have concluded and not be stuck waiting for actions."
        );
        assert_ne!(
            battle_state.game_state,
            GameState::TurnInProgress,
            "Battle should have concluded and not be stuck in the middle of a turn."
        );
        assert!(
            turn_limit > 0,
            "Battle failed to complete within the turn limit (100 turns)"
        );
    }
}
