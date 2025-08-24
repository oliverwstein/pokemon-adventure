#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{BattleEvent, BattleState};
    use crate::battle::tests::common::{create_test_player, predictable_rng, TestPokemonBuilder};
    use crate::moves::Move;
    use crate::player::{PlayerAction, TeamCondition};
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// A helper function to run a simple 1v1 turn and extract the damage dealt.
    fn get_damage_from_turn(
        attacker_pokemon: crate::pokemon::PokemonInst,
        defender_player: crate::player::BattlePlayer,
    ) -> (u16, crate::battle::state::EventBus) {
        let attacker_player = create_test_player("p1", "Player 1", vec![attacker_pokemon]);
        let mut battle_state =
            BattleState::new("test".to_string(), attacker_player, defender_player);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        let damage = event_bus
            .events()
            .iter()
            .find_map(|event| match event {
                BattleEvent::DamageDealt { damage, .. } => Some(*damage),
                _ => None,
            })
            .unwrap_or(0); // Default to 0 if no damage was dealt

        (damage, event_bus)
    }

    #[rstest]
    #[case(
        "Reflect reduces Physical damage",
        Move::Tackle,
        Species::Alakazam,
        Some(TeamCondition::Reflect),
        true
    )]
    #[case(
        "Light Screen reduces Special damage",
        Move::Confusion,
        Species::Machamp,
        Some(TeamCondition::LightScreen),
        true
    )]
    #[case(
        "Reflect does NOT reduce Special damage",
        Move::Confusion,
        Species::Machamp,
        Some(TeamCondition::Reflect),
        false
    )]
    #[case(
        "Light Screen does NOT reduce Physical damage",
        Move::Tackle,
        Species::Alakazam,
        Some(TeamCondition::LightScreen),
        false
    )]
    fn test_screen_effects_on_damage(
        #[case] desc: &str,
        #[case] attacking_move: Move,
        #[case] defender_species: Species,
        #[case] screen: Option<TeamCondition>,
        #[case] expect_reduction: bool,
    ) {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Machamp, 50)
            .with_moves(vec![attacking_move])
            .build();
        let defender = TestPokemonBuilder::new(defender_species, 50)
            .with_moves(vec![Move::Splash])
            .build();

        // --- Baseline Run (No Screen) ---
        let baseline_defender_player = create_test_player("p2", "Player 2", vec![defender.clone()]);
        let (baseline_damage, baseline_bus) =
            get_damage_from_turn(attacker.clone(), baseline_defender_player);

        // --- Modified Run (With Screen) ---
        let mut modified_defender_player = create_test_player("p2", "Player 2", vec![defender]);
        if let Some(s) = screen {
            modified_defender_player.add_team_condition(s, 5);
        }
        let (modified_damage, modified_bus) =
            get_damage_from_turn(attacker, modified_defender_player);

        // Assert
        println!("\n--- Test Case: {} ---", desc);
        baseline_bus.print_debug_with_message("Baseline Events:");
        modified_bus.print_debug_with_message("Modified Events:");
        println!(
            "Baseline Damage: {}, Modified Damage: {}",
            baseline_damage, modified_damage
        );

        if expect_reduction {
            assert!(
                modified_damage > 0,
                "Modified damage should be greater than 0"
            );
            assert!(
                modified_damage < baseline_damage,
                "Damage should have been reduced"
            );
            // Check if damage is roughly halved, allowing for rounding
            let expected_damage = baseline_damage / 2;
            assert!(
                (modified_damage as i16 - expected_damage as i16).abs() <= 2,
                "Damage reduction should be approx. 50%"
            );
        } else {
            assert_eq!(
                modified_damage, baseline_damage,
                "Damage should be identical as the screen has no effect"
            );
        }
    }
}
