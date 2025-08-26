// In: src/battle/move_effects/damage_effects.rs

// --- IMPORTS ---
// Use `super` to get the context types from the parent `mod.rs` file.
use super::EffectContext;
use crate::battle::commands::{BattleCommand, PlayerTarget};
use crate::battle::state::{BattleState, TurnRng};


// These functions are `pub(super)` to be visible only to the parent `mod.rs`.

/// Apply heal effect (targets user).
pub(super) fn apply_heal_effect(
    percentage: u8,
    context: &EffectContext,
    state: &BattleState,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    let attacker_player = &state.players[context.attacker_index];

    if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
        let max_hp = attacker_pokemon.max_hp();
        let current_hp = attacker_pokemon.current_hp();

        // Don't heal if already at full HP or fainted.
        if current_hp > 0 && current_hp < max_hp {
            let heal_amount = (max_hp as u32 * percentage as u32 / 100) as u16;
            if heal_amount > 0 {
                commands.push(BattleCommand::HealPokemon {
                    target: PlayerTarget::from_index(context.attacker_index),
                    amount: heal_amount,
                });
            }
        }
    }
    commands
}

/// Apply ante effect (Pay Day).
pub(super) fn apply_ante_effect(
    chance: u8,
    context: &EffectContext,
    state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    if rng.next_outcome("Apply Ante Check") > chance {
        return commands;
    }

    let attacker_player = &state.players[context.attacker_index];
    if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
        let pokemon_level = attacker_pokemon.level as u32;
        let ante_amount = pokemon_level * 2;
        
        commands.push(BattleCommand::AddAnte {
            target: PlayerTarget::from_index(context.attacker_index),
            amount: ante_amount,
        });
    }
    commands
}

/// Apply recoil effect (attacker takes damage based on damage dealt).
pub(super) fn apply_recoil_effect(
    percentage: u8,
    context: &EffectContext,
    damage_dealt: u16,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    let recoil_damage = (damage_dealt as u32 * percentage as u32 / 100) as u16;

    if recoil_damage > 0 {
        commands.push(BattleCommand::DealDamage {
            target: PlayerTarget::from_index(context.attacker_index),
            amount: recoil_damage,
        });
    }
    commands
}

/// Apply drain effect (attacker heals based on damage dealt).
pub(super) fn apply_drain_effect(
    percentage: u8,
    context: &EffectContext,
    state: &BattleState,
    damage_dealt: u16,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    let heal_amount = (damage_dealt as u32 * percentage as u32 / 100) as u16;

    if heal_amount > 0 {
        let attacker_player = &state.players[context.attacker_index];
        if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
            if !attacker_pokemon.is_fainted() && attacker_pokemon.current_hp() < attacker_pokemon.max_hp() {
                commands.push(BattleCommand::HealPokemon {
                    target: PlayerTarget::from_index(context.attacker_index),
                    amount: heal_amount,
                });
            }
        }
    }
    commands
}

/// Apply reckless effect (attacker takes damage based on max HP when move misses).
pub(super) fn apply_reckless_effect(
    percentage: u8,
    context: &EffectContext,
    state: &BattleState,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    let attacker_player = &state.players[context.attacker_index];
    if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
        let max_hp = attacker_pokemon.max_hp();
        let recoil_damage = (max_hp as u32 * percentage as u32 / 100) as u16;

        if recoil_damage > 0 {
            commands.push(BattleCommand::DealDamage {
                target: PlayerTarget::from_index(context.attacker_index),
                amount: recoil_damage,
            });
        }
    }
    commands
}