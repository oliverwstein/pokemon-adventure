#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleEvent, BattleState, TurnRng};
    use crate::battle::turn_orchestrator::resolve_turn;
    use crate::moves::Move;
    use crate::player::{BattlePlayer, PlayerAction};
    use crate::pokemon::{MoveInstance, PokemonInst};
    use crate::species::Species;

    fn create_test_pokemon(species: Species, moves: Vec<Move>) -> PokemonInst {
        let mut pokemon_moves = [const { None }; 4];
        for (i, mv) in moves.into_iter().enumerate() {
            if i < 4 {
                pokemon_moves[i] = Some(MoveInstance { move_: mv, pp: 20 });
            }
        }

        let mut pokemon = PokemonInst::new_for_test(
            species,
            10,
            0,
            0, // Will be set below
            [15; 6],
            [0; 6],
            [100, 80, 80, 80, 80, 80],
            pokemon_moves,
            None,
        );
        pokemon.set_hp_to_max();
        pokemon
    }

    #[test]
    fn test_reckless_effect_on_miss() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Hitmonlee,
                vec![Move::HighJumpKick],
            )], // HighJumpKick has Reckless(50)
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Tackle])],
        );

        let initial_hp = player1.active_pokemon().unwrap().current_hp();
        let max_hp = player1.active_pokemon().unwrap().max_hp();

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses High Jump Kick (will miss due to RNG), Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // High Jump Kick
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        // Use RNG that will cause High Jump Kick to miss (roll 85, but accuracy is 80)
        let test_rng = TurnRng::new_for_test(vec![85, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Reckless effect on miss test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        let final_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();
        let expected_recoil = (max_hp * 50) / 100; // 50% of max HP

        // Should have missed
        let miss_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::MoveMissed {
                        attacker: Species::Hitmonlee,
                        move_used: Move::HighJumpKick,
                        ..
                    }
                )
            })
            .collect();
        assert!(!miss_events.is_empty(), "High Jump Kick should have missed");

        // Should have taken recoil damage
        let recoil_damage_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::DamageDealt { target: Species::Hitmonlee, damage, .. } if *damage == expected_recoil))
            .collect();
        assert!(
            !recoil_damage_events.is_empty(),
            "Should have taken {} recoil damage from missed High Jump Kick",
            expected_recoil
        );

        // Verify final HP accounts for recoil damage (and possibly damage from Tackle)
        assert!(
            final_hp < initial_hp,
            "Pokemon should have taken damage from recoil (and possibly Tackle)"
        );
    }

    #[test]
    fn test_reckless_effect_on_hit_no_recoil() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Hitmonlee,
                vec![Move::HighJumpKick],
            )],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Tackle])],
        );

        let initial_hp = player1.active_pokemon().unwrap().current_hp();

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses High Jump Kick (will hit), Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // High Jump Kick
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        // Use RNG that will cause High Jump Kick to hit (roll 50, accuracy is 80)
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Reckless effect on hit (no recoil) test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        let final_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();

        // Should NOT have missed
        let miss_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::MoveMissed {
                        attacker: Species::Hitmonlee,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            miss_events.is_empty(),
            "High Jump Kick should have hit, not missed"
        );

        // Should have hit instead
        let hit_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::MoveHit {
                        attacker: Species::Hitmonlee,
                        move_used: Move::HighJumpKick,
                        ..
                    }
                )
            })
            .collect();
        assert!(!hit_events.is_empty(), "High Jump Kick should have hit");

        // Should NOT have recoil damage to the attacker (only damage from enemy Tackle if any)
        let recoil_damage_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::DamageDealt {
                        target: Species::Hitmonlee,
                        ..
                    }
                )
            })
            .collect();

        // If there are damage events to Hitmonlee, they should only be from Tackle, not recoil
        // We can check this by ensuring the damage is reasonable for Tackle, not 50% of max HP
        if !recoil_damage_events.is_empty() {
            for event in &recoil_damage_events {
                if let BattleEvent::DamageDealt { damage, .. } = event {
                    let max_hp = initial_hp;
                    let recoil_amount = (max_hp * 50) / 100;
                    assert!(
                        *damage != recoil_amount,
                        "Should not have recoil damage when move hits"
                    );
                }
            }
        }
    }

    #[test]
    fn test_reckless_different_percentages() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Hitmonlee,
                vec![Move::JumpKick],
            )], // JumpKick has Reckless(20)
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Tackle])],
        );

        let max_hp = player1.active_pokemon().unwrap().max_hp();

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Jump Kick (will miss), Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Jump Kick
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        // Use RNG that will cause Jump Kick to miss (roll 95, accuracy is 90)
        let test_rng = TurnRng::new_for_test(vec![95, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Reckless different percentages test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        let expected_recoil = (max_hp * 20) / 100; // 20% of max HP for Jump Kick

        // Should have missed
        let miss_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::MoveMissed {
                        attacker: Species::Hitmonlee,
                        move_used: Move::JumpKick,
                        ..
                    }
                )
            })
            .collect();
        assert!(!miss_events.is_empty(), "Jump Kick should have missed");

        // Should have taken 20% recoil damage (not 50% like High Jump Kick)
        let recoil_damage_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::DamageDealt { target: Species::Hitmonlee, damage, .. } if *damage == expected_recoil))
            .collect();
        assert!(
            !recoil_damage_events.is_empty(),
            "Should have taken {} recoil damage from missed Jump Kick",
            expected_recoil
        );
    }

    #[test]
    fn test_reckless_can_cause_fainting() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Hitmonlee,
                vec![Move::HighJumpKick],
            )],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Tackle])],
        );

        // Damage Player 1's Pokemon to very low HP so recoil will faint it
        let attacker_pokemon = player1.active_pokemon_mut().unwrap();
        let max_hp = attacker_pokemon.max_hp();
        let recoil_damage = (max_hp * 50) / 100; // 50% recoil
        attacker_pokemon.take_damage(max_hp - recoil_damage + 1); // Leave just enough HP that recoil will faint

        let low_hp = player1.active_pokemon().unwrap().current_hp();
        assert!(
            low_hp > 0 && low_hp <= recoil_damage,
            "Pokemon should have low HP but not be fainted yet"
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses High Jump Kick (will miss and cause fainting), Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // High Jump Kick
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        // Use RNG that will cause High Jump Kick to miss
        let test_rng = TurnRng::new_for_test(vec![85, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Reckless causes fainting test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        let final_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();

        // Should have missed
        let miss_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::MoveMissed {
                        attacker: Species::Hitmonlee,
                        ..
                    }
                )
            })
            .collect();
        assert!(!miss_events.is_empty(), "High Jump Kick should have missed");

        // Should have fainted from recoil
        assert_eq!(
            final_hp, 0,
            "Pokemon should have fainted from recoil damage"
        );

        // Should have PokemonFainted event
        let faint_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::PokemonFainted {
                        pokemon: Species::Hitmonlee,
                        player_index: 0
                    }
                )
            })
            .collect();
        assert!(
            !faint_events.is_empty(),
            "Should have PokemonFainted event from recoil damage"
        );
    }

    #[test]
    fn test_non_reckless_move_no_recoil_on_miss() {
        // Initialize move data
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path)
            .expect("Failed to initialize species data");

        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Hitmonlee, vec![Move::Tackle])], // Tackle has no Reckless effect
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::TailWhip])],
        );

        let initial_hp = player1.active_pokemon().unwrap().current_hp();

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Tackle (will miss), Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // TailWhip

        // Use RNG that will cause first Tackle to miss
        // We'll force it by using a very high roll that would miss even 100% accuracy moves if they had lower accuracy
        let test_rng = TurnRng::new_for_test(vec![101, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Non-reckless move no recoil test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        let final_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();

        // Should NOT have recoil damage to the attacker even if move missed
        // (Since Tackle doesn't have Reckless effect, there should be no self-damage)
        let self_damage_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::DamageDealt {
                        target: Species::Hitmonlee,
                        ..
                    }
                )
            })
            .collect();

        // Any damage to Hitmonlee should only be from the opponent's Tackle, not recoil
        for event in &self_damage_events {
            if let BattleEvent::DamageDealt { damage, .. } = event {
                // Recoil would be a percentage of max HP, but Tackle damage should be much lower
                let max_hp = initial_hp;
                assert!(
                    *damage < max_hp / 4,
                    "Damage should be from opponent's attack, not recoil"
                );
            }
        }
    }
}
