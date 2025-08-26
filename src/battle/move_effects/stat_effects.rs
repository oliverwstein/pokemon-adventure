// In: src/battle/move_effects/stat_effects.rs

// --- IMPORTS ---
// Use `super` to get the context types from the parent `mod.rs` file.
use super::EffectContext;
use crate::battle::commands::{BattleCommand, PlayerTarget};
use crate::battle::state::{BattleEvent, BattleState, TurnRng};
use schema::{StatType, Target, TeamCondition};

// --- STANDALONE HELPER FUNCTIONS ---
// These functions are `pub(super)` to be visible only to the parent `mod.rs`.

/// Apply stat change effect.
pub(super) fn apply_stat_change_effect(
    target: &Target,
    stat: &StatType,
    stages: i8,
    chance: u8,
    context: &EffectContext,
    state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    let rng_reason = format!("Apply {:?} {:?} Effect: ", stat, stages);
    if rng.next_outcome(&rng_reason) > chance {
        return commands;
    }

    let target_index = context.target_index(target);
    let target_player = &state.players[target_index];

    if let Some(target_pokemon) = target_player.active_pokemon() {
        // Check if Mist prevents this stat change
        let is_enemy_move = target_index != context.attacker_index;
        let is_negative_change = stages < 0;
        let has_mist = target_player.has_team_condition(&TeamCondition::Mist);

        if is_enemy_move && is_negative_change && has_mist {
            // Mist prevents the stat change
            commands.push(BattleCommand::EmitEvent(BattleEvent::StatChangeBlocked {
                target: target_pokemon.species,
                stat: *stat,
                reason: "Mist prevented stat reduction".to_string(),
            }));
        } else {
            let old_stage = target_player.get_stat_stage(*stat);
            let new_stage = (old_stage + stages).clamp(-6, 6);

            if old_stage != new_stage {
                commands.push(BattleCommand::ChangeStatStage {
                    target: PlayerTarget::from_index(target_index),
                    stat: *stat,
                    delta: new_stage - old_stage,
                });
            }
        }
    }
    commands
}

/// Apply raise all stats effect (targets user).
pub(super) fn apply_raise_all_stats_effect(
    chance: u8,
    context: &EffectContext,
    state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    if rng.next_outcome("Apply Raise All Stats Check") > chance {
        return commands;
    }

    let attacker_player = &state.players[context.attacker_index];
    if attacker_player.active_pokemon().is_some() {
        let stats_to_raise = [
            StatType::Atk,
            StatType::Def,
            StatType::SpAtk,
            StatType::SpDef,
            StatType::Spe,
        ];

        for stat in &stats_to_raise {
            let old_stage = attacker_player.get_stat_stage(*stat);
            if old_stage < 6 { // Only raise if not already maxed out
                commands.push(BattleCommand::ChangeStatStage {
                    target: PlayerTarget::from_index(context.attacker_index),
                    stat: *stat,
                    delta: 1,
                });
            }
        }
    }
    commands
}

/// Apply haze effect (clears all stat stages for both players).
pub(super) fn apply_haze_effect(
    chance: u8,
    _context: &EffectContext,
    state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    if rng.next_outcome("Apply Haze Check") > chance {
        return commands;
    }

    // Clear stat stages for both players
    for player_index in 0..2 {
        let player = &state.players[player_index];
        if player.active_pokemon().is_some() {
            let all_stats = [
                StatType::Atk, StatType::Def, StatType::SpAtk, StatType::SpDef,
                StatType::Spe, StatType::Acc, StatType::Eva, StatType::Crit,
            ];

            for stat in &all_stats {
                let current_stage = player.get_stat_stage(*stat);
                if current_stage != 0 {
                    commands.push(BattleCommand::ChangeStatStage {
                        target: PlayerTarget::from_index(player_index),
                        stat: *stat,
                        delta: -current_stage, // Reset to 0
                    });
                }
            }
        }
    }
    commands
}