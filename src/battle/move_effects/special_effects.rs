// In: src/battle/move_effects/special_effects.rs

// --- IMPORTS ---
use super::{EffectContext, EffectResult};
use crate::battle::action_stack::BattleAction;
use crate::battle::commands::{BattleCommand, PlayerTarget};
use crate::battle::conditions::{PokemonCondition, PokemonConditionType};
use crate::battle::state::{ActionFailureReason, BattleEvent, BattleState, TurnRng};
use crate::pokemon::StatusCondition;
use schema::{Move, TeamCondition};

// --- STANDALONE HELPER FUNCTIONS ---

pub(super) fn apply_in_air_special(context: &EffectContext, state: &BattleState) -> EffectResult {
    let attacker_player = &state.players[context.attacker_index];
    let attacker_target = PlayerTarget::from_index(context.attacker_index);

    if attacker_player.has_condition_type(PokemonConditionType::InAir) {
        let commands = vec![BattleCommand::RemoveCondition {
            target: attacker_target,
            condition_type: PokemonConditionType::InAir,
        }];
        return EffectResult::Continue(commands);
    }

    if attacker_player.active_pokemon().is_some() {
        let commands = vec![BattleCommand::AddCondition {
            target: attacker_target,
            condition: PokemonCondition::InAir,
        }];
        return EffectResult::Skip(commands);
    }

    EffectResult::Continue(Vec::new())
}

pub(super) fn apply_teleport_special(context: &EffectContext, state: &BattleState) -> EffectResult {
    let attacker_player = &state.players[context.attacker_index];
    if attacker_player.active_pokemon().is_some() {
        let commands = vec![BattleCommand::AddCondition {
            target: PlayerTarget::from_index(context.attacker_index),
            condition: PokemonCondition::Teleported,
        }];
        return EffectResult::Skip(commands);
    }
    EffectResult::Continue(Vec::new())
}

pub(super) fn apply_charge_up_special(context: &EffectContext, state: &BattleState) -> EffectResult {
    let attacker_player = &state.players[context.attacker_index];
    let attacker_target = PlayerTarget::from_index(context.attacker_index);

    if attacker_player.has_condition_type(PokemonConditionType::Charging) {
        let commands = vec![BattleCommand::RemoveCondition {
            target: attacker_target,
            condition_type: PokemonConditionType::Charging,
        }];
        return EffectResult::Continue(commands);
    }

    if attacker_player.active_pokemon().is_some() {
        let commands = vec![BattleCommand::AddCondition {
            target: attacker_target,
            condition: PokemonCondition::Charging,
        }];
        return EffectResult::Skip(commands);
    }
    EffectResult::Continue(Vec::new())
}

pub(super) fn apply_underground_special(context: &EffectContext, state: &BattleState) -> EffectResult {
    let attacker_player = &state.players[context.attacker_index];
    let attacker_target = PlayerTarget::from_index(context.attacker_index);

    if attacker_player.has_condition_type(PokemonConditionType::Underground) {
        let commands = vec![BattleCommand::RemoveCondition {
            target: attacker_target,
            condition_type: PokemonConditionType::Underground,
        }];
        return EffectResult::Continue(commands);
    }

    if attacker_player.active_pokemon().is_some() {
        let commands = vec![BattleCommand::AddCondition {
            target: attacker_target,
            condition: PokemonCondition::Underground,
        }];
        return EffectResult::Skip(commands);
    }
    EffectResult::Continue(Vec::new())
}

pub(super) fn apply_transform_special(context: &EffectContext, state: &BattleState) -> EffectResult {
    let attacker_player = &state.players[context.attacker_index];
    let defender_player = &state.players[context.defender_index];

    if let (Some(_), Some(target_pokemon)) = (attacker_player.active_pokemon(), defender_player.active_pokemon().cloned()) {
        let commands = vec![BattleCommand::AddCondition {
            target: PlayerTarget::from_index(context.attacker_index),
            condition: PokemonCondition::Transformed { target: target_pokemon },
        }];
        return EffectResult::Skip(commands);
    }
    EffectResult::Continue(Vec::new())
}

pub(super) fn apply_conversion_special(context: &EffectContext, state: &BattleState) -> EffectResult {
    let attacker_player = &state.players[context.attacker_index];
    let defender_player = &state.players[context.defender_index];

    if let (Some(_), Some(target_type)) = (
        attacker_player.active_pokemon(),
        defender_player.active_pokemon().and_then(|p| p.get_current_types(defender_player).into_iter().next()),
    ) {
        let commands = vec![BattleCommand::AddCondition {
            target: PlayerTarget::from_index(context.attacker_index),
            condition: PokemonCondition::Converted { pokemon_type: target_type },
        }];
        return EffectResult::Skip(commands);
    }
    EffectResult::Continue(Vec::new())
}

pub(super) fn apply_substitute_special(context: &EffectContext, state: &BattleState) -> EffectResult {
    if let Some(attacker_pokemon) = state.players[context.attacker_index].active_pokemon() {
        let substitute_hp = (attacker_pokemon.max_hp() / 4).max(1) as u8;
        let commands = vec![BattleCommand::AddCondition {
            target: PlayerTarget::from_index(context.attacker_index),
            condition: PokemonCondition::Substitute { hp: substitute_hp },
        }];
        return EffectResult::Skip(commands);
    }
    EffectResult::Continue(Vec::new())
}

pub(super) fn apply_counter_special(context: &EffectContext, state: &BattleState) -> EffectResult {
    if state.players[context.attacker_index].active_pokemon().is_some() {
        let commands = vec![BattleCommand::AddCondition {
            target: PlayerTarget::from_index(context.attacker_index),
            condition: PokemonCondition::Countering { damage: 0 },
        }];
        return EffectResult::Skip(commands);
    }
    EffectResult::Continue(Vec::new())
}

pub(super) fn apply_rampage_special(context: &EffectContext, state: &BattleState, rng: &mut TurnRng) -> EffectResult {
    let attacker_player = &state.players[context.attacker_index];
    if attacker_player.active_pokemon().is_none() {
        return EffectResult::Continue(Vec::new());
    }

    if let Some(PokemonCondition::Rampaging { turns_remaining }) =
        attacker_player.active_pokemon_conditions.values().find(|c| matches!(c, PokemonCondition::Rampaging { .. }))
    {
        if *turns_remaining > 0 {
            return EffectResult::Continue(Vec::new());
        } else {
            let commands = vec![BattleCommand::AddCondition {
                target: PlayerTarget::from_index(context.attacker_index),
                condition: PokemonCondition::Confused { turns_remaining: 2 },
            }];
            return EffectResult::Continue(commands);
        }
    }

    let turns = if rng.next_outcome("Generate Rampage Duration") <= 50 { 1 } else { 2 };
    let commands = vec![BattleCommand::AddCondition {
        target: PlayerTarget::from_index(context.attacker_index),
        condition: PokemonCondition::Rampaging { turns_remaining: turns },
    }];
    EffectResult::Continue(commands)
}

pub(super) fn apply_rage_special(context: &EffectContext, state: &BattleState) -> EffectResult {
    if state.players[context.attacker_index].active_pokemon().is_some() {
        let commands = vec![BattleCommand::AddCondition {
            target: PlayerTarget::from_index(context.attacker_index),
            condition: PokemonCondition::Enraged,
        }];
        return EffectResult::Continue(commands);
    }
    EffectResult::Continue(Vec::new())
}

pub(super) fn apply_explode_special(context: &EffectContext, state: &BattleState) -> EffectResult {
    if let Some(attacker_pokemon) = state.players[context.attacker_index].active_pokemon() {
        let commands = vec![BattleCommand::DealDamage {
            target: PlayerTarget::from_index(context.attacker_index),
            amount: attacker_pokemon.current_hp(),
        }];
        return EffectResult::Continue(commands);
    }
    EffectResult::Continue(Vec::new())
}

pub(super) fn apply_bide_special(turns: u8, context: &EffectContext, state: &BattleState) -> EffectResult {
    let attacker_player = &state.players[context.attacker_index];
    
    if let Some((turns_remaining, stored_damage)) = attacker_player.active_pokemon_conditions.values().find_map(|c| match c {
        PokemonCondition::Biding { turns_remaining, damage } => Some((*turns_remaining, *damage)),
        _ => None,
    }) {
        if turns_remaining < 1 {
            let damage_to_deal = (stored_damage * 2).max(1);
            let commands = vec![BattleCommand::DealDamage {
                target: PlayerTarget::from_index(context.defender_index),
                amount: damage_to_deal,
            }];
            return EffectResult::Skip(commands);
        } else {
            return EffectResult::Skip(Vec::new());
        }
    } else {
        if attacker_player.active_pokemon().is_some() {
            let commands = vec![BattleCommand::AddCondition {
                target: PlayerTarget::from_index(context.attacker_index),
                condition: PokemonCondition::Biding { turns_remaining: turns, damage: 0 },
            }];
            return EffectResult::Skip(commands);
        }
    }
    EffectResult::Continue(Vec::new())
}

pub(super) fn apply_rest_special(sleep_turns: u8, context: &EffectContext, state: &BattleState) -> EffectResult {
    if let Some(attacker_pokemon) = state.players[context.attacker_index].active_pokemon() {
        let mut commands = Vec::new();
        if attacker_pokemon.current_hp() < attacker_pokemon.max_hp() {
            commands.push(BattleCommand::HealPokemon {
                target: PlayerTarget::from_index(context.attacker_index),
                amount: attacker_pokemon.max_hp(),
            });
        }
        if let Some(existing_status) = attacker_pokemon.status {
            commands.push(BattleCommand::CurePokemonStatus {
                target: PlayerTarget::from_index(context.attacker_index),
                status: existing_status,
            });
        }
        commands.push(BattleCommand::SetPokemonStatus {
            target: PlayerTarget::from_index(context.attacker_index),
            status: StatusCondition::Sleep(sleep_turns),
        });
        return EffectResult::Skip(commands);
    }
    EffectResult::Continue(Vec::new())
}

pub(super) fn apply_mirror_move_special(context: &EffectContext, state: &BattleState) -> EffectResult {
    let defender_player = &state.players[context.defender_index];
    if let Some(mirrored_move) = defender_player.last_move {
        if mirrored_move == Move::MirrorMove {
            return EffectResult::Skip(vec![BattleCommand::EmitEvent(BattleEvent::ActionFailed {
                reason: ActionFailureReason::MoveFailedToExecute { move_used: Move::MirrorMove },
            })]);
        }

        if let Some(attacker_pokemon) = state.players[context.attacker_index].active_pokemon() {
            let mirrored_action = BattleAction::AttackHit {
                attacker_index: context.attacker_index,
                defender_index: context.defender_index,
                move_used: mirrored_move,
                hit_number: 1, // Must be >0 to avoid using PP
            };
            return EffectResult::Skip(vec![
                BattleCommand::EmitEvent(BattleEvent::MoveUsed {
                    player_index: context.attacker_index,
                    pokemon: attacker_pokemon.species,
                    move_used: mirrored_move,
                }),
                BattleCommand::PushAction(mirrored_action),
            ]);
        }
    }
    EffectResult::Skip(vec![BattleCommand::EmitEvent(BattleEvent::ActionFailed {
        reason: ActionFailureReason::MoveFailedToExecute { move_used: Move::MirrorMove },
    })])
}

pub(super) fn apply_metronome_special(context: &EffectContext, state: &BattleState, rng: &mut TurnRng) -> EffectResult {
    // This is a simplified list. A full implementation would need a comprehensive, static list of all possible moves.
    let all_moves: &[Move] = &[Move::Tackle, Move::Ember, Move::WaterGun, Move::VineWhip, Move::ThunderPunch];
    let random_index = (rng.next_outcome("Generate Metronome Move Select") as usize) % all_moves.len();
    let selected_move = all_moves[random_index];

    if let Some(attacker_pokemon) = state.players[context.attacker_index].active_pokemon() {
        let metronome_action = BattleAction::AttackHit {
            attacker_index: context.attacker_index,
            defender_index: context.defender_index,
            move_used: selected_move,
            hit_number: 1,
        };
        let commands = vec![
            BattleCommand::EmitEvent(BattleEvent::MoveUsed {
                player_index: context.attacker_index,
                pokemon: attacker_pokemon.species,
                move_used: selected_move,
            }),
            BattleCommand::PushAction(metronome_action),
        ];
        return EffectResult::Skip(commands);
    }
    EffectResult::Continue(Vec::new())
}

pub(super) fn apply_team_condition_effect(
    condition: &TeamCondition,
    turns: u8,
    context: &EffectContext,
) -> Vec<BattleCommand> {
    vec![BattleCommand::AddTeamCondition {
        target: PlayerTarget::from_index(context.attacker_index),
        condition: *condition,
        turns,
    }]
}