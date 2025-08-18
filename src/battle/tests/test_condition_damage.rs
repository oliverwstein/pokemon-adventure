#[cfg(test)]
mod tests {
    use crate::battle::conditions::PokemonCondition;
    use crate::battle::state::{BattleEvent, BattleState, TurnRng};
    use crate::battle::engine::resolve_turn;
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
    fn test_leech_seed_damage_and_healing() {
        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Bulbasaur, vec![Move::Splash])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Charmander, vec![Move::Splash])],
        );

        // Add Seeded condition to player1 (they will take damage)
        player1.add_condition(PokemonCondition::Seeded);

        // Damage player2 slightly so we can see healing
        player2.active_pokemon_mut().unwrap().take_damage(20);

        let initial_p1_hp = player1.active_pokemon().unwrap().current_hp();
        let initial_p2_hp = player2.active_pokemon().unwrap().current_hp();
        let max_p1_hp = player1.active_pokemon().unwrap().max_hp();

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Both players use Splash (no-op moves)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        event_bus.print_debug_with_message("Leech Seed damage and healing test events:");

        let final_p1_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();
        let final_p2_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();

        // Calculate expected damage (1/8 of max HP, minimum 1)
        let expected_damage = (max_p1_hp / 8).max(1);

        // Player 1 should have taken Leech Seed damage
        assert_eq!(
            final_p1_hp,
            initial_p1_hp - expected_damage,
            "Player 1 should have taken {} damage from Leech Seed",
            expected_damage
        );

        // Player 2 should have been healed by the same amount
        assert!(
            final_p2_hp > initial_p2_hp,
            "Player 2 should have been healed by Leech Seed"
        );
        assert_eq!(
            final_p2_hp,
            initial_p2_hp + expected_damage,
            "Player 2 should have been healed by {} HP from Leech Seed",
            expected_damage
        );

        // Should have StatusDamage event for Seeded
        let status_damage_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::StatusDamage {
                        target: Species::Bulbasaur,
                        status: PokemonCondition::Seeded,
                        damage,
                        ..
                    } if *damage == expected_damage
                )
            })
            .collect();
        assert!(
            !status_damage_events.is_empty(),
            "Should have StatusDamage event for Leech Seed"
        );

        // Should have PokemonHealed event for opponent
        let heal_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::PokemonHealed {
                        target: Species::Charmander,
                        amount,
                        ..
                    } if *amount == expected_damage
                )
            })
            .collect();
        assert!(
            !heal_events.is_empty(),
            "Should have PokemonHealed event for Leech Seed healing"
        );
    }

    #[test]
    fn test_trapped_damage() {
        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Onix, vec![Move::Splash])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Splash])],
        );

        // Add Trapped condition to player1 (they will take damage)
        player1.add_condition(PokemonCondition::Trapped { turns_remaining: 2 });

        let initial_p1_hp = player1.active_pokemon().unwrap().current_hp();
        let max_p1_hp = player1.active_pokemon().unwrap().max_hp();

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Both players use Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        event_bus.print_debug_with_message("Trapped damage test events:");

        let final_p1_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();

        // Calculate expected damage (1/16 of max HP, minimum 1)
        let expected_damage = (max_p1_hp / 16).max(1);

        // Player 1 should have taken Trapped damage
        assert_eq!(
            final_p1_hp,
            initial_p1_hp - expected_damage,
            "Player 1 should have taken {} damage from being Trapped",
            expected_damage
        );

        // Should have StatusDamage event for Trapped
        let status_damage_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::StatusDamage {
                        target: Species::Onix,
                        status: PokemonCondition::Trapped { .. },
                        damage,
                        ..
                    } if *damage == expected_damage
                )
            })
            .collect();
        assert!(
            !status_damage_events.is_empty(),
            "Should have StatusDamage event for Trapped condition"
        );

        // Trapped condition should still exist with decremented counter (2 -> 1)
        assert!(
            battle_state.players[0]
                .has_condition(&PokemonCondition::Trapped { turns_remaining: 1 })
        );
    }

    #[test]
    fn test_both_seeded_and_trapped_damage() {
        // Test that both conditions apply damage in the same turn
         let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Geodude, vec![Move::Splash])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Squirtle, vec![Move::Splash])],
        );

        // Add both conditions to player1
        player1.add_condition(PokemonCondition::Seeded);
        player1.add_condition(PokemonCondition::Trapped { turns_remaining: 3 });

        let initial_p1_hp = player1.active_pokemon().unwrap().current_hp();
        let max_p1_hp = player1.active_pokemon().unwrap().max_hp();

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Both players use Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        println!("Both Seeded and Trapped damage test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        let final_p1_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();

        // Calculate expected total damage
        let seeded_damage = (max_p1_hp / 8).max(1);
        let trapped_damage = (max_p1_hp / 16).max(1);
        let total_expected_damage = seeded_damage + trapped_damage;

        // Player 1 should have taken both types of damage
        assert_eq!(
            final_p1_hp,
            initial_p1_hp - total_expected_damage,
            "Player 1 should have taken {} total damage (Seeded: {}, Trapped: {})",
            total_expected_damage,
            seeded_damage,
            trapped_damage
        );

        // Should have StatusDamage events for both conditions
        let seeded_damage_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::StatusDamage {
                        status: PokemonCondition::Seeded,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            !seeded_damage_events.is_empty(),
            "Should have StatusDamage event for Seeded condition"
        );

        let trapped_damage_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::StatusDamage {
                        status: PokemonCondition::Trapped { .. },
                        ..
                    }
                )
            })
            .collect();
        assert!(
            !trapped_damage_events.is_empty(),
            "Should have StatusDamage event for Trapped condition"
        );
    }

    #[test]
    fn test_leech_seed_causes_fainting() {
        // Test that Leech Seed can cause a Pokemon to faint
         let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Caterpie, vec![Move::Splash])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Oddish, vec![Move::Splash])],
        );

        // Add Seeded condition and reduce HP to very low
        player1.add_condition(PokemonCondition::Seeded);
        let pokemon = player1.active_pokemon_mut().unwrap();
        let max_hp = pokemon.max_hp();
        let seeded_damage = (max_hp / 8).max(1);
        // Leave just enough HP that Leech Seed will cause fainting
        pokemon.take_damage(max_hp - seeded_damage + 1);

        let low_hp = player1.active_pokemon().unwrap().current_hp();
        assert!(
            low_hp > 0 && low_hp <= seeded_damage,
            "Pokemon should have low HP but not be fainted yet"
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Both players use Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        println!("Leech Seed causes fainting test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        let final_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();

        // Pokemon should have fainted
        assert_eq!(
            final_hp, 0,
            "Pokemon should have fainted from Leech Seed damage"
        );

        // Should have PokemonFainted event
        let faint_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::PokemonFainted {
                        pokemon: Species::Caterpie,
                        player_index: 0
                    }
                )
            })
            .collect();
        assert!(
            !faint_events.is_empty(),
            "Should have PokemonFainted event from Leech Seed damage"
        );
    }

    #[test]
    fn test_trapped_causes_fainting() {
        // Test that Trapped condition can cause a Pokemon to faint
         let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Magikarp, vec![Move::Splash])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Gyarados, vec![Move::Splash])],
        );

        // Add Trapped condition and reduce HP to very low
        player1.add_condition(PokemonCondition::Trapped { turns_remaining: 1 });
        let pokemon = player1.active_pokemon_mut().unwrap();
        let max_hp = pokemon.max_hp();
        let trapped_damage = (max_hp / 16).max(1);
        // Leave just enough HP that Trapped will cause fainting
        pokemon.take_damage(max_hp - trapped_damage + 1);

        let low_hp = player1.active_pokemon().unwrap().current_hp();
        assert!(
            low_hp > 0 && low_hp <= trapped_damage,
            "Pokemon should have low HP but not be fainted yet"
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Both players use Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        println!("Trapped causes fainting test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        let final_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();

        // Pokemon should have fainted
        assert_eq!(
            final_hp, 0,
            "Pokemon should have fainted from Trapped damage"
        );

        // Should have PokemonFainted event
        let faint_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::PokemonFainted {
                        pokemon: Species::Magikarp,
                        player_index: 0
                    }
                )
            })
            .collect();
        assert!(
            !faint_events.is_empty(),
            "Should have PokemonFainted event from Trapped damage"
        );
    }

    #[test]
    fn test_leech_seed_no_healing_if_opponent_fainted() {
        // Test that if the opponent is fainted, no healing occurs from Leech Seed
         let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Weedle, vec![Move::Splash])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Kakuna, vec![Move::Splash])],
        );

        // Add Seeded condition to player1
        player1.add_condition(PokemonCondition::Seeded);

        // Faint player2's Pokemon
        let player2_pokemon = player2.active_pokemon_mut().unwrap();
        let player2_max_hp = player2_pokemon.max_hp();
        player2_pokemon.take_damage(player2_max_hp);
        assert!(
            player2_pokemon.is_fainted(),
            "Player2's Pokemon should be fainted"
        );

        let initial_p1_hp = player1.active_pokemon().unwrap().current_hp();

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Both players use Splash (though player2 can't act)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        println!("Leech Seed no healing if opponent fainted test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        let final_p1_hp = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();
        let final_p2_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();

        // Player 1 should still take damage from Leech Seed
        let max_p1_hp = battle_state.players[0].active_pokemon().unwrap().max_hp();
        let expected_damage = (max_p1_hp / 8).max(1);
        assert_eq!(
            final_p1_hp,
            initial_p1_hp - expected_damage,
            "Player 1 should still take Leech Seed damage"
        );

        // Player 2 should remain fainted (0 HP)
        assert_eq!(final_p2_hp, 0, "Player 2 should remain fainted");

        // Should have StatusDamage event but no PokemonHealed event
        let status_damage_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::StatusDamage {
                        status: PokemonCondition::Seeded,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            !status_damage_events.is_empty(),
            "Should have StatusDamage event"
        );

        let heal_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::PokemonHealed {
                        target: Species::Kakuna,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            heal_events.is_empty(),
            "Should not have PokemonHealed event for fainted opponent"
        );
    }

    #[test]
    fn test_leech_seed_healing_caps_at_max_hp() {
        // Test that healing from Leech Seed doesn't exceed max HP
         let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Bellsprout, vec![Move::Splash])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Weepinbell, vec![Move::Splash])],
        );

        // Add Seeded condition to player1
        player1.add_condition(PokemonCondition::Seeded);

        // Damage player2 by only 1 HP (so healing will be capped)
        player2.active_pokemon_mut().unwrap().take_damage(1);

        let _initial_p2_hp = player2.active_pokemon().unwrap().current_hp();
        let max_p2_hp = player2.active_pokemon().unwrap().max_hp();

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Both players use Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        println!("Leech Seed healing caps at max HP test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        let final_p2_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();

        // Player 2 should be healed back to max HP (not over)
        assert_eq!(
            final_p2_hp, max_p2_hp,
            "Player 2 should be healed to max HP, not beyond"
        );

        // Should have PokemonHealed event for only 1 HP (the actual healing amount)
        let heal_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::PokemonHealed {
                        target: Species::Weepinbell,
                        amount: 1,
                        new_hp,
                        ..
                    } if *new_hp == max_p2_hp
                )
            })
            .collect();
        assert!(
            !heal_events.is_empty(),
            "Should have PokemonHealed event for actual healing amount (1 HP)"
        );
    }
}
