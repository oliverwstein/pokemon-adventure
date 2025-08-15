use crate::battle::conditions::PokemonCondition;
use crate::moves::Move;
use crate::pokemon::PokemonType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Include the compiled move data
include!(concat!(env!("OUT_DIR"), "/generated_data.rs"));

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
    Haze(u8), // remove all stat changes, chance %
    SetTeamCondition(crate::player::TeamCondition, u8),
    Seed(u8),  // leech seed effect, chance %
    Nightmare, // only works on sleeping targets

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

/// Result of applying a move effect, controlling execution flow
#[derive(Debug, Clone)]
pub enum EffectResult {
    /// Apply commands and continue with normal attack execution
    Continue(Vec<crate::battle::commands::BattleCommand>),
    /// Apply commands and skip normal attack execution
    Skip(Vec<crate::battle::commands::BattleCommand>),
}

impl EffectContext {
    pub fn new(
        attacker_index: usize,
        defender_index: usize,
        move_used: crate::moves::Move,
    ) -> Self {
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
    /// Apply this effect to the battle state, returning commands and execution control
    pub fn apply(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> EffectResult {

        let defender_has_substitute = state.players[context.defender_index]
            .active_pokemon_conditions
            .values()
            .any(|condition| matches!(condition, PokemonCondition::Substitute { .. }));

        // 2. If so, ask the effect if it's blocked and return early if it is.
        if defender_has_substitute && self.is_blocked_by_substitute() {
            return EffectResult::Continue(Vec::new()); // Effect is nullified but continue with attack.
        }

        // Handle all effects through unified apply system
        match self {
            // Special moves that may skip attack execution
            MoveEffect::InAir => self.apply_in_air_special(context, state),
            MoveEffect::Teleport(_) => self.apply_teleport_special(context, state),
            MoveEffect::ChargeUp => self.apply_charge_up_special(context, state),
            MoveEffect::Underground => self.apply_underground_special(context, state),
            MoveEffect::Transform => self.apply_transform_special(context, state),
            MoveEffect::Conversion => self.apply_conversion_special(context, state),
            MoveEffect::Substitute => self.apply_substitute_special(context, state),
            MoveEffect::Counter => self.apply_counter_special(context, state),
            MoveEffect::Bide(turns) => self.apply_bide_special(*turns, context, state),
            MoveEffect::MirrorMove => self.apply_mirror_move_special(context, state),
            MoveEffect::Rest(sleep_turns) => self.apply_rest_special(*sleep_turns, context, state),
            MoveEffect::Metronome => self.apply_metronome_special(context, state, rng),

            // Special moves that continue with attack execution
            MoveEffect::Rampage(_) => self.apply_rampage_special(context, state, rng),
            MoveEffect::Rage(_) => self.apply_rage_special(context, state),
            MoveEffect::Explode => self.apply_explode_special(context, state),

            // Regular effects that always continue with attack execution
            MoveEffect::Burn(chance) => {
                EffectResult::Continue(self.apply_burn_effect(*chance, context, state, rng))
            }
            MoveEffect::Paralyze(chance) => {
                EffectResult::Continue(self.apply_paralyze_effect(*chance, context, state, rng))
            }
            MoveEffect::Freeze(chance) => {
                EffectResult::Continue(self.apply_freeze_effect(*chance, context, state, rng))
            }
            MoveEffect::Poison(chance) => {
                EffectResult::Continue(self.apply_poison_effect(*chance, context, state, rng))
            }
            MoveEffect::Sedate(chance) => {
                EffectResult::Continue(self.apply_sedate_effect(*chance, context, state, rng))
            }
            MoveEffect::Flinch(chance) => {
                EffectResult::Continue(self.apply_flinch_effect(*chance, context, state, rng))
            }
            MoveEffect::Confuse(chance) => {
                EffectResult::Continue(self.apply_confuse_effect(*chance, context, state, rng))
            }
            MoveEffect::Trap(chance) => {
                EffectResult::Continue(self.apply_trap_effect(*chance, context, state, rng))
            }
            MoveEffect::Seed(chance) => {
                EffectResult::Continue(self.apply_seed_effect(*chance, context, state, rng))
            }
            MoveEffect::Exhaust(chance) => {
                EffectResult::Continue(self.apply_exhaust_effect(*chance, context, state, rng))
            }
            MoveEffect::StatChange(target, stat, stages, chance) => EffectResult::Continue(
                self.apply_stat_change_effect(target, stat, *stages, *chance, context, state, rng),
            ),
            MoveEffect::RaiseAllStats(chance) => EffectResult::Continue(
                self.apply_raise_all_stats_effect(*chance, context, state, rng),
            ),
            MoveEffect::Heal(percentage) => {
                EffectResult::Continue(self.apply_heal_effect(*percentage, context, state))
            }
            MoveEffect::Haze(chance) => {
                EffectResult::Continue(self.apply_haze_effect(*chance, context, state, rng))
            }
            MoveEffect::CureStatus(target, status_type) => EffectResult::Continue(
                self.apply_cure_status_effect(target, status_type, context, state),
            ),
            MoveEffect::SetTeamCondition(condition, turns) => {
                EffectResult::Continue(self.apply_team_condition_effect(condition, *turns, context))
            }
            MoveEffect::Ante(chance) => {
                EffectResult::Continue(self.apply_ante_effect(*chance, context, state, rng))
            }
            MoveEffect::Recoil(_) | MoveEffect::Drain(_) => {
                // Damage-based effects are handled separately in apply_damage_based_effects
                EffectResult::Continue(Vec::new())
            }
            MoveEffect::Reckless(_) => {
                // Miss-based effects are handled separately in apply_miss_based_effects
                EffectResult::Continue(Vec::new())
            }
            _ => {
                // For effects not yet migrated, return empty command list
                EffectResult::Continue(Vec::new())
            }
        }
    }

    pub fn is_blocked_by_substitute(&self) -> bool {
        use crate::move_data::{MoveEffect, Target};

        match self {
            // --- EFFECTS THAT BYPASS SUBSTITUTE ---

            // Effects that explicitly target the user.
            MoveEffect::Heal(_)
            | MoveEffect::Exhaust(_)
            | MoveEffect::RaiseAllStats(_)
            | MoveEffect::Rest(_)
            | MoveEffect::Rage(_)
            | MoveEffect::Substitute
            | MoveEffect::Transform
            | MoveEffect::Conversion
            | MoveEffect::Counter
            | MoveEffect::Bide(_)
            | MoveEffect::Explode
            | MoveEffect::Reckless(_)
            | MoveEffect::MirrorMove
            | MoveEffect::Metronome => false,

            // Damage modifiers that affect the user's calculation, not the target.
            MoveEffect::Recoil(_)
            | MoveEffect::Drain(_)
            | MoveEffect::Crit(_)
            | MoveEffect::IgnoreDef(_)
            | MoveEffect::Priority(_)
            | MoveEffect::MultiHit(_, _) => false,

            // Field effects or team conditions that affect the user's side.
            MoveEffect::Haze(_) | MoveEffect::SetTeamCondition(..) => false,

            // Conditional effects: blocked only if they target the opponent.
            MoveEffect::StatChange(target, ..) => matches!(target, Target::Target),
            MoveEffect::CureStatus(target, ..) => matches!(target, Target::Target),

            // --- EFFECTS THAT ARE BLOCKED BY SUBSTITUTE ---

            // All other effects are assumed to target the opponent and are blocked by default.
            // This includes all primary status conditions (Burn, Flinch, etc.),
            // stat-lowering effects on the target, and other debilitating conditions.
            _ => true,
        }
    }

    /// If this effect is MultiHit, calculates if another hit should be queued and returns the command.
    /// This function assumes preconditions (like the defender not fainting) have already been checked by the caller.
    pub fn apply_multi_hit_continuation(
        &self,
        context: &EffectContext,
        rng: &mut crate::battle::state::TurnRng,
        hit_number: u8,
    ) -> Option<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::BattleCommand;
        use crate::battle::engine::BattleAction;

        // This logic only applies if the effect is actually a MultiHit variant.
        if let MoveEffect::MultiHit(guaranteed_hits, continuation_chance) = self {
            let next_hit_number = hit_number + 1;

            // Determine if the next hit should be queued.
            let should_queue_next_hit = if next_hit_number <= *guaranteed_hits {
                // We are still within the guaranteed number of hits.
                true
            } else {
                // Past the guaranteed hits, so roll for continuation.
                // Assuming a max of 7 hits for any sequence.
                // This is a change from a max of 5 because we allow each hit to miss indepedently
                next_hit_number <= 7
                    && rng.next_outcome("Multi-Hit Continuation Check") <= *continuation_chance
            };

            if should_queue_next_hit {
                // Return a command to push the next hit onto the action stack.
                return Some(BattleCommand::PushAction(BattleAction::AttackHit {
                    attacker_index: context.attacker_index,
                    defender_index: context.defender_index,
                    move_used: context.move_used,
                    hit_number: next_hit_number,
                }));
            }
        }

        // If not a multi-hit effect or if the continuation roll fails, return None.
        None
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

        let target_player = &state.players[context.defender_index];
        if let Some(target_pokemon) = target_player.active_pokemon() {
            // Fire-type Pokemon are immune to burn
            let target_types = target_pokemon.get_current_types(target_player);
            if target_types.contains(&crate::pokemon::PokemonType::Fire) {
                return commands; // No RNG check, no burn application
            }

            if rng.next_outcome("Apply Burn Check") <= chance {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: Some(crate::pokemon::StatusCondition::Burn),
                    });
                    commands.push(BattleCommand::EmitEvent(
                        BattleEvent::PokemonStatusApplied {
                            target: target_pokemon.species,
                            status: crate::pokemon::StatusCondition::Burn,
                        },
                    ));
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

        let target_player = &state.players[context.defender_index];
        if let Some(target_pokemon) = target_player.active_pokemon() {
            // Electric-type Pokemon are immune to paralysis
            let target_types = target_pokemon.get_current_types(target_player);
            if target_types.contains(&crate::pokemon::PokemonType::Electric) {
                return commands; // No RNG check, no paralysis application
            }

            if rng.next_outcome("Apply Paralysis Check") <= chance {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: Some(crate::pokemon::StatusCondition::Paralysis),
                    });
                    commands.push(BattleCommand::EmitEvent(
                        BattleEvent::PokemonStatusApplied {
                            target: target_pokemon.species,
                            status: crate::pokemon::StatusCondition::Paralysis,
                        },
                    ));
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

        let target_player = &state.players[context.defender_index];
        if let Some(target_pokemon) = target_player.active_pokemon() {
            // Ice-type Pokemon are immune to freeze
            let target_types = target_pokemon.get_current_types(target_player);
            if target_types.contains(&crate::pokemon::PokemonType::Ice) {
                return commands; // No RNG check, no freeze application
            }

            if rng.next_outcome("Apply Freeze Check") <= chance {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: Some(crate::pokemon::StatusCondition::Freeze),
                    });
                    commands.push(BattleCommand::EmitEvent(
                        BattleEvent::PokemonStatusApplied {
                            target: target_pokemon.species,
                            status: crate::pokemon::StatusCondition::Freeze,
                        },
                    ));
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

        let target_player = &state.players[context.defender_index];
        if let Some(target_pokemon) = target_player.active_pokemon() {
            // Poison-type Pokemon are immune to poison
            let target_types = target_pokemon.get_current_types(target_player);
            if target_types.contains(&crate::pokemon::PokemonType::Poison) {
                return commands; // No RNG check, no poison application
            }

            if rng.next_outcome("Apply Poison Check") <= chance {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: Some(crate::pokemon::StatusCondition::Poison(0)),
                    });
                    commands.push(BattleCommand::EmitEvent(
                        BattleEvent::PokemonStatusApplied {
                            target: target_pokemon.species,
                            status: crate::pokemon::StatusCondition::Poison(0),
                        },
                    ));
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

        let target_player = &state.players[context.defender_index];
        if let Some(target_pokemon) = target_player.active_pokemon() {
            // Ghost-type Pokemon are immune to sleep
            let target_types = target_pokemon.get_current_types(target_player);
            if target_types.contains(&crate::pokemon::PokemonType::Ghost) {
                return commands; // No RNG check, no sleep application
            }

            if rng.next_outcome("Apply Sedate Check") <= chance {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    // Sleep for 1-3 turns (random)
                    let sleep_turns = (rng.next_outcome("Generate Sleep Duration") % 3) + 1; // 1, 2, or 3 turns
                    let sleep_status = crate::pokemon::StatusCondition::Sleep(sleep_turns);

                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: Some(sleep_status),
                    });
                    commands.push(BattleCommand::EmitEvent(
                        BattleEvent::PokemonStatusApplied {
                            target: target_pokemon.species,
                            status: sleep_status,
                        },
                    ));
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

        if rng.next_outcome("Apply Flinch Effect") <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(target_pokemon) = target_player.active_pokemon() {
                let condition = PokemonCondition::Flinched;

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

        if rng.next_outcome("Apply Confuse Effect") <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(target_pokemon) = target_player.active_pokemon() {
                // Confuse for 1-4 turns (random)
                let confuse_turns = (rng.next_outcome("Generate Confusion Duration") % 4) + 1;
                let condition = PokemonCondition::Confused {
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

        if rng.next_outcome("Apply Trap Check") <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(target_pokemon) = target_player.active_pokemon() {
                // Trap for 2-5 turns (random)
                let trap_turns = (rng.next_outcome("Generate Trap Duration") % 4) + 2;
                let condition = PokemonCondition::Trapped {
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

    fn apply_seed_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;

        let mut commands = Vec::new();

        if rng.next_outcome("Apply Seeded Effect") <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(target_pokemon) = target_player.active_pokemon() {
                let condition = PokemonCondition::Seeded;

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

        if rng.next_outcome("Apply Exhaust Check") <= chance {
            let attacker_player = &state.players[context.attacker_index];
            if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
                let condition = PokemonCondition::Exhausted {
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
        let rng_reason = format!("Apply {:?} {:?} Effect: ", stat, stages);
        if rng.next_outcome(&rng_reason) <= chance {
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
                let has_mist =
                    target_player.has_team_condition(&crate::player::TeamCondition::Mist);

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

        if rng.next_outcome("Apply Raise All Stats Check") <= chance {
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
        _context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;

        let mut commands = Vec::new();

        if rng.next_outcome("Apply Haze Check") <= chance {
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
                            commands.push(BattleCommand::EmitEvent(
                                BattleEvent::StatStageChanged {
                                    target: pokemon.species,
                                    stat: *stat,
                                    old_stage: current_stage,
                                    new_stage: 0,
                                },
                            ));
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
                commands.push(BattleCommand::EmitEvent(
                    BattleEvent::PokemonStatusRemoved {
                        target: target_pokemon.species,
                        status: old_status,
                    },
                ));
            }
        }

        commands
    }

    fn apply_team_condition_effect(
        &self,
        condition: &crate::player::TeamCondition,
        turns: u8,
        context: &EffectContext,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        // This function is now incredibly simple. It just creates the command.
        vec![BattleCommand::AddTeamCondition {
            target: PlayerTarget::from_index(context.attacker_index),
            condition: *condition,
            turns,
        }]
    }

    /// Apply ante effect (Pay Day)
    fn apply_ante_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;

        let mut commands = Vec::new();

        if rng.next_outcome("Apply Ante Check") <= chance {
            let attacker_player = &state.players[context.attacker_index];
            if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
                let pokemon_level = attacker_pokemon.level as u32;
                let ante_amount = pokemon_level * 2;

                // We need the defender's current ante to create the event correctly
                let defender_player = &state.players[context.defender_index];
                let new_total = defender_player.get_ante() + ante_amount;

                // Command to add the ante
                commands.push(BattleCommand::AddAnte {
                    target: PlayerTarget::from_index(context.defender_index),
                    amount: ante_amount,
                });

                // Command to emit the event
                commands.push(BattleCommand::EmitEvent(BattleEvent::AnteIncreased {
                    player_index: context.defender_index,
                    amount: ante_amount,
                    new_total,
                }));
            }
        }

        commands
    }

    /// Apply damage-based effects that require the damage amount
    pub fn apply_damage_based_effects(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        damage_dealt: u16,
    ) -> Vec<crate::battle::commands::BattleCommand> {

        let mut commands = Vec::new();

        // Only process if damage was actually dealt
        if damage_dealt == 0 {
            return commands;
        }

        match self {
            MoveEffect::Recoil(percentage) => {
                commands.extend(self.apply_recoil_effect(
                    *percentage,
                    context,
                    damage_dealt,
                ));
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

    // Special move effect implementations

    /// Apply InAir effect (Fly move pattern)
    fn apply_in_air_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;
        use crate::battle::state::BattleEvent;

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        // If already in air, this is the second turn - clear condition and proceed with normal attack
        if attacker_player.has_condition(&PokemonCondition::InAir) {
            let commands = vec![BattleCommand::RemoveCondition {
                target: attacker_target,
                condition_type: crate::battle::conditions::PokemonConditionType::InAir,
            }];
            return EffectResult::Continue(commands);
        }

        // First turn - apply condition and skip normal attack
        if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
            let condition = PokemonCondition::InAir;
            let commands = vec![
                BattleCommand::AddCondition {
                    target: attacker_target,
                    condition: condition.clone(),
                },
                BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                    target: pokemon_species,
                    status: condition,
                }),
            ];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Teleport effect
    fn apply_teleport_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;
        use crate::battle::state::BattleEvent;

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
            let condition = PokemonCondition::Teleported;
            let commands = vec![
                BattleCommand::AddCondition {
                    target: attacker_target,
                    condition: condition.clone(),
                },
                BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                    target: pokemon_species,
                    status: condition,
                }),
            ];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply ChargeUp effect (Solar Beam pattern)
    fn apply_charge_up_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;
        use crate::battle::state::BattleEvent;

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        // If already charging, this is the second turn - clear condition and proceed with normal attack
        if attacker_player.has_condition(&PokemonCondition::Charging) {
            let commands = vec![BattleCommand::RemoveCondition {
                target: attacker_target,
                condition_type: crate::battle::conditions::PokemonConditionType::Charging,
            }];
            return EffectResult::Continue(commands);
        }

        // First turn - apply condition and skip normal attack
        if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
            let condition = PokemonCondition::Charging;
            let commands = vec![
                BattleCommand::AddCondition {
                    target: attacker_target,
                    condition: condition.clone(),
                },
                BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                    target: pokemon_species,
                    status: condition,
                }),
            ];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Underground effect (Dig move pattern)
    fn apply_underground_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;
        use crate::battle::state::BattleEvent;

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        // If already underground, this is the second turn - clear condition and proceed with normal attack
        if attacker_player.has_condition(&PokemonCondition::Underground) {
            let commands = vec![BattleCommand::RemoveCondition {
                target: attacker_target,
                condition_type: crate::battle::conditions::PokemonConditionType::Underground,
            }];
            return EffectResult::Continue(commands);
        }

        // First turn - apply condition and skip normal attack
        if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
            let condition = PokemonCondition::Underground;
            let commands = vec![
                BattleCommand::AddCondition {
                    target: attacker_target,
                    condition: condition.clone(),
                },
                BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                    target: pokemon_species,
                    status: condition,
                }),
            ];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Transform effect
    fn apply_transform_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;
        use crate::battle::state::BattleEvent;

        let attacker_player = &state.players[context.attacker_index];
        let defender_player = &state.players[context.defender_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        if let (Some(attacker_species), Some(target_pokemon)) = (
            attacker_player.active_pokemon().map(|p| p.species),
            defender_player.active_pokemon().cloned(),
        ) {
            let condition = PokemonCondition::Transformed {
                target: target_pokemon,
            };
            let commands = vec![
                BattleCommand::AddCondition {
                    target: attacker_target,
                    condition: condition.clone(),
                },
                BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                    target: attacker_species,
                    status: condition,
                }),
            ];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Conversion effect
    fn apply_conversion_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;
        use crate::battle::state::BattleEvent;

        let attacker_player = &state.players[context.attacker_index];
        let defender_player = &state.players[context.defender_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        if let (Some(attacker_species), Some(target_type)) = (
            attacker_player.active_pokemon().map(|p| p.species),
            defender_player
                .active_pokemon()
                .map(|target_pokemon| target_pokemon.get_current_types(defender_player))
                .and_then(|types| types.into_iter().next()), // Take first type
        ) {
            let condition = PokemonCondition::Converted {
                pokemon_type: target_type,
            };
            let commands = vec![
                BattleCommand::AddCondition {
                    target: attacker_target,
                    condition: condition.clone(),
                },
                BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                    target: attacker_species,
                    status: condition,
                }),
            ];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Substitute effect
    fn apply_substitute_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;
        use crate::battle::state::BattleEvent;

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
            let pokemon_species = attacker_pokemon.species;
            // Substitute uses 25% of max HP
            let substitute_hp = (attacker_pokemon.max_hp() / 4).max(1) as u8;
            let condition = PokemonCondition::Substitute { hp: substitute_hp };

            let commands = vec![
                BattleCommand::AddCondition {
                    target: attacker_target,
                    condition: condition.clone(),
                },
                BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                    target: pokemon_species,
                    status: condition,
                }),
            ];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Counter effect
    fn apply_counter_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;
        use crate::battle::state::BattleEvent;

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
            let condition = PokemonCondition::Countering { damage: 0 };
            let commands = vec![
                BattleCommand::AddCondition {
                    target: attacker_target,
                    condition: condition.clone(),
                },
                BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                    target: pokemon_species,
                    status: condition,
                }),
            ];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Rampage effect
    fn apply_rampage_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;
        use crate::battle::state::BattleEvent;

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
            // Check if Pokemon is already rampaging
            if let Some(current_rampage) = attacker_player.active_pokemon_conditions.values()
                .find(|c| matches!(c, PokemonCondition::Rampaging { .. })) {
                
                if let PokemonCondition::Rampaging { turns_remaining } = current_rampage {
                    if *turns_remaining > 0 {
                        // Still rampaging, don't apply rampage again, just continue with attack
                        return EffectResult::Continue(Vec::new());
                    } else {
                        // Rampage ending (turns_remaining == 0), apply confusion instead
                        let confusion_condition = PokemonCondition::Confused {
                            turns_remaining: 2,
                        };
                        let commands = vec![
                            BattleCommand::AddCondition {
                                target: attacker_target,
                                condition: confusion_condition.clone(),
                            },
                            BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                                target: pokemon_species,
                                status: confusion_condition,
                            }),
                        ];
                        return EffectResult::Continue(commands);
                    }
                }
            }
            
            // Not rampaging yet, apply rampage normally
            // Rampage lasts 2-3 turns (50/50 chance)
            let turns = if rng.next_outcome("Generate Rampage Duration") <= 50 {
                2
            } else {
                3
            };
            let condition = PokemonCondition::Rampaging {
                turns_remaining: turns,
            };
            let commands = vec![
                BattleCommand::AddCondition {
                    target: attacker_target,
                    condition: condition.clone(),
                },
                BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                    target: pokemon_species,
                    status: condition,
                }),
            ];
            return EffectResult::Continue(commands); // Continue with attack
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Rage effect
    fn apply_rage_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;
        use crate::battle::state::BattleEvent;

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
            let condition = PokemonCondition::Enraged;
            let commands = vec![
                BattleCommand::AddCondition {
                    target: attacker_target,
                    condition: condition.clone(),
                },
                BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                    target: pokemon_species,
                    status: condition,
                }),
            ];
            return EffectResult::Continue(commands); // Continue with attack
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Explode effect
    fn apply_explode_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
            let current_hp = attacker_pokemon.current_hp();

            let commands = vec![BattleCommand::DealDamage {
                target: attacker_target,
                amount: current_hp,
            }];
            return EffectResult::Continue(commands); // Continue with attack
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Bide effect (complex state machine)
    fn apply_bide_special(
        &self,
        turns: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;
        use crate::battle::state::BattleEvent;

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);
        let defender_target = PlayerTarget::from_index(context.defender_index);

        // Check if already Biding
        if let Some(bide_condition) =
            attacker_player
                .active_pokemon_conditions
                .values()
                .find_map(|condition| match condition {
                    PokemonCondition::Biding {
                        turns_remaining,
                        damage,
                    } => Some((turns_remaining, damage)),
                    _ => None,
                })
        {
            let (turns_remaining, stored_damage) = bide_condition;

            if *turns_remaining < 1 {
                // Last turn of Bide - execute stored damage
                let damage_to_deal = (stored_damage * 2).max(1); // Double damage, minimum 1

                let commands = vec![BattleCommand::DealDamage {
                    target: defender_target,
                    amount: damage_to_deal,
                }];
                return EffectResult::Skip(commands); // Skip normal execution
            } else {
                // Still Biding, skip normal execution (do nothing this turn)
                return EffectResult::Skip(Vec::new());
            }
        } else {
            // Not currently Biding - start new Bide
            if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
                let condition = PokemonCondition::Biding {
                    turns_remaining: turns,
                    damage: 0,
                };
                let commands = vec![
                    BattleCommand::AddCondition {
                        target: attacker_target,
                        condition: condition.clone(),
                    },
                    BattleCommand::EmitEvent(BattleEvent::StatusApplied {
                        target: pokemon_species,
                        status: condition,
                    }),
                ];
                return EffectResult::Skip(commands);
            }
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Rest effect (heal, clear conditions, apply sleep)
    fn apply_rest_special(
        &self,
        sleep_turns: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
            let pokemon_species = attacker_pokemon.species;
            let max_hp = attacker_pokemon.max_hp();
            let current_hp = attacker_pokemon.current_hp();
            let mut commands = Vec::new();

            // Full heal - restore HP to maximum
            if current_hp < max_hp {
                let heal_amount = max_hp - current_hp;
                commands.push(BattleCommand::HealPokemon {
                    target: attacker_target,
                    amount: heal_amount,
                });
                commands.push(BattleCommand::EmitEvent(BattleEvent::PokemonHealed {
                    target: pokemon_species,
                    amount: heal_amount,
                    new_hp: max_hp,
                }));
            }

            // Apply Sleep status
            commands.push(BattleCommand::SetPokemonStatus {
                target: attacker_target,
                status: Some(crate::pokemon::StatusCondition::Sleep(sleep_turns)),
            });
            commands.push(BattleCommand::EmitEvent(
                BattleEvent::PokemonStatusApplied {
                    target: pokemon_species,
                    status: crate::pokemon::StatusCondition::Sleep(sleep_turns),
                },
            ));

            // Clear all active Pokemon conditions
            for condition in attacker_player.active_pokemon_conditions.values() {
                commands.push(BattleCommand::RemoveCondition {
                    target: attacker_target,
                    condition_type: condition.get_type(),
                });
                commands.push(BattleCommand::EmitEvent(BattleEvent::ConditionExpired {
                    target: pokemon_species,
                    condition: condition.clone(),
                }));
            }

            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply MirrorMove effect (mirrors opponent's last move)
    fn apply_mirror_move_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand};
        use crate::battle::state::{ActionFailureReason, BattleEvent};
        use crate::battle::engine::BattleAction;

        let defender_player = &state.players[context.defender_index];

        if let Some(mirrored_move) = defender_player.last_move {
            // Don't allow mirroring Mirror Move (would cause infinite recursion)
            if mirrored_move == crate::moves::Move::MirrorMove {
                let commands = vec![BattleCommand::EmitEvent(BattleEvent::ActionFailed {
                    reason: ActionFailureReason::MoveFailedToExecute,
                })];
                return EffectResult::Skip(commands);
            }

            // Queue the mirrored move action
            let mirrored_action = BattleAction::AttackHit {
                attacker_index: context.attacker_index,
                defender_index: context.defender_index,
                move_used: mirrored_move,
                hit_number: 1, // Must be greater than zero to avoid trying to use PP
            };

            let commands = vec![BattleCommand::PushAction(mirrored_action)];
            return EffectResult::Skip(commands);
        }

        // If no move to mirror, fail appropriately
        let commands = vec![BattleCommand::EmitEvent(BattleEvent::ActionFailed {
            reason: ActionFailureReason::MoveFailedToExecute,
        })];
        EffectResult::Skip(commands)
    }

    /// Apply Metronome effect (randomly selects and executes a move)
    fn apply_metronome_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand};
        use crate::battle::state::BattleEvent;
        use crate::battle::engine::BattleAction;
        use crate::moves::Move;

        // Get all possible moves except Metronome itself
        let all_moves = [
            // Normal Type
            Move::Pound,
            Move::Doubleslap,
            Move::PayDay,
            Move::Scratch,
            Move::Guillotine,
            Move::SwordsDance,
            Move::Cut,
            Move::Bind,
            Move::Slam,
            Move::Stomp,
            Move::Headbutt,
            Move::HornAttack,
            Move::FuryAttack,
            Move::HornDrill,
            Move::Tackle,
            Move::BodySlam,
            Move::Wrap,
            Move::Harden,
            Move::TakeDown,
            Move::Thrash,
            Move::DoubleEdge,
            Move::TailWhip,
            Move::Leer,
            Move::Bite,
            Move::Growl,
            Move::Roar,
            Move::Sing,
            Move::Supersonic,
            Move::SonicBoom,
            Move::Disable,
            Move::Agility,
            Move::QuickAttack,
            Move::Rage,
            Move::Mimic,
            Move::Screech,
            Move::DoubleTeam,
            Move::Recover,
            Move::Minimize,
            Move::Withdraw,
            Move::DefenseCurl,
            Move::Barrier,
            Move::FocusEnergy,
            Move::Bide,
            Move::MirrorMove,
            Move::SelfDestruct,
            Move::Clamp,
            Move::Swift,
            Move::SpikeCannon,
            Move::Constrict,
            Move::SoftBoiled,
            Move::Glare,
            Move::Transform,
            Move::Explosion,
            Move::FurySwipes,
            Move::Rest,
            Move::HyperFang,
            Move::Sharpen,
            Move::Conversion,
            Move::TriAttack,
            Move::SuperFang,
            Move::Slash,
            Move::Substitute,
            Move::HyperBeam,
            // Fighting Type
            Move::KarateChop,
            Move::CometPunch,
            Move::MegaPunch,
            Move::KOPunch,
            Move::DoubleKick,
            Move::MegaKick,
            Move::JumpKick,
            Move::RollingKick,
            Move::Submission,
            Move::LowKick,
            Move::Counter,
            Move::SeismicToss,
            Move::Strength,
            Move::Meditate,
            Move::HighJumpKick,
            Move::Barrage,
            Move::DizzyPunch,
            // Flying Type
            Move::RazorWind,
            Move::Gust,
            Move::WingAttack,
            Move::Whirlwind,
            Move::Fly,
            Move::Peck,
            Move::DrillPeck,
            Move::SkyAttack,
            // Rock Type
            Move::Vicegrip,
            Move::RockThrow,
            Move::SkullBash,
            Move::RockSlide,
            Move::AncientPower,
            // Ground Type
            Move::SandAttack,
            Move::Earthquake,
            Move::Fissure,
            Move::Dig,
            Move::BoneClub,
            Move::Bonemerang,
            // Poison Type
            Move::PoisonSting,
            Move::Twineedle,
            Move::Acid,
            Move::Toxic,
            Move::Haze,
            Move::Smog,
            Move::Sludge,
            Move::PoisonJab,
            Move::PoisonGas,
            Move::AcidArmor,
            // Bug Type
            Move::PinMissile,
            Move::SilverWind,
            Move::StringShot,
            Move::LeechLife,
            // Fire Type
            Move::FirePunch,
            Move::BlazeKick,
            Move::FireFang,
            Move::Ember,
            Move::Flamethrower,
            Move::WillOWisp,
            Move::FireSpin,
            Move::Smokescreen,
            Move::FireBlast,
            // Water Type
            Move::Mist,
            Move::WaterGun,
            Move::HydroPump,
            Move::Surf,
            Move::Bubblebeam,
            Move::Waterfall,
            Move::Bubble,
            Move::Splash,
            Move::Bubblehammer,
            // Grass Type
            Move::VineWhip,
            Move::Absorb,
            Move::MegaDrain,
            Move::GigaDrain,
            Move::LeechSeed,
            Move::Growth,
            Move::RazorLeaf,
            Move::SolarBeam,
            Move::PoisonPowder,
            Move::StunSpore,
            Move::SleepPowder,
            Move::PetalDance,
            Move::Spore,
            Move::EggBomb,
            // Ice Type
            Move::IcePunch,
            Move::IceBeam,
            Move::Blizzard,
            Move::AuroraBeam,
            // Electric Type
            Move::ThunderPunch,
            Move::Shock,
            Move::Discharge,
            Move::ThunderWave,
            Move::Thunderclap,
            Move::ChargeBeam,
            Move::Lightning,
            Move::Flash,
            // Psychic Type
            Move::Confusion,
            Move::Psybeam,
            Move::Perplex,
            Move::Hypnosis,
            Move::Teleport,
            Move::ConfuseRay,
            Move::LightScreen,
            Move::Reflect,
            Move::Amnesia,
            Move::Kinesis,
            Move::Psywave,
            Move::DreamEater,
            Move::LovelyKiss,
            // Ghost Type
            Move::NightShade,
            Move::Lick,
            Move::ShadowBall,
            // Dragon Type
            Move::Outrage,
            Move::DragonRage,
        ];

        // Randomly select a move
        let random_index =
            (rng.next_outcome("Generate Metronome Move Select") as usize) % all_moves.len();
        let selected_move = all_moves[random_index];

        let attacker_player = &state.players[context.attacker_index];
        if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
            // Log the Metronome selection
            let metronome_action = BattleAction::AttackHit {
                attacker_index: context.attacker_index,
                defender_index: context.defender_index,
                move_used: selected_move,
                hit_number: 1, // Must be greater than zero to avoid trying to use PP
            };

            let commands = vec![
                BattleCommand::EmitEvent(BattleEvent::MoveUsed {
                    player_index: context.attacker_index,
                    pokemon: pokemon_species,
                    move_used: selected_move,
                }),
                BattleCommand::PushAction(metronome_action),
            ];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
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

#[derive(Clone, Serialize, Deserialize)]
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
    #[allow(dead_code)]
    pub fn load_all(
        _data_path: &std::path::Path,
    ) -> Result<HashMap<Move, MoveData>, Box<dyn std::error::Error>> {
        let mut move_map = get_compiled_move_data();

        // Add hardcoded special moves that aren't in RON files
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

        Ok(move_map)
    }

    /// Get move data for a specific move from the compiled data
    pub fn get_move_data(move_: Move) -> Option<MoveData> {
        // Handle special hardcoded moves first
        match move_ {
            Move::HittingItself => {
                Some(MoveData {
                    name: "Hit Itself".to_string(),
                    move_type: PokemonType::Typeless,
                    power: Some(40),
                    category: MoveCategory::Physical,
                    accuracy: None, // Always hits
                    max_pp: 0,      // Not a real move, no PP
                    effects: vec![],
                })
            }
            Move::Struggle => {
                Some(MoveData {
                    name: "Struggle".to_string(),
                    move_type: PokemonType::Typeless,
                    power: Some(50),
                    category: MoveCategory::Physical,
                    accuracy: Some(90),
                    max_pp: 0,                             // Not a real move, no PP
                    effects: vec![MoveEffect::Recoil(25)], // 25% recoil of damage dealt
                })
            }
            _ => {
                // For regular moves, get from compiled data
                get_compiled_move_data().get(&move_).cloned()
            }
        }
    }

    pub fn get_move_max_pp(move_: Move) -> u8 {
        Self::get_move_data(move_).map(|data| data.max_pp).unwrap_or(30) // Default fallback
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
            "KOPUNCH" => Ok(Move::KOPunch),
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
            "SOLARBEAM" => Ok(Move::SolarBeam),
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
