use super::*;
use crate::battle::state::BattleEvent;

impl MoveEffect {
    /// Apply stat change effect
    pub(super) fn apply_stat_change_effect(
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
                        // Event will be automatically emitted by the command system
                    }
                }
            }
        }

        commands
    }

    /// Apply raise all stats effect (targets user)
    pub(super) fn apply_raise_all_stats_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        if rng.next_outcome("Apply Raise All Stats Check") <= chance {
            let attacker_player = &state.players[context.attacker_index];
            if let Some(_attacker_pokemon) = attacker_player.active_pokemon() {
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
                        // Event will be automatically emitted by the command system
                    }
                }
            }
        }

        commands
    }

    /// Apply haze effect (clears all stat stages for both players)
    pub(super) fn apply_haze_effect(
        &self,
        chance: u8,
        _context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        if rng.next_outcome("Apply Haze Check") <= chance {
            // Clear stat stages for both players
            for player_index in 0..2 {
                let player = &state.players[player_index];
                if let Some(_pokemon) = player.active_pokemon() {
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
                            // Event will be automatically emitted by the command system
                        }
                    }
                }
            }
        }

        commands
    }
}
