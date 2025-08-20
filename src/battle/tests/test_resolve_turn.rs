#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{BattleEvent, GameState};
    use crate::battle::tests::common::{TestPokemonBuilder, create_test_battle, predictable_rng};
    use crate::moves::Move;
    use crate::player::PlayerAction;
    use crate::species::Species;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_resolve_turn_basic_speed_order() {
        // Arrange
        // Create a higher-level Pikachu to ensure it's faster than Charmander
        let pikachu = TestPokemonBuilder::new(Species::Pikachu, 12)
            .with_moves(vec![Move::Tackle])
            .build();
        let charmander = TestPokemonBuilder::new(Species::Charmander, 10)
            .with_moves(vec![Move::Scratch])
            .build();
        let mut battle_state = create_test_battle(pikachu, charmander);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Pikachu's action
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Charmander's action

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_resolve_turn_basic_speed_order:");

        // --- Verify Turn Order from Events ---
        let move_order: Vec<usize> = event_bus
            .events()
            .iter()
            .filter_map(|e| match e {
                BattleEvent::MoveUsed { player_index, .. } => Some(*player_index),
                _ => None,
            })
            .collect();

        assert_eq!(
            move_order,
            vec![0, 1],
            "The faster Pokémon (Player 0) should always act before the slower one (Player 1)"
        );

        // --- Verify Final State ---
        assert_eq!(battle_state.turn_number, 2, "Turn number should increment");
        assert_eq!(
            battle_state.game_state,
            GameState::WaitingForActions,
            "Game state should be ready for the next turn"
        );
        assert!(
            battle_state.action_queue[0].is_none() && battle_state.action_queue[1].is_none(),
            "Action queue should be cleared after the turn is resolved"
        );
    }
}
