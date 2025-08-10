#[cfg(test)]
mod tests {
    use crate::pokemon::{PokemonInst, StatusCondition, get_species_data, initialize_species_data};
    use crate::move_data::initialize_move_data;
    use crate::player::{BattlePlayer, PokemonCondition};
    use crate::species::Species;
    use std::path::Path;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init_test_data() {
        INIT.call_once(|| {
            let data_path = Path::new("data");
            initialize_move_data(data_path).expect("Failed to initialize move data");
            initialize_species_data(data_path).expect("Failed to initialize species data");
        });
    }

    #[test]
    fn test_poison_status_damage() {
        init_test_data();
        
        // Create a test Pokemon with poison status
        let species_data = get_species_data(Species::Bulbasaur).expect("Failed to load Bulbasaur data");
        let mut pokemon = PokemonInst::new(Species::Bulbasaur, &species_data, 50, None, None);
        
        let initial_hp = pokemon.curr_stats[0];
        let max_hp = pokemon.max_hp();
        
        // Apply regular poison (severity 0)
        pokemon.status = Some(StatusCondition::Poison(0));
        
        // Tick status - should deal 1/16 of max HP damage
        let (damage, should_cure, status_changed) = pokemon.tick_status();
        
        assert_eq!(damage, (max_hp / 16).max(1));
        assert!(!should_cure);
        assert!(!status_changed); // Regular poison doesn't change severity
        assert_eq!(pokemon.curr_stats[0], initial_hp - damage);
        assert!(matches!(pokemon.status, Some(StatusCondition::Poison(0))));
    }

    #[test]
    fn test_badly_poisoned_status_damage() {
        init_test_data();
        
        // Create a test Pokemon with badly poisoned status
        let species_data = get_species_data(Species::Bulbasaur).expect("Failed to load Bulbasaur data");
        let mut pokemon = PokemonInst::new(Species::Bulbasaur, &species_data, 50, None, None);
        
        let initial_hp = pokemon.curr_stats[0];
        let max_hp = pokemon.max_hp();
        
        // Apply badly poisoned (severity 1)
        pokemon.status = Some(StatusCondition::Poison(1));
        
        // Tick status - should increase severity and deal more damage
        let (damage, should_cure, status_changed) = pokemon.tick_status();
        
        // Severity should increase to 2, damage should be 2/16 of max HP
        let expected_damage = (max_hp * 2 / 16).max(1);
        assert_eq!(damage, expected_damage);
        assert!(!should_cure);
        assert!(status_changed); // Badly poisoned severity increases
        assert_eq!(pokemon.curr_stats[0], initial_hp - damage);
        assert!(matches!(pokemon.status, Some(StatusCondition::Poison(2))));
    }

    #[test]
    fn test_burn_status_damage() {
        init_test_data();
        
        // Create a test Pokemon with burn status
        let species_data = get_species_data(Species::Charmander).expect("Failed to load Charmander data");
        let mut pokemon = PokemonInst::new(Species::Charmander, &species_data, 50, None, None);
        
        let initial_hp = pokemon.curr_stats[0];
        let max_hp = pokemon.max_hp();
        
        // Apply burn
        pokemon.status = Some(StatusCondition::Burn);
        
        // Tick status - should deal 1/16 of max HP damage
        let (damage, should_cure, status_changed) = pokemon.tick_status();
        
        assert_eq!(damage, (max_hp / 16).max(1));
        assert!(!should_cure);
        assert!(!status_changed); // Burn doesn't change
        assert_eq!(pokemon.curr_stats[0], initial_hp - damage);
        assert!(matches!(pokemon.status, Some(StatusCondition::Burn)));
    }

    #[test]
    fn test_sleep_status_countdown() {
        init_test_data();
        
        // Create a test Pokemon with sleep status
        let species_data = get_species_data(Species::Snorlax).expect("Failed to load Snorlax data");
        let mut pokemon = PokemonInst::new(Species::Snorlax, &species_data, 50, None, None);
        
        let initial_hp = pokemon.curr_stats[0];
        
        // Apply sleep for 3 turns
        pokemon.status = Some(StatusCondition::Sleep(3));
        
        // First tick - should reduce to 2 turns, no wake up
        let (damage, should_cure, status_changed) = pokemon.tick_status();
        assert_eq!(damage, 0);
        assert!(!should_cure);
        assert!(status_changed);
        assert_eq!(pokemon.curr_stats[0], initial_hp); // No damage from sleep
        assert!(matches!(pokemon.status, Some(StatusCondition::Sleep(2))));
        
        // Second tick - should reduce to 1 turn
        let (damage, should_cure, status_changed) = pokemon.tick_status();
        assert_eq!(damage, 0);
        assert!(!should_cure);
        assert!(status_changed);
        assert!(matches!(pokemon.status, Some(StatusCondition::Sleep(1))));
        
        // Third tick - should wake up
        let (damage, should_cure, status_changed) = pokemon.tick_status();
        assert_eq!(damage, 0);
        assert!(should_cure);
        assert!(status_changed);
        assert!(pokemon.status.is_none());
    }

    #[test]
    fn test_active_condition_timers() {
        init_test_data();
        
        let species_data = get_species_data(Species::Pikachu).expect("Failed to load Pikachu data");
        let pokemon = PokemonInst::new(Species::Pikachu, &species_data, 25, None, None);
        let mut player = BattlePlayer::new("test".to_string(), "Test".to_string(), vec![pokemon]);
        
        // Add some conditions with timers
        player.add_condition(PokemonCondition::Confused { turns_remaining: 3 });
        player.add_condition(PokemonCondition::Trapped { turns_remaining: 2 });
        player.add_condition(PokemonCondition::Flinched); // Should expire after 1 turn
        
        // First tick
        let expired = player.tick_active_conditions();
        assert_eq!(expired.len(), 1); // Flinched should expire
        assert!(expired.contains(&PokemonCondition::Flinched));
        
        // Check remaining conditions with actual values
        assert!(!player.has_condition(&PokemonCondition::Flinched));
        
        // Check confused condition has 2 turns remaining
        let confused_key = PokemonCondition::Confused { turns_remaining: 2 };
        match player.get_condition(&confused_key) {
            Some(condition) => {
                if let PokemonCondition::Confused { turns_remaining } = condition {
                    assert_eq!(*turns_remaining, 2);
                } else {
                    panic!("Expected Confused condition, got: {:?}", condition);
                }
            },
            None => {
                println!("Available conditions: {:?}", player.active_pokemon_conditions);
                panic!("Confused condition should exist with 2 turns remaining");
            }
        }
        
        // Check trapped condition has 1 turn remaining  
        if let Some(PokemonCondition::Trapped { turns_remaining }) = player.get_condition(&PokemonCondition::Trapped { turns_remaining: 1 }) {
            assert_eq!(*turns_remaining, 1);
        } else {
            panic!("Trapped condition should exist with 1 turn remaining");
        }
        
        // Second tick
        let expired = player.tick_active_conditions();
        assert_eq!(expired.len(), 1); // Trapped should expire
        assert!(expired.contains(&PokemonCondition::Trapped { turns_remaining: 1 }));
        
        // Check confused condition now has 1 turn remaining
        if let Some(PokemonCondition::Confused { turns_remaining }) = player.get_condition(&PokemonCondition::Confused { turns_remaining: 1 }) {
            assert_eq!(*turns_remaining, 1);
        } else {
            panic!("Confused condition should exist with 1 turn remaining");
        }
        
        // Third tick
        let expired = player.tick_active_conditions();
        assert_eq!(expired.len(), 1); // Confused should expire
        assert!(expired.contains(&PokemonCondition::Confused { turns_remaining: 1 }));
        
        // Should have no active conditions now
        assert_eq!(player.active_pokemon_conditions.len(), 0);
    }

    #[test]
    fn test_status_damage_causes_fainting() {
        init_test_data();
        
        // Create a low HP Pokemon
        let species_data = get_species_data(Species::Magikarp).expect("Failed to load Magikarp data");
        let mut pokemon = PokemonInst::new(Species::Magikarp, &species_data, 5, None, None);
        
        // Set HP very low
        pokemon.curr_stats[0] = 3;
        let max_hp = pokemon.max_hp();
        
        // Apply poison - damage should be enough to faint
        pokemon.status = Some(StatusCondition::Poison(0));
        let expected_damage = (max_hp / 16).max(1);
        
        // If damage >= current HP, should faint
        let (damage, should_cure, status_changed) = pokemon.tick_status();
        
        if expected_damage >= 3 {
            // Should faint
            assert_eq!(damage, expected_damage);
            assert!(!should_cure); // Fainting handles status removal
            assert!(status_changed); // Status changed to Faint
            assert_eq!(pokemon.curr_stats[0], 0);
            assert!(matches!(pokemon.status, Some(StatusCondition::Faint)));
        } else {
            // Should survive
            assert_eq!(pokemon.curr_stats[0], 3 - damage);
            assert!(matches!(pokemon.status, Some(StatusCondition::Poison(0))));
        }
    }
}