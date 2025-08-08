
use crate::pokemon::{PokemonInst, Type};
use crate::moves::Move;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PlayerAction {
    // The index refers to the move's position (0-3) in the active Pokémon's move list.
    UseMove { move_index: usize },
    
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
    Confused,
    Seeded,
    Underground,
    InAir,
    Teleported,
    Enraged,
    Exhausted,
    Trapped { turns_remaining: u8 },
    Charging { pokemon_move: Move },
    Rampaging { turns_remaining: u8 },
    Transformed { target: PokemonInst },
    Converted { pokemon_type: Type },
    Disabled { pokemon_move: Move },
    Substitute { hp: u8 },
    Biding { turns_remaining: u8, damage: u16 },
    Countering { damage: u16 },
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
            last_move: None,
        }
    }
    
    /// Get the currently active Pokemon
    pub fn active_pokemon(&self) -> Option<&PokemonInst> {
        self.team.get(self.active_pokemon_index)
            .and_then(|slot| slot.as_ref())
    }
    
    /// Get the currently active Pokemon mutably
    pub fn active_pokemon_mut(&mut self) -> Option<&mut PokemonInst> {
        self.team.get_mut(self.active_pokemon_index)
            .and_then(|slot| slot.as_mut())
    }
    
    /// Check if the active Pokemon has a specific condition type
    pub fn has_condition(&self, condition: &PokemonCondition) -> bool {
        self.active_pokemon_conditions.contains_key(condition)
    }
    
    /// Add or update a condition on the active Pokemon
    pub fn add_condition(&mut self, condition: PokemonCondition) {
        self.active_pokemon_conditions.insert(condition.clone(), condition);
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
    pub fn get_condition_mut(&mut self, condition: &PokemonCondition) -> Option<&mut PokemonCondition> {
        self.active_pokemon_conditions.get_mut(condition)
    }
    
    /// Switch the active Pokemon
    pub fn switch_pokemon(&mut self, new_index: usize) -> Result<(), String> {
        if new_index >= 6 || self.team[new_index].is_none() {
            return Err("Invalid Pokemon index or empty slot".to_string());
        }
        
        // Clear active Pokemon conditions and stat stages when switching
        self.active_pokemon_conditions.clear();
        self.stat_stages.clear();
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
    
    /// Get all current stat stages (for debugging/display)
    pub fn get_all_stat_stages(&self) -> &HashMap<StatType, i8> {
        &self.stat_stages
    }
}