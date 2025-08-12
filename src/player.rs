use crate::moves::Move;
use crate::pokemon::{PokemonInst, PokemonType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PlayerAction {
    // The index refers to the move's position (0-3) in the active Pokémon's move list.
    UseMove { move_index: usize },

    // A move that is forced by conditions (charging, rampage, etc.) - bypasses normal move selection
    ForcedMove { pokemon_move: Move },

    // The index refers to the Pokémon's position (0-5) in the player's team.
    SwitchPokemon { team_index: usize },

    Forfeit,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TeamCondition {
    Reflect,
    LightScreen,
    Mist,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BattlePlayer {
    // A unique identifier. For a human, this could be their UserID.
    // For an NPC, this could be "AI_YoungsterJoey".
    pub player_id: String,
    pub player_name: String,

    // The player's full team of up to 6 Pokémon instances.
    pub team: [Option<PokemonInst>; 6],

    // The index (0-5) of the Pokémon in the `team` vector that is currently active.
    pub active_pokemon_index: usize,

    // HashMap for O(1) team condition lookup/update, value is turns_remaining
    pub team_conditions: HashMap<TeamCondition, u8>,

    // HashMap for O(1) condition lookup/update, prevents duplicates
    pub active_pokemon_conditions: HashMap<PokemonCondition, PokemonCondition>,

    // HashMap for stat stage modifications, value is stage (-6 to +6)
    pub stat_stages: HashMap<StatType, i8>,

    // Money/prize amount accumulated during battle (altered by Pay Day)
    pub ante: u32,

    pub last_move: Option<Move>,
}

impl BattlePlayer {
    /// Create a new BattlePlayer
    pub fn new(player_id: String, player_name: String, team: Vec<PokemonInst>) -> Self {
        let mut team_array = [const { None }; 6];
        for (i, pokemon) in team.into_iter().take(6).enumerate() {
            team_array[i] = Some(pokemon);
        }

        BattlePlayer {
            player_id,
            player_name,
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

    /// Get the currently active Pokemon mutably
    pub fn active_pokemon_mut(&mut self) -> Option<&mut PokemonInst> {
        self.team
            .get_mut(self.active_pokemon_index)
            .and_then(|slot| slot.as_mut())
    }

    /// Check if the active Pokemon has a specific condition type
    pub fn has_condition(&self, condition: &PokemonCondition) -> bool {
        self.active_pokemon_conditions.contains_key(condition)
    }

    /// Add or update a condition on the active Pokemon
    pub fn add_condition(&mut self, condition: PokemonCondition) {
        self.active_pokemon_conditions
            .insert(condition.clone(), condition);
    }

    /// Remove a condition from the active Pokemon
    pub fn remove_condition(&mut self, condition: &PokemonCondition) -> Option<PokemonCondition> {
        self.active_pokemon_conditions.remove(condition)
    }

    /// Get a condition for reading
    pub fn get_condition(&self, condition: &PokemonCondition) -> Option<&PokemonCondition> {
        self.active_pokemon_conditions.get(condition)
    }

    /// Get a condition for modification
    pub fn get_condition_mut(
        &mut self,
        condition: &PokemonCondition,
    ) -> Option<&mut PokemonCondition> {
        self.active_pokemon_conditions.get_mut(condition)
    }

    /// Switch the active Pokemon
    pub fn switch_pokemon(&mut self, new_index: usize) -> Result<(), String> {
        if new_index >= 6 || self.team[new_index].is_none() {
            return Err("Invalid Pokemon index or empty slot".to_string());
        }

        // Clear active Pokemon conditions, stat stages, and last move when switching
        self.clear_active_pokemon_state();

        self.active_pokemon_index = new_index;

        Ok(())
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
    pub fn get_team_condition_turns(&self, condition: &TeamCondition) -> Option<u8> {
        self.team_conditions.get(condition).copied()
    }

    /// Decrement all team condition turns and remove expired ones
    pub fn tick_team_conditions(&mut self) {
        self.team_conditions.retain(|_, turns| {
            *turns = turns.saturating_sub(1);
            *turns > 0
        });
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

    /// Modify the stage for a stat type by a delta (clamped to -6 to +6)
    pub fn modify_stat_stage(&mut self, stat: StatType, delta: i8) {
        let current = self.get_stat_stage(stat);
        self.set_stat_stage(stat, current + delta);
    }

    /// Check if a stat has any stage modification
    pub fn has_stat_stage(&self, stat: StatType) -> bool {
        self.stat_stages.contains_key(&stat)
    }

    /// Remove all stat stage modifications
    pub fn clear_stat_stages(&mut self) {
        self.stat_stages.clear();
    }
    pub fn clear_active_pokemon_state(&mut self) {
        self.active_pokemon_conditions.clear();
        self.stat_stages.clear();
        self.last_move = None;
    }
    /// Get all current stat stages (for debugging/display)
    pub fn get_all_stat_stages(&self) -> &HashMap<StatType, i8> {
        &self.stat_stages
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
                    expired_conditions.push(key.clone());
                }

                // Multi-turn conditions with countdown timers
                PokemonCondition::Confused { turns_remaining } => {
                    if *turns_remaining <= 1 {
                        expired_conditions.push(key.clone());
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
                    if *turns_remaining <= 1 {
                        expired_conditions.push(key.clone());
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
                    if *turns_remaining <= 1 {
                        expired_conditions.push(key.clone());
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
                    if *turns_remaining <= 1 {
                        expired_conditions.push(key.clone());
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
                    if *turns_remaining <= 1 {
                        expired_conditions.push(key.clone());
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
                    if *turns_remaining <= 1 {
                        expired_conditions.push(key.clone());
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
            self.active_pokemon_conditions.remove(condition);
        }

        // Update conditions with decremented counters
        for (old_key, updated_condition) in updated_conditions {
            self.active_pokemon_conditions.remove(&old_key);
            self.active_pokemon_conditions
                .insert(updated_condition.clone(), updated_condition);
        }

        expired_conditions
    }

    // === Ante Management ===

    /// Get current ante amount
    pub fn get_ante(&self) -> u32 {
        self.ante
    }

    /// Add to ante amount
    pub fn add_ante(&mut self, amount: u32) {
        self.ante = self.ante.saturating_add(amount);
    }
}
