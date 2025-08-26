#[cfg(test)]
mod tests {
    use crate::battle::action_stack::ActionStack;
    use crate::battle::engine::execute_attack_hit;
    use crate::battle::state::{BattleEvent, EventBus, TurnRng};
    use crate::battle::tests::common::{create_test_battle, TestPokemonBuilder};
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use schema::Move;

    #[test]
    fn test_high_crit_move_effect() {
        // Arrange: Slash has a high critical hit ratio.
        let attacker = TestPokemonBuilder::new(Species::Scyther, 10)
            .with_moves(vec![Move::Slash])
            .build();
        let defender = TestPokemonBuilder::new(Species::Pidgey, 10).build();
        let mut battle_state = create_test_battle(attacker, defender);

        let mut bus = EventBus::new();
        // Force a hit (roll 10) and a critical hit (roll 1)
        let mut rng = TurnRng::new_for_test(vec![10, 1, 90]);
        let mut action_stack = ActionStack::new();

        // Act
        execute_attack_hit(
            0,
            1,
            Move::Slash,
            0,
            &mut action_stack,
            &mut bus,
            &mut rng,
            &mut battle_state,
        );

        // Assert
        bus.print_debug_with_message("Events for test_high_crit_move_effect:");
        let has_crit = bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::CriticalHit {
                    move_used: Move::Slash,
                    ..
                }
            )
        });
        assert!(
            has_crit,
            "Slash with a low RNG roll should result in a critical hit"
        );
    }

    #[test]
    fn test_recoil_effect() {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Tauros, 10)
            .with_moves(vec![Move::DoubleEdge])
            .build();
        let defender = TestPokemonBuilder::new(Species::Pidgey, 10).build();
        let mut battle_state = create_test_battle(attacker, defender);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50, 60, 90, 80]); // Rolls to ensure a hit
        let mut action_stack = ActionStack::new();

        // Act
        execute_attack_hit(
            0,
            1,
            Move::DoubleEdge,
            0,
            &mut action_stack,
            &mut bus,
            &mut rng,
            &mut battle_state,
        );

        // Assert
        bus.print_debug_with_message("Events for test_recoil_effect:");
        let damage_to_defender = bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::DamageDealt {
                    target: Species::Pidgey,
                    ..
                }
            )
        });
        let recoil_to_attacker = bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::DamageDealt {
                    target: Species::Tauros,
                    ..
                }
            )
        });

        assert!(
            damage_to_defender,
            "Should have dealt damage to the defender"
        );
        assert!(
            recoil_to_attacker,
            "Should have dealt recoil damage to the attacker"
        );
    }

    #[test]
    fn test_drain_effect() {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Victreebel, 10)
            .with_moves(vec![Move::MegaDrain])
            .with_hp(30) // Damaged state
            .build();
        let defender = TestPokemonBuilder::new(Species::Bulbasaur, 10).build();
        let mut battle_state = create_test_battle(attacker, defender);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50, 60, 90, 80]); // Rolls to ensure a hit
        let mut action_stack = ActionStack::new();

        // Act
        execute_attack_hit(
            0,
            1,
            Move::MegaDrain,
            0,
            &mut action_stack,
            &mut bus,
            &mut rng,
            &mut battle_state,
        );

        // Assert
        bus.print_debug_with_message("Events for test_drain_effect:");
        let damage_to_defender = bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::DamageDealt {
                    target: Species::Bulbasaur,
                    ..
                }
            )
        });
        let healing_to_attacker = bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::PokemonHealed {
                    target: Species::Victreebel,
                    ..
                }
            )
        });

        assert!(
            damage_to_defender,
            "Should have dealt damage to the defender"
        );
        assert!(
            healing_to_attacker,
            "Should have applied healing to the attacker"
        );
    }

    #[test]
    fn test_no_effects_without_damage() {
        // Arrange: Ghost-type Gastly is immune to Normal-type Double-Edge.
        let attacker = TestPokemonBuilder::new(Species::Machamp, 10)
            .with_moves(vec![Move::DoubleEdge])
            .build();
        let defender = TestPokemonBuilder::new(Species::Gastly, 10).build();
        let mut battle_state = create_test_battle(attacker, defender);

        let initial_attacker_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50, 60, 70]);
        let mut action_stack = ActionStack::new();

        // Act
        execute_attack_hit(
            0,
            1,
            Move::DoubleEdge,
            0,
            &mut action_stack,
            &mut bus,
            &mut rng,
            &mut battle_state,
        );

        // Assert
        bus.print_debug_with_message("Events for test_no_effects_without_damage:");

        let final_attacker_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();
        assert_eq!(
            final_attacker_hp, initial_attacker_hp,
            "Attacker should not take recoil damage when its move has no effect"
        );

        let had_no_effect = bus.events().iter().any(|e| {
            matches!(e, BattleEvent::AttackTypeEffectiveness { multiplier } if *multiplier < 0.1)
        });
        let recoil_to_attacker = bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::DamageDealt {
                    target: Species::Machamp,
                    ..
                }
            )
        });

        assert!(
            had_no_effect,
            "The move should have been announced as having no effect"
        );
        assert!(
            !recoil_to_attacker,
            "Should be no DamageDealt event for the attacker as no recoil occurred"
        );
    }
}
