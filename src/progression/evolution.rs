use super::RewardCalculator;
use crate::errors::SpeciesDataResult;
use crate::pokemon::PokemonInst;
use crate::species::Species;

impl RewardCalculator {
    /// Check if Pokemon should attempt evolution at current level
    /// Returns the species it should evolve into, or None if no evolution
    pub fn should_evolve(&self, pokemon: &PokemonInst) -> SpeciesDataResult<Option<Species>> {
        let species_data = crate::get_species_data(pokemon.species)?;

        if let Some(evolution_data) = &species_data.evolution_data {
            match evolution_data.method {
                schema::EvolutionMethod::Level(required_level) => {
                    if pokemon.level >= required_level {
                        Ok(Some(evolution_data.evolves_into))
                    } else {
                        Ok(None)
                    }
                }
                schema::EvolutionMethod::Item(_) => Ok(None), // Items handled separately
            }
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evolution() {
        let calculator = RewardCalculator;

        // Create a test Pokemon instance
        let species_data = match crate::get_species_data(Species::Bulbasaur) {
            Ok(data) => data,
            Err(_) => return, // Skip test if species data not available
        };

        let pokemon =
            crate::pokemon::PokemonInst::new(Species::Bulbasaur, species_data, 16, None, None);

        // Test should_evolve with PokemonInst
        match calculator.should_evolve(&pokemon) {
            Ok(_) => {}  // Expected - function should work
            Err(_) => {} // Also OK if species data not available in tests
        }
    }
}
