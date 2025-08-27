#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{BattleEvent, TurnRng};
    use crate::battle::tests::common::{create_test_battle, predictable_rng, TestPokemonBuilder};
    use crate::player::PlayerAction;
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use schema::Move;

    /// Helper to run a turn and extract damage dealt to Player 2.
    fn get_damage_from_turn(
        attacker_pokemon: crate::pokemon::PokemonInst,
        defender_pokemon: crate::pokemon::PokemonInst,
        attacker_move_idx: usize,
        defender_move_idx: usize,
        rng: TurnRng,
    ) -> u16 {
        let mut battle_state = create_test_battle(attacker_pokemon, defender_pokemon);
        battle_state.action_queue[0] = Some(PlayerAction::UseMove {
            move_index: attacker_move_idx,
        });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove {
            move_index: defender_move_idx,
        });

        let event_bus = resolve_turn(&mut battle_state, rng);
        event_bus
            .events()
            .iter()
            .find_map(|event| match event {
                BattleEvent::DamageDealt { damage, .. } => Some(*damage),
                _ => None,
            })
            .unwrap_or(0)
    }

    #[rstest]
    #[case(
        "P1 Attack increase (Swords Dance)",
        Move::SwordsDance,
        Move::Tackle,
        true
    )]
    #[case("P2 Defense decrease (Tail Whip)", Move::TailWhip, Move::Tackle, true)]
    #[case("P1 Attack decrease (Growl)", Move::Tackle, Move::Growl, false)]
    #[case("P2 Defense increase (Harden)", Move::Tackle, Move::Harden, false)]
    fn test_physical_stat_modifiers_affect_damage(
        #[case] desc: &str,
        #[case] p1_move: Move,
        #[case] p2_move: Move,
        #[case] expect_increase: bool,
    ) {
        // Arrange - Baseline damage calculation
        let p1_baseline = TestPokemonBuilder::new(Species::Machop, 20)
            .with_moves(vec![Move::Tackle])
            .build();
        let p2_baseline = TestPokemonBuilder::new(Species::Geodude, 20)
            .with_moves(vec![Move::Tackle])
            .build();

        let baseline_damage = get_damage_from_turn(
            p1_baseline,
            p2_baseline,
            0,
            0,
            TurnRng::new_for_test(vec![50; 20]),
        );

        // Arrange - Setup turn with stat modifiers
        let p1_modified = TestPokemonBuilder::new(Species::Machop, 20)
            .with_moves(vec![Move::Tackle, p1_move])
            .build();
        let p2_modified = TestPokemonBuilder::new(Species::Geodude, 20)
            .with_moves(vec![Move::Tackle, p2_move])
            .build();
        let mut battle_state = create_test_battle(p1_modified, p2_modified);

        // Act - First turn: Apply stat modifications
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 1 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 1 });
        let setup_event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Debug: Print setup events to see what stat changes occurred
        setup_event_bus.print_debug_with_message(&format!(
            "Setup events for test_physical_stat_modifiers_affect_damage [{}]:",
            desc
        ));

        // Act - Second turn: Test damage with modifications (P1 attacks P2)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // P1 uses Tackle
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // P2 uses Tackle
        let event_bus = resolve_turn(&mut battle_state, TurnRng::new_for_test(vec![50; 20]));

        // Find damage dealt by P1 to P2 (look for the first damage event)
        let modified_damage = event_bus
            .events()
            .iter()
            .find_map(|event| match event {
                BattleEvent::DamageDealt { damage, .. } => Some(*damage),
                _ => None,
            })
            .unwrap_or(0);
        // Assert
        event_bus.print_debug_with_message(&format!(
            "Events for test_physical_stat_modifiers_affect_damage [{}]:",
            desc
        ));

        if expect_increase {
            assert!(
                modified_damage > baseline_damage,
                "[{}] Damage should have increased: {} -> {}",
                desc,
                baseline_damage,
                modified_damage
            );
        } else {
            assert!(
                modified_damage < baseline_damage,
                "[{}] Damage should have decreased: {} -> {}",
                desc,
                baseline_damage,
                modified_damage
            );
        }
    }

    #[rstest]
    #[case("Accuracy decrease causes miss", -1, 0, 75, true)] // -1 accuracy stage, 75 roll should miss Slam (75% base)
    #[case("Evasion increase causes miss", 0, 1, 75, true)] // +1 evasion stage, 75 roll should miss Slam
    #[case("No modifiers, move hits", 0, 0, 75, false)] // No modifiers, 75 roll should hit Slam
    fn test_accuracy_evasion_modifiers(
        #[case] desc: &str,
        #[case] accuracy_stage: i8,
        #[case] evasion_stage: i8,
        #[case] hit_roll: u8,
        #[case] expect_miss: bool,
    ) {
        // Arrange - Create battle with Pokemon that can use Slam (75% accuracy)
        let p1 = TestPokemonBuilder::new(Species::Pikachu, 20)
            .with_moves(vec![Move::Slam])
            .build();
        let p2 = TestPokemonBuilder::new(Species::Charmander, 20)
            .with_moves(vec![Move::Tackle])
            .build();
        let mut battle_state = create_test_battle(p1, p2);

        // Manually apply stat stage changes to test their effect
        if accuracy_stage != 0 {
            battle_state.players[0].set_stat_stage(schema::StatType::Acc, accuracy_stage);
        }
        if evasion_stage != 0 {
            battle_state.players[1].set_stat_stage(schema::StatType::Eva, evasion_stage);
        }

        // Act - Test move with controlled RNG
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Slam
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        let event_bus = resolve_turn(
            &mut battle_state,
            TurnRng::new_for_test(vec![hit_roll, 50, 90, 50, 50, 50]),
        );

        // Assert
        event_bus.print_debug_with_message(&format!(
            "Events for test_accuracy_evasion_modifiers [{}]:",
            desc
        ));

        let slam_hit = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::MoveHit {
                    move_used: Move::Slam,
                    ..
                }
            )
        });
        let slam_missed = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::MoveMissed {
                    move_used: Move::Slam,
                    ..
                }
            )
        });

        if expect_miss {
            assert!(
                slam_missed && !slam_hit,
                "[{}] Slam should have missed",
                desc
            );
        } else {
            assert!(slam_hit && !slam_missed, "[{}] Slam should have hit", desc);
        }
    }

    #[test]
    fn test_speed_modifier_changes_turn_order() {
        // Arrange - Use Pokemon with closer base speeds so Agility can overcome the difference
        // Wartortle (58 speed) vs Ivysaur (60 speed) - very close
        let p1_slower = TestPokemonBuilder::new(Species::Wartortle, 50)
            .with_moves(vec![Move::Tackle, Move::Agility])
            .build();
        let p2_faster = TestPokemonBuilder::new(Species::Ivysaur, 50)
            .with_moves(vec![Move::Tackle])
            .build();
        let mut battle_state = create_test_battle(p1_slower, p2_faster);

        // Act - Turn 1: Establish baseline order
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        let bus1 = resolve_turn(&mut battle_state, predictable_rng());

        let move_order1: Vec<usize> = bus1
            .events()
            .iter()
            .filter_map(|e| match e {
                BattleEvent::MoveUsed { player_index, .. } => Some(*player_index),
                _ => None,
            })
            .collect();
        assert_eq!(
            move_order1,
            vec![1, 0],
            "Turn 1: Faster Pokémon (Ivysaur) should move first"
        );

        // Act - Turn 2: Wartortle uses Agility
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 1 }); // Agility
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        let bus2 = resolve_turn(&mut battle_state, predictable_rng());
        bus2.print_debug_with_message(
            "Events for test_speed_modifier_changes_turn_order (Turn 2 - Agility):",
        );

        // Act - Turn 3: Test new turn order
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        let bus3 = resolve_turn(&mut battle_state, predictable_rng());

        let move_order3: Vec<usize> = bus3
            .events()
            .iter()
            .filter_map(|e| match e {
                BattleEvent::MoveUsed { player_index, .. } => Some(*player_index),
                _ => None,
            })
            .collect();

        // Assert
        bus1.print_debug_with_message(
            "Events for test_speed_modifier_changes_turn_order (Turn 1):",
        );
        bus3.print_debug_with_message(
            "Events for test_speed_modifier_changes_turn_order (Turn 3):",
        );

        assert_eq!(
            move_order3,
            vec![0, 1],
            "Turn 3: Wartortle should now be faster and move first"
        );
    }
}
