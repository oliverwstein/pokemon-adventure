#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleState, TurnRng, EventBus, BattleEvent};
    use crate::battle::turn_orchestrator::{execute_attack_hit, ActionStack};
    use crate::moves::Move;
    use crate::player::{BattlePlayer, StatType};
    use crate::pokemon::PokemonInst;
    use crate::species::Species;
    use std::path::Path;

    fn init_test_data() {
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        crate::pokemon::initialize_species_data(data_path).expect("Failed to initialize species data");
    }

    fn create_test_pokemon_with_hp(species: Species, moves: Vec<Move>, hp: u16) -> PokemonInst {
        let mut pokemon = PokemonInst::new_for_test(
            species,
            0,
            0, // Will be set below
            [15, 15, 15, 15, 15, 15],
            [0, 0, 0, 0, 0, 0],
            [hp, 80, 70, 60, 60, 90], // Max HP same as current for simplicity
            [
                moves.get(0).map(|&m| crate::pokemon::MoveInstance::new(m)),
                moves.get(1).map(|&m| crate::pokemon::MoveInstance::new(m)),
                moves.get(2).map(|&m| crate::pokemon::MoveInstance::new(m)),
                moves.get(3).map(|&m| crate::pokemon::MoveInstance::new(m)),
            ],
            None,
        );
        pokemon.set_hp(hp); // Set current HP with validation
        pokemon
    }

    fn create_test_player(pokemon: PokemonInst) -> BattlePlayer {
        BattlePlayer::new(
            "test_player".to_string(),
            "TestPlayer".to_string(),
            vec![pokemon],
        )
    }

    #[test]
    fn test_critical_hit_effect() {
        init_test_data();
        
        // Create Pokemon with a move that has increased crit ratio
        let attacker = create_test_pokemon_with_hp(Species::Scyther, vec![Move::Slash], 100);
        let defender = create_test_pokemon_with_hp(Species::Pidgey, vec![Move::Tackle], 100);
        
        let player1 = create_test_player(attacker);
        let player2 = create_test_player(defender);
        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);
        
        let mut bus = EventBus::new();
        // Use a very low RNG value to guarantee critical hit
        let mut rng = TurnRng::new_for_test(vec![1, 10, 90]); // Hit check, low crit roll, damage variance
        let mut action_stack = ActionStack::new();
        
        // Execute Slash (move with increased crit ratio)
        execute_attack_hit(0, 1, Move::Slash, 0, &mut action_stack, &mut bus, &mut rng, &mut battle_state);
        
        let events = bus.events();
        
        println!("Slash critical hit test events:");
        for event in events {
            println!("  {:?}", event);
        }
        
        // Should have a critical hit event due to Slash's increased crit ratio + low RNG roll
        let has_crit = events.iter().any(|e| matches!(e, BattleEvent::CriticalHit { move_used: Move::Slash, .. }));
        assert!(has_crit, "Slash with very low RNG roll should result in critical hit");
    }

    #[test]
    fn test_recoil_effect() {
        init_test_data();
        
        // Create attacker with decent HP and a recoil move
        let attacker = create_test_pokemon_with_hp(Species::Tauros, vec![Move::DoubleEdge], 100);
        let defender = create_test_pokemon_with_hp(Species::Pidgey, vec![Move::Tackle], 50);
        
        let player1 = create_test_player(attacker);
        let player2 = create_test_player(defender);
        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);
        
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50, 60, 70, 80]); // Good rolls
        let mut action_stack = ActionStack::new();
        
        // Record attacker's initial HP
        let initial_attacker_hp = battle_state.players[0].team[0].as_ref().unwrap().current_hp();
        
        // Execute Double-Edge (recoil move)
        execute_attack_hit(0, 1, Move::DoubleEdge, 0, &mut action_stack, &mut bus, &mut rng, &mut battle_state);
        
        // Check that attacker took recoil damage
        let final_attacker_hp = battle_state.players[0].team[0].as_ref().unwrap().current_hp();
        assert!(final_attacker_hp < initial_attacker_hp, "Attacker should have taken recoil damage");
        
        let events = bus.events();
        
        // Should have damage dealt to both defender and attacker (recoil)
        let defender_damage_events: Vec<_> = events.iter().filter(|e| {
            matches!(e, BattleEvent::DamageDealt { target, .. } if *target == Species::Pidgey)
        }).collect();
        
        let attacker_damage_events: Vec<_> = events.iter().filter(|e| {
            matches!(e, BattleEvent::DamageDealt { target, .. } if *target == Species::Tauros)
        }).collect();
        
        assert!(!defender_damage_events.is_empty(), "Should have damage dealt to defender");
        assert!(!attacker_damage_events.is_empty(), "Should have recoil damage dealt to attacker");
        
        println!("Double-Edge recoil test events:");
        for event in events {
            println!("  {:?}", event);
        }
    }

    #[test]
    fn test_drain_effect() {
        init_test_data();
        
        // Create attacker with reduced HP and a drain move
        let attacker = create_test_pokemon_with_hp(Species::Victreebel, vec![Move::MegaDrain], 30); // Low HP
        let defender = create_test_pokemon_with_hp(Species::Bulbasaur, vec![Move::Tackle], 100);
        
        let player1 = create_test_player(attacker);
        let player2 = create_test_player(defender);
        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);
        
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50, 60, 70, 80]); // Good rolls
        let mut action_stack = ActionStack::new();
        
        // Record attacker's initial HP
        let initial_attacker_hp = battle_state.players[0].team[0].as_ref().unwrap().current_hp();
        
        // Execute Mega Drain (drain move)
        execute_attack_hit(0, 1, Move::MegaDrain, 0, &mut action_stack, &mut bus, &mut rng, &mut battle_state);
        
        // Check that attacker was healed
        let final_attacker_hp = battle_state.players[0].team[0].as_ref().unwrap().current_hp();
        assert!(final_attacker_hp > initial_attacker_hp, "Attacker should have been healed by drain");
        
        let events = bus.events();
        
        // Should have damage dealt to defender and healing to attacker
        let has_damage = events.iter().any(|e| {
            matches!(e, BattleEvent::DamageDealt { target, .. } if *target == Species::Bulbasaur)
        });
        
        let has_healing = events.iter().any(|e| {
            matches!(e, BattleEvent::PokemonHealed { target, .. } if *target == Species::Victreebel)
        });
        
        assert!(has_damage, "Should have damage dealt to defender");
        assert!(has_healing, "Should have healing applied to attacker");
        
        println!("Mega Drain healing test events:");
        for event in events {
            println!("  {:?}", event);
        }
    }

    #[test]
    fn test_no_effects_without_damage() {
        init_test_data();
        
        // Test that recoil/drain don't trigger when no damage is dealt (e.g., immune types)
        let attacker = create_test_pokemon_with_hp(Species::Machamp, vec![Move::DoubleEdge], 100);
        let defender = create_test_pokemon_with_hp(Species::Gastly, vec![Move::Tackle], 100); // Ghost immune to Normal
        
        let player1 = create_test_player(attacker);
        let player2 = create_test_player(defender);
        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);
        
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50, 60, 70]); // Good rolls
        let mut action_stack = ActionStack::new();
        
        // Record attacker's initial HP
        let initial_attacker_hp = battle_state.players[0].team[0].as_ref().unwrap().current_hp();
        
        // Execute Double-Edge against Ghost type (should deal 0 damage)
        execute_attack_hit(0, 1, Move::DoubleEdge, 0, &mut action_stack, &mut bus, &mut rng, &mut battle_state);
        
        // Check that attacker didn't take recoil damage (since no damage was dealt)
        let final_attacker_hp = battle_state.players[0].team[0].as_ref().unwrap().current_hp();
        assert_eq!(final_attacker_hp, initial_attacker_hp, "Attacker should not take recoil damage when no damage is dealt");
        
        let events = bus.events();
        
        // Should have type effectiveness event but no recoil damage to attacker
        let has_effectiveness = events.iter().any(|e| {
            matches!(e, BattleEvent::AttackTypeEffectiveness { multiplier } if *multiplier < 0.1)
        });
        
        let attacker_damage_events: Vec<_> = events.iter().filter(|e| {
            matches!(e, BattleEvent::DamageDealt { target, .. } if *target == Species::Machamp)
        }).collect();
        
        assert!(has_effectiveness, "Should show type effectiveness (immune)");
        assert!(attacker_damage_events.is_empty(), "Should not have recoil damage when no damage is dealt");
        
        println!("No recoil on immunity test events:");
        for event in events {
            println!("  {:?}", event);
        }
    }
}