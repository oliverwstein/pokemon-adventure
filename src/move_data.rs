use crate::moves::Move;
use crate::pokemon::PokemonType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{LazyLock, RwLock};

// Global move data storage - loaded once at startup
static MOVE_DATA: LazyLock<RwLock<HashMap<Move, MoveData>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Initialize the global move data by loading from disk
pub fn initialize_move_data(data_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let move_data_map = MoveData::load_all(data_path)?;
    let mut global_data = MOVE_DATA.write().unwrap();
    *global_data = move_data_map;
    Ok(())
}

/// Get move data for a specific move from the global store
pub fn get_move_data(move_: Move) -> Option<MoveData> {
    let global_data = MOVE_DATA.read().unwrap();
    global_data.get(&move_).cloned()
}

/// Get max PP for a specific move
pub fn get_move_max_pp(move_: Move) -> u8 {
    get_move_data(move_).map(|data| data.max_pp).unwrap_or(30) // Default fallback
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveCategory {
    Physical,
    Special,
    Other,
    Status,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatType {
    Hp,
    Atk,
    Def,
    SpAtk,
    SpDef,
    Spe,
    Acc,
    Eva,
    Crit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    User,
    Target,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReflectType {
    Physical,
    Special,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RampageEndCondition {
    Confuse,
    Exhaust,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusType {
    Sleep,
    Poison,
    Burn,
    Freeze,
    Paralysis,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MoveEffect {
    // Basic effects
    Flinch(u8),   // chance %
    Burn(u8),     // chance %
    Freeze(u8),   // chance %
    Paralyze(u8), // chance %
    Poison(u8),   // chance %
    Sedate(u8),   // chance % (sleep)
    Confuse(u8),  // chance %

    // Stat changes
    StatChange(Target, StatType, i8, u8), // target, stat, stages, chance %
    RaiseAllStats(u8),                    // chance %

    // Damage modifiers
    Recoil(u8),     // % of damage dealt
    Drain(u8),      // % of damage healed
    Crit(u8),       // increased crit ratio
    IgnoreDef(u8),  // chance % to ignore defense
    SuperFang(u8),  // chance % to halve HP
    SetDamage(u16), // fixed damage
    LevelDamage,    // damage = user level

    // Multi-hit
    MultiHit(u8, u8), // min hits, % chance of continuation

    // Status and conditions
    Trap(u8),     // chance % to trap
    Exhaust(u8),  // chance % to exhaust (skip next turn)
    Priority(i8), // move priority modifier
    ChargeUp,     // charge for 1 turn
    InAir,        // go in air (avoid ground moves)
    Underground,  // go underground
    Teleport(u8), // chance % to teleport away

    // Special mechanics
    OHKO,                         // one-hit KO
    Explode,                      // user faints
    Reckless(u8),                 // recoil if miss, chance %
    Transform,                    // copy target's appearance/stats
    Conversion,                   // change user's type
    Disable(u8),                  // disable target's last move, chance %
    Counter,                      // return double physical damage
    MirrorMove,                   // copy target's last move
    Metronome,                    // random move
    Substitute,                   // create substitute with 25% HP
    Rest(u8),                     // sleep for X turns, full heal
    Bide(u8),                     // store damage for X turns
    Rage(u8),                     // chance % to enter rage mode
    Rampage(RampageEndCondition), // rampage with end condition

    // Field effects
    Haze(u8),             // remove all stat changes, chance %
    Reflect(ReflectType), // reduce physical/special damage
    Mist,                 // prevent stat reduction
    Seed(u8),             // leech seed effect, chance %
    Nightmare,            // only works on sleeping targets

    // Utility
    Heal(u8),                       // heal % of max HP
    CureStatus(Target, StatusType), // cure specific status
    Ante(u8), // percent chance to gain money equal to 2x level (Pay Day effect)
}

/// Context information needed for move effect calculations
#[derive(Debug, Clone)]
pub struct EffectContext {
    pub attacker_index: usize,
    pub defender_index: usize,
    pub move_used: crate::moves::Move,
}

impl EffectContext {
    pub fn new(attacker_index: usize, defender_index: usize, move_used: crate::moves::Move) -> Self {
        Self {
            attacker_index,
            defender_index,
            move_used,
        }
    }
    
    pub fn target_index(&self, target: &Target) -> usize {
        match target {
            Target::User => self.attacker_index,
            Target::Target => self.defender_index,
        }
    }
}

impl MoveEffect {
    /// Apply this effect to the battle state, returning commands to execute
    pub fn apply(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        
        match self {
            MoveEffect::Burn(chance) => {
                self.apply_burn_effect(*chance, context, state, rng)
            }
            MoveEffect::Paralyze(chance) => {
                self.apply_paralyze_effect(*chance, context, state, rng)
            }
            MoveEffect::Freeze(chance) => {
                self.apply_freeze_effect(*chance, context, state, rng)
            }
            MoveEffect::Poison(chance) => {
                self.apply_poison_effect(*chance, context, state, rng)
            }
            MoveEffect::Sedate(chance) => {
                self.apply_sedate_effect(*chance, context, state, rng)
            }
            MoveEffect::Flinch(chance) => {
                self.apply_flinch_effect(*chance, context, state, rng)
            }
            MoveEffect::Confuse(chance) => {
                self.apply_confuse_effect(*chance, context, state, rng)
            }
            MoveEffect::Trap(chance) => {
                self.apply_trap_effect(*chance, context, state, rng)
            }
            MoveEffect::Exhaust(chance) => {
                self.apply_exhaust_effect(*chance, context, state, rng)
            }
            MoveEffect::StatChange(target, stat, stages, chance) => {
                self.apply_stat_change_effect(target, stat, *stages, *chance, context, state, rng)
            }
            MoveEffect::RaiseAllStats(chance) => {
                self.apply_raise_all_stats_effect(*chance, context, state, rng)
            }
            MoveEffect::Heal(percentage) => {
                self.apply_heal_effect(*percentage, context, state)
            }
            MoveEffect::Haze(chance) => {
                self.apply_haze_effect(*chance, context, state, rng)
            }
            MoveEffect::CureStatus(target, status_type) => {
                self.apply_cure_status_effect(target, status_type, context, state)
            }
            MoveEffect::Reflect(reflect_type) => {
                self.apply_reflect_effect(reflect_type, context, state)
            }
            MoveEffect::Recoil(_) | MoveEffect::Drain(_) => {
                // Damage-based effects are handled separately in apply_damage_based_effects
                Vec::new()
            }
            MoveEffect::Reckless(_) => {
                // Miss-based effects are handled separately in apply_miss_based_effects
                Vec::new()
            }
            _ => {
                // For effects not yet migrated, return empty command list
                Vec::new()
            }
        }
    }
    
    /// Apply burn status effect
    fn apply_burn_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        if rng.next_outcome() <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(target_pokemon) = target_player.active_pokemon() {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: Some(crate::pokemon::StatusCondition::Burn),
                    });
                    commands.push(BattleCommand::EmitEvent(BattleEvent::PokemonStatusApplied {
                        target: target_pokemon.species,
                        status: crate::pokemon::StatusCondition::Burn,
                    }));
                }
            }
        }
        
        commands
    }
    
    /// Apply paralyze status effect  
    fn apply_paralyze_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        if rng.next_outcome() <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(target_pokemon) = target_player.active_pokemon() {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: Some(crate::pokemon::StatusCondition::Paralysis),
                    });
                    commands.push(BattleCommand::EmitEvent(BattleEvent::PokemonStatusApplied {
                        target: target_pokemon.species,
                        status: crate::pokemon::StatusCondition::Paralysis,
                    }));
                }
            }
        }
        
        commands
    }
    
    /// Apply freeze status effect
    fn apply_freeze_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        if rng.next_outcome() <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(target_pokemon) = target_player.active_pokemon() {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: Some(crate::pokemon::StatusCondition::Freeze),
                    });
                    commands.push(BattleCommand::EmitEvent(BattleEvent::PokemonStatusApplied {
                        target: target_pokemon.species,
                        status: crate::pokemon::StatusCondition::Freeze,
                    }));
                }
            }
        }
        
        commands
    }
    
    /// Apply poison status effect
    fn apply_poison_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        if rng.next_outcome() <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(target_pokemon) = target_player.active_pokemon() {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: Some(crate::pokemon::StatusCondition::Poison(0)),
                    });
                    commands.push(BattleCommand::EmitEvent(BattleEvent::PokemonStatusApplied {
                        target: target_pokemon.species,
                        status: crate::pokemon::StatusCondition::Poison(0),
                    }));
                }
            }
        }
        
        commands
    }
    
    /// Apply sedate (sleep) status effect
    fn apply_sedate_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        if rng.next_outcome() <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(target_pokemon) = target_player.active_pokemon() {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    // Sleep for 1-3 turns (random)
                    let sleep_turns = (rng.next_outcome() % 3) + 1; // 1, 2, or 3 turns
                    let sleep_status = crate::pokemon::StatusCondition::Sleep(sleep_turns);
                    
                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: Some(sleep_status),
                    });
                    commands.push(BattleCommand::EmitEvent(BattleEvent::PokemonStatusApplied {
                        target: target_pokemon.species,
                        status: sleep_status,
                    }));
                }
            }
        }
        
        commands
    }
    
    /// Apply flinch condition effect
    fn apply_flinch_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        if rng.next_outcome() <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(target_pokemon) = target_player.active_pokemon() {
                let condition = crate::player::PokemonCondition::Flinched;
                
                commands.push(BattleCommand::AddCondition {
                    target: PlayerTarget::from_index(context.defender_index),
                    condition: condition.clone(),
                });
                commands.push(BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                    target: target_pokemon.species,
                    status: condition,
                }));
            }
        }
        
        commands
    }
    
    /// Apply confuse condition effect
    fn apply_confuse_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        if rng.next_outcome() <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(target_pokemon) = target_player.active_pokemon() {
                // Confuse for 1-4 turns (random)
                let confuse_turns = (rng.next_outcome() % 4) + 1;
                let condition = crate::player::PokemonCondition::Confused {
                    turns_remaining: confuse_turns,
                };
                
                commands.push(BattleCommand::AddCondition {
                    target: PlayerTarget::from_index(context.defender_index),
                    condition: condition.clone(),
                });
                commands.push(BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                    target: target_pokemon.species,
                    status: condition,
                }));
            }
        }
        
        commands
    }
    
    /// Apply trap condition effect
    fn apply_trap_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        if rng.next_outcome() <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(target_pokemon) = target_player.active_pokemon() {
                // Trap for 2-5 turns (random)
                let trap_turns = (rng.next_outcome() % 4) + 2;
                let condition = crate::player::PokemonCondition::Trapped {
                    turns_remaining: trap_turns,
                };
                
                commands.push(BattleCommand::AddCondition {
                    target: PlayerTarget::from_index(context.defender_index),
                    condition: condition.clone(),
                });
                commands.push(BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                    target: target_pokemon.species,
                    status: condition,
                }));
            }
        }
        
        commands
    }
    
    /// Apply exhaust condition effect (targets user, not opponent)
    fn apply_exhaust_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        if rng.next_outcome() <= chance {
            let attacker_player = &state.players[context.attacker_index];
            if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
                let condition = crate::player::PokemonCondition::Exhausted {
                    turns_remaining: 2, // Decremented same turn, so start at 2
                };
                
                commands.push(BattleCommand::AddCondition {
                    target: PlayerTarget::from_index(context.attacker_index),
                    condition: condition.clone(),
                });
                commands.push(BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                    target: attacker_pokemon.species,
                    status: condition,
                }));
            }
        }
        
        commands
    }
    
    /// Apply stat change effect
    fn apply_stat_change_effect(
        &self,
        target: &Target,
        stat: &StatType,
        stages: i8,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        if rng.next_outcome() <= chance {
            let target_index = context.target_index(target);
            let target_player = &state.players[target_index];
            
            if let Some(target_pokemon) = target_player.active_pokemon() {
                let player_stat = match stat {
                    StatType::Atk => crate::player::StatType::Attack,
                    StatType::Def => crate::player::StatType::Defense,
                    StatType::SpAtk => crate::player::StatType::SpecialAttack,
                    StatType::SpDef => crate::player::StatType::SpecialDefense,
                    StatType::Spe => crate::player::StatType::Speed,
                    StatType::Acc => crate::player::StatType::Accuracy,
                    StatType::Eva => crate::player::StatType::Evasion,
                    StatType::Crit => crate::player::StatType::Focus,
                    _ => return commands, // Skip unsupported stats
                };
                
                // Check if Mist prevents this stat change
                let is_enemy_move = target_index != context.attacker_index;
                let is_negative_change = stages < 0;
                let has_mist = target_player.has_team_condition(&crate::player::TeamCondition::Mist);
                
                if is_enemy_move && is_negative_change && has_mist {
                    // Mist prevents the stat change
                    commands.push(BattleCommand::EmitEvent(BattleEvent::StatChangeBlocked {
                        target: target_pokemon.species,
                        stat: player_stat,
                        reason: "Mist prevented stat reduction".to_string(),
                    }));
                } else {
                    let old_stage = target_player.get_stat_stage(player_stat);
                    let new_stage = (old_stage + stages).clamp(-6, 6);
                    
                    if old_stage != new_stage {
                        commands.push(BattleCommand::ChangeStatStage {
                            target: PlayerTarget::from_index(target_index),
                            stat: player_stat,
                            delta: new_stage - old_stage,
                        });
                        commands.push(BattleCommand::EmitEvent(BattleEvent::StatStageChanged {
                            target: target_pokemon.species,
                            stat: player_stat,
                            old_stage,
                            new_stage,
                        }));
                    }
                }
            }
        }
        
        commands
    }
    
    /// Apply raise all stats effect (targets user)
    fn apply_raise_all_stats_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        if rng.next_outcome() <= chance {
            let attacker_player = &state.players[context.attacker_index];
            if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
                let stats_to_raise = [
                    crate::player::StatType::Attack,
                    crate::player::StatType::Defense,
                    crate::player::StatType::SpecialAttack,
                    crate::player::StatType::SpecialDefense,
                    crate::player::StatType::Speed,
                ];
                
                for stat in &stats_to_raise {
                    let old_stage = attacker_player.get_stat_stage(*stat);
                    let new_stage = (old_stage + 1).clamp(-6, 6);
                    
                    if old_stage != new_stage {
                        commands.push(BattleCommand::ChangeStatStage {
                            target: PlayerTarget::from_index(context.attacker_index),
                            stat: *stat,
                            delta: 1,
                        });
                        commands.push(BattleCommand::EmitEvent(BattleEvent::StatStageChanged {
                            target: attacker_pokemon.species,
                            stat: *stat,
                            old_stage,
                            new_stage,
                        }));
                    }
                }
            }
        }
        
        commands
    }
    
    /// Apply heal effect (targets user)
    fn apply_heal_effect(
        &self,
        percentage: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        let attacker_player = &state.players[context.attacker_index];
        if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
            let max_hp = attacker_pokemon.max_hp();
            let current_hp = attacker_pokemon.current_hp();
            
            // Don't heal if already at full HP or fainted
            if current_hp > 0 && current_hp < max_hp {
                let heal_amount = (max_hp * (percentage as u16)) / 100;
                if heal_amount > 0 {
                    commands.push(BattleCommand::HealPokemon {
                        target: PlayerTarget::from_index(context.attacker_index),
                        amount: heal_amount,
                    });
                    
                    // Calculate new HP for event (capped at max)
                    let new_hp = (current_hp + heal_amount).min(max_hp);
                    let actual_heal = new_hp - current_hp;
                    
                    if actual_heal > 0 {
                        commands.push(BattleCommand::EmitEvent(BattleEvent::PokemonHealed {
                            target: attacker_pokemon.species,
                            amount: actual_heal,
                            new_hp,
                        }));
                    }
                }
            }
        }
        
        commands
    }
    
    /// Apply haze effect (clears all stat stages for both players)
    fn apply_haze_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        if rng.next_outcome() <= chance {
            // Clear stat stages for both players
            for player_index in 0..2 {
                let player = &state.players[player_index];
                if let Some(pokemon) = player.active_pokemon() {
                    let all_stats = [
                        crate::player::StatType::Attack,
                        crate::player::StatType::Defense,
                        crate::player::StatType::SpecialAttack,
                        crate::player::StatType::SpecialDefense,
                        crate::player::StatType::Speed,
                        crate::player::StatType::Accuracy,
                        crate::player::StatType::Evasion,
                        crate::player::StatType::Focus,
                    ];
                    
                    for stat in &all_stats {
                        let current_stage = player.get_stat_stage(*stat);
                        if current_stage != 0 {
                            commands.push(BattleCommand::ChangeStatStage {
                                target: PlayerTarget::from_index(player_index),
                                stat: *stat,
                                delta: -current_stage, // Reset to 0
                            });
                            commands.push(BattleCommand::EmitEvent(BattleEvent::StatStageChanged {
                                target: pokemon.species,
                                stat: *stat,
                                old_stage: current_stage,
                                new_stage: 0,
                            }));
                        }
                    }
                    
                    // Note: Individual stat changes already emit StatStageChanged events
                    // A general Haze event could be added to BattleEvent if needed
                }
            }
        }
        
        commands
    }
    
    /// Apply cure status effect
    fn apply_cure_status_effect(
        &self,
        target: &Target,
        status_type: &StatusType,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        let target_index = context.target_index(target);
        let target_player = &state.players[target_index];
        
        if let Some(target_pokemon) = target_player.active_pokemon() {
            // Check if the Pokemon has the status condition we want to cure
            let should_cure = match (&target_pokemon.status, status_type) {
                (Some(crate::pokemon::StatusCondition::Sleep(_)), StatusType::Sleep) => true,
                (Some(crate::pokemon::StatusCondition::Poison(_)), StatusType::Poison) => true,
                (Some(crate::pokemon::StatusCondition::Burn), StatusType::Burn) => true,
                (Some(crate::pokemon::StatusCondition::Freeze), StatusType::Freeze) => true,
                (Some(crate::pokemon::StatusCondition::Paralysis), StatusType::Paralysis) => true,
                _ => false,
            };
            
            if should_cure {
                let old_status = target_pokemon.status.clone().unwrap();
                
                commands.push(BattleCommand::SetPokemonStatus {
                    target: PlayerTarget::from_index(target_index),
                    status: None,
                });
                commands.push(BattleCommand::EmitEvent(BattleEvent::PokemonStatusRemoved {
                    target: target_pokemon.species,
                    status: old_status,
                }));
            }
        }
        
        commands
    }
    
    /// Apply reflect effect (team condition)
    fn apply_reflect_effect(
        &self,
        reflect_type: &ReflectType,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        
        let mut commands = Vec::new();
        
        let team_condition = match reflect_type {
            ReflectType::Physical => crate::player::TeamCondition::Reflect,
            ReflectType::Special => crate::player::TeamCondition::LightScreen,
        };
        
        // Apply to user's team (attacker)
        commands.push(BattleCommand::AddTeamCondition {
            target: PlayerTarget::from_index(context.attacker_index),
            condition: team_condition,
            turns: 5, // Standard duration
        });
        
        commands
    }
    
    /// Apply damage-based effects that require the damage amount
    pub fn apply_damage_based_effects(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        damage_dealt: u16,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        
        let mut commands = Vec::new();
        
        // Only process if damage was actually dealt
        if damage_dealt == 0 {
            return commands;
        }
        
        match self {
            MoveEffect::Recoil(percentage) => {
                commands.extend(self.apply_recoil_effect(*percentage, context, state, damage_dealt));
            }
            MoveEffect::Drain(percentage) => {
                commands.extend(self.apply_drain_effect(*percentage, context, state, damage_dealt));
            }
            _ => {
                // Not a damage-based effect
            }
        }
        
        commands
    }
    
    /// Apply recoil effect (attacker takes damage)
    fn apply_recoil_effect(
        &self,
        percentage: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        damage_dealt: u16,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        
        let mut commands = Vec::new();
        
        let recoil_damage = (damage_dealt * (percentage as u16)) / 100;
        if recoil_damage > 0 {
            commands.push(BattleCommand::DealDamage {
                target: PlayerTarget::from_index(context.attacker_index),
                amount: recoil_damage,
            });
        }
        
        commands
    }
    
    /// Apply drain effect (attacker heals based on damage)
    fn apply_drain_effect(
        &self,
        percentage: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        damage_dealt: u16,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;
        
        let mut commands = Vec::new();
        
        let heal_amount = (damage_dealt * (percentage as u16)) / 100;
        if heal_amount > 0 {
            let attacker_player = &state.players[context.attacker_index];
            if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
                let current_hp = attacker_pokemon.current_hp();
                let max_hp = attacker_pokemon.max_hp();
                
                // Only heal if not at full HP or fainted
                if current_hp > 0 && current_hp < max_hp {
                    commands.push(BattleCommand::HealPokemon {
                        target: PlayerTarget::from_index(context.attacker_index),
                        amount: heal_amount,
                    });
                    
                    // Calculate actual healing for event (capped at max)
                    let new_hp = (current_hp + heal_amount).min(max_hp);
                    let actual_heal = new_hp - current_hp;
                    
                    if actual_heal > 0 {
                        commands.push(BattleCommand::EmitEvent(BattleEvent::PokemonHealed {
                            target: attacker_pokemon.species,
                            amount: actual_heal,
                            new_hp,
                        }));
                    }
                }
            }
        }
        
        commands
    }
    
    /// Apply miss-based effects that trigger when a move misses
    pub fn apply_miss_based_effects(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        
        let mut commands = Vec::new();
        
        match self {
            MoveEffect::Reckless(percentage) => {
                commands.extend(self.apply_reckless_effect(*percentage, context, state));
            }
            _ => {
                // Not a miss-based effect
            }
        }
        
        commands
    }
    
    /// Apply reckless effect (attacker takes damage based on max HP when move misses)
    fn apply_reckless_effect(
        &self,
        percentage: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        
        let mut commands = Vec::new();
        
        let attacker_player = &state.players[context.attacker_index];
        if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
            let max_hp = attacker_pokemon.max_hp();
            let recoil_damage = (max_hp * (percentage as u16)) / 100;
            
            if recoil_damage > 0 {
                commands.push(BattleCommand::DealDamage {
                    target: PlayerTarget::from_index(context.attacker_index),
                    amount: recoil_damage,
                });
            }
        }
        
        commands
    }
}

impl MoveData {
    /// Apply all damage-based effects for this move
    pub fn apply_damage_based_effects(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        damage_dealt: u16,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        let mut all_commands = Vec::new();
        
        for effect in &self.effects {
            let effect_commands = effect.apply_damage_based_effects(context, state, damage_dealt);
            all_commands.extend(effect_commands);
        }
        
        all_commands
    }
    
    /// Apply all miss-based effects for this move
    pub fn apply_miss_based_effects(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        let mut all_commands = Vec::new();
        
        for effect in &self.effects {
            let effect_commands = effect.apply_miss_based_effects(context, state);
            all_commands.extend(effect_commands);
        }
        
        all_commands
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveData {
    pub name: String,
    pub move_type: PokemonType,
    pub power: Option<u8>, // None for no damage moves
    pub category: MoveCategory,
    pub accuracy: Option<u8>, // None for sure-hit moves
    pub max_pp: u8,
    pub effects: Vec<MoveEffect>,
}

impl MoveData {
    /// Load move data from RON files in the data directory
    pub fn load_all(
        data_path: &Path,
    ) -> Result<HashMap<Move, MoveData>, Box<dyn std::error::Error>> {
        let moves_dir = data_path.join("moves");

        let mut move_map = HashMap::new();
        let hitting_itself_data = MoveData {
            name: "Hit Itself".to_string(),
            move_type: PokemonType::Typeless,
            power: Some(40),
            category: MoveCategory::Physical,
            accuracy: None, // Always hits
            max_pp: 0,      // Not a real move, no PP
            effects: vec![],
        };
        move_map.insert(Move::HittingItself, hitting_itself_data);

        // Add Struggle.
        // It has fixed data and recoil. Recoil is 25% of damage dealt here.
        // Note: In some game generations, recoil is 1/4 of the user's max HP.
        let struggle_data = MoveData {
            name: "Struggle".to_string(),
            move_type: PokemonType::Typeless,
            power: Some(50),
            category: MoveCategory::Physical,
            accuracy: Some(90),
            max_pp: 0,                             // Not a real move, no PP
            effects: vec![MoveEffect::Recoil(25)], // 25% recoil of damage dealt
        };
        move_map.insert(Move::Struggle, struggle_data);

        if !moves_dir.exists() {
            return Err(format!("Moves data directory not found: {}", moves_dir.display()).into());
        }

        let entries = fs::read_dir(&moves_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    let content = fs::read_to_string(&path)?;
                    let move_data: MoveData = ron::from_str(&content)?;

                    // Parse move name from filename to get the Move enum variant
                    if let Ok(move_enum) = filename.parse::<Move>() {
                        move_map.insert(move_enum, move_data);
                    }
                }
            }
        }

        Ok(move_map)
    }

    /// Get move data for a specific move
    pub fn get_move_data(move_: Move, move_map: &HashMap<Move, MoveData>) -> Option<&MoveData> {
        move_map.get(&move_)
    }
}

// Helper function to parse Move enum from string
impl std::str::FromStr for Move {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_uppercase().replace([' ', '-', '_'], "");

        match normalized.as_str() {
            "POUND" => Ok(Move::Pound),
            "DOUBLESLAP" => Ok(Move::Doubleslap),
            "PAYDAY" => Ok(Move::PayDay),
            "SCRATCH" => Ok(Move::Scratch),
            "GUILLOTINE" => Ok(Move::Guillotine),
            "SWORDSDANCE" => Ok(Move::SwordsDance),
            "CUT" => Ok(Move::Cut),
            "BIND" => Ok(Move::Bind),
            "SLAM" => Ok(Move::Slam),
            "STOMP" => Ok(Move::Stomp),
            "HEADBUTT" => Ok(Move::Headbutt),
            "HORNATTACK" => Ok(Move::HornAttack),
            "FURYATTACK" => Ok(Move::FuryAttack),
            "HORNDRILL" => Ok(Move::HornDrill),
            "TACKLE" => Ok(Move::Tackle),
            "BODYSLAM" => Ok(Move::BodySlam),
            "WRAP" => Ok(Move::Wrap),
            "HARDEN" => Ok(Move::Harden),
            "TAKEDOWN" => Ok(Move::TakeDown),
            "THRASH" => Ok(Move::Thrash),
            "DOUBLEEDGE" => Ok(Move::DoubleEdge),
            "TAILWHIP" => Ok(Move::TailWhip),
            "LEER" => Ok(Move::Leer),
            "BITE" => Ok(Move::Bite),
            "GROWL" => Ok(Move::Growl),
            "ROAR" => Ok(Move::Roar),
            "SING" => Ok(Move::Sing),
            "SUPERSONIC" => Ok(Move::Supersonic),
            "SONICBOOM" => Ok(Move::SonicBoom),
            "DISABLE" => Ok(Move::Disable),
            "AGILITY" => Ok(Move::Agility),
            "QUICKATTACK" => Ok(Move::QuickAttack),
            "RAGE" => Ok(Move::Rage),
            "MIMIC" => Ok(Move::Mimic),
            "SCREECH" => Ok(Move::Screech),
            "DOUBLETEAM" => Ok(Move::DoubleTeam),
            "RECOVER" => Ok(Move::Recover),
            "MINIMIZE" => Ok(Move::Minimize),
            "WITHDRAW" => Ok(Move::Withdraw),
            "DEFENSECURL" => Ok(Move::DefenseCurl),
            "BARRIER" => Ok(Move::Barrier),
            "FOCUSENERGY" => Ok(Move::FocusEnergy),
            "BIDE" => Ok(Move::Bide),
            "METRONOME" => Ok(Move::Metronome),
            "MIRRORMOVE" => Ok(Move::MirrorMove),
            "SELFDESTRUCT" => Ok(Move::SelfDestruct),
            "CLAMP" => Ok(Move::Clamp),
            "SWIFT" => Ok(Move::Swift),
            "SPIKECANNON" => Ok(Move::SpikeCannon),
            "CONSTRICT" => Ok(Move::Constrict),
            "SOFTBOILED" => Ok(Move::SoftBoiled),
            "GLARE" => Ok(Move::Glare),
            "TRANSFORM" => Ok(Move::Transform),
            "EXPLOSION" => Ok(Move::Explosion),
            "FURYSWIPES" => Ok(Move::FurySwipes),
            "REST" => Ok(Move::Rest),
            "HYPERFANG" => Ok(Move::HyperFang),
            "SHARPEN" => Ok(Move::Sharpen),
            "CONVERSION" => Ok(Move::Conversion),
            "TRIATTACK" => Ok(Move::TriAttack),
            "SUPERFANG" => Ok(Move::SuperFang),
            "SLASH" => Ok(Move::Slash),
            "SUBSTITUTE" => Ok(Move::Substitute),
            "HYPERBEAM" => Ok(Move::HyperBeam),
            "KARATECHOP" => Ok(Move::KarateChop),
            "COMETPUNCH" => Ok(Move::CometPunch),
            "MEGAPUNCH" => Ok(Move::MegaPunch),
            "KOPUNCH" => Ok(Move::KoPunch),
            "DOUBLEKICK" => Ok(Move::DoubleKick),
            "MEGAKICK" => Ok(Move::MegaKick),
            "JUMPKICK" => Ok(Move::JumpKick),
            "ROLLINGKICK" => Ok(Move::RollingKick),
            "SUBMISSION" => Ok(Move::Submission),
            "LOWKICK" => Ok(Move::LowKick),
            "COUNTER" => Ok(Move::Counter),
            "SEISMICTOSS" => Ok(Move::SeismicToss),
            "STRENGTH" => Ok(Move::Strength),
            "MEDITATE" => Ok(Move::Meditate),
            "HIGHJUMPKICK" => Ok(Move::HighJumpKick),
            "BARRAGE" => Ok(Move::Barrage),
            "DIZZYPUNCH" => Ok(Move::DizzyPunch),
            "RAZORWIND" => Ok(Move::RazorWind),
            "GUST" => Ok(Move::Gust),
            "WINGATTACK" => Ok(Move::WingAttack),
            "WHIRLWIND" => Ok(Move::Whirlwind),
            "FLY" => Ok(Move::Fly),
            "PECK" => Ok(Move::Peck),
            "DRILLPECK" => Ok(Move::DrillPeck),
            "SKYATTACK" => Ok(Move::SkyAttack),
            "VICEGRIP" => Ok(Move::Vicegrip),
            "ROCKTHROW" => Ok(Move::RockThrow),
            "SKULLBASH" => Ok(Move::SkullBash),
            "ROCKSLIDE" => Ok(Move::RockSlide),
            "ANCIENTPOWER" => Ok(Move::AncientPower),
            "SANDATTACK" => Ok(Move::SandAttack),
            "EARTHQUAKE" => Ok(Move::Earthquake),
            "FISSURE" => Ok(Move::Fissure),
            "DIG" => Ok(Move::Dig),
            "BONECLUB" => Ok(Move::BoneClub),
            "BONEMERANG" => Ok(Move::Bonemerang),
            "POISONSTING" => Ok(Move::PoisonSting),
            "TWINEEDLE" => Ok(Move::Twineedle),
            "ACID" => Ok(Move::Acid),
            "TOXIC" => Ok(Move::Toxic),
            "HAZE" => Ok(Move::Haze),
            "SMOG" => Ok(Move::Smog),
            "SLUDGE" => Ok(Move::Sludge),
            "POISONJAB" => Ok(Move::PoisonJab),
            "POISONGAS" => Ok(Move::PoisonGas),
            "ACIDARMOR" => Ok(Move::AcidArmor),
            "PINMISSILE" => Ok(Move::PinMissile),
            "SILVERWIND" => Ok(Move::SilverWind),
            "STRINGSHOT" => Ok(Move::StringShot),
            "LEECHLIFE" => Ok(Move::LeechLife),
            "FIREPUNCH" => Ok(Move::FirePunch),
            "BLAZEKICK" => Ok(Move::BlazeKick),
            "FIREFANG" => Ok(Move::FireFang),
            "EMBER" => Ok(Move::Ember),
            "FLAMETHROWER" => Ok(Move::Flamethrower),
            "WILLOWISP" => Ok(Move::WillOWisp),
            "FIRESPIN" => Ok(Move::FireSpin),
            "SMOKESCREEN" => Ok(Move::Smokescreen),
            "FIREBLAST" => Ok(Move::FireBlast),
            "MIST" => Ok(Move::Mist),
            "WATERGUN" => Ok(Move::WaterGun),
            "HYDROPUMP" => Ok(Move::HydroPump),
            "SURF" => Ok(Move::Surf),
            "BUBBLEBEAM" => Ok(Move::Bubblebeam),
            "WATERFALL" => Ok(Move::Waterfall),
            "BUBBLE" => Ok(Move::Bubble),
            "SPLASH" => Ok(Move::Splash),
            "BUBBLEHAMMER" => Ok(Move::Bubblehammer),
            "VINEWHIP" => Ok(Move::VineWhip),
            "ABSORB" => Ok(Move::Absorb),
            "MEGADRAIN" => Ok(Move::MegaDrain),
            "GIGADRAIN" => Ok(Move::GigaDrain),
            "LEECHSEED" => Ok(Move::LeechSeed),
            "GROWTH" => Ok(Move::Growth),
            "RAZORLEAF" => Ok(Move::RazorLeaf),
            "SOLARBEAM" => Ok(Move::Solarbeam),
            "POISONPOWDER" => Ok(Move::PoisonPowder),
            "STUNSPORE" => Ok(Move::StunSpore),
            "SLEEPPOWDER" => Ok(Move::SleepPowder),
            "PETALDANCE" => Ok(Move::PetalDance),
            "SPORE" => Ok(Move::Spore),
            "EGGBOMB" => Ok(Move::EggBomb),
            "ICEPUNCH" => Ok(Move::IcePunch),
            "ICEBEAM" => Ok(Move::IceBeam),
            "BLIZZARD" => Ok(Move::Blizzard),
            "AURORABEAM" => Ok(Move::AuroraBeam),
            "THUNDERPUNCH" => Ok(Move::ThunderPunch),
            "SHOCK" => Ok(Move::Shock),
            "DISCHARGE" => Ok(Move::Discharge),
            "THUNDERWAVE" => Ok(Move::ThunderWave),
            "THUNDERCLAP" => Ok(Move::Thunderclap),
            "CHARGEBEAM" => Ok(Move::ChargeBeam),
            "LIGHTNING" => Ok(Move::Lightning),
            "FLASH" => Ok(Move::Flash),
            "CONFUSION" => Ok(Move::Confusion),
            "PSYBEAM" => Ok(Move::Psybeam),
            "PERPLEX" => Ok(Move::Perplex),
            "HYPNOSIS" => Ok(Move::Hypnosis),
            "TELEPORT" => Ok(Move::Teleport),
            "CONFUSERAY" => Ok(Move::ConfuseRay),
            "LIGHTSCREEN" => Ok(Move::LightScreen),
            "REFLECT" => Ok(Move::Reflect),
            "AMNESIA" => Ok(Move::Amnesia),
            "KINESIS" => Ok(Move::Kinesis),
            "PSYCHIC" => Ok(Move::Psychic),
            "PSYWAVE" => Ok(Move::Psywave),
            "DREAMEATER" => Ok(Move::DreamEater),
            "LOVELYKISS" => Ok(Move::LovelyKiss),
            "NIGHTSHADE" => Ok(Move::NightShade),
            "LICK" => Ok(Move::Lick),
            "SHADOWBALL" => Ok(Move::ShadowBall),
            "OUTRAGE" => Ok(Move::Outrage),
            "DRAGONRAGE" => Ok(Move::DragonRage),
            "STRUGGLE" => Ok(Move::Struggle),
            "HITITSELF" => Ok(Move::HittingItself),
            _ => Err(format!("Unknown move: {}", s)),
        }
    }
}
