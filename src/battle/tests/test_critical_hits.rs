#[cfg(test)]
mod tests {
    use crate::battle::engine::{collect_npc_actions, resolve_turn};
    use crate::battle::state::{BattleEvent, TurnRng};
    use crate::battle::tests::common::{TestPokemonBuilder, create_test_battle};
    use crate::moves::Move;
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(
        "guaranteed critical hits",
        // Force hit (low roll), force crit (low roll), then damage variance
        // This is repeated for the second Pokémon's turn.
        vec![10, 2, 90, 10, 2, 90], 
        true, // Expect at least one critical hit
        false // Expect no misses
    )]
    #[case(
        "guaranteed misses",
        // High rolls will cause moves with accuracy <= 99 to miss.
        vec![99, 99, 99, 99, 99, 99],
        false, // Expect no critical hits
        true // Expect at least one miss
    )]
    fn test_hit_outcomes(
        #[case] desc: &str,
        #[case] rng_values: Vec<u8>,
        #[case] expect_crit: bool,
        #[case] expect_miss: bool,
    ) {
        // Arrange
        let pokemon1 = TestPokemonBuilder::new(Species::Pikachu, 10)
            .with_moves(vec![Move::Tackle])
            .build();
        let pokemon2 = TestPokemonBuilder::new(Species::Charmander, 10)
            .with_moves(vec![Move::Scratch])
            .build();
        let mut battle_state = create_test_battle(pokemon1, pokemon2);

        // Let the AI decide on the actions for both players.
        let npc_actions = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions {
            battle_state.action_queue[player_index] = Some(action);
        }

        let test_rng = TurnRng::new_for_test(rng_values);

        // Act
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Assert
        event_bus.print_debug_with_message(&format!("Events for test_hit_outcomes [{}]:", desc));

        let has_crit = event_bus
            .events()
            .iter()
            .any(|e| matches!(e, BattleEvent::CriticalHit { .. }));
        let has_miss = event_bus
            .events()
            .iter()
            .any(|e| matches!(e, BattleEvent::MoveMissed { .. }));

        assert_eq!(has_crit, expect_crit, "Critical hit expectation mismatch");
        assert_eq!(has_miss, expect_miss, "Miss expectation mismatch");
    }
}
