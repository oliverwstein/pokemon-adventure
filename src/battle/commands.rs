use crate::battle::state::{BattleEvent, BattleState, EventBus};
use crate::battle::turn_orchestrator::{ActionStack, BattleAction};
use crate::moves::Move;
use crate::player::{PokemonCondition, StatType, TeamCondition};
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
}

/// Atomic commands representing final state changes
#[derive(Debug, Clone)]
pub enum BattleCommand {
    // Direct state changes
    SetGameState(crate::battle::state::GameState),
    IncrementTurnNumber,
    ClearActionQueue,
    
    // Pokemon modifications
    DealDamage { target: PlayerTarget, amount: u16 },
    HealPokemon { target: PlayerTarget, amount: u16 },
    SetPokemonStatus { target: PlayerTarget, status: Option<StatusCondition> },
    FaintPokemon { target: PlayerTarget },
    RestorePP { target: PlayerTarget, move_slot: usize, amount: u8 },
    
    // Player state changes
    ChangeStatStage { target: PlayerTarget, stat: StatType, delta: i8 },
    AddCondition { target: PlayerTarget, condition: PokemonCondition },
    RemoveCondition { target: PlayerTarget, condition_type: PokemonConditionType },
    AddTeamCondition { target: PlayerTarget, condition: TeamCondition, turns: u8 },
    RemoveTeamCondition { target: PlayerTarget, condition: TeamCondition },
    SetLastMove { target: PlayerTarget, move_used: Move },
    SwitchPokemon { target: PlayerTarget, new_pokemon_index: usize },
    
    // Battle flow
    EmitEvent(BattleEvent),
    PushAction(BattleAction),
}

/// Error types for command execution
#[derive(Debug, PartialEq)]
pub enum ExecutionError {
    NoPokemon,
    InvalidPlayerIndex,
    InvalidPokemonIndex,
    InvalidMove,
    StateValidationError(String),
}

/// Temporary bridge function for the incremental refactoring
/// This will be replaced by a proper CommandExecutor in Step 4
pub fn execute_commands_locally(
    commands: Vec<BattleCommand>, 
    state: &mut BattleState, 
    bus: &mut EventBus,
    action_stack: &mut ActionStack
) -> Result<(), ExecutionError> {
    for command in commands {
        execute_command_locally(command, state, bus, action_stack)?;
    }
    Ok(())
}

fn execute_command_locally(
    command: BattleCommand,
    state: &mut BattleState,
    bus: &mut EventBus,
    action_stack: &mut ActionStack
) -> Result<(), ExecutionError> {
    match command {
        BattleCommand::EmitEvent(event) => {
            bus.push(event);
            Ok(())
        }
        BattleCommand::DealDamage { target, amount } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            if let Some(pokemon) = player.team[player.active_pokemon_index].as_mut() {
                pokemon.take_damage(amount);
                Ok(())
            } else {
                Err(ExecutionError::NoPokemon)
            }
        }
        BattleCommand::HealPokemon { target, amount } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            if let Some(pokemon) = player.team[player.active_pokemon_index].as_mut() {
                pokemon.heal(amount);
                Ok(())
            } else {
                Err(ExecutionError::NoPokemon)
            }
        }
        BattleCommand::SetPokemonStatus { target, status } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            if let Some(pokemon) = player.team[player.active_pokemon_index].as_mut() {
                pokemon.status = status;
                Ok(())
            } else {
                Err(ExecutionError::NoPokemon)
            }
        }
        BattleCommand::FaintPokemon { target } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            if let Some(pokemon) = player.team[player.active_pokemon_index].as_mut() {
                // Set HP to 0, which will trigger fainting in take_damage
                pokemon.take_damage(pokemon.current_hp());
                Ok(())
            } else {
                Err(ExecutionError::NoPokemon)
            }
        }
        BattleCommand::ChangeStatStage { target, stat, delta } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            let current_stage = player.get_stat_stage(stat);
            let new_stage = (current_stage + delta).clamp(-6, 6);
            player.set_stat_stage(stat, new_stage);
            Ok(())
        }
        BattleCommand::AddCondition { target, condition } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            player.add_condition(condition);
            Ok(())
        }
        BattleCommand::RemoveCondition { target, condition_type } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            // Find and remove condition of this type
            let conditions_to_remove: Vec<_> = player.active_pokemon_conditions
                .iter()
                .filter_map(|(key, condition)| {
                    if condition.get_type() == condition_type {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
                .collect();
            
            for key in conditions_to_remove {
                player.active_pokemon_conditions.remove(&key);
            }
            Ok(())
        }
        BattleCommand::AddTeamCondition { target, condition, turns } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            player.add_team_condition(condition, turns);
            Ok(())
        }
        BattleCommand::RemoveTeamCondition { target, condition } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            player.remove_team_condition(&condition);
            Ok(())
        }
        BattleCommand::SetLastMove { target, move_used } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            player.last_move = Some(move_used);
            Ok(())
        }
        BattleCommand::SwitchPokemon { target, new_pokemon_index } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            if new_pokemon_index < player.team.len() && player.team[new_pokemon_index].is_some() {
                player.active_pokemon_index = new_pokemon_index;
                Ok(())
            } else {
                Err(ExecutionError::InvalidPokemonIndex)
            }
        }
        BattleCommand::SetGameState(new_state) => {
            state.game_state = new_state;
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
        BattleCommand::RestorePP { target, move_slot, amount } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            if let Some(pokemon) = player.team[player.active_pokemon_index].as_mut() {
                if move_slot < pokemon.moves.len() && pokemon.moves[move_slot].is_some() {
                    if let Some(move_data) = &mut pokemon.moves[move_slot] {
                        move_data.pp = (move_data.pp + amount).min(move_data.max_pp());
                    }
                    Ok(())
                } else {
                    Err(ExecutionError::InvalidMove)
                }
            } else {
                Err(ExecutionError::NoPokemon)
            }
        }
        BattleCommand::PushAction(action) => {
            action_stack.push_back(action);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::state::{BattleState, EventBus, GameState};
    use crate::battle::turn_orchestrator::ActionStack;
    use crate::player::BattlePlayer;
    use crate::pokemon::PokemonInst;
    use crate::species::Species;
    use std::collections::HashMap;

    fn create_test_battle_state() -> BattleState {
        let pokemon1 = PokemonInst::new_for_test(
            Species::Pikachu,
            1,
            0,
            100, // HP
            [15; 6],
            [0; 6],
            [100, 80, 60, 80, 60, 100],
            [const { None }; 4],
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
            [const { None }; 4],
            None,
        );

        let player1 = BattlePlayer {
            player_id: "test1".to_string(),
            player_name: "Player 1".to_string(),
            team: [Some(pokemon1), const { None }, const { None }, const { None }, const { None }, const { None }],
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
            team: [Some(pokemon2), const { None }, const { None }, const { None }, const { None }, const { None }],
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
        
        let result = execute_commands_locally(
            vec![BattleCommand::DealDamage { target: PlayerTarget::Player1, amount: 20 }],
            &mut state,
            &mut bus,
            &mut action_stack
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
        execute_commands_locally(
            vec![BattleCommand::DealDamage { target: PlayerTarget::Player1, amount: 30 }],
            &mut state,
            &mut bus,
            &mut action_stack
        ).unwrap();

        let damaged_hp = state.players[0].active_pokemon().unwrap().current_hp();

        // Then heal it
        let result = execute_commands_locally(
            vec![BattleCommand::HealPokemon { target: PlayerTarget::Player1, amount: 10 }],
            &mut state,
            &mut bus,
            &mut action_stack
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
        
        let result = execute_commands_locally(
            vec![BattleCommand::EmitEvent(event.clone())],
            &mut state,
            &mut bus,
            &mut action_stack
        );

        assert!(result.is_ok());
        assert_eq!(bus.events().len(), 1);
        assert!(matches!(bus.events()[0], BattleEvent::TurnStarted { turn_number: 1 }));
    }

    #[test]
    fn test_change_stat_stage_command() {
        let mut state = create_test_battle_state();
        let mut bus = EventBus::new();
        let mut action_stack = ActionStack::new();

        let result = execute_commands_locally(
            vec![BattleCommand::ChangeStatStage { 
                target: PlayerTarget::Player1, 
                stat: StatType::Attack, 
                delta: 2 
            }],
            &mut state,
            &mut bus,
            &mut action_stack
        );

        assert!(result.is_ok());
        assert_eq!(state.players[0].get_stat_stage(StatType::Attack), 2);
    }

    #[test]
    fn test_set_game_state_command() {
        let mut state = create_test_battle_state();
        let mut bus = EventBus::new();
        let mut action_stack = ActionStack::new();

        let result = execute_commands_locally(
            vec![BattleCommand::SetGameState(GameState::TurnInProgress)],
            &mut state,
            &mut bus,
            &mut action_stack
        );

        assert!(result.is_ok());
        assert_eq!(state.game_state, GameState::TurnInProgress);
    }
}