// in src/battle/engine.rs

// CHANGED: Use statements are updated.
use crate::battle::action_stack::{ActionStack, BattleAction};
use crate::battle::ai::{Behavior, ScoringAI};
use crate::battle::calculators::{calculate_attack_outcome, calculate_end_turn_commands, calculate_forced_action_commands};
use crate::battle::commands::{execute_command_batch, execute_command, BattleCommand, PlayerTarget};
use crate::battle::conditions::*;
use crate::battle::state::{
    ActionFailureReason, BattleEvent, BattleState, EventBus, GameState, TurnRng,
};
use crate::move_data::MoveData;
use crate::moves::Move;
use crate::player::PlayerAction;

// CHANGED: Logic is now simpler. It no longer needs to know about forced moves.
pub fn collect_npc_actions(battle_state: &BattleState) -> Vec<(usize, PlayerAction)> {
    let ai_brain = ScoringAI::new();
    let mut npc_actions = Vec::new();

    let players_to_act = match battle_state.game_state {
        GameState::WaitingForActions | GameState::WaitingForBothReplacements => vec![0, 1],
        GameState::WaitingForPlayer1Replacement => vec![0],
        GameState::WaitingForPlayer2Replacement => vec![1],
        _ => return npc_actions,
    };

    for player_index in players_to_act {
        let player = &battle_state.players[player_index];
        
        // The logic is now much cleaner: if the player is an NPC and their action slot is empty, fill it.
        if player.player_type == crate::player::PlayerType::NPC 
            && battle_state.action_queue[player_index].is_none() {
            let action = ai_brain.decide_action(player_index, battle_state);
            npc_actions.push((player_index, action));
        }
    }
    
    npc_actions
}

/// Check if the battle is ready for turn resolution.
pub fn ready_for_turn_resolution(battle_state: &BattleState) -> bool {
    match battle_state.game_state {
        GameState::WaitingForActions => {
            // The turn can start if and only if both players have a queued action.
            battle_state.action_queue[0].is_some() && battle_state.action_queue[1].is_some()
        }
        GameState::WaitingForPlayer1Replacement => battle_state.action_queue[0].is_some(),
        GameState::WaitingForPlayer2Replacement => battle_state.action_queue[1].is_some(),
        GameState::WaitingForBothReplacements => {
            battle_state.action_queue[0].is_some() && battle_state.action_queue[1].is_some()
        }
        _ => false, // Other states are not ready for turn resolution.
    }
}

/// Main entry point for turn resolution
/// Takes a battle state and RNG oracle, executes one complete turn
/// Returns EventBus containing all events that occurred during the turn
pub fn resolve_turn(battle_state: &mut BattleState, mut rng: TurnRng) -> EventBus {
    let mut bus = EventBus::new();
    
    // We only need one action_stack for the entire resolution process.
    // It is temporary to this function call.
    let mut action_stack = ActionStack::new();

    let is_replacement_phase = matches!(
        battle_state.game_state,
        GameState::WaitingForPlayer1Replacement
            | GameState::WaitingForPlayer2Replacement
            | GameState::WaitingForBothReplacements
    );

    if is_replacement_phase {
        // Pass the single action_stack here as well.
        resolve_replacement_phase(battle_state, &mut bus, &mut action_stack);
    } else {
        initialize_turn(battle_state, &mut bus);

        // Build the initial actions into our single, unified stack.
        let mut action_stack = ActionStack::build_initial(battle_state);

        // The while loop and the execution function now operate on the SAME stack.
        while let Some(action) = action_stack.pop_front() {
            execute_battle_action(action, battle_state, &mut action_stack, &mut bus, &mut rng);

            if battle_state.game_state != GameState::TurnInProgress {
                break;
            }
        }

        if battle_state.game_state == GameState::TurnInProgress {
            let end_turn_commands = calculate_end_turn_commands(battle_state, &mut rng);
            let _ = execute_command_batch(end_turn_commands, battle_state, &mut bus, &mut ActionStack::new());
        }

        // Pass the now-empty stack to finalize_turn.
        finalize_turn(battle_state, &mut bus, &mut action_stack);
    }

    bus
}

/// Handle forced replacement phase without turn progression
fn resolve_replacement_phase(battle_state: &mut BattleState, bus: &mut EventBus, action_stack: &mut ActionStack) {
    let mut turn_action_stack = ActionStack::build_initial(battle_state);

    while let Some(action) = turn_action_stack.pop_front() {
        if matches!(action, BattleAction::Switch { .. }) {
            execute_battle_action(
                action,
                battle_state,
                action_stack, // Pass the main stack
                bus,
                &mut TurnRng::new_for_test(vec![]),
            );
        }
        if matches!(battle_state.game_state, GameState::Player1Win | GameState::Player2Win | GameState::Draw) {
            break;
        }
    }

    check_win_conditions(battle_state, bus);

    if !matches!(battle_state.game_state, GameState::Player1Win | GameState::Player2Win | GameState::Draw) {
        let commands = vec![BattleCommand::SetGameState(GameState::WaitingForActions)];
        let _ = execute_command_batch(commands, battle_state, bus, action_stack);
    }

    let commands = vec![BattleCommand::ClearActionQueue];
    let _ = execute_command_batch(commands, battle_state, bus, action_stack);
}

fn initialize_turn(battle_state: &mut BattleState, bus: &mut EventBus) {
    let commands = vec![BattleCommand::SetGameState(GameState::TurnInProgress)];
    let _ = execute_command_batch(commands, battle_state, bus, &mut ActionStack::new());
    bus.push(BattleEvent::TurnStarted {
        turn_number: battle_state.turn_number,
    });
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

            let commands = execute_switch(player_index, target_pokemon_index, battle_state);
            let _ = execute_command_batch(commands, battle_state, bus, &mut ActionStack::new());
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
    battle_state: &BattleState,
) -> Vec<BattleCommand> {
    let player = &battle_state.players[player_index];
    let old_pokemon = player.active_pokemon().unwrap().species;
    let new_pokemon = player.team[target_pokemon_index].as_ref().unwrap().species;
    let target = PlayerTarget::from_index(player_index);

    vec![
        // 1. Command to clear the old state.
        BattleCommand::ClearPlayerState { target },
        // 2. Command to perform the switch.
        BattleCommand::SwitchPokemon {
            target,
            new_pokemon_index: target_pokemon_index,
        },
        // 3. Command to emit the event.
        BattleCommand::EmitEvent(BattleEvent::PokemonSwitched {
            player_index,
            old_pokemon,
            new_pokemon,
        }),
    ]
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
    }
}


fn finalize_turn(battle_state: &mut BattleState, bus: &mut EventBus, action_stack: &mut ActionStack) {
    // Step 1: Clear state for fainted Pokémon.
    for player_index in 0..2 {
        if let Some(pokemon) = battle_state.players[player_index].active_pokemon() {
            if pokemon.is_fainted() {
                let _ = execute_command(
                    BattleCommand::ClearPlayerState { target: PlayerTarget::from_index(player_index) },
                    battle_state, bus, action_stack,
                );
            }
        }
    }
    
    // Step 2: Check for win/loss conditions.
    check_win_conditions(battle_state, bus);

    // Step 3: Increment turn number if the battle is ongoing.
    if matches!(battle_state.game_state, GameState::TurnInProgress) {
        let _ = execute_command(BattleCommand::IncrementTurnNumber, battle_state, bus, action_stack);
    }
    
    // Step 4: Clear the action queue from the completed turn.
    let _ = execute_command(BattleCommand::ClearActionQueue, battle_state, bus, action_stack);

    // Step 5: If the battle hasn't ended, set the state to wait for the next set of actions.
    if !matches!(battle_state.game_state, GameState::Player1Win | GameState::Player2Win | GameState::Draw) {
        let _ = execute_command(BattleCommand::SetGameState(GameState::WaitingForActions), battle_state, bus, action_stack);
    }

    // Step 6: Check if any Pokémon fainted and require replacements, overriding the previous state if so.
    check_for_pending_replacements(battle_state, bus);

    // Step 7: NEW! Prepare the (now empty) action queue for the *next* turn by injecting forced moves.
    let forced_action_commands = calculate_forced_action_commands(battle_state);
    let _ = execute_command_batch(forced_action_commands, battle_state, bus, action_stack);

    // Step 8: Announce the end of the turn.
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
        let p1_has_replacement = battle_state.players[0].can_still_battle();

        let p2_fainted = battle_state.players[1].team[battle_state.players[1].active_pokemon_index]
            .as_ref()
            .map_or(false, |p| p.is_fainted());
        let p2_has_replacement = battle_state.players[1].can_still_battle();

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

/// Check win conditions and update battle state accordingly
fn check_win_conditions(battle_state: &mut BattleState, bus: &mut EventBus) {
    let player1_has_pokemon = battle_state.players[0].can_still_battle();
    let player2_has_pokemon = battle_state.players[1].can_still_battle();

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
