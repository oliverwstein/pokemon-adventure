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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PokemonSpecies {
    pub pokedex_number: u16,
    pub name: String,
    pub types: Vec<PokemonType>,
    pub base_stats: BaseStats,
    pub learnset: Learnset,
    pub catch_rate: u8,
    pub base_exp: u16,
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
