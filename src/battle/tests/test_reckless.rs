#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{BattleEvent, TurnRng};
    use crate::battle::tests::common::{create_test_battle, TestPokemonBuilder};
    use crate::moves::Move;
    use crate::player::PlayerAction;
    use crate::species::Species;
    use rstest::rstest;

    #[rstest]
    #[case("HighJumpKick misses and causes 50% recoil", Move::HighJumpKick, 95, 50, false)] // HJK accuracy is 80, so 95 is a miss
    #[case("HighJumpKick hits and causes no recoil", Move::HighJumpKick, 50, 0, false)]      // 50 is a hit
    #[case("JumpKick misses and causes 20% recoil", Move::JumpKick, 95, 20, false)]         // JumpKick accuracy is 90, so 95 is a miss
    #[case("HighJumpKick misses and causes fainting", Move::HighJumpKick, 95, 50, true)]   // Test recoil fainting
    fn test_reckless_move_outcomes(
        #[case] desc: &str,
        #[case] reckless_move: Move,
        #[case] accuracy_rng: u8,
        #[case] expected_recoil_percent: u16,
        #[case] should_faint: bool,
    ) {
        // Arrange
        let mut p1_builder = TestPokemonBuilder::new(Species::Hitmonlee, 50).with_moves(vec![reckless_move]);
        let p2_pokemon = TestPokemonBuilder::new(Species::Snorlax, 50).with_moves(vec![Move::Tackle]).build();

        if should_faint {
            // ** FIX START **
            // Create a separate, temporary builder to calculate max_hp without consuming the main one.
            let template_pokemon = TestPokemonBuilder::new(Species::Hitmonlee, 50).build();
            let max_hp = template_pokemon.max_hp();
            let recoil_damage = (max_hp * expected_recoil_percent) / 100;
            
            // Now, modify the original builder. This is safe because it hasn't been moved yet.
            p1_builder = p1_builder.with_hp(recoil_damage);
            // ** FIX END **
        }

        let mut battle_state = create_test_battle(p1_builder.build(), p2_pokemon);
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        let test_rng = TurnRng::new_for_test(vec![accuracy_rng, 50, 90, 50, 90, 85]);

        // Act
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Assert
        event_bus.print_debug_with_message(&format!("Events for test_reckless_move_outcomes [{}]:", desc));

        let move_missed = event_bus.events().iter().any(|e| matches!(e, BattleEvent::MoveMissed { attacker: Species::Hitmonlee, .. }));
        let recoil_damage_event = event_bus.events().iter().find(|e| matches!(e, BattleEvent::DamageDealt { target: Species::Hitmonlee, .. }));
        let fainted_event = event_bus.events().iter().any(|e| matches!(e, BattleEvent::PokemonFainted { player_index: 0, .. }));

        if expected_recoil_percent > 0 {
            assert!(move_missed, "The reckless move should have missed");
            assert!(recoil_damage_event.is_some(), "Recoil damage should have been dealt to the attacker");

            if should_faint {
                assert!(fainted_event, "Attacker should have fainted from recoil damage");
            } else {
                assert!(!fainted_event, "Attacker should not have fainted from recoil damage");
            }
        } else {
            assert!(!move_missed, "The reckless move should have hit");
        }
    }

    #[test]
    fn test_non_reckless_move_has_no_recoil_on_miss() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Hitmonlee, 10).with_moves(vec![Move::Tackle]).build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Snorlax, 10).with_moves(vec![Move::TailWhip]).build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tail Whip

        // Force a miss with a high RNG roll (Tackle's accuracy is 90)
        let test_rng = TurnRng::new_for_test(vec![95, 50, 90, 85]);

        // Act
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Assert
        event_bus.print_debug_with_message("Events for test_non_reckless_move_has_no_recoil_on_miss:");

        let move_missed = event_bus.events().iter().any(|e| matches!(e, BattleEvent::MoveMissed { attacker: Species::Hitmonlee, move_used: Move::Tackle, .. }));
        assert!(move_missed, "Tackle should have missed");

        // The key assertion: no damage should be dealt to the attacker (Hitmonlee) because Tackle is not a reckless move.
        let recoil_damage_event = event_bus.events().iter().any(|e| matches!(e, BattleEvent::DamageDealt { target: Species::Hitmonlee, .. }));
        assert!(!recoil_damage_event, "Non-reckless moves should not cause recoil damage on miss");
    }
}