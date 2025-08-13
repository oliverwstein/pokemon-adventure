use serde::{Deserialize, Serialize};

use crate::{
    moves::Move,
    pokemon::{PokemonInst, PokemonType},
};
use std::hash::{Hash, Hasher};
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PokemonCondition {
    Flinched,
    Confused {
        turns_remaining: u8,
    }, // Counts down each turn
    Seeded,
    Underground,
    InAir,
    Teleported,
    Enraged,
    Exhausted {
        turns_remaining: u8,
    }, // Prevents acting for specified turns
    Trapped {
        turns_remaining: u8,
    },
    Charging,
    Rampaging {
        turns_remaining: u8,
    },
    Transformed {
        target: PokemonInst,
    },
    Converted {
        pokemon_type: PokemonType,
    },
    Disabled {
        pokemon_move: Move,
        turns_remaining: u8,
    }, // Counts down each turn
    Substitute {
        hp: u8,
    },
    Biding {
        turns_remaining: u8,
        damage: u16,
    },
    Countering {
        damage: u16,
    },
}

impl Hash for PokemonCondition {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash only the discriminant (variant), not the data
        std::mem::discriminant(self).hash(state);
    }
}

impl Eq for PokemonCondition {}

/// Condition type without data payload for RemoveCondition commands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PokemonConditionType {
    Flinched,
    Confused,
    Seeded,
    Underground,
    InAir,
    Teleported,
    Enraged,
    Exhausted,
    Trapped,
    Charging,
    Rampaging,
    Transformed,
    Converted,
    Biding,
    Countering,
    Substitute,
    Disabled,
}

impl PokemonCondition {
    pub fn get_type(&self) -> PokemonConditionType {
        match self {
            PokemonCondition::Flinched => PokemonConditionType::Flinched,
            PokemonCondition::Confused { .. } => PokemonConditionType::Confused,
            PokemonCondition::Seeded => PokemonConditionType::Seeded,
            PokemonCondition::Underground => PokemonConditionType::Underground,
            PokemonCondition::InAir => PokemonConditionType::InAir,
            PokemonCondition::Teleported => PokemonConditionType::Teleported,
            PokemonCondition::Enraged => PokemonConditionType::Enraged,
            PokemonCondition::Exhausted { .. } => PokemonConditionType::Exhausted,
            PokemonCondition::Trapped { .. } => PokemonConditionType::Trapped,
            PokemonCondition::Charging => PokemonConditionType::Charging,
            PokemonCondition::Rampaging { .. } => PokemonConditionType::Rampaging,
            PokemonCondition::Transformed { .. } => PokemonConditionType::Transformed,
            PokemonCondition::Converted { .. } => PokemonConditionType::Converted,
            PokemonCondition::Biding { .. } => PokemonConditionType::Biding,
            PokemonCondition::Countering { .. } => PokemonConditionType::Countering,
            PokemonCondition::Substitute { .. } => PokemonConditionType::Substitute,
            PokemonCondition::Disabled { .. } => PokemonConditionType::Disabled,
        }
    }
}
