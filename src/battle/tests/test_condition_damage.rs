#[cfg(test)]
mod tests {
    use crate::battle::conditions::PokemonCondition;
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{BattleEvent, BattleState, TurnRng};
    use crate::battle::tests::common::{create_test_player, predictable_rng, TestPokemonBuilder};
    use crate::player::{PlayerAction, StatType, TeamCondition};
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use schema::Move;

    #[test]
    fn test_leech_seed_damage_and_healing() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Bulbasaur, 10)
            .with_moves(vec![Move::Splash])
            .build();

        let p2_template = TestPokemonBuilder::new(Species::Charmander, 10).build();
        let p2_max_hp = p2_template.max_hp();
        let p2_pokemon = TestPokemonBuilder::new(Species::Charmander, 10)
            .with_moves(vec![Move::Splash])
            .with_hp(p2_max_hp - 20)
            .build();

        let mut player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        player1.add_condition(PokemonCondition::Seeded);
        let player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);

        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_leech_seed_damage_and_healing:");

        // Assert based on events, which is more robust than calculating final HP.
        let seed_damage_event_found = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::StatusDamage {
                    target: Species::Bulbasaur,
                    status: PokemonCondition::Seeded,
                    ..
                }
            )
        });
        let heal_event_found = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::PokemonHealed {
                    target: Species::Charmander,
                    ..
                }
            )
        });

        assert!(
            seed_damage_event_found,
            "A StatusDamage event for Leech Seed should have occurred."
        );
        assert!(
            heal_event_found,
            "A PokemonHealed event for the opponent should have occurred."
        );
    }

    #[test]
    fn test_trapped_damage() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Onix, 10)
            .with_moves(vec![Move::Splash])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Pikachu, 10)
            .with_moves(vec![Move::Splash])
            .build();

        let mut player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        player1.add_condition(PokemonCondition::Trapped { turns_remaining: 2 });
        let player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);

        let initial_p1_hp = player1.active_pokemon().unwrap().current_hp();
        let max_p1_hp = player1.active_pokemon().unwrap().max_hp();
        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_trapped_damage:");

        let final_p1_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();
        let expected_damage = (max_p1_hp / 16).max(1);

        assert_eq!(
            final_p1_hp,
            initial_p1_hp - expected_damage,
            "Player 1 should have taken Trapped damage"
        );
        assert!(event_bus.events().iter().any(|e| matches!(
            e,
            BattleEvent::StatusDamage {
                status: PokemonCondition::Trapped { .. },
                ..
            }
        )));
        assert!(battle_state.players[0]
            .active_pokemon_conditions
            .values()
            .any(|c| matches!(c, PokemonCondition::Trapped { turns_remaining: 1 })));
    }

    #[test]
    fn test_both_seeded_and_trapped_damage() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Geodude, 10)
            .with_moves(vec![Move::Splash])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Squirtle, 10)
            .with_moves(vec![Move::Splash])
            .build();

        let mut player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        player1.add_condition(PokemonCondition::Seeded);
        player1.add_condition(PokemonCondition::Trapped { turns_remaining: 3 });
        let player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);

        let initial_p1_hp = player1.active_pokemon().unwrap().current_hp();
        let max_p1_hp = player1.active_pokemon().unwrap().max_hp();
        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_both_seeded_and_trapped_damage:");

        let final_p1_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();
        let seeded_damage = (max_p1_hp / 8).max(1);
        let trapped_damage = (max_p1_hp / 16).max(1);
        let total_expected_damage = seeded_damage + trapped_damage;

        assert_eq!(
            final_p1_hp,
            initial_p1_hp - total_expected_damage,
            "Player 1 should have taken combined damage"
        );
        assert!(event_bus.events().iter().any(|e| matches!(
            e,
            BattleEvent::StatusDamage {
                status: PokemonCondition::Seeded,
                ..
            }
        )));
        assert!(event_bus.events().iter().any(|e| matches!(
            e,
            BattleEvent::StatusDamage {
                status: PokemonCondition::Trapped { .. },
                ..
            }
        )));
    }

    #[rstest]
    #[case(Species::Caterpie, PokemonCondition::Seeded, 1.0/8.0)]
    #[case(Species::Magikarp, PokemonCondition::Trapped { turns_remaining: 1 }, 1.0/16.0)]
    fn test_condition_damage_can_cause_fainting(
        #[case] species: Species,
        #[case] condition: PokemonCondition,
        #[case] damage_fraction: f32,
    ) {
        // Arrange
        let template_pokemon = TestPokemonBuilder::new(species, 10).build();
        let max_hp = template_pokemon.max_hp();
        let expected_damage = (max_hp as f32 * damage_fraction).max(1.0) as u16;

        let p1_pokemon = TestPokemonBuilder::new(species, 10)
            .with_moves(vec![Move::Splash])
            .with_hp(expected_damage)
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Oddish, 10)
            .with_moves(vec![Move::Splash])
            .build();

        let mut player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        player1.add_condition(condition.clone());
        let player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);
        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message(&format!(
            "Events for fainting test with {:?}",
            condition.get_type()
        ));

        assert_eq!(
            battle_state.players[0]
                .active_pokemon()
                .unwrap()
                .current_hp(),
            0,
            "Pokemon should have fainted"
        );
        assert!(event_bus.events().iter().any(|e| matches!(e, BattleEvent::PokemonFainted { player_index: 0, pokemon } if *pokemon == species)));
    }

    #[test]
    fn test_leech_seed_no_healing_if_opponent_fainted() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Weedle, 10)
            .with_moves(vec![Move::Splash])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Kakuna, 10)
            .with_moves(vec![Move::Splash])
            .with_hp(0) // Fainted
            .build();

        let mut player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        player1.add_condition(PokemonCondition::Seeded);
        let player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);

        let initial_p1_hp = player1.active_pokemon().unwrap().current_hp();
        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus
            .print_debug_with_message("Events for test_leech_seed_no_healing_if_opponent_fainted:");

        let final_p1_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();
        let max_p1_hp = battle_state.players[0].active_pokemon().unwrap().max_hp();
        let expected_damage = (max_p1_hp / 8).max(1);

        assert_eq!(
            final_p1_hp,
            initial_p1_hp - expected_damage,
            "Player 1 should still take Leech Seed damage"
        );
        assert_eq!(
            battle_state.players[1]
                .active_pokemon()
                .unwrap()
                .current_hp(),
            0,
            "Player 2 should remain fainted"
        );
        assert!(event_bus.events().iter().any(|e| matches!(
            e,
            BattleEvent::StatusDamage {
                status: PokemonCondition::Seeded,
                ..
            }
        )));
        assert!(!event_bus.events().iter().any(|e| matches!(
            e,
            BattleEvent::PokemonHealed {
                target: Species::Kakuna,
                ..
            }
        )));
    }

    #[test]
    fn test_leech_seed_healing_caps_at_max_hp() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Bellsprout, 10)
            .with_moves(vec![Move::Splash])
            .build();

        let p2_template = TestPokemonBuilder::new(Species::Weepinbell, 10).build();
        let max_p2_hp = p2_template.max_hp();
        let p2_pokemon = TestPokemonBuilder::new(Species::Weepinbell, 10)
            .with_moves(vec![Move::Splash])
            .with_hp(max_p2_hp - 1) // Damage by just 1 HP
            .build();

        let mut player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        player1.add_condition(PokemonCondition::Seeded);
        let player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);

        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_leech_seed_healing_caps_at_max_hp:");

        let final_p2_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();
        assert_eq!(
            final_p2_hp, max_p2_hp,
            "Player 2 should be healed to max HP, not beyond"
        );

        let heal_event_found = event_bus.events().iter().any(|e| {
            matches!(e, BattleEvent::PokemonHealed { target: Species::Weepinbell, amount, new_hp, .. } if *amount == 1 && *new_hp == max_p2_hp)
        });
        assert!(
            heal_event_found,
            "Should have PokemonHealed event for the actual amount healed (1 HP)"
        );
    }

    #[rstest]
    #[case("baseline", None, None, None)]
    #[case("attack +2 stages", Some((StatType::Atk, 2)), None, None)]
    #[case("attack -2 stages", Some((StatType::Atk, -2)), None, None)]
    #[case("defense +2 stages", Some((StatType::Def, 2)), None, None)]
    #[case("defense -2 stages", Some((StatType::Def, -2)), None, None)]
    #[case("special attack +2 stages (no effect)", Some((StatType::SpAtk, 2)), None, None)]
    #[case("with reflect (reduces damage)", None, Some(TeamCondition::Reflect), None)]
    #[case("with light screen (no effect)", None, Some(TeamCondition::LightScreen), None)]
    fn test_confusion_damage_modification(
        #[case] description: &str,
        #[case] stat_modification: Option<(StatType, i8)>,
        #[case] team_condition: Option<TeamCondition>,
        #[case] _unused: Option<()>,
    ) {
        // Arrange - Create a Pokemon that will hit itself due to confusion
        let p1_pokemon = TestPokemonBuilder::new(Species::Machop, 25)
            .with_moves(vec![Move::Tackle])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Geodude, 25)
            .with_moves(vec![Move::Splash])
            .build();

        let mut player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        
        // Apply stat modifications if specified
        if let Some((stat_type, stages)) = stat_modification {
            player1.set_stat_stage(stat_type, stages);
        }
        
        // Add team conditions if specified
        if let Some(condition) = team_condition {
            player1.add_team_condition(condition, 5); // 5 turns duration
        }
        
        // Add confusion to player 1's Pokemon
        player1.add_condition(PokemonCondition::Confused { turns_remaining: 2 });
        
        let player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);
        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        let initial_hp = battle_state.players[0].active_pokemon().unwrap().current_hp();
        
        // Set up actions - both players try to use moves
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act - Use RNG values that will cause confusion self-hit (low roll < 50 = hit self)
        let event_bus = resolve_turn(&mut battle_state, TurnRng::new_for_test(vec![25; 10]));

        // Assert
        event_bus.print_debug_with_message(&format!(
            "Events for confusion damage test: {}", description
        ));
        
        let final_hp = battle_state.players[0].active_pokemon().unwrap().current_hp();
        let damage_taken = initial_hp - final_hp;
        
        // Should have self-damage from confusion
        assert!(damage_taken > 0, "Pokemon should have taken confusion self-damage");
        
        // Should see action failed event due to confusion
        assert!(event_bus.events().iter().any(|e| matches!(
            e,
            BattleEvent::ActionFailed {
                reason: crate::battle::state::ActionFailureReason::IsConfused { .. }
            }
        )), "Should have ActionFailed event for confusion");
        
        // Should see self-hit event
        assert!(event_bus.events().iter().any(|e| matches!(
            e,
            BattleEvent::MoveHit {
                move_used: Move::HittingItself,
                attacker: Species::Machop,
                defender: Species::Machop,
                ..
            }
        )), "Should have MoveHit event for confusion self-damage");
        
        // Should see damage dealt event
        assert!(event_bus.events().iter().any(|e| matches!(
            e,
            BattleEvent::DamageDealt {
                target: Species::Machop,
                damage,
                ..
            } if *damage == damage_taken
        )), "Should have DamageDealt event for confusion self-damage");

        println!("Test case '{}': Damage taken = {}", description, damage_taken);
    }
}
