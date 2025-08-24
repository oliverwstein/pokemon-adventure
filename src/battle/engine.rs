// in src/battle/engine.rs

use crate::battle::action_stack::{ActionStack, BattleAction};
use crate::battle::ai::{Behavior, ScoringAI};
use crate::battle::calculators::{
    calculate_action_prevention, calculate_attack_outcome, calculate_end_turn_commands,
    calculate_forced_action_commands, calculate_forfeit_commands, calculate_switch_commands,
};
use crate::battle::commands::{
    execute_command, execute_command_batch, BattleCommand, PlayerTarget,
};
use crate::battle::conditions::*;
use crate::battle::state::{
    ActionFailureReason, BattleEvent, BattleState, EventBus, GameState, TurnRng,
};
use crate::move_data::MoveData;
use crate::moves::Move;
use crate::player::PlayerAction;

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
            && battle_state.action_queue[player_index].is_none()
        {
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
            let _ = execute_command_batch(
                end_turn_commands,
                battle_state,
                &mut bus,
                &mut ActionStack::new(),
            );
        }

        // Pass the now-empty stack to finalize_turn.
        finalize_turn(battle_state, &mut bus, &mut action_stack);
    }

    bus
}

/// Handle forced replacement phase without turn progression
fn resolve_replacement_phase(
    battle_state: &mut BattleState,
    bus: &mut EventBus,
    action_stack: &mut ActionStack,
) {
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
        if matches!(
            battle_state.game_state,
            GameState::Player1Win | GameState::Player2Win | GameState::Draw
        ) {
            break;
        }
    }

    check_win_conditions(battle_state, bus);

    if !matches!(
        battle_state.game_state,
        GameState::Player1Win | GameState::Player2Win | GameState::Draw
    ) {
        let commands = vec![BattleCommand::SetGameState(GameState::WaitingForActions)];
        let _ = execute_command_batch(commands, battle_state, bus, action_stack);
    }

    let commands = vec![BattleCommand::ClearActionQueue];
    let _ = execute_command_batch(commands, battle_state, bus, action_stack);

    // Inject forced actions after replacement, just like in finalize_turn
    if !matches!(
        battle_state.game_state,
        GameState::Player1Win | GameState::Player2Win | GameState::Draw
    ) {
        let forced_action_commands = calculate_forced_action_commands(battle_state);
        let _ = execute_command_batch(forced_action_commands, battle_state, bus, action_stack);
    }
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
            let commands = calculate_forfeit_commands(player_index);
            let _ = execute_command_batch(commands, battle_state, bus, action_stack);
        }

        BattleAction::Switch {
            player_index,
            target_pokemon_index,
        } => {
            // Check if current Pokemon is fainted (switching away from fainted Pokemon is allowed)
            // But switching TO a fainted Pokemon should not be allowed
            let target_pokemon = &battle_state.players[player_index].team[target_pokemon_index];
            let player = &battle_state.players[player_index];

            // Only prevent switching if there's an active, non-fainted Pokemon that is trapped
            if player.has_condition_type(PokemonConditionType::Trapped) {
                if let Some(active_pokemon) = player.active_pokemon() {
                    if !active_pokemon.is_fainted() {
                        bus.push(BattleEvent::ActionFailed {
                            reason: crate::battle::state::ActionFailureReason::IsTrapped {
                                pokemon: active_pokemon.species,
                            },
                        });
                        return;
                    }
                }
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

            let commands =
                calculate_switch_commands(player_index, target_pokemon_index, battle_state);
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
            // Only check prevention on the first hit (hit_number 0) of a move
            let (failure_reason, prevention_commands) = if hit_number == 0 {
                calculate_action_prevention(attacker_index, battle_state, rng, move_used)
            } else {
                (None, Vec::new()) // No prevention for subsequent hits
            };

            // Execute any commands from the prevention check (status updates, etc.)
            let _ = execute_command_batch(prevention_commands, battle_state, bus, action_stack);

            if let Some(failure_reason) = failure_reason {
                // Always generate ActionFailed event first
                bus.push(BattleEvent::ActionFailed {
                    reason: failure_reason.clone(),
                });

                // Special case for confusion - also causes self-damage after the action fails
                if matches!(failure_reason, ActionFailureReason::IsConfused { .. }) {
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
                // Use PP for the move via command
                if let Err(_) = execute_command(
                    BattleCommand::UsePP {
                        target: PlayerTarget::from_index(attacker_index),
                        move_used,
                    },
                    battle_state,
                    bus,
                    action_stack,
                ) {
                    bus.push(BattleEvent::ActionFailed {
                        reason: crate::battle::state::ActionFailureReason::NoPPRemaining {
                            move_used,
                        },
                    });
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
                if battle_state.players[attacker_index]
                    .has_condition_type(PokemonConditionType::Enraged)
                    && Some(move_used) != battle_state.players[attacker_index].last_move
                // Rather than requiring use of Rage, we just require it is the same move as before.
                // This allows for multiple moves that cause the user to become Enraged.
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
                let move_data = match MoveData::get_move_data(move_used) {
                    Ok(data) => data,
                    Err(_) => {
                        // If we can't get move data, fail the action silently
                        return;
                    }
                };

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
    let commands = match calculate_attack_outcome(
        battle_state,
        attacker_index,
        defender_index,
        move_used,
        hit_number,
        rng,
    ) {
        Ok(commands) => commands,
        Err(e) => {
            eprintln!("Error calculating attack outcome: {:?}", e);
            return;
        }
    };

    // 3. Execution: Pass the resulting list of commands to the executor bridge.
    //    This step applies all the calculated state changes and emits all events.
    if let Err(e) = execute_command_batch(commands, battle_state, bus, action_stack) {
        eprintln!("Error executing attack commands: {:?}", e);
    }
}

fn finalize_turn(
    battle_state: &mut BattleState,
    bus: &mut EventBus,
    action_stack: &mut ActionStack,
) {
    // Step 1: Clear state for fainted Pokémon.
    for player_index in 0..2 {
        if let Some(pokemon) = battle_state.players[player_index].active_pokemon() {
            if pokemon.is_fainted() {
                let _ = execute_command(
                    BattleCommand::ClearPlayerState {
                        target: PlayerTarget::from_index(player_index),
                    },
                    battle_state,
                    bus,
                    action_stack,
                );
            }
        }
    }

    // Step 2: Check for win/loss conditions.
    check_win_conditions(battle_state, bus);

    // Step 3: Increment turn number if the battle is ongoing.
    if matches!(battle_state.game_state, GameState::TurnInProgress) {
        let _ = execute_command(
            BattleCommand::IncrementTurnNumber,
            battle_state,
            bus,
            action_stack,
        );
    }

    // Step 4: Clear the action queue from the completed turn.
    let _ = execute_command(
        BattleCommand::ClearActionQueue,
        battle_state,
        bus,
        action_stack,
    );

    // Step 5: If the battle hasn't ended, set the state to wait for the next set of actions.
    if !matches!(
        battle_state.game_state,
        GameState::Player1Win | GameState::Player2Win | GameState::Draw
    ) {
        let _ = execute_command(
            BattleCommand::SetGameState(GameState::WaitingForActions),
            battle_state,
            bus,
            action_stack,
        );
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
