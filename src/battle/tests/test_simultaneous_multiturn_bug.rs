#[cfg(test)]
mod tests {
    use crate::battle::conditions::PokemonConditionType;
    use crate::battle::engine::{collect_npc_actions, resolve_turn};
    use crate::battle::state::BattleState;
    use crate::battle::tests::common::{create_test_player, predictable_rng, TestPokemonBuilder};
    use crate::moves::Move;
    use crate::player::{PlayerAction, PlayerType};
    use crate::species::Species;

    #[test]
    fn test_simultaneous_multiturn_moves_resolve_correctly() {
        // This integration test verifies that the "End-of-Turn Injection" model
        // correctly handles a scenario where both players initiate a multi-turn move
        // simultaneously, preventing a deadlock.

        // Arrange - Turn 1
        let p1_pokemon = TestPokemonBuilder::new(Species::Venusaur, 50)
            .with_moves(vec![Move::SolarBeam])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Charizard, 50)
            .with_moves(vec![Move::Fly])
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        let mut player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);
        player2.player_type = PlayerType::NPC; // Ensure AI can act if needed

        let mut battle_state =
            BattleState::new("test_simultaneous_multiturn".to_string(), player1, player2);

        // --- TURN 1: Both players initiate their multi-turn moves ---
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // SolarBeam
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Fly

        // Act - Turn 1
        let bus_turn1 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 1
        bus_turn1.print_debug_with_message("Events for Turn 1 (Initiating Moves):");
        assert!(
            battle_state.players[0].has_condition_type(PokemonConditionType::Charging),
            "Player 1 should be in the Charging state after using Solar Beam"
        );
        assert!(
            battle_state.players[1].has_condition_type(PokemonConditionType::InAir),
            "Player 2 should be in the InAir state after using Fly"
        );
        assert!(
            battle_state.action_queue[0].is_some() && battle_state.action_queue[1].is_some(),
            "After turn 1, the action queue should be pre-filled with both players' forced moves for turn 2"
        );

        // --- TURN 2: Engine resolves the forced moves ---

        // Arrange - Turn 2: The action queue is already filled. Calling `collect_npc_actions` should do nothing.
        let npc_actions_turn2 = collect_npc_actions(&battle_state);
        assert!(
            npc_actions_turn2.is_empty(),
            "AI should not select an action when its queue slot is already filled."
        );

        // Act - Turn 2
        let bus_turn2 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 2
        bus_turn2.print_debug_with_message("Events for Turn 2 (Executing Moves):");
        assert!(
            !battle_state.players[0].has_condition_type(PokemonConditionType::Charging),
            "Charging condition should be cleared after Solar Beam executes"
        );
        assert!(
            !battle_state.players[1].has_condition_type(PokemonConditionType::InAir),
            "InAir condition should be cleared after Fly executes"
        );

        let solar_beam_executed = bus_turn2.events().iter().any(|e| {
            matches!(
                e,
                crate::battle::state::BattleEvent::MoveUsed {
                    move_used: Move::SolarBeam,
                    ..
                }
            )
        });
        let fly_executed = bus_turn2.events().iter().any(|e| {
            matches!(
                e,
                crate::battle::state::BattleEvent::MoveUsed {
                    move_used: Move::Fly,
                    ..
                }
            )
        });

        assert!(
            solar_beam_executed,
            "Solar Beam should have executed on the second turn"
        );
        assert!(fly_executed, "Fly should have executed on the second turn");
    }
}
