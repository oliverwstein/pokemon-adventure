use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum PokemonType {
    Normal,
    Fighting,
    Flying,
    Poison,
    Ground,
    Rock,
    Bug,
    Ghost,
    Fire,
    Water,
    Grass,
    Electric,
    Psychic,
    Ice,
    Dragon,
    Typeless,
}

impl fmt::Display for PokemonType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl PokemonType {
    /// Calculate type effectiveness multiplier for attacking type vs defending type
    /// Returns: 2.0 = Super Effective, 1.0 = Normal, 0.5 = Not Very Effective, 0.0 = No Effect
    pub fn type_effectiveness(attacking: PokemonType, defending: PokemonType) -> f32 {
        use PokemonType::*;

        match (attacking, defending) {
            // Normal
            (Normal, Ghost) => 0.0,
            (Normal, Rock) => 0.5,
            (Normal, _) => 1.0,

            // Fire
            (Fire, Fire) | (Fire, Water) | (Fire, Rock) | (Fire, Dragon) => 0.5,
            (Fire, Grass) | (Fire, Ice) | (Fire, Bug) => 2.0,
            (Fire, _) => 1.0,

            // Water
            (Water, Water) | (Water, Grass) | (Water, Dragon) => 0.5,
            (Water, Fire) | (Water, Ground) | (Water, Rock) => 2.0,
            (Water, _) => 1.0,

            // Electric
            (Electric, Electric) | (Electric, Grass) | (Electric, Dragon) => 0.5,
            (Electric, Ground) => 0.0,
            (Electric, Water) | (Electric, Flying) => 2.0,
            (Electric, _) => 1.0,

            // Grass
            (Grass, Fire)
            | (Grass, Grass)
            | (Grass, Poison)
            | (Grass, Flying)
            | (Grass, Bug)
            | (Grass, Dragon) => 0.5,
            (Grass, Water) | (Grass, Ground) | (Grass, Rock) => 2.0,
            (Grass, _) => 1.0,

            // Ice
            (Ice, Fire) | (Ice, Water) | (Ice, Ice) => 0.5,
            (Ice, Grass) | (Ice, Ground) | (Ice, Flying) | (Ice, Dragon) => 2.0,
            (Ice, _) => 1.0,

            // Fighting
            (Fighting, Poison) | (Fighting, Flying) | (Fighting, Psychic) | (Fighting, Bug) => 0.5,
            (Fighting, Ghost) => 0.0,
            (Fighting, Normal) | (Fighting, Ice) | (Fighting, Rock) => 2.0,
            (Fighting, _) => 1.0,

            // Poison
            (Poison, Poison) | (Poison, Ground) | (Poison, Rock) | (Poison, Ghost) => 0.5,
            (Poison, Grass) => 2.0,
            (Poison, _) => 1.0,

            // Ground
            (Ground, Grass) | (Ground, Bug) => 0.5,
            (Ground, Flying) => 0.0,
            (Ground, Fire) | (Ground, Electric) | (Ground, Poison) | (Ground, Rock) => 2.0,
            (Ground, _) => 1.0,

            // Flying
            (Flying, Electric) | (Flying, Rock) => 0.5,
            (Flying, Grass) | (Flying, Fighting) | (Flying, Bug) => 2.0,
            (Flying, _) => 1.0,

            // Psychic
            (Psychic, Psychic) => 0.5,
            (Psychic, Fighting) | (Psychic, Poison) => 2.0,
            (Psychic, _) => 1.0,

            // Bug
            (Bug, Fire) | (Bug, Fighting) | (Bug, Poison) | (Bug, Flying) | (Bug, Ghost) => 0.5,
            (Bug, Grass) | (Bug, Psychic) => 2.0,
            (Bug, _) => 1.0,

            // Rock
            (Rock, Fighting) | (Rock, Ground) => 0.5,
            (Rock, Fire) | (Rock, Ice) | (Rock, Flying) | (Rock, Bug) => 2.0,
            (Rock, _) => 1.0,

            // Ghost
            (Ghost, Normal) => 0.0,
            (Ghost, Ghost) => 2.0,
            (Ghost, Psychic) => 0.5,
            (Ghost, _) => 1.0,

            // Dragon
            (Dragon, Dragon) => 2.0,
            (Dragon, _) => 1.0,
            (Typeless, _) => 1.0,
        }
    }

    pub fn is_immune(attacking: PokemonType, defending: PokemonType) -> bool {
        Self::type_effectiveness(attacking, defending) == 0.0
    }
}
