use crate::battle::conditions::PokemonCondition;
use crate::errors::{SpeciesDataError, SpeciesDataResult};
use crate::species::Species;
use schema::{BaseStats, Learnset, Move, PokemonSpecies, PokemonType};
use serde::{Deserialize, Serialize};
use std::fmt;

// Include the compiled species data
use crate::move_data::{get_compiled_species_data, get_move_data, get_move_max_pp};

/// Get species data for a specific species from the compiled data
pub fn get_species_data(species: Species) -> SpeciesDataResult<PokemonSpecies> {
    // get_compiled_species_data() now returns &'static [Option<PokemonSpecies>]
    let compiled_data_slice = get_compiled_species_data();
    let index = species.pokedex_number() as usize - 1;

    // The idiomatic and safe way to access an element that might be out of bounds.
    compiled_data_slice
        .get(index) // This returns an `Option<&Option<PokemonSpecies>>`.
        .and_then(|option_species| option_species.as_ref()) // This unwraps the outer Option and converts `&Option<T>` to `Option<&T>`.
        .cloned() // This converts `Option<&PokemonSpecies>` to `Option<PokemonSpecies>` by cloning the data.
        .ok_or(SpeciesDataError::SpeciesNotFound(species)) // If we have `None` at any point, return our custom error.
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

impl fmt::Display for StatusCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let abbr = match self {
            StatusCondition::Burn => "Burned",
            StatusCondition::Poison(_) => "Poisoned",
            StatusCondition::Freeze => "Frozen",
            StatusCondition::Paralysis => "Paralyzed",
            StatusCondition::Sleep(_) => "Asleep",
            StatusCondition::Faint => "Fainted",
        };
        write!(f, "{}", abbr)
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub struct CurrentStats {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub sp_attack: u16,
    pub sp_defense: u16,
    pub speed: u16,
}
impl fmt::Display for CurrentStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use a consistent width for labels to ensure values align vertically.
        // "Sp. Defense" is the longest, so we'll use a width of 12.
        const LABEL_WIDTH: usize = 12;

        writeln!(f, "{:<LABEL_WIDTH$} : {}", "Max HP", self.hp)?;
        writeln!(f, "{:<LABEL_WIDTH$} : {}", "Attack", self.attack)?;
        writeln!(f, "{:<LABEL_WIDTH$} : {}", "Defense", self.defense)?;
        writeln!(f, "{:<LABEL_WIDTH$} : {}", "Sp. Atk", self.sp_attack)?;
        writeln!(f, "{:<LABEL_WIDTH$} : {}", "Sp. Def", self.sp_defense)?;
        // Use `write!` for the last line to avoid a trailing newline,
        // giving the caller more formatting control.
        write!(f, "{:<LABEL_WIDTH$} : {}", "Speed", self.speed)
    }
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

impl MoveInstance {
    /// Create a new move instance with max PP
    pub fn new(move_: Move) -> Self {
        let max_pp = get_move_max_pp(move_).unwrap_or(30); // fallback to 30 PP

        MoveInstance { move_, pp: max_pp }
    }

    /// Get the max PP for this move
    pub fn max_pp(&self) -> u8 {
        get_move_max_pp(self.move_).unwrap_or(30) // fallback to 30 PP
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

impl fmt::Display for MoveInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Fetch the move's static data using the `Move` enum variant.
        // This call might return None if a move is invalid, so we must handle it.
        if let Ok(move_data) = get_move_data(self.move_) {
            // The move data was found, so we can format it nicely.
            write!(
                f,
                "- {:<16} (PP: {}/{})", // Left-align name for clean formatting
                move_data.name,         // Use name from the fetched data
                self.pp,                // Use current PP from the instance
                self.max_pp()           // Use the helper method to get max PP
            )
        } else {
            // This is a fallback for safety, in case a Move variant
            // doesn't have corresponding data.
            write!(
                f,
                "- {:<16} (PP: {}/??)",
                format!("{:?}", self.move_), // Display the enum name as a fallback
                self.pp
            )
        }
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
        curr_hp: u16,
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
        // Add a guard clause for special moves that do not use PP and are not in the moveset.
        if matches!(move_to_use, Move::Struggle | Move::HittingItself) {
            return Ok(());
        }
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
    pub fn get_species_data(&self) -> SpeciesDataResult<PokemonSpecies> {
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
                if let Ok(target_species_data) = get_species_data(target.species) {
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

impl fmt::Display for PokemonInst {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // --- 1. Header Line: Name, Species, and Level ---
        let species_name = format!("{:?}", self.species);
        let name_display = if self.name != species_name {
            format!("{} ({})", self.name, species_name)
        } else {
            species_name
        };
        writeln!(f, "{} | Lvl. {}", name_display, self.level)?;

        // --- 2. HP and Status Line ---
        let hp_line = format!("HP: {}/{}", self.curr_hp, self.stats.hp);
        if let Some(status) = &self.status {
            writeln!(f, "{:<15} | Status: {}", hp_line, status)?;
        } else {
            writeln!(f, "{}", hp_line)?;
        }

        // --- The following sections are only shown in the default format ---
        if !f.alternate() {
            // Full Stats Section
            writeln!(f, "--------------------")?;
            writeln!(f, "{}", self.stats)?;

            // Moves Section
            let has_moves = self.moves.iter().any(|m| m.is_some());
            if has_moves {
                writeln!(f, "--------------------")?;
                writeln!(f, "Moves:")?;
                for move_instance in self.moves.iter().flatten() {
                    writeln!(f, "{}", move_instance)?;
                }
            }
        }

        Ok(())
    }
}
