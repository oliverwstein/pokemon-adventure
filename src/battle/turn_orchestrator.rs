use crate::battle::state::{
    ActionFailureReason, BattleEvent, BattleState, EventBus, GameState, TurnRng,
};
use crate::battle::stats::{effective_speed, move_hits, move_is_critical_hit};
use crate::move_data::get_move_data;
use crate::moves::Move;
use crate::player::PlayerAction;
use crate::player::PokemonCondition;
use crate::species::Species;
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

/// Prepares a battle state for turn resolution by collecting actions from both players
/// This function should be called before resolve_turn()
/// For now, implements a basic deterministic AI that selects first available move
pub fn collect_player_actions(battle_state: &mut BattleState) -> Result<(), String> {
    match battle_state.game_state {
        GameState::WaitingForBothActions => {
            // Generate deterministic actions for any players that haven't provided one
            for player_index in 0..2 {
                if battle_state.action_queue[player_index].is_none() {
                    let action =
                        generate_deterministic_action(&battle_state.players[player_index])?;
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
            if hit_number == 0 {
                let attacker_pokemon = battle_state.players[attacker_index].team
                    [battle_state.players[attacker_index].active_pokemon_index]
                    .as_mut()
                    .expect("Attacker Pokemon must exist to use a move");

                // Directly use the Move from the AttackHit action. No more index lookup!
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
                    let defender_species = defender_pokemon
                        .get_species_data()
                        .expect("Defender species data must exist");
                    let type_adv_multiplier = crate::battle::stats::get_type_effectiveness(
                        move_data.move_type,
                        &defender_species.types,
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

            // If all pre-hit checks pass, execute the hit.
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
    battle_state: &BattleState,
    rng: &mut TurnRng,
) -> Option<ActionFailureReason> {
    let player = &battle_state.players[player_index];
    let pokemon = player.team[player.active_pokemon_index].as_ref()?;

    // Check Pokemon status conditions first (sleep, freeze, etc.)
    if let Some(status) = pokemon.status {
        match status {
            crate::pokemon::StatusCondition::Sleep(_) => {
                return Some(ActionFailureReason::IsAsleep);
            }
            crate::pokemon::StatusCondition::Freeze => {
                return Some(ActionFailureReason::IsFrozen);
            }
            _ => {} // Other status conditions don't prevent actions
        }
    }

    // Check active Pokemon conditions
    if player.has_condition(&crate::player::PokemonCondition::Flinched) {
        return Some(ActionFailureReason::IsFlinching);
    }

    // Check for exhausted condition (any turns_remaining > 0 means still exhausted)
    for condition in player.active_pokemon_conditions.values() {
        if let crate::player::PokemonCondition::Exhausted { turns_remaining } = condition {
            if *turns_remaining > 0 {
                return Some(ActionFailureReason::IsExhausted);
            }
        }
    }

    // Check paralysis - 25% chance to be fully paralyzed
    if let Some(crate::pokemon::StatusCondition::Paralysis) = pokemon.status {
        let roll = rng.next_outcome(); // 0-100
        if roll < 25 {
            return Some(ActionFailureReason::IsParalyzed);
        }
    }

    // Check confusion - 50% chance to hit self instead
    for condition in player.active_pokemon_conditions.values() {
        if let crate::player::PokemonCondition::Confused { turns_remaining } = condition {
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

    None // No conditions prevent action
}

/// Apply all chance-based move effects after a successful hit
fn apply_move_effects(
    attacker_index: usize,
    defender_index: usize,
    move_data: &crate::move_data::MoveData,
    battle_state: &mut BattleState,
    bus: &mut EventBus,
    rng: &mut TurnRng,
) {
    for effect in &move_data.effects {
        match effect {
            // Status effects on target
            crate::move_data::MoveEffect::Burn(chance) => {
                if rng.next_outcome() <= *chance {
                    let target_pokemon = &mut battle_state.players[defender_index].team
                        [battle_state.players[defender_index].active_pokemon_index];
                    if let Some(pokemon) = target_pokemon {
                        if pokemon.status.is_none() {
                            pokemon.status = Some(crate::pokemon::StatusCondition::Burn);
                            bus.push(BattleEvent::PokemonStatusApplied {
                                target: pokemon.species,
                                status: crate::pokemon::StatusCondition::Burn,
                            });
                        }
                    }
                }
            }
            crate::move_data::MoveEffect::Paralyze(chance) => {
                if rng.next_outcome() <= *chance {
                    let target_pokemon = &mut battle_state.players[defender_index].team
                        [battle_state.players[defender_index].active_pokemon_index];
                    if let Some(pokemon) = target_pokemon {
                        if pokemon.status.is_none() {
                            pokemon.status = Some(crate::pokemon::StatusCondition::Paralysis);
                            bus.push(BattleEvent::PokemonStatusApplied {
                                target: pokemon.species,
                                status: crate::pokemon::StatusCondition::Paralysis,
                            });
                        }
                    }
                }
            }
            crate::move_data::MoveEffect::Freeze(chance) => {
                if rng.next_outcome() <= *chance {
                    let target_pokemon = &mut battle_state.players[defender_index].team
                        [battle_state.players[defender_index].active_pokemon_index];
                    if let Some(pokemon) = target_pokemon {
                        if pokemon.status.is_none() {
                            pokemon.status = Some(crate::pokemon::StatusCondition::Freeze);
                            bus.push(BattleEvent::PokemonStatusApplied {
                                target: pokemon.species,
                                status: crate::pokemon::StatusCondition::Freeze,
                            });
                        }
                    }
                }
            }
            crate::move_data::MoveEffect::Poison(chance) => {
                if rng.next_outcome() <= *chance {
                    let target_pokemon = &mut battle_state.players[defender_index].team
                        [battle_state.players[defender_index].active_pokemon_index];
                    if let Some(pokemon) = target_pokemon {
                        if pokemon.status.is_none() {
                            pokemon.status = Some(crate::pokemon::StatusCondition::Poison(0));
                            bus.push(BattleEvent::PokemonStatusApplied {
                                target: pokemon.species,
                                status: crate::pokemon::StatusCondition::Poison(0),
                            });
                        }
                    }
                }
            }
            crate::move_data::MoveEffect::Sedate(chance) => {
                if rng.next_outcome() <= *chance {
                    let target_pokemon = &mut battle_state.players[defender_index].team
                        [battle_state.players[defender_index].active_pokemon_index];
                    if let Some(pokemon) = target_pokemon {
                        if pokemon.status.is_none() {
                            // Sleep for 1-3 turns (random)
                            let sleep_turns = (rng.next_outcome() % 3) + 1; // 1, 2, or 3 turns
                            pokemon.status =
                                Some(crate::pokemon::StatusCondition::Sleep(sleep_turns));
                            bus.push(BattleEvent::PokemonStatusApplied {
                                target: pokemon.species,
                                status: crate::pokemon::StatusCondition::Sleep(sleep_turns),
                            });
                        }
                    }
                }
            }

            // Active conditions on target
            crate::move_data::MoveEffect::Flinch(chance) => {
                if rng.next_outcome() <= *chance {
                    let target_player = &mut battle_state.players[defender_index];
                    if let Some(pokemon_species) = target_player.active_pokemon().map(|p| p.species)
                    {
                        target_player.add_condition(crate::player::PokemonCondition::Flinched);
                        bus.push(BattleEvent::StatusApplied {
                            target: pokemon_species,
                            status: crate::player::PokemonCondition::Flinched,
                        });
                    }
                }
            }
            crate::move_data::MoveEffect::Confuse(chance) => {
                if rng.next_outcome() <= *chance {
                    let target_player = &mut battle_state.players[defender_index];
                    if let Some(pokemon_species) = target_player.active_pokemon().map(|p| p.species)
                    {
                        // Confuse for 1-4 turns (random)
                        let confuse_turns = (rng.next_outcome() % 4) + 1;
                        target_player.add_condition(crate::player::PokemonCondition::Confused {
                            turns_remaining: confuse_turns,
                        });
                        bus.push(BattleEvent::StatusApplied {
                            target: pokemon_species,
                            status: crate::player::PokemonCondition::Confused {
                                turns_remaining: confuse_turns,
                            },
                        });
                    }
                }
            }
            crate::move_data::MoveEffect::Exhaust(chance) => {
                if rng.next_outcome() <= *chance {
                    let target_player = &mut battle_state.players[defender_index];
                    if let Some(pokemon_species) = target_player.active_pokemon().map(|p| p.species)
                    {
                        target_player.add_condition(crate::player::PokemonCondition::Exhausted {
                            turns_remaining: 1,
                        });
                        bus.push(BattleEvent::StatusApplied {
                            target: pokemon_species,
                            status: crate::player::PokemonCondition::Exhausted {
                                turns_remaining: 1,
                            },
                        });
                    }
                }
            }
            crate::move_data::MoveEffect::Trap(chance) => {
                if rng.next_outcome() <= *chance {
                    let target_player = &mut battle_state.players[defender_index];
                    if let Some(pokemon_species) = target_player.active_pokemon().map(|p| p.species)
                    {
                        // Trap for 2-5 turns (random)
                        let trap_turns = (rng.next_outcome() % 4) + 2;
                        target_player.add_condition(crate::player::PokemonCondition::Trapped {
                            turns_remaining: trap_turns,
                        });
                        bus.push(BattleEvent::StatusApplied {
                            target: pokemon_species,
                            status: crate::player::PokemonCondition::Trapped {
                                turns_remaining: trap_turns,
                            },
                        });
                    }
                }
            }

            // Stat changes
            crate::move_data::MoveEffect::StatChange(target, stat, stages, chance) => {
                if rng.next_outcome() <= *chance {
                    let target_index = match target {
                        crate::move_data::Target::User => attacker_index,
                        crate::move_data::Target::Target => defender_index,
                    };

                    let player_stat = match stat {
                        crate::move_data::StatType::Atk => crate::player::StatType::Attack,
                        crate::move_data::StatType::Def => crate::player::StatType::Defense,
                        crate::move_data::StatType::SpAtk => crate::player::StatType::SpecialAttack,
                        crate::move_data::StatType::SpDef => {
                            crate::player::StatType::SpecialDefense
                        }
                        crate::move_data::StatType::Spe => crate::player::StatType::Speed,
                        crate::move_data::StatType::Acc => crate::player::StatType::Accuracy,
                        crate::move_data::StatType::Eva => crate::player::StatType::Evasion,
                        crate::move_data::StatType::Crit => crate::player::StatType::Focus,
                        _ => continue, // Skip unsupported stat types
                    };

                    let target_player = &mut battle_state.players[target_index];
                    if let Some(pokemon_species) = target_player.active_pokemon().map(|p| p.species)
                    {
                        let old_stage = target_player.get_stat_stage(player_stat);
                        target_player.modify_stat_stage(player_stat, *stages);
                        let new_stage = target_player.get_stat_stage(player_stat);

                        if old_stage != new_stage {
                            bus.push(BattleEvent::StatStageChanged {
                                target: pokemon_species,
                                stat: player_stat,
                                old_stage,
                                new_stage,
                            });
                        }
                    }
                }
            }
            crate::move_data::MoveEffect::RaiseAllStats(chance) => {
                if rng.next_outcome() <= *chance {
                    let attacker_player = &mut battle_state.players[attacker_index];
                    if let Some(pokemon_species) =
                        attacker_player.active_pokemon().map(|p| p.species)
                    {
                        let stats_to_raise = [
                            crate::player::StatType::Attack,
                            crate::player::StatType::Defense,
                            crate::player::StatType::SpecialAttack,
                            crate::player::StatType::SpecialDefense,
                            crate::player::StatType::Speed,
                        ];

                        for stat in &stats_to_raise {
                            let old_stage = attacker_player.get_stat_stage(*stat);
                            attacker_player.modify_stat_stage(*stat, 1);
                            let new_stage = attacker_player.get_stat_stage(*stat);

                            if old_stage != new_stage {
                                bus.push(BattleEvent::StatStageChanged {
                                    target: pokemon_species,
                                    stat: *stat,
                                    old_stage,
                                    new_stage,
                                });
                            }
                        }
                    }
                }
            }

            // Skip other effects that don't have chance percentages or are handled elsewhere
            _ => {}
        }
    }
}

/// Apply damage-based effects that always trigger when damage is dealt (recoil, drain)
fn apply_on_hit_effects(
    attacker_index: usize,
    move_data: &crate::move_data::MoveData,
    battle_state: &mut BattleState,
    bus: &mut EventBus,
    damage_dealt: u16,
) {
    if damage_dealt == 0 {
        return; // No damage-based effects if no damage was dealt
    }

    for effect in &move_data.effects {
        match effect {
            // Recoil damage to attacker
            crate::move_data::MoveEffect::Recoil(percentage) => {
                let recoil_damage = (damage_dealt * (*percentage as u16)) / 100;
                if recoil_damage > 0 {
                    let attacker_player = &mut battle_state.players[attacker_index];
                    if let Some(attacker_pokemon) =
                        attacker_player.team[attacker_player.active_pokemon_index].as_mut()
                    {
                        let attacker_species = attacker_pokemon.species;
                        let fainted = attacker_pokemon.take_damage(recoil_damage);
                        let remaining_hp = attacker_pokemon.current_hp();

                        bus.push(BattleEvent::DamageDealt {
                            target: attacker_species,
                            damage: recoil_damage,
                            remaining_hp,
                        });

                        if fainted {
                            bus.push(BattleEvent::PokemonFainted {
                                player_index: attacker_index,
                                pokemon: attacker_species,
                            });
                        }
                    }
                }
            }

            // Drain healing to attacker
            crate::move_data::MoveEffect::Drain(percentage) => {
                let heal_amount = (damage_dealt * (*percentage as u16)) / 100;
                println!("Heal amount: {}", heal_amount);
                if heal_amount > 0 {
                    let attacker_player = &mut battle_state.players[attacker_index];
                    if let Some(attacker_pokemon) =
                        attacker_player.team[attacker_player.active_pokemon_index].as_mut()
                    {
                        let attacker_species = attacker_pokemon.species;
                        let old_hp = attacker_pokemon.current_hp();
                        attacker_pokemon.heal(heal_amount);
                        let new_hp = attacker_pokemon.current_hp();
                        let actual_heal = new_hp - old_hp;
                        println!("Actual Heal: {}", actual_heal);
                        println!(
                            "Max HP: {}, Current HP: {}, Initial HP: {}",
                            attacker_pokemon.max_hp(),
                            attacker_pokemon.current_hp(),
                            old_hp
                        );
                        if actual_heal > 0 {
                            bus.push(BattleEvent::PokemonHealed {
                                target: attacker_species,
                                amount: actual_heal,
                                new_hp,
                            });
                        }
                    }
                }
            }

            // Other effects are handled elsewhere or don't apply to on-hit
            _ => {}
        }
    }
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
    let attacker_player = &battle_state.players[attacker_index];
    let attacker_pokemon = attacker_player.team[attacker_player.active_pokemon_index]
        .as_ref()
        .expect("Attacker pokemon should exist");

    let defender_player = &battle_state.players[defender_index];
    let defender_pokemon = defender_player.team[defender_player.active_pokemon_index]
        .as_ref()
        .expect("Defender pokemon should exist");

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

    // Check all action-preventing conditions
    if let Some(failure_reason) =
        check_action_preventing_conditions(attacker_index, battle_state, rng)
    {
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
                hit_number: 0,
            });
        }
        return; // Attack is prevented
    }
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
        let move_data = get_move_data(move_used).expect("Move data must exist");
        let defender_species = defender_pokemon
            .get_species_data()
            .expect("Defender species data must exist");
        let type_adv_multiplier = crate::battle::stats::get_type_effectiveness(
            move_data.move_type,
            &defender_species.types,
        );
        if (type_adv_multiplier - 1.0).abs() > 0.1 {
            bus.push(BattleEvent::AttackTypeEffectiveness {
                multiplier: type_adv_multiplier,
            });
        }

        let damage = if let Some(special_damage) =
            crate::battle::stats::calculate_special_attack_damage(
                move_used,
                attacker_pokemon,
                defender_pokemon,
            ) {
            if (type_adv_multiplier > 0.1) {
                special_damage
            } else {
                0
            }
        } else {
            let is_critical =
                move_is_critical_hit(attacker_pokemon, attacker_player, move_used, rng);

            if is_critical {
                bus.push(BattleEvent::CriticalHit {
                    attacker: attacker_pokemon.species,
                    defender: defender_pokemon.species,
                    move_used,
                });
            }
            // Calculate type effectiveness multiplier.

            crate::battle::stats::calculate_attack_damage(
                attacker_pokemon,
                defender_pokemon,
                attacker_player,
                defender_player,
                move_used,
                is_critical,
                rng,
            )
        };

        let defender_fainted = if damage > 0 {
            let defender_player_mut = &mut battle_state.players[defender_index];
            let defender_pokemon_mut = defender_player_mut.team
                [defender_player_mut.active_pokemon_index]
                .as_mut()
                .expect("Defender pokemon should exist");

            let did_faint = defender_pokemon_mut.take_damage(damage);
            let remaining_hp = defender_pokemon_mut.current_hp();

            bus.push(BattleEvent::DamageDealt {
                target: defender_pokemon_mut.species,
                damage,
                remaining_hp,
            });

            if did_faint {
                bus.push(BattleEvent::PokemonFainted {
                    player_index: defender_index,
                    pokemon: defender_pokemon_mut.species,
                });
            }
            did_faint
        } else {
            false
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
                &move_data,
                battle_state,
                bus,
                rng,
            );
        }

        // Apply damage-based effects (recoil, drain) when damage was dealt
        if damage > 0 {
            apply_on_hit_effects(attacker_index, &move_data, battle_state, bus, damage);
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
    } else {
        bus.push(BattleEvent::MoveMissed {
            attacker: attacker_pokemon.species,
            defender: defender_pokemon.species,
            move_used,
        });
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

            // 1. Process Pokemon status conditions (Sleep, Poison, Burn)
            let (status_damage, should_cure, status_changed) = pokemon.tick_status();

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

            if should_cure && status_changed {
                // Status was cured (e.g., sleep ended)
                bus.push(BattleEvent::PokemonStatusRemoved {
                    target: pokemon.species,
                    status: crate::pokemon::StatusCondition::Sleep(0), // Will be the previous status
                });
            }

            // Check for frozen Pokemon defrosting (25% chance)
            if matches!(
                pokemon.status,
                Some(crate::pokemon::StatusCondition::Freeze)
            ) {
                let defrost_roll = rng.next_outcome();
                if defrost_roll <= 25 {
                    // 25% chance (25/99)
                    pokemon.status = None;
                    bus.push(BattleEvent::PokemonStatusRemoved {
                        target: pokemon.species,
                        status: crate::pokemon::StatusCondition::Freeze,
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
