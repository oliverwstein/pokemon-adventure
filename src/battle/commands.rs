use crate::battle::conditions::{PokemonCondition, PokemonConditionType};
use crate::battle::state::{BattleEvent, BattleState, EventBus, GameState};
use crate::battle::turn_orchestrator::{ActionStack, BattleAction};
use crate::moves::Move;
use crate::player::{PlayerAction, StatType, TeamCondition};
use crate::pokemon::StatusCondition;
use std::collections::HashMap;

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
        status: Option<StatusCondition>,
    },
    FaintPokemon {
        target: PlayerTarget,
    },
    RestorePP {
        target: PlayerTarget,
        move_slot: usize,
        amount: u8,
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
    AddTeamCondition {
        target: PlayerTarget,
        condition: TeamCondition,
        turns: u8,
    },
    RemoveTeamCondition {
        target: PlayerTarget,
        condition: TeamCondition,
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

/// Error types for battle execution
#[derive(Debug, PartialEq)]
pub enum BattleExecutionError {
    InvalidPlayerAction(String),
    GameNotWaitingForActions,
    PlayerAlreadySubmitted(String),
    InvalidGameState,
    CommandExecutionFailed(ExecutionError),
}

/// Result of turn execution containing state changes and events
#[derive(Debug, Clone)]
pub struct TurnResult {
    pub events: Vec<BattleEvent>,
    pub new_state: GameState,
    pub battle_ended: bool,
    pub winner: Option<usize>,
}

impl TurnResult {
    pub fn new(events: Vec<BattleEvent>, new_state: GameState) -> Self {
        let battle_ended = matches!(new_state, GameState::Player1Win | GameState::Player2Win | GameState::Draw);
        let winner = match new_state {
            GameState::Player1Win => Some(0),
            GameState::Player2Win => Some(1),
            _ => None,
        };
        
        Self {
            events,
            new_state,
            battle_ended,
            winner,
        }
    }
}

/// Unified battle executor that handles both single-player and multiplayer scenarios
pub struct BattleExecutor {
    battle_state: BattleState,
    event_bus: EventBus,
    action_stack: ActionStack,
    pending_actions: HashMap<usize, PlayerAction>, // player_index -> action
}

impl BattleExecutor {
    /// Create a new battle executor with the given battle state
    pub fn new(battle_state: BattleState) -> Self {
        Self {
            battle_state,
            event_bus: EventBus::new(),
            action_stack: ActionStack::new(),
            pending_actions: HashMap::new(),
        }
    }

    /// Convenience method for single-player scenarios - submit both actions and execute immediately
    pub fn execute_single_turn(
        &mut self, 
        player1_action: PlayerAction, 
        player2_action: PlayerAction
    ) -> Result<TurnResult, BattleExecutionError> {
        self.submit_action(0, player1_action)?;
        self.submit_action(1, player2_action)?;
        self.execute_turn()
    }

    /// Get pending actions (for API queries)
    pub fn pending_actions(&self) -> &HashMap<usize, PlayerAction> {
        &self.pending_actions
    }

    /// Check which players still need to submit actions
    pub fn players_pending_actions(&self) -> Vec<usize> {
        match self.battle_state.game_state {
            GameState::WaitingForBothActions => {
                (0..2).filter(|&i| !self.pending_actions.contains_key(&i)).collect()
            }
            GameState::WaitingForPlayer1Replacement => {
                if self.pending_actions.contains_key(&0) { vec![] } else { vec![0] }
            }
            GameState::WaitingForPlayer2Replacement => {
                if self.pending_actions.contains_key(&1) { vec![] } else { vec![1] }
            }
            GameState::WaitingForBothReplacements => {
                (0..2).filter(|&i| !self.pending_actions.contains_key(&i)).collect()
            }
            _ => vec![],
        }
    }

    /// Submit an action for a player
    pub fn submit_action(&mut self, player_index: usize, action: PlayerAction) -> Result<(), BattleExecutionError> {
        // Validate player index
        if player_index >= 2 {
            return Err(BattleExecutionError::InvalidPlayerAction(
                format!("Invalid player index: {}", player_index)
            ));
        }

        // Check if game is in a state that accepts actions
        if !self.accepts_actions() {
            return Err(BattleExecutionError::GameNotWaitingForActions);
        }

        // Check if player already submitted an action
        if self.pending_actions.contains_key(&player_index) {
            return Err(BattleExecutionError::PlayerAlreadySubmitted(
                format!("Player {} already submitted an action", player_index)
            ));
        }

        // Validate the action against current state
        self.validate_action(player_index, &action)?;

        // Store the action
        self.pending_actions.insert(player_index, action);

        Ok(())
    }

    /// Check if the battle is ready to execute a turn
    pub fn ready_for_execution(&self) -> bool {
        match self.battle_state.game_state {
            GameState::WaitingForBothActions => self.pending_actions.len() == 2,
            GameState::WaitingForPlayer1Replacement => self.pending_actions.contains_key(&0),
            GameState::WaitingForPlayer2Replacement => self.pending_actions.contains_key(&1),
            GameState::WaitingForBothReplacements => self.pending_actions.len() == 2,
            _ => false,
        }
    }

    /// Execute a complete turn when ready, returning the results
    pub fn execute_turn(&mut self) -> Result<TurnResult, BattleExecutionError> {
        if !self.ready_for_execution() {
            return Err(BattleExecutionError::InvalidGameState);
        }

        // Clear event bus for this turn
        self.event_bus = EventBus::new();

        // Set pending actions in battle state
        for (&player_index, action) in &self.pending_actions {
            self.battle_state.action_queue[player_index] = Some(action.clone());
        }

        // Clear pending actions
        self.pending_actions.clear();

        // Execute the turn using existing turn resolution logic
        let turn_events = crate::battle::turn_orchestrator::resolve_turn(
            &mut self.battle_state, 
            crate::battle::state::TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50])
        );

        Ok(TurnResult::new(turn_events.events().to_vec(), self.battle_state.game_state.clone()))
    }

    /// Execute commands immediately (for internal use)
    pub fn execute_commands(&mut self, commands: Vec<BattleCommand>) -> Result<(), BattleExecutionError> {
        execute_command_batch(commands, &mut self.battle_state, &mut self.event_bus, &mut self.action_stack)
            .map_err(BattleExecutionError::CommandExecutionFailed)
    }

    /// Get the current battle state (read-only)
    pub fn battle_state(&self) -> &BattleState {
        &self.battle_state
    }

    /// Get the current event bus (read-only)
    pub fn events(&self) -> &EventBus {
        &self.event_bus
    }

    /// Get mutable access to battle state for testing
    #[cfg(test)]
    pub fn battle_state_mut(&mut self) -> &mut BattleState {
        &mut self.battle_state
    }

    /// Check if the executor accepts new actions in the current state
    fn accepts_actions(&self) -> bool {
        matches!(
            self.battle_state.game_state,
            GameState::WaitingForBothActions
                | GameState::WaitingForPlayer1Replacement
                | GameState::WaitingForPlayer2Replacement
                | GameState::WaitingForBothReplacements
        )
    }

    /// Validate that an action is legal in the current state
    fn validate_action(&self, player_index: usize, action: &PlayerAction) -> Result<(), BattleExecutionError> {
        let player = &self.battle_state.players[player_index];
        
        match action {
            PlayerAction::UseMove { move_index } => {
                // Check if player has an active Pokemon
                if player.active_pokemon().is_none() {
                    return Err(BattleExecutionError::InvalidPlayerAction(
                        "No active Pokemon".to_string()
                    ));
                }

                let pokemon = player.active_pokemon().unwrap();
                
                // Check if move index is valid and has PP
                if *move_index >= pokemon.moves.len() {
                    return Err(BattleExecutionError::InvalidPlayerAction(
                        "Invalid move index".to_string()
                    ));
                }

                if let Some(move_instance) = &pokemon.moves[*move_index] {
                    if move_instance.pp == 0 {
                        return Err(BattleExecutionError::InvalidPlayerAction(
                            "Move has no PP remaining".to_string()
                        ));
                    }
                } else {
                    return Err(BattleExecutionError::InvalidPlayerAction(
                        "No move in that slot".to_string()
                    ));
                }
            }
            PlayerAction::ForcedMove { pokemon_move } => {
                // Check if player has an active Pokemon
                if player.active_pokemon().is_none() {
                    return Err(BattleExecutionError::InvalidPlayerAction(
                        "No active Pokemon".to_string()
                    ));
                }

                let pokemon = player.active_pokemon().unwrap();
                
                // Check if Pokemon knows this move and has PP
                let has_move = pokemon.moves.iter().any(|move_slot| {
                    if let Some(move_instance) = move_slot {
                        move_instance.move_ == *pokemon_move && move_instance.pp > 0
                    } else {
                        false
                    }
                });

                if !has_move {
                    return Err(BattleExecutionError::InvalidPlayerAction(
                        format!("Pokemon doesn't know move {:?} or has no PP", pokemon_move)
                    ));
                }
            }
            PlayerAction::SwitchPokemon { team_index } => {
                // Check if target Pokemon exists and is not fainted
                if *team_index >= player.team.len() {
                    return Err(BattleExecutionError::InvalidPlayerAction(
                        "Invalid Pokemon index".to_string()
                    ));
                }

                if let Some(target_pokemon) = &player.team[*team_index] {
                    if target_pokemon.current_hp() == 0 {
                        return Err(BattleExecutionError::InvalidPlayerAction(
                            "Cannot switch to fainted Pokemon".to_string()
                        ));
                    }
                    if *team_index == player.active_pokemon_index {
                        return Err(BattleExecutionError::InvalidPlayerAction(
                            "Pokemon is already active".to_string()
                        ));
                    }
                } else {
                    return Err(BattleExecutionError::InvalidPlayerAction(
                        "Pokemon does not exist".to_string()
                    ));
                }
            }
            PlayerAction::Forfeit => {
                // Forfeit is always valid
            }
        }

        Ok(())
    }
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

/// Helper function specifically for DealDamage command with event emission
fn execute_deal_damage_command(
    target: PlayerTarget,
    amount: u16,
    state: &mut BattleState,
    bus: &mut EventBus,
) -> Result<(), ExecutionError> {
    let player_index = target.to_index();
    let player = &mut state.players[player_index];
    if let Some(pokemon) = player.team[player.active_pokemon_index].as_mut() {
        let did_faint = pokemon.take_damage(amount);
        let remaining_hp = pokemon.current_hp();

        // Emit DamageDealt event
        bus.push(crate::battle::state::BattleEvent::DamageDealt {
            target: pokemon.species,
            damage: amount,
            remaining_hp,
        });

        // Emit PokemonFainted event if needed
        if did_faint {
            bus.push(crate::battle::state::BattleEvent::PokemonFainted {
                player_index,
                pokemon: pokemon.species,
            });
        }

        Ok(())
    } else {
        Err(ExecutionError::NoPokemon)
    }
}

fn execute_command(
    command: BattleCommand,
    state: &mut BattleState,
    bus: &mut EventBus,
    action_stack: &mut ActionStack,
) -> Result<(), ExecutionError> {
    match command {
        BattleCommand::EmitEvent(event) => {
            bus.push(event);
            Ok(())
        }
        BattleCommand::DealDamage { target, amount } => {
            execute_deal_damage_command(target, amount, state, bus)
        }
        BattleCommand::HealPokemon { target, amount } => {
            execute_pokemon_command(target, state, |pokemon, _| {
                pokemon.heal(amount);
                Ok(())
            })
        }
        BattleCommand::SetPokemonStatus { target, status } => {
            execute_pokemon_command(target, state, |pokemon, _| {
                pokemon.status = status;
                Ok(())
            })
        }
        BattleCommand::FaintPokemon { target } => {
            execute_pokemon_command(target, state, |pokemon, _| {
                // Set HP to 0, which will trigger fainting in take_damage
                pokemon.take_damage(pokemon.current_hp());
                Ok(())
            })
        }
        BattleCommand::ChangeStatStage {
            target,
            stat,
            delta,
        } => {
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
        BattleCommand::RemoveCondition {
            target,
            condition_type,
        } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            // Find and remove condition of this type
            let conditions_to_remove: Vec<_> = player
                .active_pokemon_conditions
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
        BattleCommand::AddTeamCondition {
            target,
            condition,
            turns,
        } => {
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
        BattleCommand::SwitchPokemon {
            target,
            new_pokemon_index,
        } => {
            let player_index = target.to_index();
            let player = &mut state.players[player_index];
            if new_pokemon_index < player.team.len() && player.team[new_pokemon_index].is_some() {
                player.active_pokemon_index = new_pokemon_index;
                Ok(())
            } else {
                Err(ExecutionError::InvalidPokemonIndex)
            }
        }
        BattleCommand::AddAnte { target, amount } => {
            let player_index = target.to_index();
            state.players[player_index].add_ante(amount);
            Ok(())
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
        BattleCommand::RestorePP {
            target,
            move_slot,
            amount,
        } => {
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
            action_stack.push_front(action);
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

    // BattleExecutor tests
    #[test]
    fn test_battle_executor_creation() {
        // Initialize move data for tests
        let _ = crate::move_data::initialize_move_data(std::path::Path::new("data"));
        
        let state = create_test_battle_state();
        let executor = BattleExecutor::new(state);
        
        assert_eq!(executor.pending_actions().len(), 0);
        assert!(executor.accepts_actions());
        assert!(!executor.ready_for_execution());
    }

    #[test]
    fn test_submit_single_action() {
        let _ = crate::move_data::initialize_move_data(std::path::Path::new("data"));
        let state = create_test_battle_state();
        let mut executor = BattleExecutor::new(state);
        
        let action = PlayerAction::UseMove { move_index: 0 };
        let result = executor.submit_action(0, action.clone());
        
        assert!(result.is_ok());
        assert_eq!(executor.pending_actions().len(), 1);
        assert_eq!(executor.pending_actions()[&0], action);
        assert!(!executor.ready_for_execution()); // Still need player 2
    }

    #[test]
    fn test_submit_both_actions_and_execute() {
        let _ = crate::move_data::initialize_move_data(std::path::Path::new("data"));
        let state = create_test_battle_state();
        let mut executor = BattleExecutor::new(state);
        
        let action1 = PlayerAction::UseMove { move_index: 0 };
        let action2 = PlayerAction::UseMove { move_index: 0 };
        
        executor.submit_action(0, action1).unwrap();
        executor.submit_action(1, action2).unwrap();
        
        assert!(executor.ready_for_execution());
        
        let result = executor.execute_turn();
        assert!(result.is_ok());
        
        let turn_result = result.unwrap();
        assert!(!turn_result.events.is_empty());
        assert_eq!(executor.pending_actions().len(), 0); // Actions cleared after execution
    }

    #[test]
    fn test_single_turn_convenience_method() {
        let _ = crate::move_data::initialize_move_data(std::path::Path::new("data"));
        let state = create_test_battle_state();
        let mut executor = BattleExecutor::new(state);
        
        let action1 = PlayerAction::UseMove { move_index: 0 };
        let action2 = PlayerAction::UseMove { move_index: 0 };
        
        let result = executor.execute_single_turn(action1, action2);
        assert!(result.is_ok());
        
        let turn_result = result.unwrap();
        assert!(!turn_result.events.is_empty());
    }

    #[test]
    fn test_invalid_action_validation() {
        let _ = crate::move_data::initialize_move_data(std::path::Path::new("data"));
        let state = create_test_battle_state();
        let mut executor = BattleExecutor::new(state);
        
        // Try to use an invalid move index
        let invalid_action = PlayerAction::UseMove { move_index: 99 };
        let result = executor.submit_action(0, invalid_action);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            BattleExecutionError::InvalidPlayerAction(msg) => {
                assert!(msg.contains("Invalid move index"));
            }
            _ => panic!("Expected InvalidPlayerAction error"),
        }
    }

    #[test]
    fn test_duplicate_action_submission() {
        let _ = crate::move_data::initialize_move_data(std::path::Path::new("data"));
        let state = create_test_battle_state();
        let mut executor = BattleExecutor::new(state);
        
        let action = PlayerAction::UseMove { move_index: 0 };
        
        // Submit first action
        executor.submit_action(0, action.clone()).unwrap();
        
        // Try to submit again for same player
        let result = executor.submit_action(0, action);
        assert!(result.is_err());
        match result.unwrap_err() {
            BattleExecutionError::PlayerAlreadySubmitted(_) => {},
            _ => panic!("Expected PlayerAlreadySubmitted error"),
        }
    }

    #[test]
    fn test_players_pending_actions() {
        let _ = crate::move_data::initialize_move_data(std::path::Path::new("data"));
        let state = create_test_battle_state();
        let mut executor = BattleExecutor::new(state);
        
        // Initially both players need to submit
        assert_eq!(executor.players_pending_actions(), vec![0, 1]);
        
        // After player 0 submits
        let action = PlayerAction::UseMove { move_index: 0 };
        executor.submit_action(0, action).unwrap();
        assert_eq!(executor.players_pending_actions(), vec![1]);
        
        // After player 1 submits
        let action = PlayerAction::UseMove { move_index: 0 };
        executor.submit_action(1, action).unwrap();
        assert_eq!(executor.players_pending_actions(), Vec::<usize>::new());
    }

    #[test]
    fn test_execute_without_ready() {
        let _ = crate::move_data::initialize_move_data(std::path::Path::new("data"));
        let state = create_test_battle_state();
        let mut executor = BattleExecutor::new(state);
        
        // Try to execute without submitting actions
        let result = executor.execute_turn();
        assert!(result.is_err());
        match result.unwrap_err() {
            BattleExecutionError::InvalidGameState => {},
            _ => panic!("Expected InvalidGameState error"),
        }
    }
}
