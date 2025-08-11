#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleState, TurnRng, EventBus, BattleEvent};
    use crate::battle::turn_orchestrator::execute_end_turn_phase;
    use crate::pokemon::{PokemonInst, get_species_data, initialize_species_data};
    use crate::move_data::initialize_move_data;
    use crate::player::{BattlePlayer, PokemonCondition};
    use crate::species::Species;
    use std::path::Path;

    fn init_test_data() {
        let data_path = Path::new("data");
        initialize_move_data(data_path).expect("Failed to initialize move data");
        initialize_species_data(data_path).expect("Failed to initialize species data");
    }

    fn create_test_pokemon(species: Species, hp: u16) -> PokemonInst {
        let species_data = get_species_data(species).unwrap();
        let level = 50;
        let ivs = Some([15, 15, 15, 15, 15, 15]);
        
        // Use the new_with_hp constructor to specify current HP (will be validated)
        PokemonInst::new_with_hp(species, &species_data, level, ivs, None, Some(hp))
    }

    fn create_test_battle_state(player1_hp: u16, player2_hp: u16) -> BattleState {
        let pokemon1 = create_test_pokemon(Species::Pikachu, player1_hp);
        let pokemon2 = create_test_pokemon(Species::Charmander, player2_hp);
        
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(), 
            vec![pokemon1],
        );
        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![pokemon2],
        );
        
        BattleState::new("test_battle".to_string(), player1, player2)
    }

    #[test]
    fn test_trapped_condition_damage() {
        init_test_data();
        
        let mut battle_state = create_test_battle_state(100, 100);
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50]); // Not used in this test
        
        // Add Trapped condition to player 1's Pokemon
        battle_state.players[0].add_condition(PokemonCondition::Trapped { turns_remaining: 3 });
        
        let initial_hp = battle_state.players[0].team[0].as_ref().unwrap().current_hp();
        let max_hp = battle_state.players[0].team[0].as_ref().unwrap().max_hp();
        let expected_damage = (max_hp / 16).max(1); // 1/16 of max HP, minimum 1
        
        // Execute end-of-turn phase
        execute_end_turn_phase(&mut battle_state, &mut bus, &mut rng);
        
        // Check that HP was reduced
        let final_hp = battle_state.players[0].team[0].as_ref().unwrap().current_hp();
        assert_eq!(final_hp, initial_hp - expected_damage, "Pokemon should take trapped damage");
        
        // Check for correct events
        let events = bus.events();
        
        // Should have StatusDamage event
        let has_status_damage = events.iter().any(|e| {
            matches!(e, BattleEvent::StatusDamage { 
                target: Species::Pikachu, 
                status: PokemonCondition::Trapped { .. },
                damage
            } if *damage == expected_damage)
        });
        assert!(has_status_damage, "Should generate StatusDamage event for trapped condition");
        
        println!("Trapped condition test events:");
        for event in events {
            println!("  {:?}", event);
        }
    }

    #[test]
    fn test_trapped_condition_causes_fainting() {
        init_test_data();
        
        let mut battle_state = create_test_battle_state(5, 100); // Low HP for player 1
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50]);
        
        // Add Trapped condition
        battle_state.players[0].add_condition(PokemonCondition::Trapped { turns_remaining: 2 });
        
        // Execute end-of-turn phase
        execute_end_turn_phase(&mut battle_state, &mut bus, &mut rng);
        
        // Pokemon should be fainted
        let pokemon = battle_state.players[0].team[0].as_ref().unwrap();
        assert!(pokemon.is_fainted(), "Pokemon should faint from trapped damage");
        assert_eq!(pokemon.current_hp(), 0, "Pokemon HP should be 0");
        
        // Check for PokemonFainted event
        let events = bus.events();
        let has_fainted_event = events.iter().any(|e| {
            matches!(e, BattleEvent::PokemonFainted { 
                player_index: 0,
                pokemon: Species::Pikachu 
            })
        });
        assert!(has_fainted_event, "Should generate PokemonFainted event");
    }

    #[test]
    fn test_seeded_condition_causes_fainting_and_heals_opponent() {
        init_test_data();
        
        // --- Setup ---
        // Create a battle where Player 1's Pokémon has very low HP (3).
        // This is less than the expected Leech Seed damage, ensuring it will faint.
        let mut battle_state = create_test_battle_state(3, 100);
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50]);
        
        // Add the Seeded condition to Player 1's Pokémon
        battle_state.players[0].add_condition(PokemonCondition::Seeded);
        
        // --- Calculation ---
        // We need to calculate the *actual* damage dealt to determine the healing amount.
        let pokemon_p1_ref = battle_state.players[0].team[0].as_ref().unwrap();
        let max_hp_p1 = pokemon_p1_ref.max_hp();
        let current_hp_p1 = pokemon_p1_ref.current_hp(); // This is 3 HP

        // Leech Seed would normally drain 1/8 of max HP.
        let potential_drain = (max_hp_p1 / 8).max(1);
        
        // However, the damage is capped by the target's remaining HP.
        let actual_damage_dealt = potential_drain.min(current_hp_p1); // min(potential_drain, 3)
        
        // The user is healed by the amount of HP that was actually drained.
        let expected_healing = actual_damage_dealt; 

        // Record the opponent's initial state for the final assertion.
        let initial_hp_p2 = battle_state.players[1].team[0].as_ref().unwrap().current_hp();

        // --- Action ---
        // Execute the end-of-turn phase, which will trigger the Leech Seed effect.
        execute_end_turn_phase(&mut battle_state, &mut bus, &mut rng);
        
        // --- Verification ---
        // 1. Check that Player 1's Pokémon has fainted.
        let pokemon_p1_final = battle_state.players[0].team[0].as_ref().unwrap();
        assert!(
            pokemon_p1_final.is_fainted(),
            "Pokémon should faint when its HP (3) is less than or equal to the Leech Seed damage."
        );
        
        // 2. Check that Player 2 was healed by the correct amount, even though Player 1 fainted.
        let final_hp_p2 = battle_state.players[1].team[0].as_ref().unwrap().current_hp();
        assert_eq!(
            final_hp_p2,
            initial_hp_p2 + expected_healing,
            "Opponent should have been healed by the actual damage dealt. Expected HP: {}, Final HP: {}",
            initial_hp_p2 + expected_healing,
            final_hp_p2
        );
        
        // 3. Check that both a Faint event and a Heal event were generated.
        let events = bus.events();
        let has_fainted_event = events.iter().any(|e| {
            matches!(e, BattleEvent::PokemonFainted { player_index: 0, .. })
        });
        let has_heal_event = events.iter().any(|e| {
            matches!(e, BattleEvent::PokemonHealed { target: Species::Charmander, amount, .. } if *amount == expected_healing)
        });
        
        assert!(has_fainted_event, "A PokemonFainted event should have been generated for player 0.");
        assert!(has_heal_event, "A PokemonHealed event for the correct amount should be generated, even when the target faints.");
    }
    
    #[test]
    fn test_seeded_no_heal_when_opponent_at_full_hp() {
        init_test_data();
        
        let mut battle_state = create_test_battle_state(100, 100); // Both at full HP
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50]);
        
        // Set player 2 to max HP (simulate species max)
        battle_state.players[1].team[0].as_mut().unwrap().set_hp_to_max();
        
        // Add Seeded condition to player 1
        battle_state.players[0].add_condition(PokemonCondition::Seeded);
        
        let initial_hp_p2 = battle_state.players[1].team[0].as_ref().unwrap().current_hp();
        
        // Execute end-of-turn phase
        execute_end_turn_phase(&mut battle_state, &mut bus, &mut rng);
        
        // Player 1 should take damage
        let pokemon_p1 = battle_state.players[0].team[0].as_ref().unwrap();
        assert!(pokemon_p1.current_hp() < 100, "Seeded Pokemon should take drain damage");
        
        // Player 2 should not be healed (already at max)
        let final_hp_p2 = battle_state.players[1].team[0].as_ref().unwrap().current_hp();
        assert_eq!(final_hp_p2, initial_hp_p2, "Opponent at full HP should not be healed");
        
        // Should have drain damage but no heal event
        let events = bus.events();
        let has_drain_damage = events.iter().any(|e| {
            matches!(e, BattleEvent::StatusDamage { 
                target: Species::Pikachu,
                status: PokemonCondition::Seeded,
                ..
            })
        });
        let has_heal_event = events.iter().any(|e| {
            matches!(e, BattleEvent::PokemonHealed { .. })
        });
        
        assert!(has_drain_damage, "Should have drain damage event");
        assert!(!has_heal_event, "Should not have heal event when opponent is at full HP");
    }

    #[test]
    fn test_both_conditions_simultaneously() {
        init_test_data();
        
        let mut battle_state = create_test_battle_state(100, 80);
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50]);
        
        // Add both conditions to player 1
        battle_state.players[0].add_condition(PokemonCondition::Trapped { turns_remaining: 2 });
        battle_state.players[0].add_condition(PokemonCondition::Seeded);
        
        let initial_hp_p1 = battle_state.players[0].team[0].as_ref().unwrap().current_hp();
        let initial_hp_p2 = battle_state.players[1].team[0].as_ref().unwrap().current_hp();
        let max_hp = battle_state.players[0].team[0].as_ref().unwrap().max_hp();
        
        let expected_trapped_damage = (max_hp / 16).max(1);
        let expected_seed_damage = (max_hp / 8).max(1);
        let total_expected_damage = expected_trapped_damage + expected_seed_damage;
        
        // Execute end-of-turn phase
        execute_end_turn_phase(&mut battle_state, &mut bus, &mut rng);
        
        // Player 1 should take damage from both conditions
        let final_hp_p1 = battle_state.players[0].team[0].as_ref().unwrap().current_hp();
        assert_eq!(final_hp_p1, initial_hp_p1 - total_expected_damage, "Should take damage from both conditions");
        
        // Player 2 should be healed by seed (but not trapped)
        let final_hp_p2 = battle_state.players[1].team[0].as_ref().unwrap().current_hp();
        assert_eq!(final_hp_p2, initial_hp_p2 + expected_seed_damage, "Should only be healed by seed drain");
        
        // Should have events for both conditions
        let events = bus.events();
        let trapped_events = events.iter().filter(|e| {
            matches!(e, BattleEvent::StatusDamage { 
                status: PokemonCondition::Trapped { .. }, 
                .. 
            })
        }).count();
        
        let seeded_events = events.iter().filter(|e| {
            matches!(e, BattleEvent::StatusDamage { 
                status: PokemonCondition::Seeded, 
                .. 
            })
        }).count();
        
        let heal_events = events.iter().filter(|e| {
            matches!(e, BattleEvent::PokemonHealed { .. })
        }).count();
        
        assert_eq!(trapped_events, 1, "Should have one trapped damage event");
        assert_eq!(seeded_events, 1, "Should have one seeded damage event");
        assert_eq!(heal_events, 1, "Should have one heal event");
    }

    #[test]
    fn test_no_damage_when_no_conditions() {
        init_test_data();
        
        let mut battle_state = create_test_battle_state(100, 80);
        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50]);
        
        let initial_hp_p1 = battle_state.players[0].team[0].as_ref().unwrap().current_hp();
        let initial_hp_p2 = battle_state.players[1].team[0].as_ref().unwrap().current_hp();
        
        // Execute end-of-turn phase with no conditions
        execute_end_turn_phase(&mut battle_state, &mut bus, &mut rng);
        
        // No HP should change
        let final_hp_p1 = battle_state.players[0].team[0].as_ref().unwrap().current_hp();
        let final_hp_p2 = battle_state.players[1].team[0].as_ref().unwrap().current_hp();
        
        assert_eq!(final_hp_p1, initial_hp_p1, "Player 1 HP should not change");
        assert_eq!(final_hp_p2, initial_hp_p2, "Player 2 HP should not change");
        
        // Should have no condition-related damage events
        let events = bus.events();
        let condition_damage_events = events.iter().filter(|e| {
            matches!(e, BattleEvent::StatusDamage { 
                status: PokemonCondition::Trapped { .. } | PokemonCondition::Seeded, 
                .. 
            })
        }).count();
        
        let heal_events = events.iter().filter(|e| {
            matches!(e, BattleEvent::PokemonHealed { .. })
        }).count();
        
        assert_eq!(condition_damage_events, 0, "Should have no condition damage events");
        assert_eq!(heal_events, 0, "Should have no heal events");
    }
}