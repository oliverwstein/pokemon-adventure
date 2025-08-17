// in src/battle/engine.rs

// CHANGED: Use statements are updated.
use crate::battle::action_stack::{ActionStack, BattleAction};
use crate::battle::ai::{Behavior, ScoringAI};
use crate::battle::calculators::calculate_attack_outcome;
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

/// Calculate damage/healing effects from active Pokemon conditions (Trapped, Seeded)
/// Returns commands to execute the condition damage without directly mutating state
fn calculate_condition_damage_commands(battle_state: &BattleState) -> Vec<BattleCommand> {
    let mut commands = Vec::new();

    // Process each player's active Pokemon for condition effects
    for player_index in 0..2 {
        let opponent_index = 1 - player_index;

        // Check if this player has an active, non-fainted Pokemon
        let player = &battle_state.players[player_index];
        if let Some(pokemon) = player.active_pokemon() {
            if pokemon.is_fainted() {
                continue;
            }

            let max_hp = pokemon.max_hp();
            let has_trapped = player
                .active_pokemon_conditions
                .values()
                .any(|condition| matches!(condition, PokemonCondition::Trapped { .. }));
            let has_seeded = player.has_condition(&PokemonCondition::Seeded);

            // Handle Trapped condition (1/16 max HP damage per turn)
            if has_trapped {
                let condition_damage = (max_hp / 16).max(1); // 1/16 of max HP, minimum 1
                commands.push(BattleCommand::DealConditionDamage {
                    target: PlayerTarget::from_index(player_index),
                    condition: PokemonCondition::Trapped { turns_remaining: 1 }, 
                    amount: condition_damage,
                });
            }

            // Handle Seeded condition (1/8 max HP drained per turn, heals opponent)
            if has_seeded {
                let current_hp = pokemon.current_hp();
                let actual_damage = (max_hp / 8).max(1).min(current_hp);
                
                commands.push(BattleCommand::DealConditionDamage {
                    target: PlayerTarget::from_index(player_index),
                    condition: PokemonCondition::Seeded,
                    amount: actual_damage,
                });

                // Heal the opponent if they have an active Pokemon
                let opponent_player = &battle_state.players[opponent_index];
                if let Some(opponent_pokemon) = opponent_player.active_pokemon() {
                    if !opponent_pokemon.is_fainted() {
                        let opponent_current_hp = opponent_pokemon.current_hp();
                        let opponent_max_hp = opponent_pokemon.max_hp();
                        let actual_heal = actual_damage.min(opponent_max_hp.saturating_sub(opponent_current_hp));

                        if actual_heal > 0 {
                            commands.push(BattleCommand::HealPokemon {
                                target: PlayerTarget::from_index(opponent_index),
                                amount: actual_heal,
                            });
                        }
                    }
                }
            }
        }
    }
    
    commands
}

/// Apply damage/healing effects from active Pokemon conditions (Trapped, Seeded)
/// LEGACY FUNCTION - Will be removed when refactoring is complete
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

/// Calculate all end-of-turn effects and return commands to execute them
/// Returns commands for status damage, condition expiry, and team condition ticking
fn calculate_end_turn_commands(battle_state: &BattleState, _rng: &mut TurnRng) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    
    for player_index in 0..2 {
        let player = &battle_state.players[player_index];
        if let Some(pokemon) = player.active_pokemon() {
            // Fainted Pokemon do not take end-of-turn damage or effects.
            if pokemon.is_fainted() {
                continue;
            }

            // 1. Process Pokemon status damage (Poison, Burn) - we need to calculate this ourselves
            if let Some(status) = pokemon.status {
                let status_damage = match status {
                    crate::pokemon::StatusCondition::Poison(_) => pokemon.max_hp() / 8,
                    crate::pokemon::StatusCondition::Burn => pokemon.max_hp() / 16,
                    _ => 0,
                };
                
                if status_damage > 0 {
                    commands.push(BattleCommand::DealStatusDamage {
                        target: PlayerTarget::from_index(player_index),
                        status,
                        amount: status_damage,
                    });
                }
            }

            // 2. Process active Pokemon conditions - emit atomic commands for each condition
            let target = PlayerTarget::from_index(player_index);
            for (_condition_type, condition) in &player.active_pokemon_conditions {
                // Tick the condition
                commands.push(BattleCommand::TickPokemonCondition {
                    target,
                    condition: condition.clone(),
                });
                
                // Check if the condition should expire after ticking
                let should_expire = match condition {
                    PokemonCondition::Confused { turns_remaining } => *turns_remaining <= 0,
                    PokemonCondition::Exhausted { turns_remaining } => *turns_remaining <= 0,
                    PokemonCondition::Trapped { turns_remaining } => *turns_remaining <= 0,
                    PokemonCondition::Rampaging { turns_remaining } => *turns_remaining <= 0,
                    PokemonCondition::Disabled { turns_remaining, .. } => *turns_remaining <= 0,
                    PokemonCondition::Biding { turns_remaining, .. } => *turns_remaining <= 0,
                    PokemonCondition::Flinched => true, // Flinch always expires at end of turn
                    PokemonCondition::Teleported => true, // Teleported expires at end of turn
                    PokemonCondition::Countering { .. } => true, // Counter expires at end of turn
                    // Charging does NOT expire at end of turn - it expires when the move executes
                    _ => false, // Other conditions don't expire automatically
                };
                
                if should_expire {
                    commands.push(BattleCommand::ExpirePokemonCondition {
                        target,
                        condition: condition.clone(),
                    });
                }
            }
        }
    }
    
    // 3. Process condition-based damage effects (Trapped, Seeded)
    let condition_damage_commands = calculate_condition_damage_commands(battle_state);
    commands.extend(condition_damage_commands);

    // 4. Tick team conditions (Reflect, Light Screen, Mist) - emit atomic commands for each
    for player_index in 0..2 {
        let player = &battle_state.players[player_index];
        let target = PlayerTarget::from_index(player_index);
        
        for (condition, turns) in &player.team_conditions {
            // Tick the team condition
            commands.push(BattleCommand::TickTeamCondition {
                target,
                condition: *condition,
            });
            
            // Check if it should expire after ticking
            if *turns <= 1 {
                commands.push(BattleCommand::ExpireTeamCondition {
                    target,
                    condition: *condition,
                });
            }
        }
    }
    
    commands
}

/// LEGACY FUNCTION - Will be removed when refactoring is complete
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
    let condition_damage_commands = calculate_condition_damage_commands(battle_state);
    let _ = execute_command_batch(condition_damage_commands, battle_state, bus, &mut ActionStack::new());

    // 4. Tick team conditions (Reflect, Light Screen, Mist)
    for player_index in 0..2 {
        let expired_conditions = battle_state.players[player_index].tick_team_conditions();
        let mut commands = Vec::new();
        for condition in expired_conditions {
            // We can now generate events for this!
            commands.push(BattleCommand::RemoveTeamCondition {
                target: PlayerTarget::from_index(player_index),
                condition,
            });
            commands.push(BattleCommand::EmitEvent(BattleEvent::TeamConditionExpired {
                    player_index,
                    condition,
                }));
        }
        // Execute the generated commands
        let _ = execute_command_batch(commands, battle_state, bus, &mut ActionStack::new());
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

/// Calculate commands to queue forced actions for the next turn
fn calculate_forced_action_commands(battle_state: &BattleState) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    
    for player_index in 0..2 {
        let player = &battle_state.players[player_index];

        if let Some(forced_move) = player.forced_move() {
            if let Some(active_pokemon) = player.active_pokemon() {
                if let Some(index) = active_pokemon.moves.iter().position(|m| {
                    m.as_ref().map_or(false, |inst| inst.move_ == forced_move)
                }) {
                    commands.push(BattleCommand::QueueForcedAction {
                        target: PlayerTarget::from_index(player_index),
                        action: PlayerAction::UseMove { move_index: index },
                    });
                }
            }
        }
    }
    
    commands
}

/// LEGACY FUNCTION - Will be removed when refactoring is complete
fn prepare_next_turn_queue(battle_state: &mut BattleState) {
    for player_index in 0..2 {
        let player = &battle_state.players[player_index];

        if let Some(forced_move) = player.forced_move() {
            if let Some(active_pokemon) = player.active_pokemon() {
                if let Some(index) = active_pokemon.moves.iter().position(|m| {
                    m.as_ref().map_or(false, |inst| inst.move_ == forced_move)
                }) {
                    battle_state.action_queue[player_index] = Some(PlayerAction::UseMove { move_index: index });
                }
            }
        }
    }
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
