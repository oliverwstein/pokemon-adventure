use crate::battle::state::{BattleState, EventBus, TurnRng, GameState, BattleEvent};
use crate::battle::stats::{effective_speed, move_hits};
use crate::player::PlayerAction;
use crate::move_data::get_move_data;
use crate::moves::Move;
use crate::species::Species;
use std::collections::VecDeque;

/// Internal action types for the action stack
/// These represent atomic actions that can be executed during battle resolution
#[derive(Debug, Clone)]
enum BattleAction {
    /// Player forfeits the battle
    Forfeit { player_index: usize },
    
    /// Player switches to a different Pokemon
    Switch { player_index: usize, target_pokemon_index: usize },
    
    /// Player uses an item (not yet implemented)
    UseItem { player_index: usize, item_id: String },
    
    /// Execute a single hit of a move (for multi-hit moves, multiple actions are pushed)
    AttackHit { 
        attacker_index: usize, 
        defender_index: usize, 
        move_used: Move,
        hit_number: u8, // 0 for single hit, 0,1,2... for multi-hit
    },
}

/// Action stack for managing battle action execution
struct ActionStack {
    actions: VecDeque<BattleAction>,
}

impl ActionStack {
    fn new() -> Self {
        Self {
            actions: VecDeque::new(),
        }
    }
    
    fn push_back(&mut self, action: BattleAction) {
        self.actions.push_back(action);
    }
    
    fn push_front(&mut self, action: BattleAction) {
        self.actions.push_front(action);
    }
    
    fn pop_front(&mut self) -> Option<BattleAction> {
        self.actions.pop_front()
    }
    
    fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

/// Prepares a battle state for turn resolution by collecting actions from both players
/// This function should be called before resolve_turn()
/// For now, implements a basic deterministic AI that selects first available move
pub fn collect_player_actions(battle_state: &mut BattleState) -> Result<(), String> {
    // Ensure we're in the right state to collect actions
    if battle_state.game_state != GameState::WaitingForBothActions {
        return Err("Cannot collect actions: battle is not waiting for actions".to_string());
    }
    
    // Generate deterministic actions for any players that haven't provided one
    for player_index in 0..2 {
        if battle_state.action_queue[player_index].is_none() {
            let action = generate_deterministic_action(&battle_state.players[player_index])?;
            battle_state.action_queue[player_index] = Some(action);
        }
    }
    
    Ok(())
}

/// Generates a deterministic action for a player (basic AI)
/// Uses first available move (with PP > 0), or Struggle if no moves have PP
fn generate_deterministic_action(
    player: &crate::player::BattlePlayer,
) -> Result<PlayerAction, String> {
    // Get the active pokemon
    let active_pokemon = player.team[player.active_pokemon_index].as_ref()
        .ok_or("No active pokemon")?;
    
    // Find first move with PP > 0
    for (move_index, move_opt) in active_pokemon.moves.iter().enumerate() {
        if let Some(move_instance) = move_opt {
            if move_instance.pp > 0 {
                return Ok(PlayerAction::UseMove { move_index });
            }
        }
    }
    
    // No moves have PP - should use Struggle
    // TODO: Implement Struggle move - for now return error
    Err("No moves with PP available - need to implement Struggle".to_string())
}

/// Sets a player's action in the battle state
/// This would typically be called by the API layer when a player submits their action
pub fn set_player_action(
    battle_state: &mut BattleState, 
    player_index: usize, 
    action: PlayerAction
) -> Result<(), String> {
    if player_index >= 2 {
        return Err("Invalid player index".to_string());
    }
    
    if battle_state.game_state != GameState::WaitingForBothActions {
        return Err("Cannot set action: battle is not waiting for actions".to_string());
    }
    
    battle_state.action_queue[player_index] = Some(action);
    Ok(())
}

/// Check if battle is ready for turn resolution (both players have provided actions)
pub fn ready_for_turn_resolution(battle_state: &BattleState) -> bool {
    battle_state.game_state == GameState::WaitingForBothActions
        && battle_state.action_queue[0].is_some() 
        && battle_state.action_queue[1].is_some()
}

/// Main entry point for turn resolution
/// Takes a battle state and RNG oracle, executes one complete turn
/// Returns EventBus containing all events that occurred during the turn
pub fn resolve_turn(
    battle_state: &mut BattleState,
    mut rng: TurnRng,
) -> EventBus {
    let mut bus = EventBus::new();
    
    // 1. Initialization
    initialize_turn(battle_state, &mut bus);
    
    // 2. Build initial action stack from player actions
    let mut action_stack = build_initial_action_stack(battle_state);
    
    // 3. Execute actions from stack until empty
    while let Some(action) = action_stack.pop_front() {
        execute_battle_action(action, battle_state, &mut action_stack, &mut bus, &mut rng);
        
        // Check if battle ended (forfeit, all Pokemon fainted, etc.)
        if battle_state.game_state != GameState::TurnInProgress {
            break;
        }
    }
    
    // 4. End-of-Turn Phase (only if battle is still ongoing)
    if battle_state.game_state == GameState::TurnInProgress {
        execute_end_turn_phase(battle_state, &mut bus, &mut rng);
    }
    
    // 5. Cleanup & Finalization
    finalize_turn(battle_state, &mut bus);
    
    bus
}

fn initialize_turn(battle_state: &mut BattleState, bus: &mut EventBus) {
    battle_state.game_state = GameState::TurnInProgress;
    bus.push(BattleEvent::TurnStarted { turn_number: battle_state.turn_number });
}

/// Build initial action stack from player actions in priority order
fn build_initial_action_stack(battle_state: &BattleState) -> ActionStack {
    let mut stack = ActionStack::new();
    let action_order = determine_action_order(battle_state);
    
    // Convert PlayerActions to BattleActions and add to stack in priority order
    for &player_index in action_order.iter() {
        if let Some(player_action) = &battle_state.action_queue[player_index] {
            let battle_action = convert_player_action_to_battle_action(player_action, player_index, battle_state);
            stack.push_back(battle_action);
        }
    }
    
    stack
}

/// Convert a PlayerAction to one or more BattleActions
fn convert_player_action_to_battle_action(
    player_action: &PlayerAction,
    player_index: usize,
    battle_state: &BattleState,
) -> BattleAction {
    match player_action {
        PlayerAction::Forfeit => BattleAction::Forfeit { player_index },
        
        PlayerAction::SwitchPokemon { team_index } => BattleAction::Switch { 
            player_index, 
            target_pokemon_index: *team_index 
        },
        
        PlayerAction::UseMove { move_index } => {
            let player = &battle_state.players[player_index];
            let active_pokemon = player.team[player.active_pokemon_index].as_ref()
                .expect("Active pokemon should exist");
            let move_instance = &active_pokemon.moves[*move_index].as_ref()
                .expect("Move should exist");
            
            // Determine defender
            let defender_index = if player_index == 0 { 1 } else { 0 };
            
            // For now, all moves are single-hit. Multi-hit logic will be added later
            BattleAction::AttackHit {
                attacker_index: player_index,
                defender_index,
                move_used: move_instance.move_,
                hit_number: 0,
            }
        }
    }
}

/// Execute a single battle action, potentially adding more actions to the stack
fn execute_battle_action(
    action: BattleAction,
    battle_state: &mut BattleState,
    action_stack: &mut ActionStack,
    bus: &mut EventBus,
    rng: &mut TurnRng,
) {
    match action {
        BattleAction::Forfeit { player_index } => {
            execute_forfeit(player_index, battle_state, bus);
        }
        
        BattleAction::Switch { player_index, target_pokemon_index } => {
            execute_switch(player_index, target_pokemon_index, battle_state, bus);
        }
        
        BattleAction::UseItem { .. } => {
            // TODO: Implement item usage
        }
        
        BattleAction::AttackHit { attacker_index, defender_index, move_used, hit_number } => {
            execute_attack_hit(attacker_index, defender_index, move_used, hit_number, 
                             battle_state, action_stack, bus, rng);
        }
    }
}

/// Execute forfeit action - player loses immediately
fn execute_forfeit(player_index: usize, battle_state: &mut BattleState, bus: &mut EventBus) {
    // Set game state to opponent wins
    battle_state.game_state = if player_index == 0 {
        GameState::Player2Win
    } else {
        GameState::Player1Win
    };
    
    bus.push(BattleEvent::PlayerDefeated { player_index });
    bus.push(BattleEvent::BattleEnded { 
        winner: Some(if player_index == 0 { 1 } else { 0 }) 
    });
}

/// Execute switch action - change active Pokemon
fn execute_switch(
    player_index: usize, 
    target_pokemon_index: usize, 
    battle_state: &mut BattleState, 
    bus: &mut EventBus
) {
    let player = &mut battle_state.players[player_index];
    let old_pokemon = player.team[player.active_pokemon_index].as_ref()
        .expect("Current active Pokemon should exist").species;
    let new_pokemon = player.team[target_pokemon_index].as_ref()
        .expect("Target Pokemon should exist").species;
    
    // TODO: Clear volatile conditions and stat stages of switched-out Pokemon
    
    // Change active Pokemon
    player.active_pokemon_index = target_pokemon_index;
    
    bus.push(BattleEvent::PokemonSwitched {
        player_index,
        old_pokemon,
        new_pokemon,
    });
}

/// Execute a single hit of an attack - this is where the move accuracy/damage logic lives
fn execute_attack_hit(
    attacker_index: usize,
    defender_index: usize,
    move_used: Move,
    hit_number: u8,
    battle_state: &mut BattleState,
    _action_stack: &mut ActionStack, // For future multi-hit injection
    bus: &mut EventBus,
    rng: &mut TurnRng,
) {
    let attacker_player = &battle_state.players[attacker_index];
    let attacker_pokemon = attacker_player.team[attacker_player.active_pokemon_index].as_ref()
        .expect("Attacker pokemon should exist");
    
    let defender_player = &battle_state.players[defender_index];
    let defender_pokemon = defender_player.team[defender_player.active_pokemon_index].as_ref()
        .expect("Defender pokemon should exist");
    
    // Generate MoveUsed event (only for first hit)
    if hit_number == 0 {
        bus.push(BattleEvent::MoveUsed {
            player_index: attacker_index,
            pokemon: attacker_pokemon.species,
            move_used,
        });
    }
    
    // Check if move hits
    let hits = move_hits(
        attacker_pokemon,
        defender_pokemon,
        attacker_player,
        defender_player,
        move_used,
        rng,
    );
    
    if hits {
        bus.push(BattleEvent::MoveHit {
            attacker: attacker_pokemon.species,
            defender: defender_pokemon.species,
            move_used,
        });
        
        // TODO: Calculate and apply damage
        // TODO: Apply move effects
        // TODO: Check for faint
    } else {
        bus.push(BattleEvent::MoveMissed {
            attacker: attacker_pokemon.species,
            defender: defender_pokemon.species,
            move_used,
        });
    }
    
    // TODO: Multi-hit moves will inject additional AttackHit actions here
}

pub fn determine_action_order(battle_state: &BattleState) -> Vec<usize> {
    let mut player_priorities = Vec::new();
    
    // Calculate priority for each player's action
    for (player_index, action_opt) in battle_state.action_queue.iter().enumerate() {
        if let Some(action) = action_opt {
            let priority = calculate_action_priority(player_index, action, battle_state);
            player_priorities.push((player_index, priority));
        }
    }
    
    // Sort by priority (higher priority first), then by speed (higher speed first)
    player_priorities.sort_by(|a, b| {
        // First sort by action priority (higher first)
        let priority_cmp = b.1.action_priority.cmp(&a.1.action_priority);
        if priority_cmp != std::cmp::Ordering::Equal {
            return priority_cmp;
        }
        
        // Then by move priority if both are moves (higher first)
        let move_priority_cmp = b.1.move_priority.cmp(&a.1.move_priority);
        if move_priority_cmp != std::cmp::Ordering::Equal {
            return move_priority_cmp;
        }
        
        // Finally by speed (higher first)
        b.1.speed.cmp(&a.1.speed)
    });
    
    // Return the sorted player indices
    player_priorities.into_iter().map(|(player_index, _)| player_index).collect()
}

#[derive(Debug, Clone)]
struct ActionPriority {
    action_priority: i8, // Forfeit: 10, Switch: 6, Move: varies
    move_priority: i8,   // Only relevant for moves
    speed: u16,          // Effective speed for tiebreaking
}

fn calculate_action_priority(player_index: usize, action: &PlayerAction, battle_state: &BattleState) -> ActionPriority {
    match action {
        PlayerAction::SwitchPokemon { .. } => {
            let speed = get_player_speed(player_index, battle_state);
            ActionPriority {
                action_priority: 6, // Switches go first
                move_priority: 0,   // N/A for switches
                speed,
            }
        }
        PlayerAction::Forfeit => {
            ActionPriority {
                action_priority: 10, // Forfeit goes first, before everything else
                move_priority: 0,    // N/A for forfeit
                speed: 0,
            }
        }
        PlayerAction::UseMove { move_index } => {
            let player = &battle_state.players[player_index];
            let active_pokemon = &player.team[player.active_pokemon_index].as_ref()
                .expect("Active pokemon should exist");
            
            let move_instance = &active_pokemon.moves[*move_index].as_ref()
                .expect("Move should exist");
            
            let move_data = get_move_data(move_instance.move_)
                .expect("Move data should exist");
            
            let speed = effective_speed(active_pokemon, player);
            
            // Extract priority from move effects
            let move_priority = move_data.effects.iter()
                .find_map(|effect| match effect {
                    crate::move_data::MoveEffect::Priority(priority) => Some(*priority),
                    _ => None,
                })
                .unwrap_or(0); // Default priority is 0
            
            ActionPriority {
                action_priority: 0,         // Moves go last
                move_priority,
                speed,
            }
        }
    }
}

fn get_player_speed(player_index: usize, battle_state: &BattleState) -> u16 {
    let player = &battle_state.players[player_index];
    if let Some(active_pokemon) = &player.team[player.active_pokemon_index] {
        effective_speed(active_pokemon, player)
    } else {
        0 // Should not happen, but safety fallback
    }
}


fn execute_end_turn_phase(
    _battle_state: &mut BattleState,
    _bus: &mut EventBus,
    _rng: &mut TurnRng,
) {
    // TODO: Apply end-of-turn effects
    // - Status damage (poison, burn)
    // - Condition timers
    // - Field effects
}

fn finalize_turn(battle_state: &mut BattleState, bus: &mut EventBus) {
    // Check win conditions
    // TODO: Implement win condition checking
    
    // Clear action queue
    battle_state.action_queue = [None, None];
    
    // Increment turn number
    battle_state.turn_number += 1;
    
    // Set state back to waiting for actions (unless battle ended)
    if battle_state.game_state == GameState::TurnInProgress {
        battle_state.game_state = GameState::WaitingForBothActions;
    }
    
    bus.push(BattleEvent::TurnEnded);
}