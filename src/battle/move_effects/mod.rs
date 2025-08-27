// In: src/battle/move_effects/mod.rs

// --- 1. DECLARE HELPER MODULES ---
// This tells Rust to look for and include the logic from these files.
mod damage_effects;
mod special_effects;
mod stat_effects;
mod status_effects;

// --- 2. IMPORTS ---
use crate::battle::action_stack::BattleAction;
use crate::battle::commands::BattleCommand;
use crate::battle::conditions::PokemonCondition;
use crate::battle::state::{BattleState, TurnRng};
use schema::{Move, MoveEffect, Target};
// Bring the standalone helper functions from our private modules into scope.
use self::{damage_effects::*, special_effects::*, stat_effects::*, status_effects::*};

// --- 3. BATTLE-SPECIFIC DATA STRUCTURES ---
// These are defined here as they are the "public" types for this module.
#[derive(Debug, Clone)]
pub struct EffectContext {
    pub attacker_index: usize,
    pub defender_index: usize,
    pub move_used: Move,
}

impl EffectContext {
    pub fn new(attacker_index: usize, defender_index: usize, move_used: Move) -> Self {
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

#[derive(Debug, Clone)]
pub enum EffectResult {
    Continue(Vec<BattleCommand>),
    Skip(Vec<BattleCommand>),
    Ensured(Vec<BattleCommand>), // Always happens regardless of hit/miss/immunity
}

// --- 4. THE PUBLIC EXTENSION TRAIT (AND BATTLEDATAEXT) ---
// These are the public contracts for the rest of the battle engine to use.

pub trait BattleMoveEffectExt {
    fn apply_multi_hit_continuation(
        &self,
        context: &EffectContext,
        rng: &mut TurnRng,
        hit_number: u8,
    ) -> Option<BattleCommand>;
    fn apply(
        &self,
        context: &EffectContext,
        state: &BattleState,
        rng: &mut TurnRng,
    ) -> EffectResult;
}

// Minimal change: Also include the BattleMoveDataExt trait here for cohesion.
use schema::MoveData;
pub trait BattleMoveDataExt {
    fn apply_damage_based_effects(
        &self,
        context: &EffectContext,
        state: &BattleState,
        damage_dealt: u16,
    ) -> Vec<BattleCommand>;
    fn apply_miss_based_effects(
        &self,
        context: &EffectContext,
        state: &BattleState,
    ) -> Vec<BattleCommand>;
}

// --- 5. THE LEAN IMPLEMENTATIONS ---
// These blocks satisfy the compiler but delegate all the work.

impl BattleMoveEffectExt for MoveEffect {
    fn apply(
        &self,
        context: &EffectContext,
        state: &BattleState,
        rng: &mut TurnRng,
    ) -> EffectResult {
        let defender_has_substitute = state.players[context.defender_index]
            .active_pokemon_conditions
            .values()
            .any(|condition| matches!(condition, PokemonCondition::Substitute { .. }));

        // NOTE: The call is now to a standalone function `is_blocked_by_substitute(self)`.
        if defender_has_substitute && is_blocked_by_substitute(self) {
            return EffectResult::Continue(Vec::new());
        }

        // NOTE: Each of these is now a call to a standalone helper function.
        match self {
            Self::InAir => apply_in_air_special(context, state),
            Self::Teleport(_) => apply_teleport_special(context, state),
            Self::ChargeUp => apply_charge_up_special(context, state),
            Self::Underground => apply_underground_special(context, state),
            Self::Transform => apply_transform_special(context, state),
            Self::Conversion => apply_conversion_special(context, state),
            Self::Substitute => apply_substitute_special(context, state),
            Self::Counter => apply_counter_special(context, state),
            Self::Bide(turns) => apply_bide_special(*turns, context, state),
            Self::MirrorMove => apply_mirror_move_special(context, state),
            Self::Rest(sleep_turns) => apply_rest_special(*sleep_turns, context, state),
            Self::Metronome => apply_metronome_special(context, state, rng),
            Self::Rampage => apply_rampage_special(context, state, rng),
            Self::Rage(_) => apply_rage_special(context, state),
            Self::Explode => apply_explode_special(context, state),
            Self::Burn(chance) => {
                EffectResult::Continue(apply_burn_effect(*chance, context, state, rng))
            }
            Self::Paralyze(chance) => {
                EffectResult::Continue(apply_paralyze_effect(*chance, context, state, rng))
            }
            Self::Freeze(chance) => {
                EffectResult::Continue(apply_freeze_effect(*chance, context, state, rng))
            }
            Self::Poison(chance) => {
                EffectResult::Continue(apply_poison_effect(*chance, context, state, rng))
            }
            Self::Sedate(chance) => {
                EffectResult::Continue(apply_sedate_effect(*chance, context, state, rng))
            }
            Self::Flinch(chance) => {
                EffectResult::Continue(apply_flinch_effect(*chance, context, state, rng))
            }
            Self::Confuse(chance) => {
                EffectResult::Continue(apply_confuse_effect(*chance, context, state, rng))
            }
            Self::Trap(chance) => {
                EffectResult::Continue(apply_trap_effect(*chance, context, state, rng))
            }
            Self::Seed(chance) => {
                EffectResult::Continue(apply_seed_effect(*chance, context, state, rng))
            }
            Self::Exhaust(chance) => {
                EffectResult::Continue(apply_exhaust_effect(*chance, context, state, rng))
            }
            Self::StatChange(target, stat, stages, chance) => EffectResult::Continue(
                apply_stat_change_effect(target, stat, *stages, *chance, context, state, rng),
            ),
            Self::RaiseAllStats(chance) => {
                EffectResult::Continue(apply_raise_all_stats_effect(*chance, context, state, rng))
            }
            Self::Heal(percentage) => {
                EffectResult::Continue(apply_heal_effect(*percentage, context, state))
            }
            Self::Haze(chance) => {
                EffectResult::Continue(apply_haze_effect(*chance, context, state, rng))
            }
            Self::CureStatus(target, status_type) => EffectResult::Continue(
                apply_cure_status_effect(target, status_type, context, state),
            ),
            Self::SetTeamCondition(condition, turns) => {
                EffectResult::Continue(apply_team_condition_effect(condition, *turns, context))
            }
            Self::Ante(chance) => {
                EffectResult::Continue(apply_ante_effect(*chance, context, state, rng))
            }
            _ => EffectResult::Continue(Vec::new()),
        }
    }

    fn apply_multi_hit_continuation(
        &self,
        context: &EffectContext,
        rng: &mut TurnRng,
        hit_number: u8,
    ) -> Option<BattleCommand> {
        // This is the exact logic from your original file, pasted here.
        if let MoveEffect::MultiHit(guaranteed_hits, continuation_chance) = self {
            let next_hit_number = hit_number + 1;

            let should_queue_next_hit = if next_hit_number < *guaranteed_hits {
                true
            } else {
                next_hit_number <= 7
                    && rng.next_outcome("Multi-Hit Continuation Check") <= *continuation_chance
            };

            if should_queue_next_hit {
                return Some(BattleCommand::PushAction(BattleAction::AttackHit {
                    attacker_index: context.attacker_index,
                    defender_index: context.defender_index,
                    move_used: context.move_used,
                    hit_number: next_hit_number,
                }));
            }
        }
        None
    }
}

// This logic lives here now as a standalone helper function.
fn is_blocked_by_substitute(effect: &MoveEffect) -> bool {
    match effect {
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
        | MoveEffect::Metronome
        | MoveEffect::Recoil(_)
        | MoveEffect::Drain(_)
        | MoveEffect::Crit(_)
        | MoveEffect::IgnoreDef(_)
        | MoveEffect::Priority(_)
        | MoveEffect::MultiHit(_, _)
        | MoveEffect::Haze(_)
        | MoveEffect::SetTeamCondition(..) => false,
        MoveEffect::StatChange(target, ..) => matches!(target, Target::Target),
        MoveEffect::CureStatus(target, ..) => matches!(target, Target::Target),
        _ => true,
    }
}

// The implementation for MoveDataExt also lives here.
impl BattleMoveDataExt for MoveData {
    fn apply_damage_based_effects(
        &self,
        context: &EffectContext,
        state: &BattleState,
        damage_dealt: u16,
    ) -> Vec<BattleCommand> {
        let mut all_commands = Vec::new();
        for effect in &self.effects {
            if damage_dealt == 0 {
                continue;
            }
            match effect {
                MoveEffect::Recoil(percentage) => {
                    all_commands.extend(apply_recoil_effect(*percentage, context, damage_dealt));
                }
                MoveEffect::Drain(percentage) => {
                    all_commands.extend(apply_drain_effect(
                        *percentage,
                        context,
                        state,
                        damage_dealt,
                    ));
                }
                _ => {}
            }
        }
        all_commands
    }

    fn apply_miss_based_effects(
        &self,
        context: &EffectContext,
        state: &BattleState,
    ) -> Vec<BattleCommand> {
        let mut all_commands = Vec::new();
        for effect in &self.effects {
            match effect {
                MoveEffect::Reckless(percentage) => {
                    all_commands.extend(apply_reckless_effect(*percentage, context, state));
                }
                _ => {}
            }
        }
        all_commands
    }
}
