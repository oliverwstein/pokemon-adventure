use crate::battle::conditions::{PokemonCondition, PokemonConditionType};
use crate::moves::Move;
use crate::pokemon::{PokemonInst};
use serde::{Deserialize, Serialize};
use core::fmt;
use std::collections::HashMap;
use std::hash::{Hash};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PlayerAction {
    // The index refers to the move's position (0-3) in the active Pokémon's move list.
    UseMove { move_index: usize },

    // The index refers to the Pokémon's position (0-5) in the player's team.
    SwitchPokemon { team_index: usize },

    Forfeit,
}
impl fmt::Display for PlayerAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // For UseMove, we de-structure to get the move_index.
            PlayerAction::UseMove { move_index } => {
                write!(f, "Use Move (index: {})", move_index)
            }
            // Same for SwitchPokemon and team_index.
            PlayerAction::SwitchPokemon { team_index } => {
                write!(f, "Switch Pokémon (index: {})", team_index)
            }
            // Forfeit is a simple, static string.
            PlayerAction::Forfeit => {
                write!(f, "Forfeit")
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum TeamCondition {
    Reflect,
    LightScreen,
    Mist,
}

impl fmt::Display for TeamCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // We match on `self` to get the specific variant and write its
        // human-readable name to the formatter.
        let display_name = match self {
            TeamCondition::Reflect => "Reflect",
            TeamCondition::LightScreen => "Light Screen", // Use a space for better readability
            TeamCondition::Mist => "Mist",
        };
        
        // The write! macro handles writing the string to the output.
        write!(f, "{}", display_name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatType {
    Attack,
    Defense,
    Speed,
    SpecialAttack,
    SpecialDefense,
    Accuracy,
    Evasion,
    Focus,
}

impl From<crate::move_data::StatType> for StatType {
    fn from(stat: crate::move_data::StatType) -> Self {
        use crate::move_data::StatType as MoveDataStat;

        match stat {
            MoveDataStat::Atk => Self::Attack,
            MoveDataStat::Def => Self::Defense,
            MoveDataStat::SpAtk => Self::SpecialAttack,
            MoveDataStat::SpDef => Self::SpecialDefense,
            MoveDataStat::Spe => Self::Speed,
            MoveDataStat::Acc => Self::Accuracy,
            MoveDataStat::Eva => Self::Evasion,
            MoveDataStat::Crit => Self::Focus,
            // The `Hp` variant in move_data::StatType is not used for stat stages,
            // so we can ignore it here. The compiler will warn us if we miss any.
            MoveDataStat::Hp => {
                // This case should ideally not be hit in stat stage logic.
                // We'll default to Attack and maybe log a warning in a real app.
                Self::Attack
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerType {
    Human,
    NPC,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BattlePlayer {
    // A unique identifier. For a human, this could be their UserID.
    // For an NPC, this could be "AI_YoungsterJoey".
    pub player_id: String,
    pub player_name: String,
    pub player_type: PlayerType,
    // The player's full team of up to 6 Pokémon instances.
    pub team: [Option<PokemonInst>; 6],

    // The index (0-5) of the Pokémon in the `team` vector that is currently active.
    pub active_pokemon_index: usize,

    // HashMap for O(1) team condition lookup/update, value is turns_remaining
    pub team_conditions: HashMap<TeamCondition, u8>,

    // HashMap for O(1) condition lookup/update, prevents duplicates
    pub active_pokemon_conditions: HashMap<PokemonConditionType, PokemonCondition>,

    // HashMap for stat stage modifications, value is stage (-6 to +6)
    pub stat_stages: HashMap<StatType, i8>,

    // Money/prize amount accumulated during battle (altered by Pay Day)
    pub ante: u32,

    pub last_move: Option<Move>,
}

impl BattlePlayer {
    /// Create a new BattlePlayer
    pub fn new(player_id: String, player_name: String, team: Vec<PokemonInst>) -> Self {
        // Call the new, more explicit constructor with the default value.
        Self::new_with_player_type(player_id, player_name, team, PlayerType::NPC)
    }
    pub fn new_with_player_type(
        player_id: String,
        player_name: String,
        team: Vec<PokemonInst>,
        player_type: PlayerType, // <-- The new parameter
    ) -> Self {
        let mut team_array = [const { None }; 6];
        for (i, pokemon) in team.into_iter().take(6).enumerate() {
            team_array[i] = Some(pokemon);
        }

        BattlePlayer {
            player_id,
            player_name,
            player_type, // <-- Use the provided player type
            team: team_array,
            active_pokemon_index: 0,
            team_conditions: HashMap::new(),
            active_pokemon_conditions: HashMap::new(),
            stat_stages: HashMap::new(),
            ante: 0,
            last_move: None,
        }
    }
    /// Get the currently active Pokemon
    pub fn active_pokemon(&self) -> Option<&PokemonInst> {
        self.team
            .get(self.active_pokemon_index)
            .and_then(|slot| slot.as_ref())
    }

    /// Check if a player has any non-fainted Pokemon in their team
    pub fn can_still_battle(&self) -> bool {
        self.team.iter().any(|pokemon_opt| {
            pokemon_opt.as_ref().map_or(false, |pokemon| !pokemon.is_fainted())
        })
    }

    #[allow(dead_code)]
    pub fn validate_action(&self, action: &PlayerAction) -> Result<(), String> {
        match action {
            PlayerAction::UseMove { move_index } => {
                let pokemon = self.active_pokemon().ok_or("No active Pokemon to use a move.")?;

                if *move_index >= pokemon.moves.len() {
                    return Err("Invalid move index.".to_string());
                }

                if let Some(move_instance) = &pokemon.moves[*move_index] {
                    // It's valid to select a move with 0 PP; the engine will convert it to Struggle.
                    // We only need to check for explicitly disabled moves.
                    if self.active_pokemon_conditions.values().any(|cond| {
                        matches!(cond, PokemonCondition::Disabled { pokemon_move, .. } if *pokemon_move == move_instance.move_)
                    }) {
                        return Err("This move is currently disabled.".to_string());
                    }
                } else {
                    return Err("There is no move in that slot.".to_string());
                }
            }
            PlayerAction::SwitchPokemon { team_index } => {
                if self.has_condition_type(PokemonConditionType::Trapped) {
                    return Err("The Pokémon is trapped and cannot switch out!".to_string());
                }

                if *team_index >= self.team.len() {
                    return Err("Invalid team index for switching.".to_string());
                }
                
                if let Some(target_pokemon) = &self.team[*team_index] {
                    if target_pokemon.is_fainted() {
                        return Err("Cannot switch to a fainted Pokémon.".to_string());
                    }
                    if *team_index == self.active_pokemon_index {
                        return Err("This Pokémon is already in battle.".to_string());
                    }
                } else {
                    return Err("No Pokémon in that team slot.".to_string());
                }
            }
            PlayerAction::Forfeit => {
                // Forfeiting is always a valid action.
            }
        }

        Ok(())
    }

    /// This checks for conditions like being fainted, exhausted, or having moves
    /// that are disabled or out of PP. It will return a `Struggle` action if no
    /// other moves are available.
    pub fn get_valid_moves(&self) -> Vec<PlayerAction> {
        let mut moves = Vec::new();
        if let Some(active_pokemon) = self.active_pokemon() {
            // Check if the Pokémon can even attempt to use a move.
            let can_use_moves = !self.has_condition_type(PokemonConditionType::Exhausted) 
                                && !active_pokemon.is_fainted();

            if can_use_moves {
                let usable_moves: Vec<_> = active_pokemon.moves.iter().enumerate()
                    .filter_map(|(i, slot)| {
                        slot.as_ref().and_then(|inst| {
                            let is_disabled = self.active_pokemon_conditions.values().any(|cond| {
                                matches!(cond, PokemonCondition::Disabled { pokemon_move, .. } if *pokemon_move == inst.move_)
                            });
                            // We allow selecting a move with 0 PP; the engine will turn it into Struggle.
                            if !is_disabled { Some(PlayerAction::UseMove { move_index: i }) } else { None }
                        })
                    })
                    .collect();

                if !usable_moves.is_empty() {
                    moves.extend(usable_moves);
                } else {
                    // If no moves are usable (all disabled), the only option is Struggle.
                    moves.push(PlayerAction::UseMove { move_index: 0 });
                }
            }
        }
        moves
    }

    /// Generates a list of valid Pokémon to switch to from the team.
    /// This checks for conditions like `Trapped` and ensures that switch targets
    /// are not fainted or already active.
    pub fn get_valid_switches(&self) -> Vec<PlayerAction> {
        let mut switches = Vec::new();
        
        // If trapped, no switches are possible.
        if self.has_condition_type(PokemonConditionType::Trapped) {
            return switches;
        }

        for (i, pokemon_slot) in self.team.iter().enumerate() {
            if let Some(pokemon) = pokemon_slot {
                if i != self.active_pokemon_index && !pokemon.is_fainted() {
                    switches.push(PlayerAction::SwitchPokemon { team_index: i });
                }
            }
        }
        switches
    }

    pub fn forced_move(&self) -> Option<Move> {
        // Check for Biding condition, which forces the Bide move.
        if self.active_pokemon_conditions.values().any(|c| matches!(c, PokemonCondition::Biding { .. })) {
            return Some(Move::Bide);
        }

        // Check for conditions that force a repeat of the last move.
        if let Some(last_move) = self.last_move {
            let is_locked_into_repeating = self.active_pokemon_conditions.values().any(|c| {
                matches!(c, PokemonCondition::Charging | PokemonCondition::InAir | PokemonCondition::Underground | PokemonCondition::Rampaging { .. })
            });

            if is_locked_into_repeating {
                return Some(last_move);
            }
        }

        None
    }

    /// Get the currently active Pokemon mutably
    #[cfg(test)]
    pub fn active_pokemon_mut(&mut self) -> Option<&mut PokemonInst> {
        self.team
            .get_mut(self.active_pokemon_index)
            .and_then(|slot| slot.as_mut())
    }

    /// Check if the active Pokemon has a condition of the specified type
    pub fn has_condition_type(&self, condition_type: PokemonConditionType) -> bool {
        self.active_pokemon_conditions.contains_key(&condition_type)
    }
    
    /// Check if the active Pokemon has this exact condition (type AND data must match)
    pub fn has_condition(&self, condition: &PokemonCondition) -> bool {
        if let Some(existing_condition) = self.active_pokemon_conditions.get(&condition.get_type()) {
            existing_condition == condition
        } else {
            false
        }
    }

    /// Add or update a condition on the active Pokemon
    pub fn add_condition(&mut self, condition: PokemonCondition) {
        self.active_pokemon_conditions
            .insert(condition.get_type(), condition);
    }

    /// Get a condition for reading
    #[cfg(test)]
    pub fn get_condition(&self, condition: &PokemonCondition) -> Option<&PokemonCondition> {
        self.active_pokemon_conditions.get(&condition.get_type())
    }

    /// Check if the team has a specific condition
    pub fn has_team_condition(&self, condition: &TeamCondition) -> bool {
        self.team_conditions.contains_key(condition)
    }

    /// Add or update a team condition with turns remaining
    pub fn add_team_condition(&mut self, condition: TeamCondition, turns_remaining: u8) {
        self.team_conditions.insert(condition, turns_remaining);
    }

    /// Remove a team condition
    pub fn remove_team_condition(&mut self, condition: &TeamCondition) -> Option<u8> {
        self.team_conditions.remove(condition)
    }

    /// Get turns remaining for a team condition
    #[cfg(test)]
    pub fn get_team_condition_turns(&self, condition: &TeamCondition) -> Option<u8> {
        self.team_conditions.get(condition).copied()
    }

    /// Decrement all team condition turns and remove expired ones
    pub fn tick_team_conditions(&mut self) -> Vec<TeamCondition> {
        let mut expired = Vec::new();
        self.team_conditions.retain(|condition, turns| {
            *turns = turns.saturating_sub(1);
            if *turns == 0 {
                expired.push(*condition);
                false // Remove from map
            } else {
                true // Keep in map
            }
        });
        expired
    }

    // === Stat Stage Management ===

    /// Get the current stage for a stat type (0 if not set)
    pub fn get_stat_stage(&self, stat: StatType) -> i8 {
        self.stat_stages.get(&stat).copied().unwrap_or(0)
    }

    /// Set the stage for a stat type (clamped to -6 to +6)
    pub fn set_stat_stage(&mut self, stat: StatType, stage: i8) {
        let clamped_stage = stage.clamp(-6, 6);
        if clamped_stage == 0 {
            self.stat_stages.remove(&stat);
        } else {
            self.stat_stages.insert(stat, clamped_stage);
        }
    }

    pub fn clear_active_pokemon_state(&mut self) {
        self.active_pokemon_conditions.clear();
        self.stat_stages.clear();
        self.last_move = None;
    }

    /// Update active Pokemon condition timers and return which conditions should be removed
    /// Returns a vector of conditions that expired and should be removed
    pub fn tick_active_conditions(&mut self) -> Vec<PokemonCondition> {
        let mut expired_conditions = Vec::new();
        let mut updated_conditions = Vec::new();

        // Process each condition and check for expiration/updates
        for (key, condition) in self.active_pokemon_conditions.iter() {
            match condition {
                // Conditions that expire after one turn (cleared at end-of-turn)
                PokemonCondition::Flinched
                | PokemonCondition::Teleported
                | PokemonCondition::Countering { .. } => {
                    expired_conditions.push(condition.clone());
                }

                // Multi-turn conditions with countdown timers
                PokemonCondition::Confused { turns_remaining } => {
                    if *turns_remaining < 1 {
                        expired_conditions.push(condition.clone());
                    } else {
                        updated_conditions.push((
                            key.clone(),
                            PokemonCondition::Confused {
                                turns_remaining: turns_remaining - 1,
                            },
                        ));
                    }
                }

                PokemonCondition::Exhausted { turns_remaining } => {
                    if *turns_remaining < 1 {
                        expired_conditions.push(condition.clone());
                    } else {
                        updated_conditions.push((
                            key.clone(),
                            PokemonCondition::Exhausted {
                                turns_remaining: turns_remaining - 1,
                            },
                        ));
                    }
                }

                PokemonCondition::Trapped { turns_remaining } => {
                    if *turns_remaining < 1 {
                        expired_conditions.push(condition.clone());
                    } else {
                        updated_conditions.push((
                            key.clone(),
                            PokemonCondition::Trapped {
                                turns_remaining: turns_remaining - 1,
                            },
                        ));
                    }
                }

                PokemonCondition::Disabled {
                    pokemon_move,
                    turns_remaining,
                } => {
                    if *turns_remaining < 1 {
                        expired_conditions.push(condition.clone());
                    } else {
                        updated_conditions.push((
                            key.clone(),
                            PokemonCondition::Disabled {
                                pokemon_move: *pokemon_move,
                                turns_remaining: turns_remaining - 1,
                            },
                        ));
                    }
                }

                PokemonCondition::Rampaging { turns_remaining } => {
                    if *turns_remaining < 1 {
                        expired_conditions.push(condition.clone());
                    } else {
                        updated_conditions.push((
                            key.clone(),
                            PokemonCondition::Rampaging {
                                turns_remaining: turns_remaining - 1,
                            },
                        ));
                    }
                }

                PokemonCondition::Biding {
                    turns_remaining,
                    damage,
                } => {
                    if *turns_remaining < 1 {
                        expired_conditions.push(condition.clone());
                    } else {
                        updated_conditions.push((
                            key.clone(),
                            PokemonCondition::Biding {
                                turns_remaining: turns_remaining - 1,
                                damage: *damage,
                            },
                        ));
                    }
                }

                // Conditions that persist until explicitly removed
                PokemonCondition::Seeded
                | PokemonCondition::Underground
                | PokemonCondition::InAir
                | PokemonCondition::Enraged
                | PokemonCondition::Charging
                | PokemonCondition::Transformed { .. }
                | PokemonCondition::Converted { .. }
                | PokemonCondition::Substitute { .. } => {
                    // These don't have automatic expiration timers
                    // They're removed by specific game events
                }
            }
        }

        // Remove expired conditions
        for condition in &expired_conditions {
            self.active_pokemon_conditions.remove(&condition.get_type());
        }

        // Update conditions with decremented counters
        for (old_key, updated_condition) in updated_conditions {
            self.active_pokemon_conditions.remove(&old_key);
            self.active_pokemon_conditions
                .insert(updated_condition.get_type(), updated_condition);
        }

        expired_conditions
    }

    /// Get current ante amount
    pub fn get_ante(&self) -> u32 {
        self.ante
    }

    /// Add to ante amount
    pub fn add_ante(&mut self, amount: u32) {
        self.ante = self.ante.saturating_add(amount);
    }
}
