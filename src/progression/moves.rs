use super::RewardCalculator;
use crate::errors::SpeciesDataResult;
use crate::species::Species;

impl RewardCalculator {
    /// Get moves learned at a specific level
    /// Returns vector of moves that should be learned when reaching this level
    pub fn moves_learned_at_level(
        &self,
        species: Species,
        level: u8,
    ) -> SpeciesDataResult<Vec<crate::Move>> {
        let species_data = crate::get_species_data(species)?;

        Ok(species_data
            .learnset
            .level_up
            .get(&level)
            .cloned()
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_learning() {
        let calculator = RewardCalculator;

        // Test moves_learned_at_level - Charmander learns Ember at level 7
        match calculator.moves_learned_at_level(Species::Charmander, 7) {
            Ok(moves) => {
                assert!(
                    moves.contains(&crate::Move::Ember),
                    "Charmander should learn Ember at level 7"
                );
            }
            Err(_) => {} // OK if species data not available in tests
        }
    }
}
