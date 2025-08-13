use serde::{Deserialize, Serialize};

use crate::{
    battle::commands::{BattleCommand, PlayerTarget},
    battle::state::BattleEvent,
    moves::Move,
    pokemon::{PokemonInst, PokemonType},
    player::StatType,
};
use std::hash::{Hash, Hasher};
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

/// Condition type without data payload for RemoveCondition commands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PokemonConditionType {
    Flinched,
    Confused,
    Seeded,
    Underground,
    InAir,
    Teleported,
    Enraged,
    Exhausted,
    Trapped,
    Charging,
    Rampaging,
    Transformed,
    Converted,
    Biding,
    Countering,
    Substitute,
    Disabled,
}

impl PokemonCondition {
    pub fn get_type(&self) -> PokemonConditionType {
        match self {
            PokemonCondition::Flinched => PokemonConditionType::Flinched,
            PokemonCondition::Confused { .. } => PokemonConditionType::Confused,
            PokemonCondition::Seeded => PokemonConditionType::Seeded,
            PokemonCondition::Underground => PokemonConditionType::Underground,
            PokemonCondition::InAir => PokemonConditionType::InAir,
            PokemonCondition::Teleported => PokemonConditionType::Teleported,
            PokemonCondition::Enraged => PokemonConditionType::Enraged,
            PokemonCondition::Exhausted { .. } => PokemonConditionType::Exhausted,
            PokemonCondition::Trapped { .. } => PokemonConditionType::Trapped,
            PokemonCondition::Charging => PokemonConditionType::Charging,
            PokemonCondition::Rampaging { .. } => PokemonConditionType::Rampaging,
            PokemonCondition::Transformed { .. } => PokemonConditionType::Transformed,
            PokemonCondition::Converted { .. } => PokemonConditionType::Converted,
            PokemonCondition::Biding { .. } => PokemonConditionType::Biding,
            PokemonCondition::Countering { .. } => PokemonConditionType::Countering,
            PokemonCondition::Substitute { .. } => PokemonConditionType::Substitute,
            PokemonCondition::Disabled { .. } => PokemonConditionType::Disabled,
        }
    }

    /// Handle reactions when this condition's pokemon takes damage
    pub fn on_damage_taken(
        &self,
        damage: u16,
        attacker_target: PlayerTarget,
        defender_target: PlayerTarget,
        defender_pokemon_species: crate::species::Species,
        move_category: crate::move_data::MoveCategory,
        defender_current_hp: u16,
        defender_stat_stage: i8,
    ) -> Vec<BattleCommand> {
        let mut commands = Vec::new();

        match self {
            // Counter: Retaliate with 2x physical damage if defender survives
            PokemonCondition::Countering { .. } => {
                let defender_will_faint = damage >= defender_current_hp;
                
                if matches!(move_category, crate::move_data::MoveCategory::Physical) && !defender_will_faint {
                    let counter_damage = damage * 2;
                    commands.push(BattleCommand::DealDamage {
                        target: attacker_target,
                        amount: counter_damage,
                    });
                }
            }
            
            // Bide: Accumulate damage for future release
            PokemonCondition::Biding { turns_remaining, damage: stored_damage } => {
                // Remove old condition
                commands.push(BattleCommand::RemoveCondition {
                    target: defender_target,
                    condition_type: PokemonConditionType::Biding,
                });
                // Add updated condition with accumulated damage
                commands.push(BattleCommand::AddCondition {
                    target: defender_target,
                    condition: PokemonCondition::Biding {
                        turns_remaining: *turns_remaining,
                        damage: stored_damage + damage,
                    },
                });
            }
            
            // Enraged: Increase attack stat when hit
            PokemonCondition::Enraged => {
                let new_stage = (defender_stat_stage + 1).min(6); // Cap at +6
                
                if defender_stat_stage != new_stage {
                    commands.push(BattleCommand::ChangeStatStage {
                        target: defender_target,
                        stat: StatType::Attack,
                        delta: 1,
                    });
                    commands.push(BattleCommand::EmitEvent(BattleEvent::StatStageChanged {
                        target: defender_pokemon_species,
                        stat: StatType::Attack,
                        old_stage: defender_stat_stage,
                        new_stage,
                    }));
                }
            }
            
            // Most conditions don't react to damage
            _ => {}
        }

        commands
    }
}
