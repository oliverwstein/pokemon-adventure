use crate::{Move, PokemonType, Species};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Item {
    // Evolution stones
    FireStone,
    WaterStone,
    ThunderStone,
    LeafStone,
    MoonStone,
    // Add more items as needed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseStats {
    pub hp: u8,
    pub attack: u8,
    pub defense: u8,
    pub sp_attack: u8,
    pub sp_defense: u8,
    pub speed: u8,
}

impl BaseStats {
    pub fn total(&self) -> u16 {
        [
            self.hp,
            self.attack,
            self.defense,
            self.sp_attack,
            self.sp_defense,
            self.speed,
        ]
        .iter()
        .map(|&stat| u16::from(stat)) 
        .sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Learnset {
    pub level_up: HashMap<u8, Vec<Move>>, // level -> moves learned at that level
    pub signature: Option<Move>,          // Evolution line signature move
    pub can_learn: Vec<Move>,             // Moves learnable through tutoring/witnessing
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvolutionMethod {
    Level(u8),
    Item(Item),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionData {
    pub evolves_into: Species, // Species name
    pub method: EvolutionMethod,
}

/// Experience groups that determine leveling speed and curves
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ExperienceGroup {
    Fast,
    MediumFast,
    MediumSlow,
    Slow,
    Fluctuating,
    Erratic,
}

impl ExperienceGroup {
    /// Calculate total experience required to reach a given level
    /// Uses unified formula: A × n³ + B × n² × sin(C × n)
    pub fn exp_for_level(self, level: u8) -> u32 {
        let n = level as f64;
        
        let (a, b, c) = match self {
            ExperienceGroup::Fast => (0.8, 0.0, 0.0),
            ExperienceGroup::MediumFast => (1.0, 0.0, 0.0),
            ExperienceGroup::MediumSlow => (1.2, 0.0, 0.0),
            ExperienceGroup::Slow => (1.4, 0.0, 0.0),
            ExperienceGroup::Fluctuating => (1.0, 0.3, 0.5),
            ExperienceGroup::Erratic => (1.1, 0.2, 0.1),
        };
        
        let base = a * n.powi(3);
        let fluctuation = if b != 0.0 { b * n.powi(2) * (c * n).sin() } else { 0.0 };
        (base + fluctuation).max(0.0) as u32
    }
    
    /// Calculate what level a Pokemon should be based on total experience
    pub fn calculate_level_from_exp(self, total_exp: u32) -> u8 {
        // Linear search - could optimize with binary search if needed
        for level in 1..=100 {
            if self.exp_for_level(level) > total_exp {
                return level - 1;
            }
        }
        100 // Max level
    }

    pub fn can_level_up(self, current_level: u8, total_exp: u32) -> bool {
        if current_level >= 100 {
            return false;
        }
        let next_level_exp = self.exp_for_level(current_level + 1);
        total_exp >= next_level_exp
    }

    
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PokemonSpecies {
    pub pokedex_number: u16,
    pub name: String,
    pub types: Vec<PokemonType>,
    pub base_stats: BaseStats,
    pub learnset: Learnset,
    pub catch_rate: u8,
    pub base_exp: u16,
    pub experience_group: ExperienceGroup,
    pub description: String,
    pub evolution_data: Option<EvolutionData>,
}

impl Learnset {
    #[allow(dead_code)]
    pub fn learns_at_level(&self, level: u8) -> Option<&Vec<Move>> {
        self.level_up.get(&level)
    }

    #[allow(dead_code)]
    pub fn can_learn_move(&self, move_: Move) -> bool {
        // Check if move is in signature, level-up, or can_learn lists
        if let Some(signature) = self.signature {
            if signature == move_ {
                return true;
            }
        }

        // Check level-up moves
        for moves_at_level in self.level_up.values() {
            if moves_at_level.contains(&move_) {
                return true;
            }
        }

        // Check can_learn list
        self.can_learn.contains(&move_)
    }
}
