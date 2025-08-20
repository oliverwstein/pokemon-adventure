use crate::battle::conditions::PokemonCondition;
use crate::moves::Move;
use crate::species::Species;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt};

// Include the compiled species data
use crate::move_data::{get_compiled_species_data, MoveData};

/// Get species data for a specific species from the compiled data
pub fn get_species_data(species: Species) -> Option<PokemonSpecies> {
    let compiled_data = get_compiled_species_data();
    let index = species.pokedex_number() as usize - 1; // 0-indexed
    compiled_data[index].clone()
}

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
            (Ghost, Normal) | (Ghost, Psychic) => 0.0,
            (Ghost, Ghost) => 2.0,
            (Ghost, _) => 1.0,

            // Dragon
            (Dragon, Dragon) => 2.0,
            (Dragon, _) => 1.0,
            (Typeless, _) => 1.0,
        }
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub enum StatusCondition {
    Sleep(u8),
    Poison(u8),
    Burn,
    Freeze,
    Paralysis,
    Faint, // Pokemon has 0 HP, can replace any other status
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct MoveInstance {
    pub move_: Move,
    pub pp: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UseMoveError {
    NoPPRemaining,
    MoveNotKnown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct PokemonInst {
    pub name: String,     // Species name if no nickname
    pub species: Species, // Species enum for type-safe lookup
    pub level: u8,        // Pokemon's level (1-100)
    pub curr_exp: u8,     // Only really relevant for single-player
    curr_hp: u16,         // Current HP (private, use methods to access)
    pub ivs: [u8; 6],     // HP, ATK, DEF, SP.ATK, SP.DEF, SPD
    pub evs: [u8; 6],     // HP, ATK, DEF, SP.ATK, SP.DEF, SPD
    pub stats: CurrentStats,
    pub moves: [Option<MoveInstance>; 4], // Up to 4 moves
    pub status: Option<StatusCondition>,  // Status condition with optional parameter
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub struct CurrentStats {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub sp_attack: u16,
    pub sp_defense: u16,
    pub speed: u16,
}

impl From<[u16; 6]> for CurrentStats {
    fn from(stats: [u16; 6]) -> Self {
        CurrentStats {
            hp: stats[0],
            attack: stats[1],
            defense: stats[2],
            sp_attack: stats[3],
            sp_defense: stats[4],
            speed: stats[5],
        }
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
        if self.signature == Some(move_) {
            return true;
        }

        for moves in self.level_up.values() {
            if moves.contains(&move_) {
                return true;
            }
        }

        self.can_learn.contains(&move_)
    }
}


impl MoveInstance {
    /// Create a new move instance with max PP
    pub fn new(move_: Move) -> Self {
        let max_pp = MoveData::get_move_max_pp(move_);

        MoveInstance { move_, pp: max_pp }
    }

    /// Get the max PP for this move
    pub fn max_pp(&self) -> u8 {
        MoveData::get_move_max_pp(self.move_)
    }

    /// Use the move (decrease PP)
    pub fn use_move(&mut self) -> bool {
        if self.pp > 0 {
            self.pp -= 1;
            true
        } else {
            false
        }
    }

    /// Restore PP
    #[allow(dead_code)]
    pub fn restore_pp(&mut self, amount: u8) {
        let max_pp = self.max_pp();
        self.pp = (self.pp + amount).min(max_pp);
    }
}

impl PokemonInst {
    /// Create a new Pokemon instance from species data.
    pub fn new(
        species: Species,
        species_data: &PokemonSpecies,
        level: u8,
        ivs: Option<[u8; 6]>,
        moves: Option<Vec<Move>>,
    ) -> Self {
        Self::new_with_hp(species, species_data, level, ivs, moves, 999)
    }

    /// Create a new Pokemon instance with a specific starting HP.
    pub fn new_with_hp(
        species: Species,
        species_data: &PokemonSpecies,
        level: u8,
        ivs: Option<[u8; 6]>,
        moves: Option<Vec<Move>>,
        curr_hp:u16,
    ) -> Self {
        // Use default IVs (all 0) if not provided.
        // In a full implementation, you might want random IVs here.
        let ivs = ivs.unwrap_or_default();

        // Initialize EVs to 0.
        let evs = [0u8; 6];

        // Calculate stats, which now returns a `CurrentStats` struct.
        let stats = Self::calculate_stats(&species_data.base_stats, level, &ivs, &evs);

        // Derive moves from the learnset if not provided.
        let moves = moves
            .unwrap_or_else(|| Self::derive_moves_from_learnset(&species_data.learnset, level));

        // Create move instances with max PP.
        let mut move_array = [const { None }; 4];
        for (i, move_) in moves.into_iter().take(4).enumerate() {
            move_array[i] = Some(MoveInstance::new(move_));
        }

        let mut pokemon = PokemonInst {
            name: species_data.name.clone(),
            species,
            level,
            curr_exp: 0, // Simplified for now
            curr_hp: 0,  // Will be set below with validation
            ivs,
            evs,
            stats, // Assign the new `CurrentStats` struct here
            moves: move_array,
            status: None,
        };

        // Set HP using the validated setter. If no HP is provided, default to max HP.
        pokemon.set_hp(curr_hp);
        pokemon
    }

    /// Create a Pokemon instance for testing, maintaining the old array-based API.
    /// This bypasses stat calculation and allows direct control over all values.
    #[allow(dead_code)]
    pub fn new_for_test(
        species: Species,
        level: u8,
        curr_exp: u8,
        curr_hp: u16,
        ivs: [u8; 6],
        evs: [u8; 6],
        curr_stats: [u16; 6], // <-- Signature remains unchanged for test compatibility
        moves: [Option<MoveInstance>; 4],
        status: Option<StatusCondition>,
    ) -> Self {
        let mut pokemon = PokemonInst {
            name: species.name().to_string(),
            species,
            level,
            curr_exp,
            curr_hp: 0, // Will be set below with validation
            ivs,
            evs,
            stats: curr_stats.into(), // <-- Convert the array into our struct
            moves,
            status,
        };

        // Set HP using the validated setter.
        pokemon.set_hp(curr_hp);
        pokemon
    }

    /// Calculate current stats based on base stats, level, IVs, and EVs.
    /// Returns a `CurrentStats` struct.
    fn calculate_stats(
        base_stats: &BaseStats,
        level: u8,
        ivs: &[u8; 6],
        evs: &[u8; 6],
    ) -> CurrentStats {
        let level = level as u16;

        // Formula for HP
        let hp_base = 2 * base_stats.hp as u16 + ivs[0] as u16 + (evs[0] as u16 / 4);
        let hp = (hp_base * level) / 100 + level + 10;

        // Helper closure for the other 5 stats, which share a formula
        let calc_other_stat = |base: u8, iv: u8, ev: u8| -> u16 {
            let stat_base = 2 * base as u16 + iv as u16 + (ev as u16 / 4);
            (stat_base * level) / 100 + 5
        };

        CurrentStats {
            hp,
            attack: calc_other_stat(base_stats.attack, ivs[1], evs[1]),
            defense: calc_other_stat(base_stats.defense, ivs[2], evs[2]),
            sp_attack: calc_other_stat(base_stats.sp_attack, ivs[3], evs[3]),
            sp_defense: calc_other_stat(base_stats.sp_defense, ivs[4], evs[4]),
            speed: calc_other_stat(base_stats.speed, ivs[5], evs[5]),
        }
    }

    /// Derive moves from learnset based on current level.
    /// Returns the 4 most recent moves the Pokemon would know at this level.
    fn derive_moves_from_learnset(learnset: &Learnset, level: u8) -> Vec<Move> {
        let mut learned_moves = Vec::new();

        // Collect all moves learned at or before the current level
        for learn_level in 1..=level {
            if let Some(moves_at_level) = learnset.level_up.get(&learn_level) {
                learned_moves.extend(moves_at_level);
            }
        }

        // Take the last 4 moves learned.
        learned_moves.into_iter().rev().take(4).rev().collect()
    }

    /// Decrement PP for a known move.
    pub fn use_move(&mut self, move_to_use: Move) -> Result<(), UseMoveError> {
        for move_slot in self.moves.iter_mut() {
            if let Some(move_instance) = move_slot {
                if move_instance.move_ == move_to_use {
                    if move_instance.use_move() {
                        return Ok(());
                    } else {
                        return Err(UseMoveError::NoPPRemaining);
                    }
                }
            }
        }
        Err(UseMoveError::MoveNotKnown)
    }

    /// Get the species data for this Pokemon instance.
    pub fn get_species_data(&self) -> Option<PokemonSpecies> {
        get_species_data(self.species)
    }

    /// Get the current types, accounting for Transform and Conversion conditions.
    pub fn get_current_types(&self, player: &crate::player::BattlePlayer) -> Vec<PokemonType> {
        if let Some(p_cond) = player
            .active_pokemon_conditions
            .values()
            .find(|c| matches!(c, PokemonCondition::Converted { .. }))
        {
            if let PokemonCondition::Converted { pokemon_type } = p_cond {
                return vec![*pokemon_type];
            }
        }

        if let Some(p_cond) = player
            .active_pokemon_conditions
            .values()
            .find(|c| matches!(c, PokemonCondition::Transformed { .. }))
        {
            if let PokemonCondition::Transformed { target } = p_cond {
                if let Some(target_species_data) = get_species_data(target.species) {
                    return target_species_data.types.clone();
                }
            }
        }

        self.get_species_data()
            .map(|data| data.types)
            .unwrap_or_default()
    }

    /// Check if this Pokemon is fainted.
    pub fn is_fainted(&self) -> bool {
        self.curr_hp == 0 || matches!(self.status, Some(StatusCondition::Faint))
    }

    /// Get current HP.
    pub fn current_hp(&self) -> u16 {
        self.curr_hp
    }

    /// Get max HP from the calculated stats.
    pub fn max_hp(&self) -> u16 {
        self.stats.hp
    }

    /// Set current HP with validation (clamps to 0..=max_hp).
    pub fn set_hp(&mut self, hp: u16) {
        let max_hp = self.max_hp();
        self.curr_hp = hp.min(max_hp);
    }

    /// Set current HP to its maximum value.
    #[allow(dead_code)]
    pub fn set_hp_to_max(&mut self) {
        self.curr_hp = self.max_hp();
    }

    /// Restore HP to full and remove any status conditions.
    #[allow(dead_code)]
    pub fn restore_fully(&mut self) {
        self.set_hp_to_max();
        self.status = None;
    }

    /// Take damage and handle fainting.
    /// Returns true if the Pokemon fainted from this damage.
    pub fn take_damage(&mut self, damage: u16) -> bool {
        self.curr_hp = self.curr_hp.saturating_sub(damage);
        if self.curr_hp == 0 {
            self.status = Some(StatusCondition::Faint);
            true
        } else {
            false
        }
    }

    /// Heal HP (cannot exceed max HP or revive fainted Pokemon).
    pub fn heal(&mut self, heal_amount: u16) {
        if self.is_fainted() {
            return;
        }
        let max_hp = self.max_hp();
        self.curr_hp = (self.curr_hp + heal_amount).min(max_hp);
    }

    /// Revive a fainted Pokemon with a specified HP amount.
    #[allow(dead_code)]
    pub fn revive(&mut self, hp_amount: u16) {
        if self.is_fainted() {
            self.status = None; // Remove faint status
            let max_hp = self.max_hp();
            self.curr_hp = hp_amount.min(max_hp).max(1); // Revive with at least 1 HP
        }
    }

    /// Update status condition counters without dealing damage.
    /// Should be called at the start of turn when Pokemon tries to act.
    /// Returns (should_cure, status_changed).
    pub fn update_status_progress(&mut self) -> (bool, bool) {
        let original_status = self.status;

        let should_cure = match &mut self.status {
            Some(StatusCondition::Sleep(turns)) => {
                if *turns == 0 {
                    true // Wake up if already at 0
                } else {
                    *turns = turns.saturating_sub(1);
                    false // Don't wake up until next turn when it starts at 0
                }
            }
            Some(StatusCondition::Poison(severity)) => {
                // Only increment Toxic poison (severity > 0)
                if *severity > 0 {
                    *severity += 1;
                }
                false // Poison never cures itself
            }
            _ => false,
        };

        if should_cure {
            self.status = None;
        }

        (should_cure, self.status != original_status)
    }

    /// Calculate status damage without mutating state.
    /// Returns the amount of damage that would be dealt by status conditions.
    pub fn calculate_status_damage(&self) -> u16 {
        let max_hp = self.max_hp();
        let current_hp = self.current_hp();

        let theoretical_damage = match &self.status {
            Some(StatusCondition::Poison(severity)) => {
                if *severity == 0 {
                    (max_hp / 16).max(1) // Regular poison: 1/16 max HP
                } else {
                    (max_hp * (*severity as u16) / 16).max(1) // Toxic poison: severity/16 max HP
                }
            }
            Some(StatusCondition::Burn) => (max_hp / 8).max(1), // Burn: 1/8 max HP
            _ => 0,
        };
        
        // Cap damage to current HP to get actual damage that will be dealt
        theoretical_damage.min(current_hp)
    }
}
