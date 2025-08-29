use crate::battle::state::TurnRng;
use crate::pokemon::{get_species_data, PokemonInst, StatusCondition};

/// Calculate the catch rate using authentic Gen 1 formula
/// Formula: catch_rate = min(255, (species_catch_rate * status_multiplier * ball_multiplier * hp_multiplier) / 3)
pub fn calculate_catch_rate(
    target_pokemon: &PokemonInst,
    ball_multiplier: f32, // 1.0 for regular pokeball, extensible for other ball types
) -> f32 {
    // Get species catch rate from data
    let species_data = get_species_data(target_pokemon.species)
        .expect("Species data must exist for target pokemon");
    let base_catch_rate = species_data.catch_rate as f32;

    // Status condition multiplier
    let status_multiplier = calculate_status_multiplier(&target_pokemon.status);

    // HP-based multiplier: (max_hp * 3 - current_hp * 2) / (max_hp * 3)
    let max_hp = target_pokemon.max_hp() as f32;
    let current_hp = target_pokemon.current_hp() as f32;
    let hp_multiplier = (max_hp * 3.0 - current_hp * 2.0) / (max_hp * 3.0);

    // Calculate final catch rate (Gen 1 formula)
    let catch_rate = (base_catch_rate * status_multiplier * ball_multiplier * hp_multiplier) / 3.0;

    // Cap at 255 (Gen 1 maximum)
    catch_rate.min(255.0)
}

/// Calculate status condition multiplier for catch rate
fn calculate_status_multiplier(status: &Option<StatusCondition>) -> f32 {
    match status {
        Some(StatusCondition::Sleep(_)) => 2.0,
        Some(StatusCondition::Freeze) => 2.0,
        Some(StatusCondition::Paralysis) => 1.5,
        Some(StatusCondition::Burn) => 1.5,
        Some(StatusCondition::Poison(_)) => 1.5,
        Some(StatusCondition::Faint) => 1.0, // Shouldn't catch fainted Pokemon anyway
        None => 1.0,
    }
}

/// Roll for catch success using the calculated catch rate
/// Returns true if the catch succeeds
pub fn roll_catch_success(catch_rate: f32, rng: &mut TurnRng) -> bool {
    let roll = rng.next_outcome("catch roll") as f32;
    roll < catch_rate
}

/// Get a descriptive catch rate category for display purposes
pub fn get_catch_rate_description(catch_rate: f32) -> &'static str {
    match catch_rate {
        r if r >= 200.0 => "Excellent",
        r if r >= 150.0 => "Very Good",
        r if r >= 100.0 => "Good",
        r if r >= 50.0 => "Fair",
        r if r >= 25.0 => "Poor",
        _ => "Very Poor",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pokemon::StatusCondition;
    use crate::species::Species;

    fn create_test_pokemon(
        species: Species,
        current_hp_ratio: f32,
        status: Option<StatusCondition>,
    ) -> PokemonInst {
        let species_data = get_species_data(species).unwrap();
        let mut pokemon = PokemonInst::new(species, &species_data, 25, None, None);

        // Set HP ratio
        let target_hp = (pokemon.max_hp() as f32 * current_hp_ratio) as u16;
        if target_hp < pokemon.current_hp() {
            pokemon.take_damage(pokemon.current_hp() - target_hp);
        }

        // Set status
        if let Some(s) = status {
            pokemon.status = Some(s);
        }

        pokemon
    }

    #[test]
    fn test_status_multipliers() {
        assert_eq!(calculate_status_multiplier(&None), 1.0);
        assert_eq!(
            calculate_status_multiplier(&Some(StatusCondition::Sleep(3))),
            2.0
        );
        assert_eq!(
            calculate_status_multiplier(&Some(StatusCondition::Freeze)),
            2.0
        );
        assert_eq!(
            calculate_status_multiplier(&Some(StatusCondition::Paralysis)),
            1.5
        );
        assert_eq!(
            calculate_status_multiplier(&Some(StatusCondition::Burn)),
            1.5
        );
        assert_eq!(
            calculate_status_multiplier(&Some(StatusCondition::Poison(0))),
            1.5
        );
    }

    #[test]
    fn test_catch_rate_calculation_healthy_pokemon() {
        let pokemon = create_test_pokemon(Species::Pidgey, 1.0, None); // Full HP, no status
        let catch_rate = calculate_catch_rate(&pokemon, 1.0);

        // Pidgey has high catch rate (255), so even at full HP should have decent rate
        // Formula: (255 * 1.0 * 1.0 * (1.0/3.0)) / 3.0 = 28.33...
        assert!((catch_rate - 28.33).abs() < 1.0);
    }

    #[test]
    fn test_catch_rate_calculation_low_hp() {
        let pokemon = create_test_pokemon(Species::Pidgey, 0.1, None); // 10% HP
        let catch_rate = calculate_catch_rate(&pokemon, 1.0);

        // Should be much higher than full HP
        // HP multiplier at 10%: (3 - 0.2) / 3 = 2.8/3 = 0.933
        // Formula: (255 * 1.0 * 1.0 * 0.933) / 3.0 = 79.33
        assert!(catch_rate > 70.0);
    }

    #[test]
    fn test_catch_rate_calculation_with_sleep() {
        let pokemon = create_test_pokemon(Species::Pidgey, 0.5, Some(StatusCondition::Sleep(3))); // Half HP, asleep
        let catch_rate = calculate_catch_rate(&pokemon, 1.0);

        // Should be very high with sleep bonus
        // HP multiplier at 50%: (3 - 1) / 3 = 2/3 = 0.666
        // Formula: (255 * 2.0 * 1.0 * 0.666) / 3.0 = 113.33
        assert!(catch_rate > 100.0);
    }

    #[test]
    fn test_catch_rate_calculation_legendary() {
        // Use a legendary with very low catch rate (3)
        let pokemon = create_test_pokemon(Species::Articuno, 0.1, Some(StatusCondition::Sleep(3)));
        let catch_rate = calculate_catch_rate(&pokemon, 1.0);

        // Even with best conditions, should still be quite difficult
        // HP multiplier at 10%: 0.933, Status: 2.0
        // Formula: (3 * 2.0 * 1.0 * 0.933) / 3.0 = 1.866
        assert!(catch_rate < 10.0);
        assert!(catch_rate > 1.0);
    }

    #[test]
    fn test_catch_rate_cap() {
        // Test that catch rate caps at 255
        let pokemon = create_test_pokemon(Species::Pidgey, 0.01, Some(StatusCondition::Sleep(3))); // 1% HP, asleep
        let catch_rate = calculate_catch_rate(&pokemon, 10.0); // Super ball

        assert_eq!(catch_rate, 255.0);
    }

    #[test]
    fn test_catch_success_rolls() {
        let mut rng = TurnRng::new_for_test(vec![50, 100, 200]);

        // Roll of 50 should succeed with catch rate 100
        assert!(roll_catch_success(100.0, &mut rng));

        // Roll of 100 should fail with catch rate 50
        assert!(!roll_catch_success(50.0, &mut rng));

        // Roll of 200 should fail with any reasonable catch rate
        assert!(!roll_catch_success(150.0, &mut rng));
    }

    #[test]
    fn test_catch_rate_descriptions() {
        assert_eq!(get_catch_rate_description(250.0), "Excellent");
        assert_eq!(get_catch_rate_description(175.0), "Very Good");
        assert_eq!(get_catch_rate_description(125.0), "Good");
        assert_eq!(get_catch_rate_description(75.0), "Fair");
        assert_eq!(get_catch_rate_description(35.0), "Poor");
        assert_eq!(get_catch_rate_description(10.0), "Very Poor");
    }
}
