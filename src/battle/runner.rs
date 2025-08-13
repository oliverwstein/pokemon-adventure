use crate::battle::state::{BattleEvent, BattleState, EventBus, GameState, TurnRng};
use crate::battle::turn_orchestrator::{collect_player_actions, resolve_turn};
use crate::player::{BattlePlayer, PlayerAction};
use crate::pokemon::PokemonInst;
use crate::species::Species;
use std::collections::HashMap;

/// High-level battle management interface that abstracts turn orchestrator complexity
/// Provides clean API for NPCs, humans, and networked battles
#[derive(Debug)]
pub struct BattleRunner {
    battle_state: BattleState,
    pending_actions: HashMap<usize, PlayerAction>,
    accumulated_events: Vec<BattleEvent>,
}

/// Information about the current battle state for API queries
#[derive(Debug, Clone)]
pub struct BattleInfo {
    pub battle_id: String,
    pub turn_number: u32,
    pub game_state: GameState,
    pub players: Vec<PlayerInfo>,
}

/// Information about a player in the battle
#[derive(Debug, Clone)]
pub struct PlayerInfo {
    pub player_id: String,
    pub player_name: String,
    pub active_pokemon: Option<PokemonInfo>,
    pub team: Vec<Option<PokemonInfo>>,
    pub team_size: usize,
    pub fainted_count: usize,
}

/// Information about a Pokemon for API queries
#[derive(Debug, Clone)]
pub struct PokemonInfo {
    pub species: Species,
    pub name: String,
    pub current_hp: u16,
    pub max_hp: u16,
    pub is_fainted: bool,
    pub status: Option<crate::pokemon::StatusCondition>,
}

/// Result of executing a battle turn or action
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub events: Vec<BattleEvent>,
    pub new_game_state: GameState,
    pub battle_ended: bool,
    pub winner: Option<usize>,
}

/// Errors that can occur when using the battle runner
#[derive(Debug, Clone, PartialEq)]
pub enum BattleRunnerError {
    InvalidPlayerIndex(usize),
    PlayerAlreadySubmitted(usize),
    GameNotAcceptingActions,
    InvalidActionForGameState(String),
    InvalidPlayerAction(String),
    InternalError(String),
}

impl BattleRunner {
    /// Create a new battle runner with the given players
    pub fn new(battle_id: String, player1: BattlePlayer, player2: BattlePlayer) -> Self {
        let battle_state = BattleState::new(battle_id, player1, player2);
        
        Self {
            battle_state,
            pending_actions: HashMap::new(),
            accumulated_events: Vec::new(),
        }
    }

    /// Get current battle information for API queries
    pub fn get_battle_info(&self) -> BattleInfo {
        let players = self.battle_state.players.iter().map(|player| {
            let active_pokemon = player.active_pokemon().map(|p| PokemonInfo {
                species: p.species,
                name: p.name.clone(),
                current_hp: p.current_hp(),
                max_hp: p.max_hp(),
                is_fainted: p.is_fainted(),
                status: p.status,
            });

            let team = player.team.iter().map(|pokemon_opt| {
                pokemon_opt.as_ref().map(|p| PokemonInfo {
                    species: p.species,
                    name: p.name.clone(),
                    current_hp: p.current_hp(),
                    max_hp: p.max_hp(),
                    is_fainted: p.is_fainted(),
                    status: p.status,
                })
            }).collect();

            let fainted_count = player.team.iter()
                .filter_map(|p| p.as_ref())
                .filter(|p| p.is_fainted())
                .count();

            PlayerInfo {
                player_id: player.player_id.clone(),
                player_name: player.player_name.clone(),
                active_pokemon,
                team,
                team_size: player.team.iter().filter(|p| p.is_some()).count(),
                fainted_count,
            }
        }).collect();

        BattleInfo {
            battle_id: self.battle_state.battle_id.clone(),
            turn_number: self.battle_state.turn_number,
            game_state: self.battle_state.game_state.clone(),
            players,
        }
    }

    /// Check if the battle has ended
    pub fn is_battle_ended(&self) -> bool {
        matches!(
            self.battle_state.game_state,
            GameState::Player1Win | GameState::Player2Win | GameState::Draw
        )
    }

    /// Get the winner if the battle has ended
    pub fn get_winner(&self) -> Option<usize> {
        match self.battle_state.game_state {
            GameState::Player1Win => Some(0),
            GameState::Player2Win => Some(1),
            _ => None,
        }
    }

    /// Submit an action for a player
    /// Automatically executes the battle phase when all required actions are submitted
    pub fn submit_action(&mut self, player_index: usize, action: PlayerAction) -> Result<Option<ExecutionResult>, BattleRunnerError> {
        // Validate player index
        if player_index >= 2 {
            return Err(BattleRunnerError::InvalidPlayerIndex(player_index));
        }

        // Check if battle has ended
        if self.is_battle_ended() {
            return Err(BattleRunnerError::GameNotAcceptingActions);
        }

        // Check if player already submitted an action
        if self.pending_actions.contains_key(&player_index) {
            return Err(BattleRunnerError::PlayerAlreadySubmitted(player_index));
        }

        // Validate that this player can submit actions in the current game state
        let can_submit = match (player_index, &self.battle_state.game_state) {
            (0, GameState::WaitingForBothActions) => true,
            (1, GameState::WaitingForBothActions) => true,
            (0, GameState::WaitingForPlayer1Replacement) => true,
            (1, GameState::WaitingForPlayer2Replacement) => true,
            (0, GameState::WaitingForBothReplacements) => true,
            (1, GameState::WaitingForBothReplacements) => true,
            _ => false,
        };

        if !can_submit {
            return Err(BattleRunnerError::InvalidActionForGameState(
                format!("Player {} cannot submit actions in state {:?}", player_index, self.battle_state.game_state)
            ));
        }

        // Validate the action type for replacement states
        if matches!(
            self.battle_state.game_state,
            GameState::WaitingForPlayer1Replacement 
                | GameState::WaitingForPlayer2Replacement 
                | GameState::WaitingForBothReplacements
        ) {
            if !matches!(action, PlayerAction::SwitchPokemon { .. }) {
                return Err(BattleRunnerError::InvalidActionForGameState(
                    "Only switch actions allowed during forced replacement".to_string()
                ));
            }
        }

        // Detailed action validation
        self.validate_action_details(player_index, &action)?;

        // Store the action
        self.pending_actions.insert(player_index, action);

        // Automatically execute if all required actions are now available
        if self.ready_for_execution() {
            Ok(Some(self.execute_internal()?))
        } else {
            Ok(None)
        }
    }

    /// Check which players still need to submit actions
    pub fn players_needing_actions(&self) -> Vec<usize> {
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
            _ => vec![], // Battle ended or in progress
        }
    }

    /// Check if all required actions have been submitted
    pub fn ready_for_execution(&self) -> bool {
        self.players_needing_actions().is_empty()
    }

    /// Internal execution method - executes the next phase of the battle
    /// Returns events generated and whether the battle ended
    fn execute_internal(&mut self) -> Result<ExecutionResult, BattleRunnerError> {
        // Copy pending actions to the battle state
        for (player_index, action) in &self.pending_actions {
            self.battle_state.action_queue[*player_index] = Some(action.clone());
        }

        // Generate RNG for the turn
        let rng = TurnRng::new_random();

        // Execute the turn/replacement
        let event_bus = resolve_turn(&mut self.battle_state, rng);
        let events = event_bus.events().to_vec();

        // Store events for later retrieval
        self.accumulated_events.extend(events.clone());

        // Clear pending actions
        self.pending_actions.clear();

        // Return execution result
        let result = ExecutionResult {
            events,
            new_game_state: self.battle_state.game_state.clone(),
            battle_ended: self.is_battle_ended(),
            winner: self.get_winner(),
        };

        Ok(result)
    }

    /// Auto-generate actions for NPCs and execute if all actions are available
    /// This is a convenience method for single-player scenarios
    pub fn auto_execute_if_ready(&mut self) -> Result<Option<ExecutionResult>, BattleRunnerError> {
        // Try to generate missing actions
        if let Err(e) = collect_player_actions(&mut self.battle_state) {
            return Err(BattleRunnerError::InternalError(format!("Failed to generate NPC actions: {}", e)));
        }

        // Copy generated actions to pending actions and check if we can execute
        let mut any_new_actions = false;
        for (i, action_opt) in self.battle_state.action_queue.iter().enumerate() {
            if let Some(action) = action_opt {
                if !self.pending_actions.contains_key(&i) {
                    self.pending_actions.insert(i, action.clone());
                    any_new_actions = true;
                }
            }
        }

        // If we added actions and are now ready, execute automatically
        if any_new_actions && self.ready_for_execution() {
            Ok(Some(self.execute_internal()?))
        } else {
            Ok(None)
        }
    }

    /// Get all events that have occurred in the battle so far
    pub fn get_all_events(&self) -> &[BattleEvent] {
        &self.accumulated_events
    }

    /// Get events since a certain index (for incremental updates)
    pub fn get_events_since(&self, index: usize) -> &[BattleEvent] {
        if index < self.accumulated_events.len() {
            &self.accumulated_events[index..]
        } else {
            &[]
        }
    }

    /// Clear accumulated events (useful for memory management in long battles)
    pub fn clear_event_history(&mut self) {
        self.accumulated_events.clear();
    }

    /// Get the current game state
    pub fn get_game_state(&self) -> &GameState {
        &self.battle_state.game_state
    }

    /// Get the current turn number
    pub fn get_turn_number(&self) -> u32 {
        self.battle_state.turn_number
    }

    /// Get the battle ID
    pub fn get_battle_id(&self) -> &str {
        &self.battle_state.battle_id
    }

    /// Execute both player actions immediately (convenience method for testing/single-player)
    pub fn execute_single_turn(
        &mut self, 
        player1_action: PlayerAction, 
        player2_action: PlayerAction
    ) -> Result<ExecutionResult, BattleRunnerError> {
        // Clear any existing pending actions
        self.pending_actions.clear();
        
        // Submit both actions
        self.submit_action(0, player1_action)?;
        let result = self.submit_action(1, player2_action)?;
        
        // Should auto-execute since both actions are submitted
        result.ok_or_else(|| BattleRunnerError::InternalError(
            "Expected execution after submitting both actions".to_string()
        ))
    }

    /// Detailed action validation
    fn validate_action_details(&self, player_index: usize, action: &PlayerAction) -> Result<(), BattleRunnerError> {
        let player = &self.battle_state.players[player_index];
        
        match action {
            PlayerAction::UseMove { move_index } => {
                // Check if player has an active Pokemon
                let pokemon = player.active_pokemon().ok_or_else(|| {
                    BattleRunnerError::InvalidPlayerAction("No active Pokemon".to_string())
                })?;
                
                // Check if move index is valid
                if *move_index >= pokemon.moves.len() {
                    return Err(BattleRunnerError::InvalidPlayerAction(
                        "Invalid move index".to_string()
                    ));
                }

                // Check if move exists and has PP
                if let Some(move_instance) = &pokemon.moves[*move_index] {
                    if move_instance.pp == 0 {
                        return Err(BattleRunnerError::InvalidPlayerAction(
                            "Move has no PP remaining".to_string()
                        ));
                    }
                } else {
                    return Err(BattleRunnerError::InvalidPlayerAction(
                        "No move in that slot".to_string()
                    ));
                }
            }
            PlayerAction::ForcedMove { pokemon_move } => {
                // Check if player has an active Pokemon
                let pokemon = player.active_pokemon().ok_or_else(|| {
                    BattleRunnerError::InvalidPlayerAction("No active Pokemon".to_string())
                })?;
                
                // Check if Pokemon knows this move and has PP
                let has_move = pokemon.moves.iter().any(|move_slot| {
                    if let Some(move_instance) = move_slot {
                        move_instance.move_ == *pokemon_move && move_instance.pp > 0
                    } else {
                        false
                    }
                });

                if !has_move {
                    return Err(BattleRunnerError::InvalidPlayerAction(
                        format!("Pokemon doesn't know move {:?} or has no PP", pokemon_move)
                    ));
                }
            }
            PlayerAction::SwitchPokemon { team_index } => {
                // Check if target Pokemon exists
                if *team_index >= player.team.len() {
                    return Err(BattleRunnerError::InvalidPlayerAction(
                        "Invalid Pokemon index".to_string()
                    ));
                }

                // Check if target Pokemon is not fainted and not already active
                if let Some(target_pokemon) = &player.team[*team_index] {
                    if target_pokemon.is_fainted() {
                        return Err(BattleRunnerError::InvalidPlayerAction(
                            "Cannot switch to fainted Pokemon".to_string()
                        ));
                    }
                    if *team_index == player.active_pokemon_index {
                        return Err(BattleRunnerError::InvalidPlayerAction(
                            "Pokemon is already active".to_string()
                        ));
                    }
                } else {
                    return Err(BattleRunnerError::InvalidPlayerAction(
                        "No Pokemon in that team slot".to_string()
                    ));
                }
            }
            PlayerAction::Forfeit => {
                // Forfeit is always valid if the game accepts actions
            }
        }

        Ok(())
    }
}

impl std::fmt::Display for BattleRunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BattleRunnerError::InvalidPlayerIndex(idx) => write!(f, "Invalid player index: {}", idx),
            BattleRunnerError::PlayerAlreadySubmitted(idx) => write!(f, "Player {} already submitted an action", idx),
            BattleRunnerError::GameNotAcceptingActions => write!(f, "Game is not currently accepting actions"),
            BattleRunnerError::InvalidActionForGameState(msg) => write!(f, "Invalid action for current game state: {}", msg),
            BattleRunnerError::InvalidPlayerAction(msg) => write!(f, "Invalid player action: {}", msg),
            BattleRunnerError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for BattleRunnerError {}