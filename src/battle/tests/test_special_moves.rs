#[cfg(test)]
mod tests {
    use crate::battle::conditions::{PokemonCondition, PokemonConditionType};
    use crate::battle::engine::{collect_npc_actions, resolve_turn};
    use crate::battle::state::{BattleEvent, TurnRng};
    use crate::battle::tests::common::{
        create_test_battle, create_test_player, predictable_rng, TestPokemonBuilder,
    };
    use crate::player::PlayerAction;
    use crate::pokemon::PokemonType;
    use crate::species::Species;
    use pokemon_adventure_schema::Move;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    // --- Integration Tests for Multi-Turn and Forced Moves ---

    #[rstest]
    #[case(
        "Charging (SolarBeam)",
        Move::SolarBeam,
        PokemonConditionType::Charging,
        Species::Venusaur,
        Species::Charizard
    )]
    #[case(
        "InAir (Fly)",
        Move::Fly,
        PokemonConditionType::InAir,
        Species::Pidgeot,
        Species::Rattata
    )]
    #[case(
        "Underground (Dig)",
        Move::Dig,
        PokemonConditionType::Underground,
        Species::Sandslash,
        Species::Geodude
    )]
    fn test_two_turn_moves(
        #[case] desc: &str,
        #[case] two_turn_move: Move,
        #[case] expected_condition: PokemonConditionType,
        #[case] attacker_species: Species,
        #[case] defender_species: Species,
    ) {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(attacker_species, 25)
            .with_moves(vec![two_turn_move])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(defender_species, 25)
            .with_moves(vec![Move::Tackle])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        // --- TURN 1: Initiate the move ---
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act - Turn 1
        let bus1 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 1
        bus1.print_debug_with_message(&format!("Events for {} [Turn 1 - Initiate]:", desc));
        assert!(
            battle_state.players[0].has_condition_type(expected_condition),
            "Player should have the correct two-turn condition"
        );
        assert_eq!(battle_state.players[0].last_move, Some(two_turn_move));
        assert!(
            battle_state.action_queue[0].is_some(),
            "Action queue should be pre-filled for Turn 2"
        );

        // --- TURN 2: Execute the move ---
        let npc_actions = collect_npc_actions(&battle_state);
        for (i, action) in npc_actions {
            battle_state.action_queue[i] = Some(action);
        }

        // Act - Turn 2
        let bus2 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 2
        bus2.print_debug_with_message(&format!("Events for {} [Turn 2 - Execute]:", desc));
        assert!(
            !battle_state.players[0].has_condition_type(expected_condition),
            "Two-turn condition should be cleared after execution"
        );
        let move_executed = bus2.events().iter().any(|e| matches!(e, BattleEvent::MoveUsed { player_index: 0, move_used, .. } if *move_used == two_turn_move));
        let damage_dealt = bus2.events().iter().any(
            |e| matches!(e, BattleEvent::DamageDealt { target, .. } if *target == defender_species),
        );
        assert!(move_executed, "The two-turn move should have been executed");
        assert!(damage_dealt, "The two-turn move should have dealt damage");
    }

    #[test]
    fn test_rampage_forces_action_and_ends_with_confusion() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Tauros, 25)
            .with_moves(vec![Move::Thrash])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Slowpoke, 50)
            .with_moves(vec![Move::TailWhip])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        // --- TURN 1: Initiate Rampage (force 2-turn duration with RNG) ---
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        let bus1 = resolve_turn(&mut battle_state, TurnRng::new_for_test(vec![50; 20])); // Roll <= 50 gives 2 turns

        // Assert - Turn 1
        bus1.print_debug_with_message("Events for Rampage [Turn 1 - Initiate]:");
        assert!(battle_state.players[0].has_condition_type(PokemonConditionType::Rampaging));
        assert!(
            battle_state.action_queue[0].is_some(),
            "Action queue should be pre-filled for Turn 2"
        );

        // --- TURN 2: Forced continuation ---
        collect_npc_actions(&battle_state)
            .into_iter()
            .for_each(|(i, a)| battle_state.action_queue[i] = Some(a));
        let bus2 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 2
        bus2.print_debug_with_message("Events for Rampage [Turn 2 - Final Turn]:");
        assert!(bus2.events().iter().any(|e| matches!(
            e,
            BattleEvent::MoveUsed {
                player_index: 0,
                move_used: Move::Thrash,
                ..
            }
        )));

        // After this turn, the rampage ends and confusion should be applied.
        assert!(
            !battle_state.players[0].has_condition_type(PokemonConditionType::Rampaging),
            "Rampaging should be cleared"
        );
        assert!(
            battle_state.players[0].has_condition_type(PokemonConditionType::Confused),
            "Should become confused after rampage"
        );
        assert!(bus2.events().iter().any(|e| matches!(
            e,
            BattleEvent::StatusApplied {
                status: PokemonCondition::Confused { .. },
                ..
            }
        )));
    }

    // --- Tests for Other Special Moves ---

    #[rstest]
    #[case("succeeds with valid last move", Some(Move::Tackle), true)]
    #[case("fails with no last move", None, false)]
    #[case("fails when copying Mirror Move", Some(Move::MirrorMove), false)]
    fn test_mirror_move_outcomes(
        #[case] desc: &str,
        #[case] opponent_last_move: Option<Move>,
        #[case] should_succeed: bool,
    ) {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Pidgeot, 25)
            .with_moves(vec![Move::MirrorMove])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Tackle])
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        let mut player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);
        player2.last_move = opponent_last_move; // Set the condition for the test

        let mut battle_state =
            crate::battle::state::BattleState::new("test".to_string(), player1, player2);
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus
            .print_debug_with_message(&format!("Events for test_mirror_move_outcomes [{}]:", desc));

        let move_failed = event_bus
            .events()
            .iter()
            .any(|e| matches!(e, BattleEvent::ActionFailed { .. }));
        let mirrored_move_used = event_bus.events().iter().any(|e| {
            matches!(e, BattleEvent::MoveUsed { player_index: 0, pokemon: Species::Pidgeot, move_used } if *move_used != Move::MirrorMove)
        });

        if should_succeed {
            assert!(!move_failed, "Mirror Move should not have failed");
            assert!(
                mirrored_move_used,
                "The mirrored move should have been used"
            );
        } else {
            assert!(move_failed, "Mirror Move should have failed");
            assert!(!mirrored_move_used, "No other move should have been used");
        }
    }

    #[test]
    fn test_explosion_faints_user_and_deals_damage() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Electrode, 50)
            .with_moves(vec![Move::Explosion])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Snorlax, 50)
            .with_moves(vec![Move::Rest])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus
            .print_debug_with_message("Events for test_explosion_faints_user_and_deals_damage:");
        assert!(
            battle_state.players[0]
                .active_pokemon()
                .unwrap()
                .is_fainted(),
            "User should have fainted from Explosion"
        );
        let faint_event = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::PokemonFainted {
                    player_index: 0,
                    ..
                }
            )
        });
        let damage_event = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::DamageDealt {
                    target: Species::Snorlax,
                    ..
                }
            )
        });

        assert!(
            faint_event,
            "A fainted event should be present for the user"
        );
        assert!(
            damage_event,
            "A damage event should be present for the target"
        );
    }

    // --- Unit Tests for Special Condition Effects ---

    #[test]
    fn test_transformed_and_converted_conditions_unit() {
        // Arrange
        let ditto = TestPokemonBuilder::new(Species::Ditto, 50).build();
        let charizard = TestPokemonBuilder::new(Species::Charizard, 50).build();
        let mut player = create_test_player("p1", "Player 1", vec![ditto]);

        // Test Transformed
        player.add_condition(PokemonCondition::Transformed { target: charizard });
        let transformed_types = player.active_pokemon().unwrap().get_current_types(&player);
        assert_eq!(
            transformed_types,
            vec![PokemonType::Fire, PokemonType::Flying],
            "Should have Charizard's types"
        );

        // Test Converted (should override Transformed)
        player.add_condition(PokemonCondition::Converted {
            pokemon_type: PokemonType::Electric,
        });
        let converted_types = player.active_pokemon().unwrap().get_current_types(&player);
        assert_eq!(
            converted_types,
            vec![PokemonType::Electric],
            "Converted should override Transform"
        );
    }

    // --- Integration Tests for Special Conditions ---

    #[rstest]
    #[case("damage", Move::Lightning, PokemonConditionType::Substitute, false)] // Substitute should block damage
    #[case(
        "status effect",
        Move::ThunderWave,
        PokemonConditionType::Substitute,
        false
    )] // Substitute should block status
    #[case(
        "stat decrease",
        Move::SandAttack,
        PokemonConditionType::Substitute,
        false
    )] // Substitute should block stat drops
    #[case(
        "active condition",
        Move::ConfuseRay,
        PokemonConditionType::Substitute,
        false
    )] // Substitute should block confusion
    fn test_substitute_blocks_effects(
        #[case] desc: &str,
        #[case] incoming_move: Move,
        #[case] _condition_type: PokemonConditionType, // Placeholder for future conditions
        #[case] _should_succeed: bool,
    ) {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![incoming_move])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Alakazam, 25)
            .with_moves(vec![Move::Tackle])
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        let mut player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);
        player2.add_condition(PokemonCondition::Substitute { hp: 25 }); // Add the substitute

        let mut battle_state =
            crate::battle::state::BattleState::new("test".to_string(), player1, player2);
        let original_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message(&format!(
            "Events for test_substitute_blocks_effects [{}]:",
            desc
        ));

        // Check that the Pokémon behind the sub took no damage and received no status/conditions.
        let final_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();
        let final_status = battle_state.players[1].active_pokemon().unwrap().status;
        // Check conditions based on whether substitute was destroyed or not
        let substitute_destroyed = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::SubstituteDamaged {
                    substitute_destroyed: true,
                    ..
                }
            )
        });
        let expected_conditions = if substitute_destroyed { 0 } else { 1 }; // 0 if destroyed, 1 if substitute remains
        let final_conditions_count = battle_state.players[1].active_pokemon_conditions.len();
        let final_conditions_correct = final_conditions_count == expected_conditions;

        assert_eq!(
            final_hp, original_hp,
            "HP of Pokémon behind substitute should not change"
        );
        assert!(
            final_status.is_none(),
            "Status should not be applied to a Pokémon behind a substitute"
        );
        assert!(
            final_conditions_correct,
            "Active conditions should not be applied to a Pokémon behind a substitute"
        );
    }
}
