use crate::battle::calculators::calculate_attack_outcome;
use crate::battle::commands::execute_commands_locally;
use crate::battle::conditions::*;
use crate::battle::state::{
    ActionFailureReason, BattleEvent, BattleState, EventBus, GameState, TurnRng,
};
use crate::battle::stats::effective_speed;
use crate::move_data::get_move_data;
use crate::moves::Move;
use crate::player::PlayerAction;
use std::collections::VecDeque;

/// Internal action types for the action stack
/// These represent atomic actions that can be executed during battle resolution
#[derive(Debug, Clone)]
pub enum BattleAction {
    /// Player forfeits the battle
    Forfeit { player_index: usize },

    /// Player switches to a different Pokemon
    Switch {
        player_index: usize,
        target_pokemon_index: usize,
    },

    /// Player uses an item (not yet implemented)
    UseItem {
        player_index: usize,
        item_id: String,
    },

    /// Execute a single hit of a move (for multi-hit moves, multiple actions are pushed)
    AttackHit {
        attacker_index: usize,
        defender_index: usize,
        move_used: Move,
        hit_number: u8, // 0 for single hit, 0,1,2... for multi-hit
    },
}

/// Action stack for managing battle action execution
pub struct ActionStack {
    actions: VecDeque<BattleAction>,
}

impl ActionStack {
    pub fn new() -> Self {
        Self {
            actions: VecDeque::new(),
        }
    }

    pub fn push_back(&mut self, action: BattleAction) {
        self.actions.push_back(action);
    }

    pub fn push_front(&mut self, action: BattleAction) {
        self.actions.push_front(action);
    }

    pub fn pop_front(&mut self) -> Option<BattleAction> {
        self.actions.pop_front()
    }

    fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

/// Generate a forced action if the player has conditions that require it
/// Returns Some(PlayerAction::ForcedMove) if action is forced, None if player can choose
fn check_for_forced_action(
    player: &crate::player::BattlePlayer,
) -> Option<crate::player::PlayerAction> {
    // Check if player has a last move to potentially repeat
    let last_move = player.last_move?;

    // Check for forcing conditions that repeat last move
    let has_forcing_condition = player.active_pokemon_conditions.values().any(|condition| {
        matches!(
            condition,
            PokemonCondition::Charging
                | PokemonCondition::InAir
                | PokemonCondition::Underground
                | PokemonCondition::Rampaging { .. }
                | PokemonCondition::Biding { .. }
        )
    });

    if has_forcing_condition {
        return Some(crate::player::PlayerAction::ForcedMove {
            pokemon_move: last_move,
        });
    }

    // Check for Biding condition - forces Bide action regardless of last move
    if player
        .active_pokemon_conditions
        .values()
        .any(|condition| matches!(condition, PokemonCondition::Biding { .. }))
    {
        return Some(crate::player::PlayerAction::ForcedMove {
            pokemon_move: crate::moves::Move::Bide,
        });
    }

    None
}

/// Prepares a battle state for turn resolution by collecting actions from both players
/// This function should be called before resolve_turn()
/// For now, implements a basic deterministic AI that selects first available move
pub fn collect_player_actions(battle_state: &mut BattleState) -> Result<(), String> {
    match battle_state.game_state {
        GameState::WaitingForBothActions => {
            // Generate actions for any players that haven't provided one
            for player_index in 0..2 {
                if battle_state.action_queue[player_index].is_none() {
                    // First check if the action is forced by conditions
                    let action = if let Some(forced_action) =
                        check_for_forced_action(&battle_state.players[player_index])
                    {
                        forced_action
                    } else {
                        // No forced action, generate deterministic action
                        generate_deterministic_action(&battle_state.players[player_index])?
                    };
                    battle_state.action_queue[player_index] = Some(action);
                }
            }
        }
        GameState::WaitingForPlayer1Replacement => {
            // Player 1 needs to send out a replacement Pokemon
            if battle_state.action_queue[0].is_none() {
                let action = generate_replacement_action(&battle_state.players[0])?;
                battle_state.action_queue[0] = Some(action);
            }
            // Player 2 already has their action or doesn't need one
        }
        GameState::WaitingForPlayer2Replacement => {
            // Player 2 needs to send out a replacement Pokemon
            if battle_state.action_queue[1].is_none() {
                let action = generate_replacement_action(&battle_state.players[1])?;
                battle_state.action_queue[1] = Some(action);
            }
            // Player 1 already has their action or doesn't need one
        }
        GameState::WaitingForBothReplacements => {
            // Both players need to send out replacement Pokemon
            for player_index in 0..2 {
                if battle_state.action_queue[player_index].is_none() {
                    let action = generate_replacement_action(&battle_state.players[player_index])?;
                    battle_state.action_queue[player_index] = Some(action);
                }
            }
        }
        _ => {
            return Err(
                "Cannot collect actions: battle is not in a state that accepts actions".to_string(),
            );
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
    let active_pokemon = player.team[player.active_pokemon_index]
        .as_ref()
        .ok_or("No active pokemon")?;

    // If active Pokemon is fainted, it cannot act
    if active_pokemon.is_fainted() {
        return Err("Active Pokemon is fainted and cannot act".to_string());
    }

    // Find first move with PP > 0
    for (move_index, move_opt) in active_pokemon.moves.iter().enumerate() {
        if let Some(move_instance) = move_opt {
            if move_instance.pp > 0 {
                return Ok(PlayerAction::UseMove { move_index });
            }
        }
    }

    // No moves have PP - force Struggle by returning a UseMove action on the first slot.
    // The orchestrator will convert this to Struggle because its PP is 0.
    Ok(PlayerAction::UseMove { move_index: 0 })
}

/// Generates a replacement Pokemon action for a player with a fainted active Pokemon
fn generate_replacement_action(
    player: &crate::player::BattlePlayer,
) -> Result<PlayerAction, String> {
    // Find first non-fainted Pokemon in team that isn't the current active Pokemon
    for (team_index, pokemon_opt) in player.team.iter().enumerate() {
        if let Some(pokemon) = pokemon_opt {
            if !pokemon.is_fainted() && team_index != player.active_pokemon_index {
                return Ok(PlayerAction::SwitchPokemon { team_index });
            }
        }
    }

    Err("No non-fainted Pokemon available to switch to".to_string())
}

/// Sets a player's action in the battle state
/// This would typically be called by the API layer when a player submits their action
pub fn set_player_action(
    battle_state: &mut BattleState,
    player_index: usize,
    action: PlayerAction,
) -> Result<(), String> {
    if player_index >= 2 {
        return Err("Invalid player index".to_string());
    }

    let valid_states = match player_index {
        0 => matches!(
            battle_state.game_state,
            GameState::WaitingForBothActions
                | GameState::WaitingForPlayer1Replacement
                | GameState::WaitingForBothReplacements
        ),
        1 => matches!(
            battle_state.game_state,
            GameState::WaitingForBothActions
                | GameState::WaitingForPlayer2Replacement
                | GameState::WaitingForBothReplacements
        ),
        _ => false,
    };

    if !valid_states {
        return Err(
            "Cannot set action: battle is not waiting for this player's action".to_string(),
        );
    }

    // Validate that replacement actions are switches to non-fainted Pokemon
    if matches!(
        battle_state.game_state,
        GameState::WaitingForPlayer1Replacement
            | GameState::WaitingForPlayer2Replacement
            | GameState::WaitingForBothReplacements
    ) {
        if let PlayerAction::SwitchPokemon { team_index } = &action {
            let player = &battle_state.players[player_index];
            if let Some(pokemon) = &player.team[*team_index] {
                if pokemon.is_fainted() {
                    return Err(
                        "Cannot switch to fainted Pokemon during forced replacement".to_string()
                    );
                }
            } else {
                return Err("Cannot switch to empty team slot".to_string());
            }
        } else {
            return Err("Only switch actions are allowed during forced replacement".to_string());
        }
    }

    battle_state.action_queue[player_index] = Some(action);
    Ok(())
}

/// Check if battle is ready for turn resolution (both players have provided actions)
pub fn ready_for_turn_resolution(battle_state: &BattleState) -> bool {
    match battle_state.game_state {
        GameState::WaitingForBothActions => {
            battle_state.action_queue[0].is_some() && battle_state.action_queue[1].is_some()
        }
        GameState::WaitingForPlayer1Replacement => battle_state.action_queue[0].is_some(),
        GameState::WaitingForPlayer2Replacement => battle_state.action_queue[1].is_some(),
        GameState::WaitingForBothReplacements => {
            battle_state.action_queue[0].is_some() && battle_state.action_queue[1].is_some()
        }
        _ => false, // Other states are not ready for turn resolution
    }
}

/// Main entry point for turn resolution
/// Takes a battle state and RNG oracle, executes one complete turn
/// Returns EventBus containing all events that occurred during the turn
pub fn resolve_turn(battle_state: &mut BattleState, mut rng: TurnRng) -> EventBus {
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
    bus.push(BattleEvent::TurnStarted {
        turn_number: battle_state.turn_number,
    });
}

/// Build initial action stack from player actions in priority order
fn build_initial_action_stack(battle_state: &BattleState) -> ActionStack {
    let mut stack = ActionStack::new();
    let action_order = determine_action_order(battle_state);

    // Convert PlayerActions to BattleActions and add to stack in priority order
    for &player_index in action_order.iter() {
        if let Some(player_action) = &battle_state.action_queue[player_index] {
            let battle_action =
                convert_player_action_to_battle_action(player_action, player_index, battle_state);
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
            target_pokemon_index: *team_index,
        },

        PlayerAction::UseMove { move_index } => {
            let player = &battle_state.players[player_index];
            let active_pokemon = player.team[player.active_pokemon_index]
                .as_ref()
                .expect("Active pokemon should exist");
            let move_instance = &active_pokemon.moves[*move_index]
                .as_ref()
                .expect("Move should exist");
            let final_move = if move_instance.pp > 0 {
                // The move has PP, use it normally.
                move_instance.move_
            } else {
                // The move has no PP, substitute Struggle.
                Move::Struggle
            };
            // Determine defender
            let defender_index = if player_index == 0 { 1 } else { 0 };

            BattleAction::AttackHit {
                attacker_index: player_index,
                defender_index,
                move_used: final_move,
                hit_number: 0,
            }
        }

        PlayerAction::ForcedMove {
            pokemon_move: forced_move,
        } => {
            // Forced moves bypass PP restrictions and move selection - use the move directly
            let defender_index = if player_index == 0 { 1 } else { 0 };

            BattleAction::AttackHit {
                attacker_index: player_index,
                defender_index,
                move_used: *forced_move,
                hit_number: 0,
            }
        }
    }
}

/// Execute a single battle action, potentially adding more actions to the stack
pub fn execute_battle_action(
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

        BattleAction::Switch {
            player_index,
            target_pokemon_index,
        } => {
            // Check if current Pokemon is fainted (switching away from fainted Pokemon is allowed)
            // But switching TO a fainted Pokemon should not be allowed
            let target_pokemon = &battle_state.players[player_index].team[target_pokemon_index];
            let player = &battle_state.players[player_index];
            if player.has_condition(&PokemonCondition::Trapped { turns_remaining: 0 }) {
                // IMPORTANT: has_condition just checks if we have the condition, so the value of turns_remaining DOES NOT MATTER
                bus.push(BattleEvent::ActionFailed {
                    reason: crate::battle::state::ActionFailureReason::IsTrapped,
                });
                return;
            }
            if let Some(target_pokemon) = target_pokemon {
                if target_pokemon.is_fainted() {
                    // Cannot switch to a fainted Pokemon
                    bus.push(BattleEvent::ActionFailed {
                        reason: crate::battle::state::ActionFailureReason::PokemonFainted,
                    });
                    return;
                }
            }

            execute_switch(player_index, target_pokemon_index, battle_state, bus);
        }

        BattleAction::UseItem { .. } => {
            // TODO: Implement item usage
        }

        BattleAction::AttackHit {
            attacker_index,
            defender_index,
            move_used,
            hit_number,
        } => {
            // Check if attacker is fainted (cannot act)
            let attacker_player = &battle_state.players[attacker_index];
            let attacker_pokemon =
                attacker_player.team[attacker_player.active_pokemon_index].as_ref();

            if let Some(attacker_pokemon) = attacker_pokemon {
                if attacker_pokemon.is_fainted() {
                    // Skip action - attacker has fainted and cannot act
                    bus.push(BattleEvent::ActionFailed {
                        reason: crate::battle::state::ActionFailureReason::PokemonFainted,
                    });
                    return;
                }
            }

            // Check all action-preventing conditions (sleep, freeze, paralysis, confusion, etc.)
            // This needs to happen BEFORE any move processing (including special moves)
            if let Some(failure_reason) = check_action_preventing_conditions(
                attacker_index,
                battle_state,
                rng,
                move_used,
                bus,
            ) {
                // Always generate ActionFailed event first
                bus.push(BattleEvent::ActionFailed {
                    reason: failure_reason.clone(),
                });

                // Special case for confusion - also causes self-damage after the action fails
                if matches!(failure_reason, ActionFailureReason::IsConfused) {
                    // Add confusion self-attack action to the stack
                    action_stack.push_front(BattleAction::AttackHit {
                        attacker_index,
                        defender_index: attacker_index, // Attack self
                        move_used: Move::HittingItself,
                        hit_number: 1, // Hit number > 0 to avoid pp check.
                    });
                }
                return; // Attack is prevented
            }
            if hit_number == 0 {
                let attacker_pokemon = battle_state.players[attacker_index].team
                    [battle_state.players[attacker_index].active_pokemon_index]
                    .as_mut()
                    .expect("Attacker Pokemon must exist to use a move");

                // Directly use the Move from the AttackHit action.
                if let Err(e) = attacker_pokemon.use_move(move_used) {
                    let reason = match e {
                        crate::pokemon::UseMoveError::NoPPRemaining => {
                            crate::battle::state::ActionFailureReason::NoPPRemaining
                        }
                        crate::pokemon::UseMoveError::MoveNotKnown => {
                            crate::battle::state::ActionFailureReason::NoPPRemaining
                        }
                    };
                    bus.push(BattleEvent::ActionFailed { reason });
                    return;
                }

                // Update last move used for conditions that depend on it
                battle_state.players[attacker_index].last_move = Some(move_used);

                // Check if Enraged Pokemon used a move other than Rage - if so, remove Enraged condition
                if battle_state.players[attacker_index].has_condition(&PokemonCondition::Enraged)
                    && move_used != crate::moves::Move::Rage
                {
                    battle_state.players[attacker_index]
                        .remove_condition(&PokemonCondition::Enraged);
                    if let Some(pokemon) = battle_state.players[attacker_index].active_pokemon() {
                        bus.push(BattleEvent::StatusRemoved {
                            target: pokemon.species,
                            status: PokemonCondition::Enraged,
                        });
                    }
                }
            }
            // Perform pre-hit checks on the defender.
            let defender_player = &battle_state.players[defender_index];
            if let Some(defender_pokemon) =
                defender_player.team[defender_player.active_pokemon_index].as_ref()
            {
                let move_data = get_move_data(move_used)
                    .expect("Move data should exist for the executing move");

                if defender_pokemon.is_fainted() {
                    // Target has fainted. Only allow non-offensive moves (e.g., self-buffs).
                    match move_data.category {
                        crate::move_data::MoveCategory::Physical
                        | crate::move_data::MoveCategory::Special
                        | crate::move_data::MoveCategory::Other => {
                            // This is an offensive move against a fainted target. It fails.
                            bus.push(BattleEvent::ActionFailed {
                                reason: crate::battle::state::ActionFailureReason::NoEnemyPresent,
                            });
                            return;
                        }
                        crate::move_data::MoveCategory::Status => {
                            // This is a status move, it can proceed even if the opponent is fainted.
                        }
                    }
                } else {
                    // --- IMMUNITY CHECK ---
                    // Target is not fainted, check for type immunity.
                    let defender_types = defender_pokemon.get_current_types(defender_player);
                    let type_adv_multiplier = crate::battle::stats::get_type_effectiveness(
                        move_data.move_type,
                        &defender_types,
                    );

                    if type_adv_multiplier < 0.01 {
                        // Check for 0.0 immunity
                        match move_data.category {
                            crate::move_data::MoveCategory::Physical
                            | crate::move_data::MoveCategory::Special
                            | crate::move_data::MoveCategory::Other => {
                                // This is an offensive action against an immune target. It fails.
                                // Announce the immunity and stop the action.
                                bus.push(BattleEvent::AttackTypeEffectiveness { multiplier: 0.0 });
                                return;
                            }
                            crate::move_data::MoveCategory::Status => {
                                // Status moves don't target the enemy, so they aren't affected by immunity.
                            }
                        }
                    }
                }
            }

            // If all pre-hit checks pass, check for special move behavior first, then execute the hit.
            let skip_standard_execution = perform_special_move(
                attacker_index,
                defender_index,
                move_used,
                battle_state,
                bus,
                action_stack,
                rng,
            );

            // Only execute standard attack if not handled by user condition effects
            if !skip_standard_execution {
                execute_attack_hit(
                    attacker_index,
                    defender_index,
                    move_used,
                    hit_number,
                    action_stack,
                    bus,
                    rng,
                    battle_state,
                );
            }
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
        winner: Some(if player_index == 0 { 1 } else { 0 }),
    });
}

/// Execute switch action - change active Pokemon
fn execute_switch(
    player_index: usize,
    target_pokemon_index: usize,
    battle_state: &mut BattleState,
    bus: &mut EventBus,
) {
    let player = &mut battle_state.players[player_index];
    let old_pokemon = player.team[player.active_pokemon_index]
        .as_ref()
        .expect("Current active Pokemon should exist")
        .species;
    let new_pokemon = player.team[target_pokemon_index]
        .as_ref()
        .expect("Target Pokemon should exist")
        .species;

    player.clear_active_pokemon_state();

    // Change active Pokemon
    player.active_pokemon_index = target_pokemon_index;

    bus.push(BattleEvent::PokemonSwitched {
        player_index,
        old_pokemon,
        new_pokemon,
    });
}

/// Check all conditions that can prevent a Pokemon from taking action
/// Returns Some(ActionFailureReason) if action should be prevented, None if action can proceed
fn check_action_preventing_conditions(
    player_index: usize,
    battle_state: &mut BattleState,
    rng: &mut TurnRng,
    move_used: Move,
    bus: &mut EventBus,
) -> Option<ActionFailureReason> {
    // Check Pokemon status conditions BEFORE updating counters
    let pokemon_status = battle_state.players[player_index].team
        [battle_state.players[player_index].active_pokemon_index]
        .as_ref()?
        .status;

    // First check if Pokemon should fail to act (including Sleep > 0)
    if let Some(status) = pokemon_status {
        match status {
            crate::pokemon::StatusCondition::Sleep(turns) => {
                if turns > 0 {
                    // Pokemon is still asleep, update counters after determining failure
                    if let Some(pokemon) = battle_state.players[player_index].team
                        [battle_state.players[player_index].active_pokemon_index]
                        .as_mut()
                    {
                        let (should_cure, status_changed) = pokemon.update_status_progress();

                        if should_cure && status_changed {
                            let old_status = pokemon.status; // Save before clearing
                            bus.push(BattleEvent::PokemonStatusRemoved {
                                target: pokemon.species,
                                status: old_status
                                    .unwrap_or(crate::pokemon::StatusCondition::Sleep(0)),
                            });
                        }
                    }
                    return Some(ActionFailureReason::IsAsleep);
                }
            }
            crate::pokemon::StatusCondition::Freeze => {
                // 25% chance to thaw out when trying to act
                let roll = rng.next_outcome(); // 0-100
                if roll < 25 {
                    // Pokemon thaws out
                    if let Some(pokemon_mut) = battle_state.players[player_index].team
                        [battle_state.players[player_index].active_pokemon_index]
                        .as_mut()
                    {
                        let species = pokemon_mut.species;
                        pokemon_mut.status = None;

                        bus.push(BattleEvent::PokemonStatusRemoved {
                            target: species,
                            status: crate::pokemon::StatusCondition::Freeze,
                        });
                    }
                    // Pokemon can act this turn after thawing
                } else {
                    return Some(ActionFailureReason::IsFrozen);
                }
            }
            _ => {} // Other status conditions don't prevent actions
        }
    }

    // Update status counters for Pokemon that are not asleep with turns > 0 (they were handled above)
    let current_status = battle_state.players[player_index].team
        [battle_state.players[player_index].active_pokemon_index]
        .as_ref()?
        .status;

    // Only update counters if Pokemon doesn't have sleep with turns > 0 (those were already updated above)
    let should_update_counters = match current_status {
        Some(crate::pokemon::StatusCondition::Sleep(turns)) => turns == 0,
        _ => true,
    };

    if should_update_counters {
        if let Some(pokemon) = battle_state.players[player_index].team
            [battle_state.players[player_index].active_pokemon_index]
            .as_mut()
        {
            let (should_cure, status_changed) = pokemon.update_status_progress();

            if should_cure && status_changed {
                let old_status = pokemon.status; // Save before clearing
                bus.push(BattleEvent::PokemonStatusRemoved {
                    target: pokemon.species,
                    status: old_status.unwrap_or(crate::pokemon::StatusCondition::Sleep(0)),
                });
            }
        }
    }

    let player = &battle_state.players[player_index];

    // Check active Pokemon conditions
    if player.has_condition(&PokemonCondition::Flinched) {
        return Some(ActionFailureReason::IsFlinching);
    }

    // Check for exhausted condition (any turns_remaining > 0 means still exhausted)
    for condition in player.active_pokemon_conditions.values() {
        if let PokemonCondition::Exhausted { turns_remaining } = condition {
            if *turns_remaining > 0 {
                return Some(ActionFailureReason::IsExhausted);
            }
        }
    }

    // Check paralysis - 25% chance to be fully paralyzed
    if let Some(crate::pokemon::StatusCondition::Paralysis) = pokemon_status {
        let roll = rng.next_outcome(); // 0-100
        if roll < 25 {
            return Some(ActionFailureReason::IsParalyzed);
        }
    }

    // Check confusion - 50% chance to hit self instead
    for condition in player.active_pokemon_conditions.values() {
        if let PokemonCondition::Confused { turns_remaining } = condition {
            if *turns_remaining > 0 {
                let roll = rng.next_outcome(); // 1-100
                if roll < 50 {
                    return Some(ActionFailureReason::IsConfused);
                }
                // If not confused this turn, action proceeds normally
                break; // Only check once
            }
        }
    }

    // Check for disabled moves
    for condition in player.active_pokemon_conditions.values() {
        if let PokemonCondition::Disabled {
            pokemon_move,
            turns_remaining,
        } = condition
        {
            if *turns_remaining > 0 && *pokemon_move == move_used {
                return Some(ActionFailureReason::MoveFailedToExecute);
            }
        }
    }

    // Check for Nightmare effect - move fails unless target is asleep
    if let Some(move_data) = crate::move_data::get_move_data(move_used) {
        for effect in &move_data.effects {
            if matches!(effect, crate::move_data::MoveEffect::Nightmare) {
                // Get the target (enemy) index - if we're player 0, target is 1, and vice versa
                let target_index = if player_index == 0 { 1 } else { 0 };
                let target_player = &battle_state.players[target_index];

                if let Some(target_pokemon) = target_player.active_pokemon() {
                    // Check if target is asleep
                    let is_asleep = matches!(
                        target_pokemon.status,
                        Some(crate::pokemon::StatusCondition::Sleep(_))
                    );

                    if !is_asleep {
                        return Some(ActionFailureReason::MoveFailedToExecute);
                    }
                }
            }
        }
    }

    None // No conditions prevent action
}

/// Apply all chance-based move effects using the new command-based system
fn apply_move_effects(
    attacker_index: usize,
    defender_index: usize,
    move_used: Move,
    battle_state: &mut BattleState,
    bus: &mut EventBus,
    rng: &mut TurnRng,
) {
    use crate::battle::commands::execute_commands_locally;
    use crate::move_data::{EffectContext, get_move_data};

    let move_data = get_move_data(move_used).expect("Move data must exist for effects");

    // The orchestrator creates the simplest possible context.
    let context = EffectContext::new(attacker_index, defender_index, move_used);

    let mut all_commands = Vec::new();

    for effect in &move_data.effects {
        all_commands.extend(effect.apply(&context, battle_state, rng));
    }

    if !all_commands.is_empty() {
        let mut temp_action_stack = ActionStack::new();
        if let Err(error) =
            execute_commands_locally(all_commands, battle_state, bus, &mut temp_action_stack)
        {
            eprintln!("Error executing move effect commands: {:?}", error);
        }
    }
}

/// Apply damage-based effects that always trigger when damage is dealt (recoil, drain)
fn apply_on_damage_effects(
    attacker_index: usize,
    move_used: Move,
    battle_state: &mut BattleState,
    bus: &mut EventBus,
    damage_dealt: u16,
) {
    use crate::battle::commands::execute_commands_locally;
    use crate::move_data::{EffectContext, MoveEffect, get_move_data};
    // Early return if no damage was dealt
    if damage_dealt == 0 {
        return;
    }
    let move_data = get_move_data(move_used).expect("Move data must exist");
    let context = EffectContext::new(attacker_index, 0, move_used);
    // Create context for damage-based effects
    let action_stack = &mut ActionStack::new(); // Temporary action stack

    // Get commands from the new damage-based effects system
    let damage_commands =
        move_data.apply_damage_based_effects(&context, battle_state, damage_dealt);

    // Execute all generated commands
    if !damage_commands.is_empty() {
        if let Err(error) =
            execute_commands_locally(damage_commands, battle_state, bus, action_stack)
        {
            eprintln!("Error executing damage-based effect commands: {:?}", error);
        }
    }
}

/// Apply user-targeted PokemonConditions from moves
/// Returns true if any user condition effects were applied (indicating custom attack behavior)
fn perform_special_move(
    attacker_index: usize,
    defender_index: usize,
    move_used: Move,
    battle_state: &mut BattleState,
    bus: &mut EventBus,
    action_stack: &mut ActionStack,
    rng: &mut TurnRng,
) -> bool {
    let move_data = get_move_data(move_used).expect("Move data must exist");

    for effect in &move_data.effects {
        match effect {
            crate::move_data::MoveEffect::InAir => {
                let attacker_player = &mut battle_state.players[attacker_index];

                // If already in air, this is the second turn - clear condition and proceed with normal attack
                if attacker_player.has_condition(&PokemonCondition::InAir) {
                    attacker_player.remove_condition(&PokemonCondition::InAir);
                    return false;
                }

                // First turn - apply condition and skip normal attack
                if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
                    attacker_player.add_condition(PokemonCondition::InAir);
                    bus.push(BattleEvent::StatusApplied {
                        target: pokemon_species,
                        status: PokemonCondition::InAir,
                    });
                }
                return true;
            }
            crate::move_data::MoveEffect::Teleport(_) => {
                let attacker_player = &mut battle_state.players[attacker_index];
                if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
                    attacker_player.add_condition(PokemonCondition::Teleported);
                    bus.push(BattleEvent::StatusApplied {
                        target: pokemon_species,
                        status: PokemonCondition::Teleported,
                    });
                }
                return true;
            }
            crate::move_data::MoveEffect::ChargeUp => {
                let attacker_player = &mut battle_state.players[attacker_index];

                // If already charging, this is the second turn - clear condition and proceed with normal attack
                if attacker_player.has_condition(&PokemonCondition::Charging) {
                    attacker_player.remove_condition(&PokemonCondition::Charging);
                    return false;
                }

                // First turn - apply condition and skip normal attack
                if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
                    let condition = PokemonCondition::Charging;
                    attacker_player.add_condition(condition.clone());
                    bus.push(BattleEvent::StatusApplied {
                        target: pokemon_species,
                        status: condition,
                    });
                }
                return true;
            }
            crate::move_data::MoveEffect::Underground => {
                let attacker_player = &mut battle_state.players[attacker_index];

                // If already underground, this is the second turn - clear condition and proceed with normal attack
                if attacker_player.has_condition(&PokemonCondition::Underground) {
                    attacker_player.remove_condition(&PokemonCondition::Underground);
                    return false;
                }

                // First turn - apply condition and skip normal attack
                if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
                    attacker_player.add_condition(PokemonCondition::Underground);
                    bus.push(BattleEvent::StatusApplied {
                        target: pokemon_species,
                        status: PokemonCondition::Underground,
                    });
                }
                return true;
            }
            crate::move_data::MoveEffect::Transform => {
                // First, get the data we need without holding references
                let (attacker_species, target_pokemon) = {
                    let attacker_player = &battle_state.players[attacker_index];
                    let defender_player = &battle_state.players[defender_index];
                    (
                        attacker_player.active_pokemon().map(|p| p.species),
                        defender_player.active_pokemon().cloned(),
                    )
                };

                if let (Some(attacker_species), Some(target_pokemon)) =
                    (attacker_species, target_pokemon)
                {
                    let condition = PokemonCondition::Transformed {
                        target: target_pokemon,
                    };
                    let attacker_player = &mut battle_state.players[attacker_index];
                    attacker_player.add_condition(condition.clone());
                    bus.push(BattleEvent::StatusApplied {
                        target: attacker_species,
                        status: condition,
                    });
                }
                return true;
            }
            crate::move_data::MoveEffect::Conversion => {
                // First, get the data we need without holding references
                let (attacker_species, target_type) = {
                    let attacker_player = &battle_state.players[attacker_index];
                    let defender_player = &battle_state.players[defender_index];
                    let attacker_species = attacker_player.active_pokemon().map(|p| p.species);
                    let target_type = defender_player
                        .active_pokemon()
                        .map(|target_pokemon| target_pokemon.get_current_types(defender_player))
                        .and_then(|types| types.into_iter().next()); // Take first type
                    (attacker_species, target_type)
                };

                if let (Some(attacker_species), Some(target_type)) = (attacker_species, target_type)
                {
                    let condition = PokemonCondition::Converted {
                        pokemon_type: target_type,
                    };
                    let attacker_player = &mut battle_state.players[attacker_index];
                    attacker_player.add_condition(condition.clone());
                    bus.push(BattleEvent::StatusApplied {
                        target: attacker_species,
                        status: condition,
                    });
                }
                return true;
            }
            crate::move_data::MoveEffect::Substitute => {
                let attacker_player = &mut battle_state.players[attacker_index];
                if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
                    let pokemon_species = attacker_pokemon.species;
                    // Substitute uses 25% of max HP
                    let substitute_hp = (attacker_pokemon.max_hp() / 4).max(1) as u8;
                    attacker_player
                        .add_condition(PokemonCondition::Substitute { hp: substitute_hp });
                    bus.push(BattleEvent::StatusApplied {
                        target: pokemon_species,
                        status: PokemonCondition::Substitute { hp: substitute_hp },
                    });
                }
                return true;
            }
            crate::move_data::MoveEffect::Counter => {
                let attacker_player = &mut battle_state.players[attacker_index];
                if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
                    attacker_player.add_condition(PokemonCondition::Countering { damage: 0 });
                    bus.push(BattleEvent::StatusApplied {
                        target: pokemon_species,
                        status: PokemonCondition::Countering { damage: 0 },
                    });
                }
                return true;
            }
            crate::move_data::MoveEffect::Rampage(end_condition) => {
                let attacker_player = &mut battle_state.players[attacker_index];
                if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
                    // Rampage lasts 2-3 turns (50/50 chance)
                    let turns = if rng.next_outcome() <= 50 { 2 } else { 3 };
                    let condition = PokemonCondition::Rampaging {
                        turns_remaining: turns,
                    };
                    attacker_player.add_condition(condition.clone());
                    bus.push(BattleEvent::StatusApplied {
                        target: pokemon_species,
                        status: condition,
                    });
                }
                return false;
            }
            crate::move_data::MoveEffect::Rage(_) => {
                let attacker_player = &mut battle_state.players[attacker_index];
                if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
                    attacker_player.add_condition(PokemonCondition::Enraged);
                    bus.push(BattleEvent::StatusApplied {
                        target: pokemon_species,
                        status: PokemonCondition::Enraged,
                    });
                }
                return false;
            }
            crate::move_data::MoveEffect::Bide(turns) => {
                let attacker_player = &mut battle_state.players[attacker_index];

                // Check if already Biding
                if let Some(bide_condition) = attacker_player
                    .active_pokemon_conditions
                    .values()
                    .find_map(|condition| match condition {
                        PokemonCondition::Biding {
                            turns_remaining,
                            damage,
                        } => Some((turns_remaining, damage)),
                        _ => None,
                    })
                {
                    let (turns_remaining, stored_damage) = bide_condition;

                    if *turns_remaining < 1 {
                        // Last turn of Bide - execute stored damage
                        let damage_to_deal = (stored_damage * 2).max(1); // Double damage, minimum 1

                        // Deal the Bide damage to opponent
                        if damage_to_deal > 0 {
                            let defender_player_mut = &mut battle_state.players[defender_index];
                            if let Some(defender_pokemon) = defender_player_mut.team
                                [defender_player_mut.active_pokemon_index]
                                .as_mut()
                            {
                                if !defender_pokemon.is_fainted() {
                                    let did_faint = defender_pokemon.take_damage(damage_to_deal);
                                    let remaining_hp = defender_pokemon.current_hp();

                                    bus.push(BattleEvent::DamageDealt {
                                        target: defender_pokemon.species,
                                        damage: damage_to_deal,
                                        remaining_hp,
                                    });

                                    if did_faint {
                                        bus.push(BattleEvent::PokemonFainted {
                                            player_index: defender_index,
                                            pokemon: defender_pokemon.species,
                                        });
                                    }
                                }
                            }
                        }

                        // Bide condition will be removed by tick_active_conditions at end of turn
                        return true; // Skip normal execution
                    } else {
                        // Still Biding, skip normal execution (do nothing this turn)
                        return true;
                    }
                } else {
                    // Not currently Biding - start new Bide
                    if let Some(pokemon_species) =
                        attacker_player.active_pokemon().map(|p| p.species)
                    {
                        let condition = PokemonCondition::Biding {
                            turns_remaining: *turns,
                            damage: 0,
                        };
                        attacker_player.add_condition(condition.clone());
                        bus.push(BattleEvent::StatusApplied {
                            target: pokemon_species,
                            status: condition,
                        });
                    }
                    return true;
                }
            }
            crate::move_data::MoveEffect::Explode => {
                let attacker_player = &mut battle_state.players[attacker_index];
                if let Some(attacker_pokemon) = attacker_player.active_pokemon_mut() {
                    let pokemon_species = attacker_pokemon.species;
                    // Make the user faint by dealing lethal damage
                    let current_hp = attacker_pokemon.current_hp();
                    let fainted = attacker_pokemon.take_damage(current_hp);

                    bus.push(BattleEvent::DamageDealt {
                        target: pokemon_species,
                        damage: current_hp,
                        remaining_hp: 0,
                    });

                    if fainted {
                        bus.push(BattleEvent::PokemonFainted {
                            player_index: attacker_index,
                            pokemon: pokemon_species,
                        });
                    }
                }
                return false;
            }
            crate::move_data::MoveEffect::MirrorMove => {
                // Mirror Move uses the defender's last move
                let defender_player = &battle_state.players[defender_index];
                if let Some(mirrored_move) = defender_player.last_move {
                    // Don't allow mirroring Mirror Move (would cause infinite recursion)
                    if mirrored_move == Move::MirrorMove {
                        bus.push(BattleEvent::ActionFailed {
                            reason: crate::battle::state::ActionFailureReason::MoveFailedToExecute,
                        });
                        return true; // Skip standard execution since we handled the failure
                    }

                    // Create a BattleAction for the mirrored move and execute it
                    let mirrored_action = BattleAction::AttackHit {
                        attacker_index,
                        defender_index,
                        move_used: mirrored_move,
                        hit_number: 1, // Must be greater than zero to avoid trying to use PP
                    };

                    // Execute the mirrored move with full battle action processing
                    execute_battle_action(mirrored_action, battle_state, action_stack, bus, rng);
                    return true; // Skip standard execution
                }
                // If no move to mirror, fail appropriately
                bus.push(BattleEvent::ActionFailed {
                    reason: crate::battle::state::ActionFailureReason::MoveFailedToExecute,
                });
                return true; // Skip standard execution since we handled the failure
            }
            crate::move_data::MoveEffect::Rest(sleep_turns) => {
                let attacker_player = &mut battle_state.players[attacker_index];

                // Get Pokemon species first
                let pokemon_species = if let Some(pokemon) = attacker_player.active_pokemon() {
                    pokemon.species
                } else {
                    return true;
                };

                // Full heal - restore HP to maximum and apply sleep
                if let Some(attacker_pokemon) = attacker_player.active_pokemon_mut() {
                    let max_hp = attacker_pokemon.max_hp();
                    let current_hp = attacker_pokemon.current_hp();
                    if current_hp < max_hp {
                        let heal_amount = max_hp - current_hp;
                        attacker_pokemon.set_hp_to_max();
                        bus.push(BattleEvent::PokemonHealed {
                            target: pokemon_species,
                            amount: heal_amount,
                            new_hp: max_hp,
                        });
                    }

                    // Apply Sleep status for specified turns
                    attacker_pokemon.status =
                        Some(crate::pokemon::StatusCondition::Sleep(*sleep_turns));
                    bus.push(BattleEvent::PokemonStatusApplied {
                        target: pokemon_species,
                        status: crate::pokemon::StatusCondition::Sleep(*sleep_turns),
                    });
                }

                // Clear all active Pokemon conditions (after releasing the pokemon borrow)
                let cleared_conditions: Vec<_> = attacker_player
                    .active_pokemon_conditions
                    .keys()
                    .cloned()
                    .collect();
                for condition_key in cleared_conditions {
                    if let Some(removed_condition) = attacker_player
                        .active_pokemon_conditions
                        .remove(&condition_key)
                    {
                        bus.push(BattleEvent::ConditionExpired {
                            target: pokemon_species,
                            condition: removed_condition,
                        });
                    }
                }

                return true;
            }
            crate::move_data::MoveEffect::Metronome => {
                // Get all possible moves except Metronome itself
                let all_moves = [
                    // Normal Type
                    Move::Pound,
                    Move::Doubleslap,
                    Move::PayDay,
                    Move::Scratch,
                    Move::Guillotine,
                    Move::SwordsDance,
                    Move::Cut,
                    Move::Bind,
                    Move::Slam,
                    Move::Stomp,
                    Move::Headbutt,
                    Move::HornAttack,
                    Move::FuryAttack,
                    Move::HornDrill,
                    Move::Tackle,
                    Move::BodySlam,
                    Move::Wrap,
                    Move::Harden,
                    Move::TakeDown,
                    Move::Thrash,
                    Move::DoubleEdge,
                    Move::TailWhip,
                    Move::Leer,
                    Move::Bite,
                    Move::Growl,
                    Move::Roar,
                    Move::Sing,
                    Move::Supersonic,
                    Move::SonicBoom,
                    Move::Disable,
                    Move::Agility,
                    Move::QuickAttack,
                    Move::Rage,
                    Move::Mimic,
                    Move::Screech,
                    Move::DoubleTeam,
                    Move::Recover,
                    Move::Minimize,
                    Move::Withdraw,
                    Move::DefenseCurl,
                    Move::Barrier,
                    Move::FocusEnergy,
                    Move::Bide,
                    Move::MirrorMove,
                    Move::SelfDestruct,
                    Move::Clamp,
                    Move::Swift,
                    Move::SpikeCannon,
                    Move::Constrict,
                    Move::SoftBoiled,
                    Move::Glare,
                    Move::Transform,
                    Move::Explosion,
                    Move::FurySwipes,
                    Move::Rest,
                    Move::HyperFang,
                    Move::Sharpen,
                    Move::Conversion,
                    Move::TriAttack,
                    Move::SuperFang,
                    Move::Slash,
                    Move::Substitute,
                    Move::HyperBeam,
                    // Fighting Type
                    Move::KarateChop,
                    Move::CometPunch,
                    Move::MegaPunch,
                    Move::KoPunch,
                    Move::DoubleKick,
                    Move::MegaKick,
                    Move::JumpKick,
                    Move::RollingKick,
                    Move::Submission,
                    Move::LowKick,
                    Move::Counter,
                    Move::SeismicToss,
                    Move::Strength,
                    Move::Meditate,
                    Move::HighJumpKick,
                    Move::Barrage,
                    Move::DizzyPunch,
                    // Flying Type
                    Move::RazorWind,
                    Move::Gust,
                    Move::WingAttack,
                    Move::Whirlwind,
                    Move::Fly,
                    Move::Peck,
                    Move::DrillPeck,
                    Move::SkyAttack,
                    // Rock Type
                    Move::Vicegrip,
                    Move::RockThrow,
                    Move::SkullBash,
                    Move::RockSlide,
                    Move::AncientPower,
                    // Ground Type
                    Move::SandAttack,
                    Move::Earthquake,
                    Move::Fissure,
                    Move::Dig,
                    Move::BoneClub,
                    Move::Bonemerang,
                    // Poison Type
                    Move::PoisonSting,
                    Move::Twineedle,
                    Move::Acid,
                    Move::Toxic,
                    Move::Haze,
                    Move::Smog,
                    Move::Sludge,
                    Move::PoisonJab,
                    Move::PoisonGas,
                    Move::AcidArmor,
                    // Bug Type
                    Move::PinMissile,
                    Move::SilverWind,
                    Move::StringShot,
                    Move::LeechLife,
                    // Fire Type
                    Move::FirePunch,
                    Move::BlazeKick,
                    Move::FireFang,
                    Move::Ember,
                    Move::Flamethrower,
                    Move::WillOWisp,
                    Move::FireSpin,
                    Move::Smokescreen,
                    Move::FireBlast,
                    // Water Type
                    Move::Mist,
                    Move::WaterGun,
                    Move::HydroPump,
                    Move::Surf,
                    Move::Bubblebeam,
                    Move::Waterfall,
                    Move::Bubble,
                    Move::Splash,
                    Move::Bubblehammer,
                    // Grass Type
                    Move::VineWhip,
                    Move::Absorb,
                    Move::MegaDrain,
                    Move::GigaDrain,
                    Move::LeechSeed,
                    Move::Growth,
                    Move::RazorLeaf,
                    Move::Solarbeam,
                    Move::PoisonPowder,
                    Move::StunSpore,
                    Move::SleepPowder,
                    Move::PetalDance,
                    Move::Spore,
                    Move::EggBomb,
                    // Ice Type
                    Move::IcePunch,
                    Move::IceBeam,
                    Move::Blizzard,
                    Move::AuroraBeam,
                    // Electric Type
                    Move::ThunderPunch,
                    Move::Shock,
                    Move::Discharge,
                    Move::ThunderWave,
                    Move::Thunderclap,
                    Move::ChargeBeam,
                    Move::Lightning,
                    Move::Flash,
                    // Psychic Type
                    Move::Confusion,
                    Move::Psybeam,
                    Move::Perplex,
                    Move::Hypnosis,
                    Move::Teleport,
                    Move::ConfuseRay,
                    Move::LightScreen,
                    Move::Reflect,
                    Move::Amnesia,
                    Move::Kinesis,
                    Move::Psychic,
                    Move::Psywave,
                    Move::DreamEater,
                    Move::LovelyKiss,
                    // Ghost Type
                    Move::NightShade,
                    Move::Lick,
                    Move::ShadowBall,
                    // Dragon Type
                    Move::Outrage,
                    Move::DragonRage,
                ];

                // Randomly select a move
                let random_index = (rng.next_outcome() as usize) % all_moves.len();
                let selected_move = all_moves[random_index];

                // Get Pokemon species for event logging
                let pokemon_species =
                    if let Some(pokemon) = battle_state.players[attacker_index].active_pokemon() {
                        pokemon.species
                    } else {
                        return true;
                    };

                // Log the Metronome selection
                bus.push(BattleEvent::MoveUsed {
                    player_index: attacker_index,
                    pokemon: pokemon_species,
                    move_used: selected_move,
                });

                // Create a BattleAction for the selected move and execute it
                let metronome_action = BattleAction::AttackHit {
                    attacker_index,
                    defender_index,
                    move_used: selected_move,
                    hit_number: 1, // Must be greater than zero to avoid trying to use PP
                };

                // Execute the selected move with full battle action processing
                execute_battle_action(metronome_action, battle_state, action_stack, bus, rng);
                return true; // Skip standard execution
            }

            // Other effects are handled elsewhere
            _ => {}
        }
    }

    false // Continue with standard attack execution
}

/// Execute a single hit of an attack
pub fn execute_attack_hit(
    attacker_index: usize,
    defender_index: usize,
    move_used: Move,
    hit_number: u8,
    action_stack: &mut ActionStack, // Used for multi-hit injection
    bus: &mut EventBus,
    rng: &mut TurnRng,
    battle_state: &mut BattleState, // Swapped order to satisfy linter/convention
) {
    // If the defender has already fainted (e.g., from a previous hit in a multi-hit sequence),
    // the subsequent hits should fail immediately.
    if battle_state.players[defender_index].team
        [battle_state.players[defender_index].active_pokemon_index]
        .as_ref()
        .unwrap()
        .is_fainted()
    {
        // We don't even log an ActionFailed event here, because the move sequence just silently stops.
        return;
    }

    // === THE BRIDGE ===
    // Use the new pure calculator for hit/miss logic
    let hit_miss_commands = calculate_attack_outcome(
        battle_state,
        attacker_index,
        defender_index,
        move_used,
        hit_number,
        rng,
    );

    // Extract damage amount from calculator commands BEFORE executing
    let damage = hit_miss_commands
        .iter()
        .find_map(|cmd| match cmd {
            crate::battle::commands::BattleCommand::DealDamage { amount, .. } => Some(*amount),
            _ => None,
        })
        .unwrap_or(0);

    // Execute the commands immediately using local bridge function
    if let Err(e) = execute_commands_locally(hit_miss_commands, battle_state, bus, action_stack) {
        eprintln!("Error executing hit/miss commands: {:?}", e);
        return;
    }

    // Determine if the move hit by checking the current state
    // (This is temporary until we expand the calculator to handle all hit logic)
    let hits = bus
        .events()
        .iter()
        .rev()
        .take(10)
        .any(|event| matches!(event, BattleEvent::MoveHit { .. }));

    // Extract critical hit information from calculator events
    let _is_critical = bus
        .events()
        .iter()
        .rev()
        .take(10)
        .any(|event| matches!(event, BattleEvent::CriticalHit { .. }));

    // Get the player and pokemon references AFTER executing commands
    let attacker_player = &battle_state.players[attacker_index];
    let attacker_pokemon = attacker_player.team[attacker_player.active_pokemon_index]
        .as_ref()
        .expect("Attacker pokemon should exist");

    let defender_player = &battle_state.players[defender_index];
    let defender_pokemon = defender_player.team[defender_player.active_pokemon_index]
        .as_ref()
        .expect("Defender pokemon should exist");

    if hits {
        // === END BRIDGE ===
        // Type effectiveness, critical hit, damage calculation, and substitute logic now handled by calculator

        // Check if damage was absorbed by substitute by looking for 0-damage DamageDealt event
        let damage_absorbed_by_substitute = bus
            .events()
            .iter()
            .rev()
            .take(10)
            .any(|event| matches!(event, BattleEvent::DamageDealt { damage: 0, .. }));

        let defender_fainted = if damage > 0 && !damage_absorbed_by_substitute {
            // Normal damage case - calculator issued DealDamage command, and damage wasn't absorbed by substitute
            false // Fainting will be handled in next iteration
        } else {
            // Either no damage or substitute absorbed it
            false // No fainting in these cases
        };

        // Apply move effects after damage is dealt (for damage moves) or on hit (for Other/Status category moves)
        let move_data = get_move_data(move_used).expect("Move data must exist");
        if damage > 0
            || matches!(
                move_data.category,
                crate::move_data::MoveCategory::Other | crate::move_data::MoveCategory::Status
            )
        {
            apply_move_effects(
                attacker_index,
                defender_index,
                move_used,
                battle_state,
                bus,
                rng,
            );
        }

        // Apply damage-based effects (recoil, drain) when damage was dealt
        if damage > 0 {
            apply_on_damage_effects(attacker_index, move_used, battle_state, bus, damage);
        }

        // If the defender faints, the multi-hit sequence stops.
        if defender_fainted {
            return;
        }

        // --- PROBABILISTIC MULTI-HIT LOGIC ---
        // Arguably this should be incorporated into apply_move_effects?
        let move_data = get_move_data(move_used).expect("Move data must exist");
        for effect in &move_data.effects {
            if let crate::move_data::MoveEffect::MultiHit(guaranteed_hits, continuation_chance) =
                effect
            {
                let next_hit_number = hit_number + 1;

                // Check if we should queue another hit.
                let should_queue_next_hit = if next_hit_number < *guaranteed_hits {
                    // We haven't met the guaranteed number of hits yet, so always continue.
                    true
                } else {
                    // We are past the guaranteed hits, so roll for continuation.
                    rng.next_outcome() <= *continuation_chance
                };

                if should_queue_next_hit {
                    action_stack.push_front(BattleAction::AttackHit {
                        attacker_index,
                        defender_index,
                        move_used,
                        hit_number: next_hit_number,
                    });
                }
                // We found the MultiHit effect, so we don't need to check other effects for this.
                break;
            }
        }
    }
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
    player_priorities
        .into_iter()
        .map(|(player_index, _)| player_index)
        .collect()
}

#[derive(Debug, Clone)]
struct ActionPriority {
    action_priority: i8, // Forfeit: 10, Switch: 6, Move: varies
    move_priority: i8,   // Only relevant for moves
    speed: u16,          // Effective speed for tiebreaking
}

fn calculate_action_priority(
    player_index: usize,
    action: &PlayerAction,
    battle_state: &BattleState,
) -> ActionPriority {
    match action {
        PlayerAction::SwitchPokemon { .. } => {
            ActionPriority {
                action_priority: 6,         // Switches go first
                move_priority: 0,           // N/A for switches
                speed: player_index as u16, // Just always have player 0 switch first if they both switch.
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
            let active_pokemon = &player.team[player.active_pokemon_index]
                .as_ref()
                .expect("Active pokemon should exist");

            let move_instance = &active_pokemon.moves[*move_index]
                .as_ref()
                .expect("Move should exist");

            let move_data = get_move_data(move_instance.move_).expect("Move data should exist");

            let speed = effective_speed(active_pokemon, player);

            // Extract priority from move effects
            let move_priority = move_data
                .effects
                .iter()
                .find_map(|effect| match effect {
                    crate::move_data::MoveEffect::Priority(priority) => Some(*priority),
                    _ => None,
                })
                .unwrap_or(0); // Default priority is 0

            ActionPriority {
                action_priority: 0, // Moves go last
                move_priority,
                speed,
            }
        }

        PlayerAction::ForcedMove {
            pokemon_move: forced_move,
        } => {
            let player = &battle_state.players[player_index];
            let active_pokemon = &player.team[player.active_pokemon_index]
                .as_ref()
                .expect("Active pokemon should exist");

            let move_data = get_move_data(*forced_move).expect("Move data should exist");

            let speed = effective_speed(active_pokemon, player);

            // Extract priority from move effects
            let move_priority = move_data
                .effects
                .iter()
                .find_map(|effect| match effect {
                    crate::move_data::MoveEffect::Priority(priority) => Some(*priority),
                    _ => None,
                })
                .unwrap_or(0); // Default priority is 0

            ActionPriority {
                action_priority: 0, // Forced moves have same priority as regular moves
                move_priority,
                speed,
            }
        }
    }
}

/// Apply damage/healing effects from active Pokemon conditions (Trapped, Seeded)
fn apply_condition_damage(battle_state: &mut BattleState, bus: &mut EventBus) {
    // Process each player's active Pokemon for condition effects
    for player_index in 0..2 {
        let opponent_index = 1 - player_index;

        // First, collect the condition info without borrowing
        let (pokemon_species, max_hp, has_trapped, has_seeded) = {
            let player = &battle_state.players[player_index];
            if let Some(pokemon) = player.active_pokemon() {
                if pokemon.is_fainted() {
                    continue;
                }

                let species = pokemon.species;
                let max_hp = pokemon.max_hp();
                let has_trapped = player
                    .active_pokemon_conditions
                    .values()
                    .any(|condition| matches!(condition, PokemonCondition::Trapped { .. }));
                let has_seeded = player.has_condition(&PokemonCondition::Seeded);

                (species, max_hp, has_trapped, has_seeded)
            } else {
                continue;
            }
        };

        // Handle Trapped condition (1/16 max HP damage per turn)
        if has_trapped {
            let condition_damage = (max_hp / 16).max(1); // 1/16 of max HP, minimum 1

            let pokemon_mut = battle_state.players[player_index].team
                [battle_state.players[player_index].active_pokemon_index]
                .as_mut()
                .unwrap();
            let current_hp = pokemon_mut.current_hp();
            let actual_damage = condition_damage.min(current_hp);
            let fainted = pokemon_mut.take_damage(condition_damage);

            bus.push(BattleEvent::StatusDamage {
                target: pokemon_species,
                status: PokemonCondition::Trapped { turns_remaining: 1 },
                damage: actual_damage,
            });

            if fainted {
                bus.push(BattleEvent::PokemonFainted {
                    player_index,
                    pokemon: pokemon_species,
                });
            }
        }

        // Handle Seeded condition (1/8 max HP drained per turn, heals opponent)
        if has_seeded {
            let pokemon_mut = battle_state.players[player_index].team
                [battle_state.players[player_index].active_pokemon_index]
                .as_mut()
                .unwrap();
            let current_hp = pokemon_mut.current_hp();
            let actual_damage = (max_hp / 8).max(1).min(current_hp);
            let fainted = pokemon_mut.take_damage(actual_damage);

            bus.push(BattleEvent::StatusDamage {
                target: pokemon_species,
                status: PokemonCondition::Seeded,
                damage: actual_damage,
            });

            if fainted {
                bus.push(BattleEvent::PokemonFainted {
                    player_index,
                    pokemon: pokemon_species,
                });
            }
            // Heal the opponent if they have an active Pokemon
            let opponent_player = &mut battle_state.players[opponent_index];
            if let Some(opponent_pokemon) =
                opponent_player.team[opponent_player.active_pokemon_index].as_mut()
            {
                if !opponent_pokemon.is_fainted() {
                    let current_hp = opponent_pokemon.current_hp();
                    let max_hp = opponent_pokemon.max_hp();
                    let actual_heal = actual_damage.min(max_hp.saturating_sub(current_hp));

                    if actual_heal > 0 {
                        opponent_pokemon.heal(actual_heal);
                        bus.push(BattleEvent::PokemonHealed {
                            target: opponent_pokemon.species,
                            amount: actual_heal,
                            new_hp: opponent_pokemon.current_hp(),
                        });
                    }
                }
            }
        }
    }
}

pub fn execute_end_turn_phase(
    battle_state: &mut BattleState,
    bus: &mut EventBus,
    rng: &mut TurnRng,
) {
    for player_index in 0..2 {
        let player = &mut battle_state.players[player_index];
        if let Some(pokemon) = player.team[player.active_pokemon_index].as_mut() {
            // Fainted Pokemon do not take end-of-turn damage or effects.
            if pokemon.is_fainted() {
                continue;
            }

            // 1. Process Pokemon status damage (Poison, Burn)
            let (status_damage, status_changed) = pokemon.deal_status_damage();

            if status_damage > 0 {
                // Generate status damage event
                if let Some(status) = pokemon.status {
                    bus.push(BattleEvent::PokemonStatusDamage {
                        target: pokemon.species,
                        status,
                        damage: status_damage,
                        remaining_hp: pokemon.current_hp(),
                    });
                }
            }
        }

        // 2. Process active Pokemon conditions (outside of pokemon borrow to avoid conflicts)
        let player = &mut battle_state.players[player_index];
        let expired_conditions = player.tick_active_conditions();

        // Generate events for expired conditions
        if let Some(pokemon) = player.active_pokemon() {
            for condition in expired_conditions {
                bus.push(BattleEvent::ConditionExpired {
                    target: pokemon.species,
                    condition,
                });
            }
        }
    }

    // 3. Process condition-based damage effects (Trapped, Seeded)
    apply_condition_damage(battle_state, bus);

    // 4. Tick team conditions (Reflect, Light Screen, Mist)
    for player_index in 0..2 {
        let player = &mut battle_state.players[player_index];
        player.tick_team_conditions();
    }
}

fn finalize_turn(battle_state: &mut BattleState, bus: &mut EventBus) {
    for player_index in 0..2 {
        if let Some(pokemon) = battle_state.players[player_index].active_pokemon() {
            if pokemon.is_fainted() {
                battle_state.players[player_index].clear_active_pokemon_state();
            }
        }
    }
    // Check for win conditions first, as they override the need for replacements.
    check_win_conditions(battle_state, bus);

    // Increment turn number
    battle_state.turn_number += 1;

    // Set state back to waiting for actions (unless battle ended or replacements needed)
    if matches!(battle_state.game_state, GameState::TurnInProgress) {
        battle_state.game_state = GameState::WaitingForBothActions;
    }

    // Now, check if we need to enter a replacement state. This overrides WaitingForBothActions.
    check_for_pending_replacements(battle_state);

    // Clear action queue for the next turn
    battle_state.action_queue = [None, None];

    bus.push(BattleEvent::TurnEnded);
}

/// At the end of the turn, checks if any active Pokemon have fainted and if replacements are needed.
fn check_for_pending_replacements(battle_state: &mut BattleState) {
    // This should only trigger if the battle is still technically ongoing.
    if !matches!(
        battle_state.game_state,
        GameState::Player1Win | GameState::Player2Win | GameState::Draw
    ) {
        let p1_fainted = battle_state.players[0].team[battle_state.players[0].active_pokemon_index]
            .as_ref()
            .map_or(false, |p| p.is_fainted());
        let p1_has_replacement = has_non_fainted_pokemon(&battle_state.players[0]);

        let p2_fainted = battle_state.players[1].team[battle_state.players[1].active_pokemon_index]
            .as_ref()
            .map_or(false, |p| p.is_fainted());
        let p2_has_replacement = has_non_fainted_pokemon(&battle_state.players[1]);

        let p1_needs_replacement = p1_fainted && p1_has_replacement;
        let p2_needs_replacement = p2_fainted && p2_has_replacement;

        let new_game_state = match (p1_needs_replacement, p2_needs_replacement) {
            (true, true) => Some(GameState::WaitingForBothReplacements),
            (true, false) => Some(GameState::WaitingForPlayer1Replacement),
            (false, true) => Some(GameState::WaitingForPlayer2Replacement),
            (false, false) => None,
        };

        if let Some(state) = new_game_state {
            battle_state.game_state = state;
        }
    }
}

/// Check if a player has any non-fainted Pokemon in their team
fn has_non_fainted_pokemon(player: &crate::player::BattlePlayer) -> bool {
    player.team.iter().any(|pokemon_opt| {
        if let Some(pokemon) = pokemon_opt {
            !pokemon.is_fainted()
        } else {
            false
        }
    })
}

/// Check win conditions and update battle state accordingly
fn check_win_conditions(battle_state: &mut BattleState, bus: &mut EventBus) {
    let player1_has_pokemon = has_non_fainted_pokemon(&battle_state.players[0]);
    let player2_has_pokemon = has_non_fainted_pokemon(&battle_state.players[1]);

    match (player1_has_pokemon, player2_has_pokemon) {
        (false, false) => {
            // Both players out of Pokemon - draw
            battle_state.game_state = GameState::Draw;
            bus.push(BattleEvent::BattleEnded { winner: None });
        }
        (false, true) => {
            // Player 1 out of Pokemon - Player 2 wins
            battle_state.game_state = GameState::Player2Win;
            bus.push(BattleEvent::PlayerDefeated { player_index: 0 });
            bus.push(BattleEvent::BattleEnded { winner: Some(1) });
        }
        (true, false) => {
            // Player 2 out of Pokemon - Player 1 wins
            battle_state.game_state = GameState::Player1Win;
            bus.push(BattleEvent::PlayerDefeated { player_index: 1 });
            bus.push(BattleEvent::BattleEnded { winner: Some(0) });
        }
        (true, true) => {
            // Both players have Pokemon - continue battle
            // No change to game state needed here
        }
    }
}
