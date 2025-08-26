// In: src/battle/tests/test_immunity.rs

#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::BattleEvent;
    use crate::battle::tests::common::{create_test_battle, predictable_rng, TestPokemonBuilder};
    use crate::player::PlayerAction;
    use crate::species::Species;
    use rstest::rstest;
    use schema::Move;

    #[rstest]
    #[case(
        "Immune target (Body Slam vs Gastly)",
        Move::BodySlam,      // Normal-type move with 30% paralysis chance
        Species::Gastly,     // Ghost-type, immune to Normal
        false                // Should NOT be paralyzed
    )]
    #[case(
        "Non-immune target (Ember vs Gastly)",
        Move::Ember,         // Fire-type move with 10% burn chance
        Species::Gastly,     // Ghost/Poison-type, not immune to Fire
        true                 // SHOULD be burned
    )]
    fn test_immunity_blocks_secondary_effects(
        #[case] desc: &str,
        #[case] attacking_move: Move,
        #[case] defender_species: Species,
        #[case] expect_effect: bool,
    ) {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Snorlax, 50)
            .with_moves(vec![attacking_move])
            .build();
        let defender = TestPokemonBuilder::new(defender_species, 50)
            .with_moves(vec![Move::Splash])
            .build();
        let mut battle_state = create_test_battle(attacker, defender);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message(&format!(
            "Events for test_immunity_blocks_secondary_effects [{}]:",
            desc
        ));

        let status_applied = event_bus.events().iter().any(|e| {
            matches!(e, BattleEvent::PokemonStatusApplied { target, .. } if *target == defender_species)
        });

        if expect_effect {
            assert!(
                status_applied,
                "A secondary status effect should have been applied"
            );
            assert!(
                battle_state.players[1]
                    .active_pokemon()
                    .unwrap()
                    .status
                    .is_some(),
                "Defender's status should not be None"
            );
        } else {
            // Key assertion: For the immune case, no status should be applied.
            assert!(
                !status_applied,
                "A secondary status effect should NOT have been applied to an immune target"
            );
            assert!(
                battle_state.players[1]
                    .active_pokemon()
                    .unwrap()
                    .status
                    .is_none(),
                "Defender's status should remain None"
            );

            // Also verify the "no effect" message was sent.
            let no_effect_event = event_bus.events().iter().any(|e| {
                matches!(e, BattleEvent::AttackTypeEffectiveness { multiplier } if *multiplier < 0.01)
            });
            assert!(no_effect_event, "The 'no effect' event should have been emitted for the immune interaction");
        }
    }
}