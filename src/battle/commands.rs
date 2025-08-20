use crate::battle::action_stack::{ActionStack, BattleAction};
use crate::battle::conditions::{PokemonCondition, PokemonConditionType};
use crate::battle::state::{BattleEvent, BattleState, EventBus};
use crate::moves::Move;
use crate::player::{PlayerAction, StatType, TeamCondition};
use crate::pokemon::StatusCondition;

/// Player target for commands - provides type safety over raw indices
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerTarget {
    Player1,
    Player2,
}

impl PlayerTarget {
    pub fn to_index(self) -> usize {
        match self {
            PlayerTarget::Player1 => 0,
            PlayerTarget::Player2 => 1,
        }
    }

    #[allow(dead_code)]
    pub fn opponent(self) -> PlayerTarget {
        match self {
            PlayerTarget::Player1 => PlayerTarget::Player2,
            PlayerTarget::Player2 => PlayerTarget::Player1,
        }
    }

    pub fn from_index(index: usize) -> PlayerTarget {
        match index {
            0 => PlayerTarget::Player1,
            1 => PlayerTarget::Player2,
            _ => panic!("Invalid player index: {}", index),
        }
    }
}

/// Atomic commands representing final state changes
#[derive(Debug, Clone)]
pub enum BattleCommand {
    // Direct state changes
    SetGameState(crate::battle::state::GameState),
    IncrementTurnNumber,
    ClearActionQueue,

    // Pokemon modifications
    DealDamage {
        target: PlayerTarget,
        amount: u16,
    },
    HealPokemon {
        target: PlayerTarget,
        amount: u16,
    },
    SetPokemonStatus {
        target: PlayerTarget,
        status: StatusCondition,
    },
    CurePokemonStatus {
        target: PlayerTarget,
        status: StatusCondition,
    },
    UsePP {
        target: PlayerTarget,
        move_used: Move,
    },
    // Player state changes
    ChangeStatStage {
        target: PlayerTarget,
        stat: StatType,
        delta: i8,
    },
    AddCondition {
        target: PlayerTarget,
        condition: PokemonCondition,
    },
    RemoveCondition {
        target: PlayerTarget,
        condition_type: PokemonConditionType,
    },
    RemoveSpecificCondition {
        target: PlayerTarget,
        condition: PokemonCondition,
    },
    AddTeamCondition {
        target: PlayerTarget,
        condition: TeamCondition,
        turns: u8,
    },
    AddAnte {
        target: PlayerTarget,
        amount: u32,
    },
    SetLastMove {
        target: PlayerTarget,
        move_used: Move,
    },
    SwitchPokemon {
        target: PlayerTarget,
        new_pokemon_index: usize,
    },
    ClearPlayerState {
        target: PlayerTarget,
    },

    // Pokemon status progress and condition effects
    DealStatusDamage {
        target: PlayerTarget,
        status: StatusCondition,
        amount: u16,
    },
    DealConditionDamage {
        target: PlayerTarget,
        condition: PokemonCondition,
        amount: u16,
    },
    UpdateStatusProgress {
        target: PlayerTarget,
    },
    TickPokemonCondition {
        target: PlayerTarget,
        condition: PokemonCondition,
    },
    ExpirePokemonCondition {
        target: PlayerTarget,
        condition: PokemonCondition,
    },
    TickTeamCondition {
        target: PlayerTarget,
        condition: TeamCondition,
    },
    ExpireTeamCondition {
        target: PlayerTarget,
        condition: TeamCondition,
    },
    QueueForcedAction {
        target: PlayerTarget,
        action: PlayerAction,
    },

    // Battle flow
    EmitEvent(BattleEvent),
    PushAction(BattleAction),
}

/// Error types for command execution
#[derive(Debug, PartialEq)]
pub enum ExecutionError {
    NoPokemon,
    InvalidPokemonIndex,
}

impl BattleCommand {
    /// Generate events that should be emitted after this command executes successfully
    pub fn emit_events(&self, state: &BattleState) -> Vec<BattleEvent> {
        match self {
            BattleCommand::DealDamage { target, amount } => {
                emit_damage_and_faint_events(*target, *amount, state, None, None)
            }
            BattleCommand::DealStatusDamage {
                target,
                status,
                amount,
            } => emit_damage_and_faint_events(*target, *amount, state, Some(*status), None),
            BattleCommand::DealConditionDamage {
                target,
                condition,
                amount,
            } => {
                emit_damage_and_faint_events(*target, *amount, state, None, Some(condition.clone()))
            }
            BattleCommand::HealPokemon { target, amount } => {
                let player_index = target.to_index();
                let player = &state.players[player_index];
                if let Some(pokemon) = player.team[player.active_pokemon_index].as_ref() {
                    if *amount > 0 {
                        vec![BattleEvent::PokemonHealed {
                            target: pokemon.species,
                            amount: *amount,
                            new_hp: pokemon.current_hp(),
                        }]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
            BattleCommand::SetPokemonStatus { target, status } => {
                let player_index = target.to_index();
                let player = &state.players[player_index];
                if let Some(pokemon) = player.team[player.active_pokemon_index].as_ref() {
                    // Don't emit status application events for fainted Pokemon
                    if pokemon.is_fainted() {
                        vec![]
                    } else {
                        vec![BattleEvent::PokemonStatusApplied {
                            target: pokemon.species,
                            status: *status,
                        }]
                    }
                } else {
                    vec![]
                }
            }
            BattleCommand::CurePokemonStatus { target, status } => {
                let player_index = target.to_index();
                let player = &state.players[player_index];
                if let Some(pokemon) = player.team[player.active_pokemon_index].as_ref() {
                    vec![BattleEvent::PokemonStatusRemoved {
                        target: pokemon.species,
                        status: *status,
                    }]
                } else {
                    vec![]
                }
            }
            BattleCommand::UsePP { .. } => {
                // PP usage is silent - no events emitted
                vec![]
            }
            BattleCommand::ChangeStatStage {
                target,
                stat,
                delta,
            } => {
                let player_index = target.to_index();
                let player = &state.players[player_index];
                if let Some(pokemon) = player.team[player.active_pokemon_index].as_ref() {
                    let new_stage = player.get_stat_stage(*stat);
                    vec![BattleEvent::StatStageChanged {
                        target: pokemon.species,
                        stat: *stat,
                        old_stage: new_stage - delta,
                        new_stage,
                    }]
                } else {
                    vec![]
                }
            }
            BattleCommand::AddCondition { target, condition } => {
                let player_index = target.to_index();
                let player = &state.players[player_index];
                if let Some(pokemon) = player.team[player.active_pokemon_index].as_ref() {
                    // Don't emit condition application events for fainted Pokemon
                    if pokemon.is_fainted() {
                        vec![]
                    } else {
                        vec![BattleEvent::StatusApplied {
                            target: pokemon.species,
                            status: condition.clone(),
                        }]
                    }
                } else {
                    vec![]
                }
            }
            BattleCommand::RemoveCondition {
                target,
                condition_type,
            } => {
                let player_index = target.to_index();
                let player = &state.players[player_index];
                if let Some(pokemon) = player.team[player.active_pokemon_index].as_ref() {
                    // Find the actual condition being removed
                    if let Some(actual_condition) =
                        player.active_pokemon_conditions.get(condition_type)
                    {
                        vec![BattleEvent::StatusRemoved {
                            target: pokemon.species,
                            status: actual_condition.clone(),
                        }]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
            BattleCommand::RemoveSpecificCondition { target, condition } => {
                let player_index = target.to_index();
                let player = &state.players[player_index];
                if let Some(pokemon) = player.team[player.active_pokemon_index].as_ref() {
                    vec![BattleEvent::StatusRemoved {
                        target: pokemon.species,
                        status: condition.clone(),
                    }]
                } else {
                    vec![]
                }
            }
            BattleCommand::AddTeamCondition {
                target, condition, ..
            } => {
                let player_index = target.to_index();
                vec![BattleEvent::TeamConditionApplied {
                    player_index,
                    condition: *condition,
                }]
            }
            BattleCommand::SwitchPokemon {
                target,
                new_pokemon_index,
            } => {
                let player_index = target.to_index();
                let player = &state.players[player_index];
                let old_pokemon = player.team[player.active_pokemon_index]
                    .as_ref()
                    .map(|p| p.species);
                let new_pokemon = player.team[*new_pokemon_index].as_ref().map(|p| p.species);

                if let (Some(old), Some(new)) = (old_pokemon, new_pokemon) {
                    vec![BattleEvent::PokemonSwitched {
                        player_index,
                        old_pokemon: old,
                        new_pokemon: new,
                    }]
                } else {
                    vec![]
                }
            }
            BattleCommand::UpdateStatusProgress { target: _ } => {
                // This command can potentially cure a status, so we need to check if we should emit a removed event
                // However, the actual determination happens during state change, so we return empty here
                // The state change function will emit the appropriate event if needed
                vec![]
            }
            BattleCommand::TickPokemonCondition {
                target: _,
                condition: _,
            } => {
                // Ticking conditions doesn't generate damage events - only DealConditionDamage does
                vec![]
            }
            BattleCommand::ExpirePokemonCondition { target, condition } => {
                let player_index = target.to_index();
                let player = &state.players[player_index];
                if let Some(pokemon) = player.team[player.active_pokemon_index].as_ref() {
                    vec![BattleEvent::ConditionExpired {
                        target: pokemon.species,
                        condition: condition.clone(),
                    }]
                } else {
                    vec![]
                }
            }
            BattleCommand::TickTeamCondition {
                target: _,
                condition: _,
            } => {
                // let player_index = target.to_index();
                // Team conditions don't usually emit tick events, but this is where we'd add them
                vec![]
            }
            BattleCommand::ExpireTeamCondition { target, condition } => {
                let player_index = target.to_index();
                vec![BattleEvent::TeamConditionExpired {
                    player_index,
                    condition: *condition,
                }]
            }
            BattleCommand::QueueForcedAction { .. } => {
                // Queuing actions doesn't generate events
                vec![]
            }

            // Commands that emit manual events via EmitEvent should pass through
            BattleCommand::EmitEvent(event) => vec![event.clone()],

            // Commands that don't generate automatic events
            BattleCommand::SetGameState(_)
            | BattleCommand::IncrementTurnNumber
            | BattleCommand::ClearActionQueue
            | BattleCommand::SetLastMove { .. }
            | BattleCommand::AddAnte { .. }
            | BattleCommand::ClearPlayerState { .. }
            | BattleCommand::PushAction(_) => vec![],
        }
    }
}

/// Centralized function to emit damage events and check for fainting.
fn emit_damage_and_faint_events(
    target: PlayerTarget,
    amount: u16,
    state: &BattleState,
    status: Option<StatusCondition>,
    condition: Option<PokemonCondition>,
) -> Vec<BattleEvent> {
    let player_index = target.to_index();
    let player = &state.players[player_index];
    if let Some(pokemon) = player.active_pokemon() {
        let mut events = Vec::new();

        // 1. Create the appropriate primary damage event.
        if let Some(s) = status {
            events.push(BattleEvent::PokemonStatusDamage {
                target: pokemon.species,
                status: s,
                damage: amount,
                remaining_hp: pokemon.current_hp(),
            });
        } else if let Some(c) = condition {
            events.push(BattleEvent::StatusDamage {
                target: pokemon.species,
                status: c,
                damage: amount,
            });
        } else {
            events.push(BattleEvent::DamageDealt {
                target: pokemon.species,
                damage: amount,
                remaining_hp: pokemon.current_hp(),
            });
        }

        // 2. Check for fainting and add the event if necessary.
        if pokemon.is_fainted() {
            events.push(BattleEvent::PokemonFainted {
                player_index,
                pokemon: pokemon.species,
            });
        }
        return events;
    }
    vec![]
}

/// Execute a batch of commands atomically
pub fn execute_command_batch(
    commands: Vec<BattleCommand>,
    state: &mut BattleState,
    bus: &mut EventBus,
    action_stack: &mut ActionStack,
) -> Result<(), ExecutionError> {
    for command in commands {
        execute_command(command, state, bus, action_stack)?;
    }
    Ok(())
}

/// Helper function to execute commands that operate on the active Pokemon
fn execute_pokemon_command<F>(
    target: PlayerTarget,
    state: &mut BattleState,
    operation: F,
) -> Result<(), ExecutionError>
where
    F: FnOnce(&mut crate::pokemon::PokemonInst, usize) -> Result<(), ExecutionError>,
{
    let player_index = target.to_index();
    let player = &mut state.players[player_index];
    if let Some(pokemon) = player.team[player.active_pokemon_index].as_mut() {
        operation(pokemon, player_index)
    } else {
        Err(ExecutionError::NoPokemon)
    }
}

/// Helper function for DealDamage command state change only (no events)
fn execute_deal_damage_command_state_only(
    target: PlayerTarget,
    amount: u16,
    state: &mut BattleState,
) -> Result<(), ExecutionError> {
    let player_index = target.to_index();
    let player = &mut state.players[player_index];
    if let Some(pokemon) = player.team[player.active_pokemon_index].as_mut() {
        pokemon.take_damage(amount);
        Ok(())
    } else {
        Err(ExecutionError::NoPokemon)
    }
}

pub fn execute_command(
    command: BattleCommand,
    state: &mut BattleState,
    bus: &mut EventBus,
    action_stack: &mut ActionStack,
) -> Result<(), ExecutionError> {
    // Special handling for EmitEvent - just emit the event and return
    if let BattleCommand::EmitEvent(event) = &command {
        bus.push(event.clone());
        return Ok(());
    }

    // Execute the state change
    let result = execute_state_change(&command, state, action_stack);

    // If successful, auto-emit events
    if result.is_ok() {
        for event in command.emit_events(state) {
            bus.push(event);
        }
    }

    result
}

/// Execute the actual state change for a command
fn execute_state_change(
    command: &BattleCommand,
    state: &mut BattleState,
    action_stack: &mut ActionStack,
) -> Result<(), ExecutionError> {
    match command {
        BattleCommand::EmitEvent(_) => {
            // This should not reach here due to early return above
            unreachable!("EmitEvent should be handled before execute_state_change")
        }
        BattleCommand::DealDamage { target, amount } => {
            execute_deal_damage_command_state_only(*target, *amount, state)
        }
        BattleCommand::HealPokemon { target, amount } => {
            execute_pokemon_command(*target, state, |pokemon, _| {
                pokemon.heal(*amount);
                Ok(())
            })
        }
        BattleCommand::SetPokemonStatus { target, status } => {
            execute_pokemon_command(*target, state, |pokemon, _| {
                // Don't apply status to fainted Pokemon
                if pokemon.is_fainted() {
                    Ok(())
                } else {
                    pokemon.status = Some(*status);
                    Ok(())
                }
            })
        }
        BattleCommand::CurePokemonStatus { target, status: _ } => {
            execute_pokemon_command(*target, state, |pokemon, _| {
                pokemon.status = None;
                Ok(())
            })
        }
        BattleCommand::UsePP { target, move_used } => {
            execute_pokemon_command(*target, state, |pokemon, _| {
                pokemon
                    .use_move(*move_used)
                    .map_err(|_| ExecutionError::NoPokemon)
            })
        }
        BattleCommand::ChangeStatStage {
            target,
            stat,
            delta,
        } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            let current_stage = player.get_stat_stage(*stat);
            let new_stage = (current_stage + delta).clamp(-6, 6);
            player.set_stat_stage(*stat, new_stage);
            Ok(())
        }
        BattleCommand::AddCondition { target, condition } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            // Don't apply conditions to fainted Pokemon
            if let Some(pokemon) = player.active_pokemon() {
                if !pokemon.is_fainted() {
                    player.add_condition(condition.clone());
                }
            }
            Ok(())
        }
        BattleCommand::RemoveCondition {
            target,
            condition_type,
        } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            player.active_pokemon_conditions.remove(condition_type);
            Ok(())
        }
        BattleCommand::RemoveSpecificCondition { target, condition } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            player
                .active_pokemon_conditions
                .remove(&condition.get_type());
            Ok(())
        }
        BattleCommand::AddTeamCondition {
            target,
            condition,
            turns,
        } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            player.add_team_condition(*condition, *turns);
            Ok(())
        }
        BattleCommand::SetLastMove { target, move_used } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            player.last_move = Some(*move_used);
            Ok(())
        }
        BattleCommand::SwitchPokemon {
            target,
            new_pokemon_index,
        } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            if *new_pokemon_index < player.team.len() && player.team[*new_pokemon_index].is_some() {
                player.active_pokemon_index = *new_pokemon_index;
                Ok(())
            } else {
                Err(ExecutionError::InvalidPokemonIndex)
            }
        }
        BattleCommand::AddAnte { target, amount } => {
            let player_index = target.to_index();
            state.players[player_index].add_ante(*amount);
            Ok(())
        }
        BattleCommand::SetGameState(new_state) => {
            state.game_state = *new_state;
            Ok(())
        }
        BattleCommand::IncrementTurnNumber => {
            state.turn_number += 1;
            Ok(())
        }
        BattleCommand::ClearActionQueue => {
            state.action_queue = [None, None];
            Ok(())
        }
        BattleCommand::PushAction(action) => {
            action_stack.push_front(action.clone());
            Ok(())
        }
        BattleCommand::ClearPlayerState { target } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            player.clear_active_pokemon_state();
            Ok(())
        }
        BattleCommand::DealStatusDamage {
            target,
            status: _,
            amount,
        } => {
            execute_pokemon_command(*target, state, |pokemon, _| {
                let actual_damage = pokemon.take_damage(*amount);
                // Note: We don't emit events here as they're handled by emit_events()
                // The fainted check will be handled by the DealDamage-style logic in emit_events()
                if actual_damage {
                    // Pokemon fainted from status damage - this is handled by the event system
                }
                Ok(())
            })
        }
        BattleCommand::DealConditionDamage {
            target,
            condition: _,
            amount,
        } => {
            execute_pokemon_command(*target, state, |pokemon, _| {
                let actual_damage = pokemon.take_damage(*amount);
                // Note: We don't emit events here as they're handled by emit_events()
                // The fainted check will be handled by the DealDamage-style logic in emit_events()
                if actual_damage {
                    // Pokemon fainted from status damage - this is handled by the event system
                }
                Ok(())
            })
        }
        BattleCommand::UpdateStatusProgress { target } => {
            execute_pokemon_command(*target, state, |pokemon, _| {
                let (should_cure, _status_changed) = pokemon.update_status_progress();
                if should_cure {
                    // Status cured - this will be detected by emit_events() when it checks the pokemon's status
                }
                Ok(())
            })
        }
        BattleCommand::TickPokemonCondition { target, condition } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];

            // Apply tick effect for this specific condition
            if let Some(existing_condition) = player
                .active_pokemon_conditions
                .get_mut(&condition.get_type())
            {
                // Apply condition-specific tick behavior
                match existing_condition {
                    crate::battle::conditions::PokemonCondition::Confused { turns_remaining } => {
                        *turns_remaining = turns_remaining.saturating_sub(1);
                    }
                    crate::battle::conditions::PokemonCondition::Exhausted { turns_remaining } => {
                        *turns_remaining = turns_remaining.saturating_sub(1);
                    }
                    crate::battle::conditions::PokemonCondition::Trapped { turns_remaining } => {
                        *turns_remaining = turns_remaining.saturating_sub(1);
                    }
                    crate::battle::conditions::PokemonCondition::Rampaging { turns_remaining } => {
                        *turns_remaining = turns_remaining.saturating_sub(1);
                    }
                    crate::battle::conditions::PokemonCondition::Disabled {
                        turns_remaining,
                        ..
                    } => {
                        *turns_remaining = turns_remaining.saturating_sub(1);
                    }
                    crate::battle::conditions::PokemonCondition::Biding {
                        turns_remaining, ..
                    } => {
                        *turns_remaining = turns_remaining.saturating_sub(1);
                    }
                    _ => {} // Other conditions don't have turns to tick
                }
            }
            Ok(())
        }
        BattleCommand::ExpirePokemonCondition { target, condition } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            player
                .active_pokemon_conditions
                .remove(&condition.get_type());
            Ok(())
        }
        BattleCommand::TickTeamCondition { target, condition } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];

            // Decrement turns for this specific team condition
            if let Some(turns) = player.team_conditions.get_mut(condition) {
                *turns = turns.saturating_sub(1);
            }
            Ok(())
        }
        BattleCommand::ExpireTeamCondition { target, condition } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            player.team_conditions.remove(condition);
            Ok(())
        }
        BattleCommand::QueueForcedAction { target, action } => {
            let player_index = target.to_index();
            state.action_queue[player_index] = Some(action.clone());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::action_stack::ActionStack;
    use crate::battle::state::{BattleState, EventBus, GameState};
    use crate::player::BattlePlayer;
    use crate::pokemon::PokemonInst;
    use crate::species::Species;
    use std::collections::HashMap;

    fn create_test_battle_state() -> BattleState {
        use crate::pokemon::MoveInstance;

        let moves1 = [
            Some(MoveInstance::new(Move::Tackle)),
            Some(MoveInstance::new(Move::Scratch)),
            None,
            None,
        ];

        let moves2 = [
            Some(MoveInstance::new(Move::Tackle)),
            Some(MoveInstance::new(Move::Scratch)),
            None,
            None,
        ];

        let pokemon1 = PokemonInst::new_for_test(
            Species::Pikachu,
            1,
            0,
            100, // HP
            [15; 6],
            [0; 6],
            [100, 80, 60, 80, 60, 100],
            moves1,
            None,
        );

        let pokemon2 = PokemonInst::new_for_test(
            Species::Charmander,
            1,
            0,
            100, // HP
            [15; 6],
            [0; 6],
            [100, 80, 60, 80, 60, 100],
            moves2,
            None,
        );

        let player1 = BattlePlayer {
            player_id: "test1".to_string(),
            player_name: "Player 1".to_string(),
            player_type: crate::player::PlayerType::NPC,
            team: [
                Some(pokemon1),
                const { None },
                const { None },
                const { None },
                const { None },
                const { None },
            ],
            active_pokemon_index: 0,
            stat_stages: HashMap::new(),
            team_conditions: HashMap::new(),
            active_pokemon_conditions: HashMap::new(),
            last_move: None,
            ante: 200,
        };

        let player2 = BattlePlayer {
            player_id: "test2".to_string(),
            player_name: "Player 2".to_string(),
            player_type: crate::player::PlayerType::NPC,
            team: [
                Some(pokemon2),
                const { None },
                const { None },
                const { None },
                const { None },
                const { None },
            ],
            active_pokemon_index: 0,
            stat_stages: HashMap::new(),
            team_conditions: HashMap::new(),
            active_pokemon_conditions: HashMap::new(),
            last_move: None,
            ante: 200,
        };

        BattleState::new("test_battle".to_string(), player1, player2)
    }

    #[test]
    fn test_player_target_conversion() {
        assert_eq!(PlayerTarget::Player1.to_index(), 0);
        assert_eq!(PlayerTarget::Player2.to_index(), 1);
        assert_eq!(PlayerTarget::from_index(0), PlayerTarget::Player1);
        assert_eq!(PlayerTarget::from_index(1), PlayerTarget::Player2);
        assert_eq!(PlayerTarget::Player1.opponent(), PlayerTarget::Player2);
        assert_eq!(PlayerTarget::Player2.opponent(), PlayerTarget::Player1);
    }

    #[test]
    fn test_deal_damage_command() {
        let mut state = create_test_battle_state();
        let mut bus = EventBus::new();
        let mut action_stack = ActionStack::new();

        let initial_hp = state.players[0].active_pokemon().unwrap().current_hp();

        let result = execute_command_batch(
            vec![BattleCommand::DealDamage {
                target: PlayerTarget::Player1,
                amount: 20,
            }],
            &mut state,
            &mut bus,
            &mut action_stack,
        );

        assert!(result.is_ok());
        assert_eq!(
            state.players[0].active_pokemon().unwrap().current_hp(),
            initial_hp - 20
        );
    }

    #[test]
    fn test_heal_pokemon_command() {
        let mut state = create_test_battle_state();
        let mut bus = EventBus::new();
        let mut action_stack = ActionStack::new();

        // First damage the Pokemon
        execute_command_batch(
            vec![BattleCommand::DealDamage {
                target: PlayerTarget::Player1,
                amount: 30,
            }],
            &mut state,
            &mut bus,
            &mut action_stack,
        )
        .unwrap();

        let damaged_hp = state.players[0].active_pokemon().unwrap().current_hp();

        // Then heal it
        let result = execute_command_batch(
            vec![BattleCommand::HealPokemon {
                target: PlayerTarget::Player1,
                amount: 10,
            }],
            &mut state,
            &mut bus,
            &mut action_stack,
        );

        assert!(result.is_ok());
        assert_eq!(
            state.players[0].active_pokemon().unwrap().current_hp(),
            damaged_hp + 10
        );
    }

    #[test]
    fn test_emit_event_command() {
        let mut state = create_test_battle_state();
        let mut bus = EventBus::new();
        let mut action_stack = ActionStack::new();

        let event = BattleEvent::TurnStarted { turn_number: 1 };

        let result = execute_command_batch(
            vec![BattleCommand::EmitEvent(event.clone())],
            &mut state,
            &mut bus,
            &mut action_stack,
        );

        assert!(result.is_ok());
        assert_eq!(bus.events().len(), 1);
        assert!(matches!(
            bus.events()[0],
            BattleEvent::TurnStarted { turn_number: 1 }
        ));
    }

    #[test]
    fn test_change_stat_stage_command() {
        let mut state = create_test_battle_state();
        let mut bus = EventBus::new();
        let mut action_stack = ActionStack::new();

        let result = execute_command_batch(
            vec![BattleCommand::ChangeStatStage {
                target: PlayerTarget::Player1,
                stat: StatType::Attack,
                delta: 2,
            }],
            &mut state,
            &mut bus,
            &mut action_stack,
        );

        assert!(result.is_ok());
        assert_eq!(state.players[0].get_stat_stage(StatType::Attack), 2);
    }

    #[test]
    fn test_set_game_state_command() {
        let mut state = create_test_battle_state();
        let mut bus = EventBus::new();
        let mut action_stack = ActionStack::new();

        let result = execute_command_batch(
            vec![BattleCommand::SetGameState(GameState::TurnInProgress)],
            &mut state,
            &mut bus,
            &mut action_stack,
        );

        assert!(result.is_ok());
        assert_eq!(state.game_state, GameState::TurnInProgress);
    }
}
