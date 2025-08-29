#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{BattleEvent, BattleState};
    use crate::battle::tests::common::{create_test_player, predictable_rng, TestPokemonBuilder};
    use crate::player::PlayerAction;
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use schema::Move;

    #[rstest]
    #[case(100, 50)] // 100 HP -> 50 damage
    #[case(80, 40)] // 80 HP -> 40 damage
    #[case(60, 30)] // 60 HP -> 30 damage
    #[case(30, 15)] // 30 HP -> 15 damage
    #[case(1, 1)] // 1 HP -> 1 damage (minimum)
    fn test_super_fang_damage(#[case] defender_hp: u16, #[case] expected_damage: u16) {
        // Arrange - Create a high-HP Pokemon and set its HP to the test value
        let attacker_pokemon = TestPokemonBuilder::new(Species::Rattata, 25)
            .with_moves(vec![Move::SuperFang])
            .build();
        let defender_pokemon = TestPokemonBuilder::new(Species::Snorlax, 50) // High HP Pokemon
            .with_moves(vec![Move::Splash])
            .with_hp(defender_hp)
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![attacker_pokemon]);
        let player2 = create_test_player("p2", "Player 2", vec![defender_pokemon]);
        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message(&format!(
            "SuperFang test: {} HP -> {} damage",
            defender_hp, expected_damage
        ));

        let final_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();
        let damage_taken = defender_hp - final_hp;

        assert_eq!(
            damage_taken, expected_damage,
            "SuperFang should deal exactly half of current HP"
        );

        // Verify the damage event
        assert!(
            event_bus.events().iter().any(|e| matches!(
                e,
                BattleEvent::DamageDealt {
                    target: Species::Snorlax,
                    damage,
                    ..
                } if *damage == expected_damage
            )),
            "Should have DamageDealt event with correct damage amount"
        );
    }

    #[test]
    fn test_dragon_rage_always_40_damage() {
        // Test Dragon Rage against different Pokemon with different levels and stats
        // Using higher level Pokemon to ensure they have enough HP to take 40 damage
        let test_cases = vec![
            (Species::Snorlax, 20),   // High HP Pokemon
            (Species::Blastoise, 30), // High level, high HP
            (Species::Onix, 25),      // High defense
            (Species::Golem, 30),     // High defense
        ];

        for (defender_species, level) in test_cases {
            // Arrange
            let attacker_pokemon = TestPokemonBuilder::new(Species::Dratini, 25)
                .with_moves(vec![Move::DragonRage])
                .build();
            let defender_pokemon = TestPokemonBuilder::new(defender_species, level)
                .with_moves(vec![Move::Splash])
                .build();

            let player1 = create_test_player("p1", "Player 1", vec![attacker_pokemon]);
            let player2 = create_test_player("p2", "Player 2", vec![defender_pokemon]);
            let mut battle_state = BattleState::new("test".to_string(), player1, player2);

            let initial_hp = battle_state.players[1]
                .active_pokemon()
                .unwrap()
                .current_hp();

            battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
            battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

            // Act
            let event_bus = resolve_turn(&mut battle_state, predictable_rng());

            // Assert
            event_bus.print_debug_with_message(&format!(
                "DragonRage test against {} level {}",
                defender_species, level
            ));

            let final_hp = battle_state.players[1]
                .active_pokemon()
                .unwrap()
                .current_hp();
            let damage_taken = initial_hp - final_hp;

            assert_eq!(
                damage_taken, 40,
                "Dragon Rage should always deal exactly 40 damage"
            );

            // Verify the damage event
            assert!(
                event_bus.events().iter().any(|e| matches!(
                    e,
                    BattleEvent::DamageDealt {
                        target: species,
                        damage: 40,
                        ..
                    } if *species == defender_species
                )),
                "Should have DamageDealt event with exactly 40 damage"
            );
        }
    }

    #[test]
    fn test_sonic_boom_always_20_damage() {
        // Test Sonic Boom against different Pokemon with different levels and stats
        let test_cases = vec![
            (Species::Pidgey, 10),  // Low level, low HP
            (Species::Snorlax, 40), // High HP
            (Species::Onix, 35),    // High defense
            (Species::Golem, 30),   // High defense
        ];

        for (defender_species, level) in test_cases {
            // Arrange
            let attacker_pokemon = TestPokemonBuilder::new(Species::Magnemite, 25)
                .with_moves(vec![Move::SonicBoom])
                .build();
            let defender_pokemon = TestPokemonBuilder::new(defender_species, level)
                .with_moves(vec![Move::Splash])
                .build();

            let player1 = create_test_player("p1", "Player 1", vec![attacker_pokemon]);
            let player2 = create_test_player("p2", "Player 2", vec![defender_pokemon]);
            let mut battle_state = BattleState::new("test".to_string(), player1, player2);

            let initial_hp = battle_state.players[1]
                .active_pokemon()
                .unwrap()
                .current_hp();

            battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
            battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

            // Act
            let event_bus = resolve_turn(&mut battle_state, predictable_rng());

            // Assert
            event_bus.print_debug_with_message(&format!(
                "SonicBoom test against {} level {}",
                defender_species, level
            ));

            let final_hp = battle_state.players[1]
                .active_pokemon()
                .unwrap()
                .current_hp();
            let damage_taken = initial_hp - final_hp;

            assert_eq!(
                damage_taken, 20,
                "Sonic Boom should always deal exactly 20 damage"
            );

            // Verify the damage event
            assert!(
                event_bus.events().iter().any(|e| matches!(
                    e,
                    BattleEvent::DamageDealt {
                        target: species,
                        damage: 20,
                        ..
                    } if *species == defender_species
                )),
                "Should have DamageDealt event with exactly 20 damage"
            );
        }
    }

    #[rstest]
    #[case(Move::SeismicToss, Species::Machop, 25, 25)] // Seismic Toss: Level 25 -> 25 damage
    #[case(Move::Psywave, Species::Abra, 15, 15)] // Psywave: Level 15 -> 15 damage
    #[case(Move::NightShade, Species::Gastly, 30, 30)] // Night Shade: Level 30 -> 30 damage
    #[case(Move::SeismicToss, Species::Machamp, 50, 50)] // Seismic Toss: Level 50 -> 50 damage
    fn test_level_damage_moves(
        #[case] move_used: Move,
        #[case] attacker_species: Species,
        #[case] attacker_level: u8,
        #[case] expected_damage: u16,
    ) {
        // Arrange
        let attacker_pokemon = TestPokemonBuilder::new(attacker_species, attacker_level)
            .with_moves(vec![move_used])
            .build();
        let defender_pokemon = TestPokemonBuilder::new(Species::Lapras, 50) // High HP Pokemon
            .with_moves(vec![Move::Splash])
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![attacker_pokemon]);
        let player2 = create_test_player("p2", "Player 2", vec![defender_pokemon]);
        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        let initial_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message(&format!(
            "Level damage test: {:?} by {} level {} should deal {} damage",
            move_used, attacker_species, attacker_level, expected_damage
        ));

        let final_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();
        let damage_taken = initial_hp - final_hp;

        assert_eq!(
            damage_taken, expected_damage,
            "{:?} level {} should deal exactly {} damage, but dealt {}",
            move_used, attacker_level, expected_damage, damage_taken
        );

        // Verify the damage event
        assert!(
            event_bus.events().iter().any(|e| matches!(
                e,
                BattleEvent::DamageDealt {
                    target: Species::Lapras,
                    damage,
                    ..
                } if *damage == expected_damage
            )),
            "Should have DamageDealt event with {} damage for {:?} level {}, but events were: {:?}",
            expected_damage,
            move_used,
            attacker_level,
            event_bus
                .events()
                .iter()
                .filter(|e| matches!(e, BattleEvent::DamageDealt { .. }))
                .collect::<Vec<_>>()
        );
    }

    #[rstest]
    #[case(Species::Snorlax, "Normal type (neutral)")]
    #[case(Species::Dragonite, "Dragon type (weak to Dragon)")]
    #[case(Species::Blastoise, "Water type (neutral)")]
    fn test_dragon_rage_ignores_type_effectiveness(
        #[case] defender_species: Species,
        #[case] description: &str,
    ) {
        // Arrange
        let attacker_pokemon = TestPokemonBuilder::new(Species::Dratini, 25)
            .with_moves(vec![Move::DragonRage])
            .build();
        let defender_pokemon = TestPokemonBuilder::new(defender_species, 30)
            .with_moves(vec![Move::Splash])
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![attacker_pokemon]);
        let player2 = create_test_player("p2", "Player 2", vec![defender_pokemon]);
        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        let initial_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message(&format!(
            "DragonRage vs {}: {}",
            defender_species, description
        ));

        let final_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();
        let damage_taken = initial_hp - final_hp;

        assert_eq!(damage_taken, 40,
                  "Dragon Rage should always deal exactly 40 damage regardless of type effectiveness vs {}", 
                  description);

        // Should NOT see type effectiveness event
        assert!(
            !event_bus.contains(|e| matches!(e, BattleEvent::AttackTypeEffectiveness { .. })),
            "Dragon Rage should not show type effectiveness"
        );
    }

    #[rstest]
    #[case(Species::Pidgey, "Normal type (neutral)")]
    #[case(Species::Geodude, "Rock type (resists Normal)")]
    #[case(Species::Machop, "Fighting type (neutral)")]
    fn test_sonic_boom_ignores_type_effectiveness(
        #[case] defender_species: Species,
        #[case] description: &str,
    ) {
        // Arrange
        let attacker_pokemon = TestPokemonBuilder::new(Species::Magnemite, 25)
            .with_moves(vec![Move::SonicBoom])
            .build();
        let defender_pokemon = TestPokemonBuilder::new(defender_species, 30)
            .with_moves(vec![Move::Splash])
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![attacker_pokemon]);
        let player2 = create_test_player("p2", "Player 2", vec![defender_pokemon]);
        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        let initial_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message(&format!(
            "SonicBoom vs {}: {}",
            defender_species, description
        ));

        let final_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();
        let damage_taken = initial_hp - final_hp;

        assert_eq!(damage_taken, 20,
                  "Sonic Boom should always deal exactly 20 damage regardless of type effectiveness vs {}", 
                  description);

        // Should NOT see type effectiveness event
        assert!(
            !event_bus.contains(|e| matches!(e, BattleEvent::AttackTypeEffectiveness { .. })),
            "Sonic Boom should not show type effectiveness"
        );
    }

    #[rstest]
    #[case(Species::Snorlax, 30, "Normal type (super effective vs Fighting)")]
    #[case(Species::Pidgey, 30, "Flying type (not very effective vs Fighting)")]
    #[case(Species::Alakazam, 30, "Psychic type (not very effective vs Fighting)")]
    fn test_seismic_toss_ignores_type_effectiveness(
        #[case] defender_species: Species,
        #[case] expected_damage: u16,
        #[case] description: &str,
    ) {
        // Arrange
        let attacker_pokemon = TestPokemonBuilder::new(Species::Machop, expected_damage as u8)
            .with_moves(vec![Move::SeismicToss])
            .build();
        let defender_pokemon = TestPokemonBuilder::new(defender_species, 40)
            .with_moves(vec![Move::Splash])
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![attacker_pokemon]);
        let player2 = create_test_player("p2", "Player 2", vec![defender_pokemon]);
        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        let initial_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message(&format!(
            "SeismicToss level {} vs {}: {}",
            expected_damage, defender_species, description
        ));

        let final_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();
        let damage_taken = initial_hp - final_hp;

        assert_eq!(
            damage_taken, expected_damage,
            "Seismic Toss should deal exactly {} damage regardless of type effectiveness vs {}",
            expected_damage, description
        );

        // Should NOT see type effectiveness event
        assert!(
            !event_bus
                .events()
                .iter()
                .any(|e| matches!(e, BattleEvent::AttackTypeEffectiveness { .. })),
            "Seismic Toss should not show type effectiveness"
        );
    }

    #[rstest]
    #[case(Species::Gastly, Species::Snorlax, 0, "Night Shade vs Normal (immune)")]
    #[case(
        Species::Gastly,
        Species::Alakazam,
        30,
        "Night Shade vs Psychic (neutral)"
    )]
    fn test_night_shade_immunity_vs_resistance(
        #[case] attacker_species: Species,
        #[case] defender_species: Species,
        #[case] expected_damage: u16,
        #[case] description: &str,
    ) {
        // Arrange
        let attacker_pokemon = TestPokemonBuilder::new(attacker_species, 30)
            .with_moves(vec![Move::NightShade])
            .build();
        let defender_pokemon = TestPokemonBuilder::new(defender_species, 40)
            .with_moves(vec![Move::Splash])
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![attacker_pokemon]);
        let player2 = create_test_player("p2", "Player 2", vec![defender_pokemon]);
        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        let initial_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message(&format!("NightShade test: {}", description));

        let final_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();
        let damage_taken = initial_hp - final_hp;

        assert_eq!(
            damage_taken, expected_damage,
            "Night Shade: {} should deal {} damage",
            description, expected_damage
        );

        if expected_damage == 0 {
            // Should see type effectiveness event showing immunity
            assert!(
                event_bus
                    .events()
                    .iter()
                    .any(|e| matches!(e, BattleEvent::AttackTypeEffectiveness { multiplier: 0.0 })),
                "Should show type immunity for Ghost vs Normal"
            );
        } else {
            // Should NOT see type effectiveness event (ignores non-immunity modifiers)
            assert!(
                !event_bus
                    .events()
                    .iter()
                    .any(|e| matches!(e, BattleEvent::AttackTypeEffectiveness { .. })),
                "Should not show type effectiveness when not immune"
            );
        }
    }
}
