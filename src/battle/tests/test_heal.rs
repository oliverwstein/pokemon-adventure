#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{BattleEvent};
    use crate::battle::tests::common::{create_test_battle, predictable_rng, TestPokemonBuilder};
    use crate::moves::Move;
    use crate::player::PlayerAction;
    use crate::species::Species;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_recover_heals_damaged_pokemon() {
        // Arrange
        let template_pokemon = TestPokemonBuilder::new(Species::Chansey, 10).build();
        let max_hp = template_pokemon.max_hp();
        let starting_hp = max_hp / 2; // Start at half health

        let p1_pokemon = TestPokemonBuilder::new(Species::Chansey, 10)
            .with_moves(vec![Move::Recover])
            .with_hp(starting_hp)
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Snorlax, 10)
            .with_moves(vec![Move::Tackle])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Recover
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_recover_heals_damaged_pokemon:");
        
        // Recover is a status move and should execute before Tackle.
        // We verify that a heal event occurred.
        let heal_event_found = event_bus.events().iter().any(|e| {
            matches!(e, BattleEvent::PokemonHealed { target: Species::Chansey, .. })
        });
        assert!(heal_event_found, "A PokemonHealed event should have been emitted for Chansey");

        // We also verify that Chansey still took damage from Snorlax's Tackle after healing.
        let damage_event_found = event_bus.events().iter().any(|e| {
            matches!(e, BattleEvent::DamageDealt { target: Species::Chansey, .. })
        });
        assert!(damage_event_found, "Chansey should have taken damage from Tackle after healing");
    }

    #[test]
    fn test_recover_does_not_heal_at_full_hp() {
        // Arrange: Chansey is at full HP.
        let p1_pokemon = TestPokemonBuilder::new(Species::Chansey, 10)
            .with_moves(vec![Move::Recover])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Snorlax, 10)
            .with_moves(vec![Move::Tackle])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Recover
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_recover_does_not_heal_at_full_hp:");

        // Recover, a status move, goes first. Since Chansey is at full HP, no healing occurs.
        // Therefore, no PokemonHealed event should be generated.
        let heal_event_found = event_bus.events().iter().any(|e| {
            matches!(e, BattleEvent::PokemonHealed { target: Species::Chansey, .. })
        });
        assert!(!heal_event_found, "A PokemonHealed event should NOT be emitted when Recover is used at full HP");

        // The move should still be marked as used.
        let recover_used_event = event_bus.events().iter().any(|e| {
            matches!(e, BattleEvent::MoveUsed { move_used: Move::Recover, .. })
        });
        assert!(recover_used_event, "Recover should still be registered as used");
    }

    #[test]
    fn test_recover_does_not_heal_fainted_pokemon() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Chansey, 10)
            .with_moves(vec![Move::Recover])
            .with_hp(0) // Fainted
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Snorlax, 10)
            .with_moves(vec![Move::Tackle])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Recover
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_recover_does_not_heal_fainted_pokemon:");

        // The action for the fainted Pokémon should fail.
        let action_failed_event = event_bus.events().iter().any(|e| {
            matches!(e, BattleEvent::ActionFailed { .. })
        });
        assert!(action_failed_event, "The fainted Pokémon's action should have failed");

        // No healing event should be generated.
        let heal_event_found = event_bus.events().iter().any(|e| {
            matches!(e, BattleEvent::PokemonHealed { target: Species::Chansey, .. })
        });
        assert!(!heal_event_found, "A fainted Pokémon should not be healed");

        // The final HP should still be 0.
        assert_eq!(battle_state.players[0].active_pokemon().unwrap().current_hp(), 0);
    }
}