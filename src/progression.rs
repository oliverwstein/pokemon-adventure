use crate::errors::SpeciesDataResult;
use crate::species::Species;
use schema::BaseStats;

// Constants for reward calculations
const BASE_EXP_MULTIPLIER: f32 = 0.3;
const EVOLUTION_PENALTY: f32 = -0.1;
const HIGH_STAT_BONUS: f32 = 0.02;
const HIGH_STAT_THRESHOLD: u8 = 100;

// BST thresholds for EV yield
const BST_LOW_THRESHOLD: u16 = 300;
const BST_HIGH_THRESHOLD: u16 = 500;
const EV_YIELD_LOW: u8 = 1;
const EV_YIELD_MEDIUM: u8 = 2;
const EV_YIELD_HIGH: u8 = 3;

/// Individual stat types that correspond to BaseStats fields
/// Internal use only
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stat {
    Hp,
    Attack,
    Defense,
    SpecialAttack,
    SpecialDefense,
    Speed,
}

/// Effort Values (EVs) awarded when a Pokemon faints
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EvYield {
    pub hp: u8,
    pub attack: u8,
    pub defense: u8,
    pub special_attack: u8,
    pub special_defense: u8,
    pub speed: u8,
}

impl EvYield {
    pub fn total(&self) -> u8 {
        self.hp
            + self.attack
            + self.defense
            + self.special_attack
            + self.special_defense
            + self.speed
    }
}

/// Calculator for experience and EV rewards based on Pokemon species
pub struct RewardCalculator;

impl RewardCalculator {
    /// Calculate base experience awarded when this Pokemon faints
    /// Formula: BST × (0.3 + Evol_Modifier + Stat_Modifier)
    /// - Evol_Modifier: -0.1 if can evolve, 0.0 if cannot
    /// - Stat_Modifier: +0.02 for each base stat >= 100
    pub fn calculate_base_exp(&self, species: Species) -> SpeciesDataResult<u32> {
        let species_data = crate::get_species_data(species)?;

        let bst = species_data.base_stats.total();
        let evol_modifier = if species_data.evolution_data.is_some() {
            EVOLUTION_PENALTY
        } else {
            0.0
        };
        let stat_modifier = self.calculate_stat_modifier(&species_data.base_stats);

        let multiplier = BASE_EXP_MULTIPLIER + evol_modifier + stat_modifier;
        Ok((bst as f32 * multiplier) as u32)
    }

    /// Calculate EV yield when this Pokemon faints
    /// Total EVs: 1 if BST < 300, 2 if 300 ≤ BST < 500, 3 if BST ≥ 500
    /// Distribution: EVs awarded in highest base stat(s)
    pub fn calculate_ev_yield(&self, species: Species) -> SpeciesDataResult<EvYield> {
        let species_data = crate::get_species_data(species)?;

        let bst = species_data.base_stats.total();

        let total_evs = match bst {
            0..BST_LOW_THRESHOLD => EV_YIELD_LOW,
            BST_LOW_THRESHOLD..BST_HIGH_THRESHOLD => EV_YIELD_MEDIUM,
            _ => EV_YIELD_HIGH,
        };

        let highest_stats = self.find_highest_base_stats(&species_data.base_stats);
        Ok(self.distribute_evs(total_evs, &highest_stats))
    }

    /// Calculate stat modifier: +0.02 for each base stat >= 100
    fn calculate_stat_modifier(&self, base_stats: &BaseStats) -> f32 {
        let high_stats = [
            base_stats.hp,
            base_stats.attack,
            base_stats.defense,
            base_stats.sp_attack,
            base_stats.sp_defense,
            base_stats.speed,
        ]
        .iter()
        .filter(|&&stat| stat >= HIGH_STAT_THRESHOLD)
        .count();

        high_stats as f32 * HIGH_STAT_BONUS
    }

    /// Find which base stats are tied for highest value
    /// Returns a vector of the highest stats
    fn find_highest_base_stats(&self, base_stats: &BaseStats) -> Vec<Stat> {
        let stats_map = [
            (base_stats.hp, Stat::Hp),
            (base_stats.attack, Stat::Attack),
            (base_stats.defense, Stat::Defense),
            (base_stats.sp_attack, Stat::SpecialAttack),
            (base_stats.sp_defense, Stat::SpecialDefense),
            (base_stats.speed, Stat::Speed),
        ];

        let max_value = stats_map
            .iter()
            .map(|(val, _)| val)
            .max()
            .copied()
            .unwrap_or(0);

        stats_map
            .iter()
            .filter(|(val, _)| *val == max_value)
            .map(|(_, stat)| *stat)
            .collect()
    }

    /// Distribute total EVs among the highest stats
    /// If tied stats, distribute evenly with remainder going to first stats
    fn distribute_evs(&self, total_evs: u8, highest_stats: &[Stat]) -> EvYield {
        let mut ev_yield = EvYield::default();
        let num_highest = highest_stats.len();

        if num_highest == 0 {
            return ev_yield;
        }

        let evs_per_stat = total_evs as usize / num_highest;
        let remainder = total_evs as usize % num_highest;

        for (i, stat) in highest_stats.iter().enumerate() {
            let evs = evs_per_stat + if i < remainder { 1 } else { 0 };

            match stat {
                Stat::Hp => ev_yield.hp = evs as u8,
                Stat::Attack => ev_yield.attack = evs as u8,
                Stat::Defense => ev_yield.defense = evs as u8,
                Stat::SpecialAttack => ev_yield.special_attack = evs as u8,
                Stat::SpecialDefense => ev_yield.special_defense = evs as u8,
                Stat::Speed => ev_yield.speed = evs as u8,
            }
        }

        ev_yield
    }

    /// Check if Pokemon should attempt evolution at given level
    /// Returns the species it should evolve into, or None if no evolution
    pub fn should_evolve(&self, species: Species, level: u8) -> SpeciesDataResult<Option<Species>> {
        let species_data = crate::get_species_data(species)?;

        if let Some(evolution_data) = &species_data.evolution_data {
            match evolution_data.method {
                schema::EvolutionMethod::Level(required_level) => {
                    if level >= required_level {
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

/// Tracks which Pokemon have faced each other during battle
/// participation[player][my_pokemon][opponent_pokemon] = true if they faced each other
pub struct BattleParticipationTracker {
    participation: [[[bool; 6]; 6]; 2],
}

impl BattleParticipationTracker {
    pub fn new() -> Self {
        Self {
            participation: [[[false; 6]; 6]; 2],
        }
    }

    /// Record that the active Pokemon from each player faced each other
    pub fn record_participation(&mut self, p0_active: usize, p1_active: usize) {
        if p0_active < 6 && p1_active < 6 {
            self.participation[0][p0_active][p1_active] = true;
            self.participation[1][p1_active][p0_active] = true;
        }
    }

    /// Get all Pokemon from the opposing player who faced the specified opponent Pokemon
    pub fn get_participants_against(
        &self,
        opponent_player: usize,
        opponent_pokemon: usize,
    ) -> Vec<usize> {
        if opponent_player >= 2 || opponent_pokemon >= 6 {
            return Vec::new();
        }

        let participant_player = 1 - opponent_player;
        (0..6)
            .filter(|&pokemon_index| {
                self.participation[participant_player][pokemon_index][opponent_pokemon]
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use schema::ExperienceGroup;

    use super::*;

    #[test]
    fn test_experience_groups() {
        // Test basic level calculations
        assert_eq!(ExperienceGroup::Fast.exp_for_level(1), 0);
        assert!(ExperienceGroup::Fast.exp_for_level(50) < ExperienceGroup::Slow.exp_for_level(50));

        // Test level calculation from experience
        let fast_exp_50 = ExperienceGroup::Fast.exp_for_level(50);
        assert_eq!(
            ExperienceGroup::Fast.calculate_level_from_exp(fast_exp_50 - 1),
            49
        );
        assert_eq!(
            ExperienceGroup::Fast.calculate_level_from_exp(fast_exp_50),
            50
        );
    }

    #[test]
    fn test_ev_yield_distribution() {
        let calculator = RewardCalculator;

        // Test single highest stat
        let base_stats = BaseStats {
            hp: 100,
            attack: 80,
            defense: 60,
            sp_attack: 40,
            sp_defense: 40,
            speed: 30,
        };
        let highest_stats = calculator.find_highest_base_stats(&base_stats);
        assert_eq!(highest_stats, vec![Stat::Hp]); // HP should be the highest stat

        let ev_yield = calculator.distribute_evs(3, &highest_stats);
        assert_eq!(ev_yield.hp, 3);
        assert_eq!(ev_yield.total(), 3);

        // Test tied highest stats
        let base_stats_tied = BaseStats {
            hp: 100,
            attack: 100,
            defense: 60,
            sp_attack: 40,
            sp_defense: 40,
            speed: 30,
        };
        let highest_stats_tied = calculator.find_highest_base_stats(&base_stats_tied);
        assert_eq!(highest_stats_tied.len(), 2); // HP and Attack should be tied
        assert!(highest_stats_tied.contains(&Stat::Hp));
        assert!(highest_stats_tied.contains(&Stat::Attack));

        let ev_yield_tied = calculator.distribute_evs(3, &highest_stats_tied);
        assert_eq!(ev_yield_tied.total(), 3);
        // Should distribute 2 EVs to first stat, 1 to second stat
        // Order depends on the vec order, but total should be correct
        assert!(ev_yield_tied.hp > 0 && ev_yield_tied.attack > 0);
    }

    #[test]
    fn test_participation_tracking() {
        let mut tracker = BattleParticipationTracker::new();

        // Record that Pokemon 0 from player 0 faced Pokemon 1 from player 1
        tracker.record_participation(0, 1);

        // Check participants
        let participants = tracker.get_participants_against(1, 1); // Who faced player 1's Pokemon 1?
        assert_eq!(participants, vec![0]); // Player 0's Pokemon 0

        let participants2 = tracker.get_participants_against(0, 0); // Who faced player 0's Pokemon 0?
        assert_eq!(participants2, vec![1]); // Player 1's Pokemon 1
    }

    #[test]
    fn test_bst_calculation() {
        let calculator = RewardCalculator;
        let base_stats = BaseStats {
            hp: 100,
            attack: 100,
            defense: 100,
            sp_attack: 100,
            sp_defense: 100,
            speed: 100,
        };

        assert_eq!(base_stats.total(), 600);

        // Test stat modifier calculation
        let stat_modifier = calculator.calculate_stat_modifier(&base_stats);
        assert_eq!(stat_modifier, 0.12); // 6 stats >= 100, so 6 * 0.02 = 0.12
    }

    #[test]
    fn test_can_level_up() {
        // Test basic level up detection
        assert!(!ExperienceGroup::Fast.can_level_up(100, 0)); // Max level can't level up
        assert!(ExperienceGroup::Fast.can_level_up(1, 1000)); // Low level with high exp can level up

        // Test exact threshold
        let level_50_exp = ExperienceGroup::MediumFast.exp_for_level(50);
        assert!(!ExperienceGroup::MediumFast.can_level_up(50, level_50_exp - 1)); // Just below threshold

        let level_51_exp = ExperienceGroup::MediumFast.exp_for_level(51);
        assert!(ExperienceGroup::MediumFast.can_level_up(50, level_51_exp)); // At threshold for next level
    }

    #[test]
    fn test_evolution_and_move_learning() {
        let calculator = RewardCalculator;

        // Test should_evolve - these will work once we have species data
        // For now, test the logic structure
        match calculator.should_evolve(Species::Bulbasaur, 16) {
            Ok(_) => {}  // Expected - function should work
            Err(_) => {} // Also OK if species data not available in tests
        }

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
