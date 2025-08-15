use crate::battle::ai::{Behavior, ScoringAI};
use crate::battle::calculators::calculate_attack_outcome;
use crate::battle::commands::{execute_command_batch, execute_command, BattleCommand, PlayerTarget};
use crate::battle::conditions::*;
use crate::battle::state::{
    ActionFailureReason, BattleEvent, BattleState, EventBus, GameState, TurnRng,
};
use crate::battle::stats::effective_speed;
use crate::move_data::MoveData;
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
}

/// Check if the player has conditions that force a specific move
/// Returns Some(Move) if a move is forced, None if player can choose freely
fn check_for_forced_move(player: &crate::player::BattlePlayer) -> Option<crate::moves::Move> {
    // Check for Biding condition - forces Bide action regardless of last move
    if player
        .active_pokemon_conditions
        .values()
        .any(|condition| matches!(condition, PokemonCondition::Biding { .. }))
    {
        return Some(crate::moves::Move::Bide);
    }

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
        )
    });

    if has_forcing_condition {
        return Some(last_move);
    }

    None
}

/// Prepares a battle state for turn resolution by collecting actions from both players
/// This function should be called before resolve_turn()
pub fn collect_player_actions(
    battle_state: &mut BattleState,
) -> Result<(), String> {
    let ai_brain = ScoringAI::new();

    let players_to_act = match battle_state.game_state {
        GameState::WaitingForActions | GameState::WaitingForBothReplacements => vec![0, 1],
        GameState::WaitingForPlayer1Replacement => vec![0],
        GameState::WaitingForPlayer2Replacement => vec![1],
        _ => return Ok(()),
    };

    for player_index in players_to_act {
        // --- LOGIC ---
        // 1. Check if an action is already queued for this player.
        // 2. Check if this player has a forced move.
        // If either is true, we do nothing and let the engine handle it later.
        if battle_state.action_queue[player_index].is_none() 
            && check_for_forced_move(&battle_state.players[player_index]).is_none() {
            
            // This player is free to choose an action.
            // For now, we only have an AI to make this choice. In a real game,
            // this is where you'd wait for human input for player 0.
            let action = ai_brain.decide_action(player_index, battle_state);
            println!("Chosen Action for player {}: {}", player_index, action);
            battle_state.action_queue[player_index] = Some(action);
        }
    }

    Ok(())
}

/// Validates a player action for detailed correctness
/// Checks move PP, bounds, switch targets, etc.
#[allow(dead_code)]
pub fn validate_player_action(
    battle_state: &BattleState,
    player_index: usize,
    action: &PlayerAction,
) -> Result<(), String> {
    if player_index >= 2 {
        return Err("Invalid player index".to_string());
    }

    let player = &battle_state.players[player_index];

    match action {
        PlayerAction::UseMove { move_index } => {
            // Check if player has an active Pokemon
            let pokemon = player
                .active_pokemon()
                .ok_or_else(|| "No active Pokemon".to_string())?;

            // Check if move index is valid
            if *move_index >= pokemon.moves.len() {
                return Err("Invalid move index".to_string());
            }

            // Check if move exists and has PP
            if let Some(move_instance) = &pokemon.moves[*move_index] {
                // Allow moves without pp -- they will just become Struggle.
                // if move_instance.pp == 0 {
                //     return Err("Move has no PP remaining".to_string());
                // }
                if player.active_pokemon_conditions.values().any(|cond| {
                        matches!(cond, PokemonCondition::Disabled { pokemon_move, .. } if *pokemon_move == move_instance.move_)
                    }) {
                        return Err("Move is disabled".to_string());
                    }
            } else {
                return Err("No move in that slot".to_string());
            }
        }
        PlayerAction::SwitchPokemon { team_index } => {
            // Check if target Pokemon exists
            if *team_index >= player.team.len() {
                return Err("Invalid Pokemon index".to_string());
            }
            if player.has_condition(&PokemonCondition::Trapped { turns_remaining: 0 }) {  // The number does not matter
                return Err("The pokemon is trapped!".to_string());
            }
            // Check if target Pokemon is not fainted and not already active
            if let Some(target_pokemon) = &player.team[*team_index] {
                if target_pokemon.is_fainted() {
                    return Err("Cannot switch to fainted Pokemon".to_string());
                }
                if *team_index == player.active_pokemon_index {
                    return Err("Pokemon is already active".to_string());
                }
            } else {
                return Err("No Pokemon in that team slot".to_string());
            }
        }
        PlayerAction::Forfeit => {
            // Forfeit is always valid if the game accepts actions
        }
    }

    Ok(())
}

pub fn get_valid_actions(state: &BattleState, player_index: usize) -> Vec<PlayerAction> {
    let player = &state.players[player_index];
    let mut actions = Vec::new();

    // --- Phase 1: Check for Forced Replacement ---
    // If a player's active Pokémon has fainted, their only valid action is to switch.
    let is_replacement_phase = match state.game_state {
        GameState::WaitingForPlayer1Replacement => player_index == 0,
        GameState::WaitingForPlayer2Replacement => player_index == 1,
        GameState::WaitingForBothReplacements => true,
        _ => false,
    };

    if is_replacement_phase {
        for (i, pokemon_slot) in player.team.iter().enumerate() {
            if let Some(pokemon) = pokemon_slot {
                // The only valid switch targets are non-fainted Pokémon that are not already active.
                if i != player.active_pokemon_index && !pokemon.is_fainted() {
                    actions.push(PlayerAction::SwitchPokemon { team_index: i });
                }
            }
        }
        // During a replacement phase, switching is the ONLY valid action type.
        // If this list is empty, it means the player has no valid Pokémon to switch to,
        // and has therefore lost. The main game loop will detect this win condition.
        return actions;
    }

    // --- Phase 2: Standard Turn Action Generation ---

    // A. Generate "Use Move" Actions
    if let Some(active_pokemon) = player.active_pokemon() {
        // A player cannot use moves if they are recharging (e.g., after Hyper Beam).
        let can_use_moves = !player.has_condition(&PokemonCondition::Exhausted { turns_remaining: 0 }) && !active_pokemon.is_fainted();

        if can_use_moves {
            // First, find all moves that are actually usable (have PP and are not disabled).
            let usable_moves: Vec<PlayerAction> = active_pokemon.moves.iter().enumerate()
                .filter_map(|(i, slot)| {
                    slot.as_ref().and_then(|inst| {
                        let has_pp = inst.pp > 0;
                        let is_disabled = player.active_pokemon_conditions.values().any(|cond| {
                            matches!(cond, PokemonCondition::Disabled { pokemon_move, .. } if *pokemon_move == inst.move_)
                        });

                        if has_pp && !is_disabled {
                            Some(PlayerAction::UseMove { move_index: i })
                        } else {
                            None
                        }
                    })
                })
                .collect();

            if !usable_moves.is_empty() {
                // If there are usable moves, they are the valid options.
                actions.extend(usable_moves);
            } else {
                // If no moves are usable (all are 0 PP or disabled), the player's only
                // "Fight" option is Struggle. We represent this intent by adding a single,
                // default move action. The turn orchestrator will interpret this as Struggle.
                actions.push(PlayerAction::UseMove { move_index: 0 });
            }
        }
    }

    // B. Generate "Switch Pokemon" Actions
    let is_trapped = player.has_condition(&PokemonCondition::Trapped { turns_remaining: 0 });
    if !is_trapped {
        for (i, pokemon_slot) in player.team.iter().enumerate() {
            if let Some(pokemon) = pokemon_slot {
                if i != player.active_pokemon_index && !pokemon.is_fainted() {
                    actions.push(PlayerAction::SwitchPokemon { team_index: i });
                }
            }
        }
    }

    // C. Add "Forfeit" Action
    actions.push(PlayerAction::Forfeit);

    actions
}

/// Check if battle is ready for turn resolution (both players have provided actions)
pub fn ready_for_turn_resolution(battle_state: &BattleState) -> bool {
    match battle_state.game_state {
        GameState::WaitingForActions => {
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

    // Check if this is a replacement phase (inter-turn action)
    let is_replacement_phase = matches!(
        battle_state.game_state,
        GameState::WaitingForPlayer1Replacement
            | GameState::WaitingForPlayer2Replacement
            | GameState::WaitingForBothReplacements
    );

    if is_replacement_phase {
        // Handle forced replacements without turn progression
        resolve_replacement_phase(battle_state, &mut bus);
    } else {
        // Normal battle turn processing
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
        finalize_turn(battle_state, &mut bus, &mut action_stack);
    }

    bus
}

/// Handle forced replacement phase without turn progression
fn resolve_replacement_phase(battle_state: &mut BattleState, bus: &mut EventBus) {
    // Build action stack for replacement actions only
    let mut action_stack = build_initial_action_stack(battle_state);

    // Execute replacement actions
    while let Some(action) = action_stack.pop_front() {
        // Only process switch actions during replacement phase
        if matches!(action, BattleAction::Switch { .. }) {
            execute_battle_action(
                action,
                battle_state,
                &mut action_stack,
                bus,
                &mut TurnRng::new_for_test(vec![]),
            );
        }

        // Check if battle ended (all Pokemon fainted, etc.)
        if matches!(
            battle_state.game_state,
            GameState::Player1Win | GameState::Player2Win | GameState::Draw
        ) {
            break;
        }
    }

    // After replacements, check win conditions and set next state
    check_win_conditions(battle_state, bus);

    // If battle is still ongoing, transition to waiting for actions
    if !matches!(
        battle_state.game_state,
        GameState::Player1Win | GameState::Player2Win | GameState::Draw
    ) {
        let commands = vec![BattleCommand::SetGameState(GameState::WaitingForActions)];
        let _ = execute_command_batch(commands, battle_state, bus, &mut ActionStack::new());
    }

    // Clear action queue for next turn
    let commands = vec![BattleCommand::ClearActionQueue];
    let _ = execute_command_batch(commands, battle_state, bus, &mut ActionStack::new());
}

fn initialize_turn(battle_state: &mut BattleState, bus: &mut EventBus) {
    let commands = vec![BattleCommand::SetGameState(GameState::TurnInProgress)];
    let _ = execute_command_batch(commands, battle_state, bus, &mut ActionStack::new());
    bus.push(BattleEvent::TurnStarted {
        turn_number: battle_state.turn_number,
    });
}

/// Build initial action stack from player actions in priority order
fn build_initial_action_stack(battle_state: &BattleState) -> ActionStack {
    let mut stack = ActionStack::new();
    
    // --- START NEW LOGIC FOR ACTION QUEUEING ---
    let mut actions_to_prioritize: Vec<(usize, PlayerAction)> = Vec::new();

    for player_index in 0..2 {
        // Check if the player is forced to make a move. This takes highest priority.
        if let Some(forced_move) = check_for_forced_move(&battle_state.players[player_index]) {
            // The player is forced. We generate the action for them.
            // We find the move_index, defaulting to 0 as a safe fallback.
            let move_index = battle_state.players[player_index].active_pokemon()
                .and_then(|p| p.moves.iter().position(|m| m.as_ref().map_or(false, |inst| inst.move_ == forced_move)))
                .unwrap_or(0);
            
            actions_to_prioritize.push((player_index, PlayerAction::UseMove { move_index }));

        } else if let Some(action) = &battle_state.action_queue[player_index] {
            // The player is not forced, so we use their chosen action from the queue.
            actions_to_prioritize.push((player_index, action.clone()));
        }
    }
    // --- END NEW LOGIC ---

    // Now, determine the order of the collected actions based on priority and speed.
    let action_order = determine_action_order(battle_state, &actions_to_prioritize);

    for (player_index, player_action) in action_order {
        let battle_action =
            convert_player_action_to_battle_action(&player_action, player_index, battle_state);
        stack.push_back(battle_action);
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
            let active_pokemon = player.active_pokemon().expect("Active pokemon should exist");
            
            // Simplified logic: The correct move_index is now guaranteed.
            let final_move = active_pokemon.moves[*move_index]
                .as_ref()
                .map(|inst| {
                    if inst.pp > 0 {
                        inst.move_
                    } else {
                        Move::Struggle
                    }
                })
                .unwrap_or(Move::Struggle); // Use struggle if the move slot is empty

            BattleAction::AttackHit {
                attacker_index: player_index,
                defender_index: 1 - player_index,
                move_used: final_move,
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
                action_stack,
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
                execute_command(
                    BattleCommand::SetLastMove {
                        target: PlayerTarget::from_index(attacker_index),
                        move_used,
                    },
                    battle_state,
                    bus,
                    action_stack,
                )
                .expect("SetLastMove command should always succeed");

                // Check if Enraged Pokemon used a move other than Rage - if so, remove Enraged condition
                if battle_state.players[attacker_index].has_condition(&PokemonCondition::Enraged)
                    && move_used != crate::moves::Move::Rage
                {
                    if let Some(pokemon) = battle_state.players[attacker_index].active_pokemon() {
                        execute_command(
                            BattleCommand::EmitEvent(BattleEvent::StatusRemoved {
                                target: pokemon.species,
                                status: PokemonCondition::Enraged,
                            }),
                            battle_state,
                            bus,
                            action_stack,
                        )
                        .expect("EmitEvent command should always succeed");
                    }
                    execute_command(
                        BattleCommand::RemoveCondition {
                            target: PlayerTarget::from_index(attacker_index),
                            condition_type: PokemonConditionType::Enraged,
                        },
                        battle_state,
                        bus,
                        action_stack,
                    )
                    .expect("RemoveCondition command should always succeed");
                }
            }
            // Perform pre-hit checks on the defender.
            let defender_player = &battle_state.players[defender_index];
            if let Some(defender_pokemon) =
                defender_player.team[defender_player.active_pokemon_index].as_ref()
            {
                let move_data = MoveData::get_move_data(move_used)
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

/// Execute forfeit action - player loses immediately
fn execute_forfeit(player_index: usize, battle_state: &mut BattleState, bus: &mut EventBus) {
    // Set game state to opponent wins
    let new_state = if player_index == 0 {
        GameState::Player2Win
    } else {
        GameState::Player1Win
    };
    let commands = vec![BattleCommand::SetGameState(new_state)];
    let _ = execute_command_batch(commands, battle_state, bus, &mut ActionStack::new());

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
    action_stack: &mut ActionStack,
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
                let roll = rng.next_outcome("Defrost Check"); // 0-100
                if roll < 25 {
                    // Pokemon thaws out
                    if let Some(pokemon) = battle_state.players[player_index].active_pokemon() {
                        execute_command(
                            BattleCommand::EmitEvent(BattleEvent::PokemonStatusRemoved {
                                target: pokemon.species,
                                status: crate::pokemon::StatusCondition::Freeze,
                            }),
                            battle_state,
                            bus,
                            action_stack,
                        )
                        .expect("EmitEvent command should always succeed");
                    }
                    execute_command(
                        BattleCommand::SetPokemonStatus {
                            target: PlayerTarget::from_index(player_index),
                            status: None,
                        },
                        battle_state,
                        bus,
                        action_stack,
                    )
                    .expect("SetPokemonStatus command should always succeed");
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
        let roll = rng.next_outcome("Immobilized by Paralysis Check"); // 0-100
        if roll < 25 {
            return Some(ActionFailureReason::IsParalyzed);
        }
    }

    // Check confusion - 50% chance to hit self instead
    for condition in player.active_pokemon_conditions.values() {
        if let PokemonCondition::Confused { turns_remaining } = condition {
            if *turns_remaining > 0 {
                let roll = rng.next_outcome("Hit Itself in Confusion Check"); // 1-100
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
    if let Some(move_data) = MoveData::get_move_data(move_used) {
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

/// Execute a single hit of an attack
pub fn execute_attack_hit(
    attacker_index: usize,
    defender_index: usize,
    move_used: Move,
    hit_number: u8,
    action_stack: &mut ActionStack,
    bus: &mut EventBus,
    rng: &mut TurnRng,
    battle_state: &mut BattleState,
) {
    // 1. Guard Clause: If the defender is already fainted (from a previous hit in a
    //    multi-hit sequence), the entire action is silently stopped.
    if battle_state.players[defender_index]
        .active_pokemon()
        .map_or(true, |p| p.is_fainted())
    {
        return;
    }

    // 2. Calculation: Delegate ALL game logic to the pure calculator function.
    //    This single call determines everything that should happen as a result of the attack.
    let commands = calculate_attack_outcome(
        battle_state,
        attacker_index,
        defender_index,
        move_used,
        hit_number,
        rng,
    );

    // 3. Execution: Pass the resulting list of commands to the executor bridge.
    //    This step applies all the calculated state changes and emits all events.
    if let Err(e) = execute_command_batch(commands, battle_state, bus, action_stack) {
        eprintln!("Error executing attack commands: {:?}", e);
        // In a real application, this might warrant more robust error handling.
    }
}

pub fn determine_action_order<'a>(
    battle_state: &'a BattleState,
    actions: &'a [(usize, PlayerAction)],
) -> Vec<(usize, PlayerAction)> {
    let mut player_priorities = Vec::new();

    // Calculate priority for each player's action from the provided list.
    for (player_index, action) in actions {
        let priority = calculate_action_priority(*player_index, action, battle_state);
        player_priorities.push((*player_index, action.clone(), priority));
    }

    // Sort by priority (higher priority first), then by speed (higher speed first)
    player_priorities.sort_by(|a, b| {
        let priority_cmp = b.2.action_priority.cmp(&a.2.action_priority);
        if priority_cmp != std::cmp::Ordering::Equal {
            return priority_cmp;
        }

        let move_priority_cmp = b.2.move_priority.cmp(&a.2.move_priority);
        if move_priority_cmp != std::cmp::Ordering::Equal {
            return move_priority_cmp;
        }

        b.2.speed.cmp(&a.2.speed)
    });

    // Return the sorted (player_index, PlayerAction) tuples.
    player_priorities
        .into_iter()
        .map(|(player_index, action, _)| (player_index, action))
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

            let move_data = MoveData::get_move_data(move_instance.move_).expect("Move data should exist");

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
    _rng: &mut TurnRng,
) {
    for player_index in 0..2 {
        let player = &mut battle_state.players[player_index];
        if let Some(pokemon) = player.team[player.active_pokemon_index].as_mut() {
            // Fainted Pokemon do not take end-of-turn damage or effects.
            if pokemon.is_fainted() {
                continue;
            }

            // 1. Process Pokemon status damage (Poison, Burn)
            let (status_damage, _) = pokemon.deal_status_damage();

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

fn finalize_turn(battle_state: &mut BattleState, bus: &mut EventBus, action_stack: &mut ActionStack) {
    // 1. Clear state for any fainted Pokémon
    for player_index in 0..2 {
        if let Some(pokemon) = battle_state.players[player_index].active_pokemon() {
            if pokemon.is_fainted() {
                execute_command(
                    BattleCommand::ClearPlayerState {
                        target: PlayerTarget::from_index(player_index),
                    },
                    battle_state,
                    bus,
                    action_stack,
                )
                .expect("ClearPlayerState command should always succeed");
            }
        }
    }
    
    // 2. Check for win conditions, which override everything else
    check_win_conditions(battle_state, bus);

    // 3. Increment turn number if it was a real battle turn
    if matches!(battle_state.game_state, GameState::TurnInProgress) {
        let commands = vec![BattleCommand::IncrementTurnNumber];
        let _ = execute_command_batch(commands, battle_state, bus, &mut ActionStack::new());
    }

    // 4. Set the default next state if the battle is ongoing
    if matches!(battle_state.game_state, GameState::TurnInProgress) {
        let commands = vec![BattleCommand::SetGameState(GameState::WaitingForActions)];
        let _ = execute_command_batch(commands, battle_state, bus, &mut ActionStack::new());
    }

    // 5. Check if the default state needs to be overridden by a replacement phase
    check_for_pending_replacements(battle_state, bus);

    // 6. Clear the action queue from the turn that just ended
    let commands = vec![BattleCommand::ClearActionQueue];
    let _ = execute_command_batch(commands, battle_state, bus, &mut ActionStack::new());

    // 7. Announce the end of the turn
    bus.push(BattleEvent::TurnEnded);
}

/// At the end of the turn, checks if any active Pokemon have fainted and if replacements are needed.
fn check_for_pending_replacements(battle_state: &mut BattleState, bus: &mut EventBus) {
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
            let commands = vec![BattleCommand::SetGameState(state)];
            let _ = execute_command_batch(commands, battle_state, bus, &mut ActionStack::new());
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
            let commands = vec![BattleCommand::SetGameState(GameState::Draw)];
            let _ = execute_command_batch(commands, battle_state, bus, &mut ActionStack::new());
            bus.push(BattleEvent::BattleEnded { winner: None });
        }
        (false, true) => {
            // Player 1 out of Pokemon - Player 2 wins
            let commands = vec![BattleCommand::SetGameState(GameState::Player2Win)];
            let _ = execute_command_batch(commands, battle_state, bus, &mut ActionStack::new());
            bus.push(BattleEvent::PlayerDefeated { player_index: 0 });
            bus.push(BattleEvent::BattleEnded { winner: Some(1) });
        }
        (true, false) => {
            // Player 2 out of Pokemon - Player 1 wins
            let commands = vec![BattleCommand::SetGameState(GameState::Player1Win)];
            let _ = execute_command_batch(commands, battle_state, bus, &mut ActionStack::new());
            bus.push(BattleEvent::PlayerDefeated { player_index: 1 });
            bus.push(BattleEvent::BattleEnded { winner: Some(0) });
        }
        (true, true) => {
            // Both players have Pokemon - continue battle
            // No change to game state needed here
        }
    }
}
