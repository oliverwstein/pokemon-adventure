use crate::battle::state::{BattleState, EventBus, TurnRng, GameState, BattleEvent};
use crate::player::PlayerAction;

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
    
    // 2. Action Prioritization  
    let action_order = determine_action_order(battle_state);
    
    // 3. Execute Actions in Order
    execute_switch_phase(battle_state, &action_order, &mut bus, &mut rng);
    execute_item_phase(battle_state, &action_order, &mut bus, &mut rng);
    execute_move_phase(battle_state, &action_order, &mut bus, &mut rng);
    
    // 4. End-of-Turn Phase
    execute_end_turn_phase(battle_state, &mut bus, &mut rng);
    
    // 5. Cleanup & Finalization
    finalize_turn(battle_state, &mut bus);
    
    bus
}

fn initialize_turn(battle_state: &mut BattleState, bus: &mut EventBus) {
    battle_state.game_state = GameState::TurnInProgress;
    bus.push(BattleEvent::TurnStarted { turn_number: battle_state.turn_number });
}

fn determine_action_order(battle_state: &BattleState) -> Vec<usize> {
    // TODO: Implement action prioritization logic
    // - Switch actions first
    // - Item actions second  
    // - Move actions by priority, then speed
    vec![0, 1] // Placeholder
}

fn execute_switch_phase(
    battle_state: &mut BattleState,
    action_order: &[usize],
    bus: &mut EventBus,
    rng: &mut TurnRng,
) {
    // TODO: Handle switch actions
}

fn execute_item_phase(
    battle_state: &mut BattleState,
    action_order: &[usize], 
    bus: &mut EventBus,
    rng: &mut TurnRng,
) {
    // TODO: Handle item usage (not yet implemented)
}

fn execute_move_phase(
    battle_state: &mut BattleState,
    action_order: &[usize],
    bus: &mut EventBus, 
    rng: &mut TurnRng,
) {
    // TODO: Handle move execution
    // This will be the most complex phase
}

fn execute_end_turn_phase(
    battle_state: &mut BattleState,
    bus: &mut EventBus,
    rng: &mut TurnRng,
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