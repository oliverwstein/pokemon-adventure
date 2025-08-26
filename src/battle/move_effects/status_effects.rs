// In: src/battle/move_effects/status_effects.rs

// --- IMPORTS ---
use super::EffectContext;
use crate::battle::commands::{BattleCommand, PlayerTarget};
use crate::battle::conditions::PokemonCondition;
use crate::battle::state::{BattleState, TurnRng};
use crate::pokemon::{PokemonType, StatusCondition};
use schema::{StatusType, Target};

// --- STANDALONE HELPER FUNCTIONS ---

pub(super) fn apply_burn_effect(
    chance: u8,
    context: &EffectContext,
    state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    let target_player = &state.players[context.defender_index];

    if let Some(target_pokemon) = target_player.active_pokemon() {
        if target_pokemon.status.is_some() || target_pokemon.get_current_types(target_player).contains(&PokemonType::Fire) {
            return commands;
        }

        if rng.next_outcome("Apply Burn Check") <= chance {
            commands.push(BattleCommand::SetPokemonStatus {
                target: PlayerTarget::from_index(context.defender_index),
                status: StatusCondition::Burn,
            });
        }
    }
    commands
}

pub(super) fn apply_paralyze_effect(
    chance: u8,
    context: &EffectContext,
    state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    let target_player = &state.players[context.defender_index];

    if let Some(target_pokemon) = target_player.active_pokemon() {
        if target_pokemon.status.is_some() || target_pokemon.get_current_types(target_player).contains(&PokemonType::Electric) {
            return commands;
        }

        if rng.next_outcome("Apply Paralysis Check") <= chance {
            commands.push(BattleCommand::SetPokemonStatus {
                target: PlayerTarget::from_index(context.defender_index),
                status: StatusCondition::Paralysis,
            });
        }
    }
    commands
}

pub(super) fn apply_freeze_effect(
    chance: u8,
    context: &EffectContext,
    state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    let target_player = &state.players[context.defender_index];

    if let Some(target_pokemon) = target_player.active_pokemon() {
        if target_pokemon.status.is_some() || target_pokemon.get_current_types(target_player).contains(&PokemonType::Ice) {
            return commands;
        }

        if rng.next_outcome("Apply Freeze Check") <= chance {
            commands.push(BattleCommand::SetPokemonStatus {
                target: PlayerTarget::from_index(context.defender_index),
                status: StatusCondition::Freeze,
            });
        }
    }
    commands
}

pub(super) fn apply_poison_effect(
    chance: u8,
    context: &EffectContext,
    state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    let target_player = &state.players[context.defender_index];

    if let Some(target_pokemon) = target_player.active_pokemon() {
        if target_pokemon.status.is_some() || target_pokemon.get_current_types(target_player).contains(&PokemonType::Poison) {
            return commands;
        }

        if rng.next_outcome("Apply Poison Check") <= chance {
            commands.push(BattleCommand::SetPokemonStatus {
                target: PlayerTarget::from_index(context.defender_index),
                status: StatusCondition::Poison(0),
            });
        }
    }
    commands
}

pub(super) fn apply_sedate_effect(
    chance: u8,
    context: &EffectContext,
    state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    let target_player = &state.players[context.defender_index];

    if let Some(target_pokemon) = target_player.active_pokemon() {
        // In Gen 1, no types are immune to sleep.
        if target_pokemon.status.is_some() {
            return commands;
        }

        if rng.next_outcome("Apply Sedate Check") <= chance {
            let sleep_turns = (rng.next_outcome("Generate Sleep Duration") % 3) + 1;
            commands.push(BattleCommand::SetPokemonStatus {
                target: PlayerTarget::from_index(context.defender_index),
                status: StatusCondition::Sleep(sleep_turns),
            });
        }
    }
    commands
}

pub(super) fn apply_flinch_effect(
    chance: u8,
    context: &EffectContext,
    state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    if rng.next_outcome("Apply Flinch Effect") > chance {
        return commands;
    }
    if state.players[context.defender_index].active_pokemon().is_some() {
        commands.push(BattleCommand::AddCondition {
            target: PlayerTarget::from_index(context.defender_index),
            condition: PokemonCondition::Flinched,
        });
    }
    commands
}

pub(super) fn apply_confuse_effect(
    chance: u8,
    context: &EffectContext,
    state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    let target_player = &state.players[context.defender_index];
    if target_player.active_pokemon().is_none() {
        return commands;
    }

    if rng.next_outcome("Apply Confuse Effect") <= chance {
        let confuse_turns = (rng.next_outcome("Generate Confusion Duration") % 4) + 1;
        commands.push(BattleCommand::AddCondition {
            target: PlayerTarget::from_index(context.defender_index),
            condition: PokemonCondition::Confused {
                turns_remaining: confuse_turns,
            },
        });
    }
    commands
}

pub(super) fn apply_trap_effect(
    chance: u8,
    context: &EffectContext,
    state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    if state.players[context.defender_index].active_pokemon().is_none() {
        return commands;
    }

    if rng.next_outcome("Apply Trap Check") <= chance {
        let trap_turns = (rng.next_outcome("Generate Trap Duration") % 4) + 2;
        commands.push(BattleCommand::AddCondition {
            target: PlayerTarget::from_index(context.defender_index),
            condition: PokemonCondition::Trapped {
                turns_remaining: trap_turns,
            },
        });
    }
    commands
}

pub(super) fn apply_seed_effect(
    chance: u8,
    context: &EffectContext,
    state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    if state.players[context.defender_index].active_pokemon().is_none() {
        return commands;
    }

    if rng.next_outcome("Apply Seeded Effect") <= chance {
        commands.push(BattleCommand::AddCondition {
            target: PlayerTarget::from_index(context.defender_index),
            condition: PokemonCondition::Seeded,
        });
    }
    commands
}

pub(super) fn apply_exhaust_effect(
    chance: u8,
    context: &EffectContext,
    state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    if state.players[context.attacker_index].active_pokemon().is_none() {
        return commands;
    }

    if rng.next_outcome("Apply Exhaust Check") <= chance {
        commands.push(BattleCommand::AddCondition {
            target: PlayerTarget::from_index(context.attacker_index),
            condition: PokemonCondition::Exhausted {
                turns_remaining: 2,
            },
        });
    }
    commands
}

pub(super) fn apply_cure_status_effect(
    target: &Target,
    status_type: &StatusType,
    context: &EffectContext,
    state: &BattleState,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    let target_index = context.target_index(target);
    let target_player = &state.players[target_index];

    if let Some(target_pokemon) = target_player.active_pokemon() {
        let status_to_cure = match (&target_pokemon.status, status_type) {
            (Some(status @ StatusCondition::Sleep(_)), StatusType::Sleep) => Some(*status),
            (Some(status @ StatusCondition::Poison(_)), StatusType::Poison) => Some(*status),
            (Some(status @ StatusCondition::Burn), StatusType::Burn) => Some(*status),
            (Some(status @ StatusCondition::Freeze), StatusType::Freeze) => Some(*status),
            (Some(status @ StatusCondition::Paralysis), StatusType::Paralysis) => Some(*status),
            _ => None,
        };

        if let Some(status) = status_to_cure {
            commands.push(BattleCommand::CurePokemonStatus {
                target: PlayerTarget::from_index(target_index),
                status,
            });
        }
    }
    commands
}