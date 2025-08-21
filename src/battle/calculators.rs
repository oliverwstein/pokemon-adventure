use crate::battle::commands::{BattleCommand, PlayerTarget};
use crate::battle::conditions::{PokemonCondition, PokemonConditionType};
use crate::battle::state::{ActionFailureReason, BattleEvent, BattleState, TurnRng};
use crate::battle::stats::{move_hits, move_is_critical_hit};
use crate::move_data::MoveData;
use crate::moves::Move;
use crate::player::PlayerAction;

/// Calculate the outcome of an attack attempt
///
/// This function coordinates the entire attack sequence through helper functions.
pub fn calculate_attack_outcome(
    state: &BattleState,
    attacker_index: usize,
    defender_index: usize,
    move_used: Move,
    hit_number: u8,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();

    let attacker_player = &state.players[attacker_index];
    let defender_player = &state.players[defender_index];

    let (attacker_pokemon, defender_pokemon) =
        match validate_pokemon_participation(attacker_player, defender_player) {
            Ok(pokemon) => pokemon,
            Err(error_command) => return vec![error_command],
        };

    // Emit MoveUsed event for the first hit of any move attempt.
    if hit_number == 0 {
        commands.push(BattleCommand::EmitEvent(BattleEvent::MoveUsed {
            player_index: attacker_index,
            pokemon: attacker_pokemon.species,
            move_used,
        }));
    }

    let move_data = MoveData::get_move_data(move_used).expect("Move data must exist");

    // --- NEW LOGIC START ---
    // First, check for any special move effects that might skip the normal attack sequence.
    let context = crate::move_data::EffectContext::new(attacker_index, defender_index, move_used);
    let mut regular_effect_commands = Vec::new();

    for effect in &move_data.effects {
        let effect_result = effect.apply(&context, state, rng);
        match effect_result {
            crate::move_data::EffectResult::Skip(special_commands) => {
                // This is a special move like ChargeUp, Fly, Rest, etc.
                // We return ONLY its commands and stop all further processing.
                commands.extend(special_commands);
                return commands;
            }
            crate::move_data::EffectResult::Continue(effect_commands) => {
                // This is a regular secondary effect (like Burn or StatChange).
                // We'll store its commands to be added later if the move hits.
                regular_effect_commands.extend(effect_commands);
            }
        }
    }
    // --- NEW LOGIC END ---

    // If we've reached this point, no effect returned 'Skip', so we proceed with a normal attack.
    let hit_result = move_hits(
        attacker_pokemon,
        defender_pokemon,
        attacker_player,
        defender_player,
        move_used,
        rng,
    );

    if hit_result {
        let hit_commands = handle_successful_hit(
            attacker_pokemon,
            defender_pokemon,
            attacker_player,
            defender_player,
            attacker_index,
            defender_index,
            move_used,
            rng,
        );
        commands.extend(hit_commands.clone());

        // Add the regular effect commands that we collected earlier.
        commands.extend(regular_effect_commands);

        let damage = hit_commands
            .iter()
            .find_map(|cmd| match cmd {
                BattleCommand::DealDamage { amount, .. } => Some(*amount),
                _ => None,
            })
            .unwrap_or(0);

        if damage > 0 {
            let damage_commands = move_data.apply_damage_based_effects(&context, state, damage);
            commands.extend(damage_commands);
        }
    } else {
        commands.push(BattleCommand::EmitEvent(BattleEvent::MoveMissed {
            attacker: attacker_pokemon.species,
            defender: defender_pokemon.species,
            move_used,
        }));

        let miss_commands = move_data.apply_miss_based_effects(&context, state);
        commands.extend(miss_commands);
    }

    // Handle Multi-hit logic (this remains the same).
    for effect in &move_data.effects {
        if let Some(command) = effect.apply_multi_hit_continuation(&context, rng, hit_number) {
            commands.push(command);
            break;
        }
    }

    commands
}

/// Validate that both Pokemon can participate in the attack
fn validate_pokemon_participation<'a>(
    attacker_player: &'a crate::player::BattlePlayer,
    defender_player: &'a crate::player::BattlePlayer,
) -> Result<
    (
        &'a crate::pokemon::PokemonInst,
        &'a crate::pokemon::PokemonInst,
    ),
    BattleCommand,
> {
    let attacker_pokemon = attacker_player.active_pokemon().ok_or_else(|| {
        // This should fail silently
        BattleCommand::EmitEvent(BattleEvent::ActionFailed {
            reason: crate::battle::state::ActionFailureReason::PokemonFainted,
        })
    })?;

    let defender_pokemon = defender_player.active_pokemon().ok_or_else(|| {
        // This should fail silently
        BattleCommand::EmitEvent(BattleEvent::ActionFailed {
            reason: crate::battle::state::ActionFailureReason::NoEnemyPresent,
        })
    })?;

    Ok((attacker_pokemon, defender_pokemon))
}

/// Handle all logic for a successful hit
fn handle_successful_hit(
    attacker_pokemon: &crate::pokemon::PokemonInst,
    defender_pokemon: &crate::pokemon::PokemonInst,
    attacker_player: &crate::player::BattlePlayer,
    defender_player: &crate::player::BattlePlayer,
    attacker_index: usize,
    defender_index: usize,
    move_used: Move,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();

    // Emit hit event
    commands.push(BattleCommand::EmitEvent(BattleEvent::MoveHit {
        attacker: attacker_pokemon.species,
        defender: defender_pokemon.species,
        move_used,
    }));

    // Calculate type effectiveness and damage
    let move_data = MoveData::get_move_data(move_used).expect("Move data must exist");
    let type_adv_multiplier = calculate_and_emit_type_effectiveness(
        &move_data,
        defender_pokemon,
        defender_player,
        &mut commands,
    );

    let damage = calculate_move_damage(
        attacker_pokemon,
        defender_pokemon,
        attacker_player,
        defender_player,
        move_used,
        type_adv_multiplier,
        rng,
        &mut commands,
    );

    // Handle damage application and conditions
    if damage > 0 {
        handle_damage_application(
            damage,
            defender_pokemon,
            defender_player,
            defender_index,
            &mut commands,
        );

        // Handle damage-triggered condition reactions
        handle_damage_triggered_conditions(
            damage,
            defender_pokemon,
            defender_player,
            attacker_index,
            defender_index,
            move_used,
            &mut commands,
        );
    }

    commands
}

/// Calculate type effectiveness and emit event if significant
fn calculate_and_emit_type_effectiveness(
    move_data: &crate::move_data::MoveData,
    defender_pokemon: &crate::pokemon::PokemonInst,
    defender_player: &crate::player::BattlePlayer,
    commands: &mut Vec<BattleCommand>,
) -> f64 {
    // Status moves don't have type effectiveness
    if matches!(move_data.category, crate::move_data::MoveCategory::Status) {
        return 1.0;
    }

    let defender_types = defender_pokemon.get_current_types(defender_player);
    let type_adv_multiplier =
        crate::battle::stats::get_type_effectiveness(move_data.move_type, &defender_types);

    // Emit type effectiveness event if significant
    if (type_adv_multiplier - 1.0).abs() > 0.1 {
        commands.push(BattleCommand::EmitEvent(
            BattleEvent::AttackTypeEffectiveness {
                multiplier: type_adv_multiplier,
            },
        ));
    }

    type_adv_multiplier
}

/// Calculate damage for the move, handling both special and normal damage
fn calculate_move_damage(
    attacker_pokemon: &crate::pokemon::PokemonInst,
    defender_pokemon: &crate::pokemon::PokemonInst,
    attacker_player: &crate::player::BattlePlayer,
    defender_player: &crate::player::BattlePlayer,
    move_used: Move,
    type_adv_multiplier: f64,
    rng: &mut TurnRng,
    commands: &mut Vec<BattleCommand>,
) -> u16 {
    let theoretical_damage = if let Some(special_damage) =
        crate::battle::stats::calculate_special_attack_damage(
            move_used,
            attacker_pokemon,
            defender_pokemon,
        ) {
        // Special damage move
        if type_adv_multiplier > 0.1 {
            special_damage
        } else {
            0
        }
    } else {
        // Normal damage move - check for critical hit first
        let is_critical = move_is_critical_hit(attacker_pokemon, attacker_player, move_used, rng);

        if is_critical {
            commands.push(BattleCommand::EmitEvent(BattleEvent::CriticalHit {
                attacker: attacker_pokemon.species,
                defender: defender_pokemon.species,
                move_used,
            }));
        }

        // Calculate normal attack damage
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

    // Cap damage to defender's current HP to get actual damage that will be dealt
    theoretical_damage.min(defender_pokemon.current_hp())
}

/// Handle damage application, including substitute protection
fn handle_damage_application(
    damage: u16,
    defender_pokemon: &crate::pokemon::PokemonInst,
    defender_player: &crate::player::BattlePlayer,
    defender_index: usize,
    commands: &mut Vec<BattleCommand>,
) {
    // Check for Substitute protection
    if let Some(substitute_hp) =
        defender_player
            .active_pokemon_conditions
            .values()
            .find_map(|condition| match condition {
                PokemonCondition::Substitute { hp } => Some(hp),
                _ => None,
            })
    {
        handle_substitute_damage_absorption(
            damage,
            *substitute_hp,
            defender_pokemon,
            defender_index,
            commands,
        );
    } else {
        // No substitute, normal damage to Pokemon
        commands.push(BattleCommand::DealDamage {
            target: PlayerTarget::from_index(defender_index),
            amount: damage,
        });
    }
}

/// Handle substitute damage absorption and destruction
fn handle_substitute_damage_absorption(
    damage: u16,
    substitute_hp: u8,
    defender_pokemon: &crate::pokemon::PokemonInst,
    defender_index: usize,
    commands: &mut Vec<BattleCommand>,
) {
    let actual_damage = damage.min(substitute_hp as u16);
    let remaining_substitute_hp = substitute_hp.saturating_sub(actual_damage as u8);
    let substitute_destroyed = remaining_substitute_hp == 0;

    if substitute_destroyed {
        // Substitute is destroyed
        commands.push(BattleCommand::RemoveSpecificCondition {
            target: PlayerTarget::from_index(defender_index),
            condition: PokemonCondition::Substitute { hp: substitute_hp },
        });
    } else {
        // Update substitute HP - remove old and add new
        commands.push(BattleCommand::RemoveCondition {
            target: PlayerTarget::from_index(defender_index),
            condition_type: PokemonConditionType::Substitute,
        });
        commands.push(BattleCommand::AddCondition {
            target: PlayerTarget::from_index(defender_index),
            condition: PokemonCondition::Substitute {
                hp: remaining_substitute_hp,
            },
        });
    }

    // Emit substitute damage event instead of confusing zero damage event
    commands.push(BattleCommand::EmitEvent(BattleEvent::SubstituteDamaged {
        target: defender_pokemon.species,
        damage: actual_damage,
        remaining_substitute_hp,
        substitute_destroyed,
    }));
}

/// Handle conditions triggered by damage using the new condition method system
fn handle_damage_triggered_conditions(
    damage: u16,
    defender_pokemon: &crate::pokemon::PokemonInst,
    defender_player: &crate::player::BattlePlayer,
    attacker_index: usize,
    defender_index: usize,
    move_used: Move,
    commands: &mut Vec<BattleCommand>,
) {
    // Only trigger if damage wasn't absorbed by substitute
    let damage_absorbed_by_substitute = defender_player
        .active_pokemon_conditions
        .values()
        .any(|condition| matches!(condition, PokemonCondition::Substitute { .. }));

    if damage_absorbed_by_substitute {
        return;
    }

    let move_data = MoveData::get_move_data(move_used).expect("Move data must exist");
    let attacker_target = PlayerTarget::from_index(attacker_index);
    let defender_target = PlayerTarget::from_index(defender_index);

    // Let each condition handle its own damage reaction
    for condition in defender_player.active_pokemon_conditions.values() {
        let condition_commands = condition.on_damage_taken(
            damage,
            attacker_target,
            defender_target,
            defender_pokemon.species,
            move_data.category,
            defender_pokemon.current_hp(),
            defender_player.get_stat_stage(crate::player::StatType::Attack),
        );
        commands.extend(condition_commands);
    }
}

/// Other Calculations

/// Calculate damage/healing effects from active Pokemon conditions (Trapped, Seeded)
/// Returns commands to execute the condition damage without directly mutating state
pub fn calculate_condition_damage_commands(battle_state: &BattleState) -> Vec<BattleCommand> {
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
            let has_seeded = player.has_condition_type(PokemonConditionType::Seeded);

            // Handle Trapped condition (1/16 max HP damage per turn)
            if has_trapped {
                let current_hp = pokemon.current_hp();
                let condition_damage = (max_hp / 16).max(1).min(current_hp); // Cap to current HP
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
                        let actual_heal =
                            actual_damage.min(opponent_max_hp.saturating_sub(opponent_current_hp));

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

/// Calculate all end-of-turn effects and return commands to execute them
/// Returns commands for status damage, condition expiry, and team condition ticking
pub fn calculate_end_turn_commands(
    battle_state: &BattleState,
    _rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();

    for player_index in 0..2 {
        let player = &battle_state.players[player_index];
        if let Some(pokemon) = player.active_pokemon() {
            // Fainted Pokemon do not take end-of-turn damage or effects.
            if pokemon.is_fainted() {
                continue;
            }

            // 1. Process Pokemon status damage (Poison, Burn) - use Pokemon's own calculation logic
            if let Some(status) = pokemon.status {
                let status_damage = pokemon.calculate_status_damage();

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
                    PokemonCondition::Disabled {
                        turns_remaining, ..
                    } => *turns_remaining <= 0,
                    PokemonCondition::Biding {
                        turns_remaining, ..
                    } => *turns_remaining <= 0,
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

/// Calculate commands to queue forced actions for the next turn
pub fn calculate_forced_action_commands(battle_state: &BattleState) -> Vec<BattleCommand> {
    let mut commands = Vec::new();

    for player_index in 0..2 {
        let player = &battle_state.players[player_index];

        if let Some(forced_move) = player.forced_move() {
            if let Some(active_pokemon) = player.active_pokemon() {
                if let Some(index) = active_pokemon
                    .moves
                    .iter()
                    .position(|m| m.as_ref().map_or(false, |inst| inst.move_ == forced_move))
                {
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

/// Calculate all conditions that can prevent a Pokemon from taking action
/// Returns (Option<ActionFailureReason>, Vec<BattleCommand>) where the commands handle
/// status updates and condition changes that occur during the prevention check
pub fn calculate_action_prevention(
    player_index: usize,
    battle_state: &BattleState,
    rng: &mut TurnRng,
    move_used: Move,
) -> (Option<ActionFailureReason>, Vec<BattleCommand>) {
    let mut commands = Vec::new();

    // Check Pokemon status conditions BEFORE updating counters
    let (pokemon_status, pokemon_species) = if let Some(pokemon) = battle_state.players
        [player_index]
        .team[battle_state.players[player_index].active_pokemon_index]
        .as_ref()
    {
        (pokemon.status, pokemon.species)
    } else {
        return (Some(ActionFailureReason::PokemonFainted), commands);
    };

    // First check if Pokemon should fail to act (including Sleep > 0)
    if let Some(status) = pokemon_status {
        match status {
            crate::pokemon::StatusCondition::Sleep(turns) => {
                if turns > 0 {
                    // Pokemon is still asleep, update counters after determining failure
                    commands.push(BattleCommand::UpdateStatusProgress {
                        target: PlayerTarget::from_index(player_index),
                    });
                    return (
                        Some(ActionFailureReason::IsAsleep {
                            pokemon: pokemon_species,
                        }),
                        commands,
                    );
                }
            }
            crate::pokemon::StatusCondition::Freeze => {
                // 25% chance to thaw out when trying to act
                let roll = rng.next_outcome("Defrost Check"); // 0-100
                if roll < 25 {
                    // Pokemon thaws out
                    commands.push(BattleCommand::CurePokemonStatus {
                        target: PlayerTarget::from_index(player_index),
                        status: crate::pokemon::StatusCondition::Freeze,
                    });
                    // Pokemon can act this turn after thawing
                } else {
                    return (
                        Some(ActionFailureReason::IsFrozen {
                            pokemon: pokemon_species,
                        }),
                        commands,
                    );
                }
            }
            _ => {} // Other status conditions don't prevent actions
        }
    }

    // Update status counters for Pokemon that are not asleep with turns > 0 (they were handled above)
    let current_status = if let Some(pokemon) = battle_state.players[player_index].team
        [battle_state.players[player_index].active_pokemon_index]
        .as_ref()
    {
        pokemon.status
    } else {
        return (Some(ActionFailureReason::PokemonFainted), commands);
    };

    // Only update counters if Pokemon doesn't have sleep with turns > 0 (those were already updated above)
    let should_update_counters = match current_status {
        Some(crate::pokemon::StatusCondition::Sleep(turns)) => turns == 0,
        _ => true,
    };

    if should_update_counters {
        commands.push(BattleCommand::UpdateStatusProgress {
            target: PlayerTarget::from_index(player_index),
        });
    }

    let player = &battle_state.players[player_index];

    // Check active Pokemon conditions
    if player.has_condition_type(PokemonConditionType::Flinched) {
        return (
            Some(ActionFailureReason::IsFlinching {
                pokemon: pokemon_species,
            }),
            commands,
        );
    }

    // Check for exhausted condition (any turns_remaining > 0 means still exhausted)
    for condition in player.active_pokemon_conditions.values() {
        if let PokemonCondition::Exhausted { turns_remaining } = condition {
            if *turns_remaining > 0 {
                return (
                    Some(ActionFailureReason::IsExhausted {
                        pokemon: pokemon_species,
                    }),
                    commands,
                );
            }
        }
    }

    // Check paralysis - 25% chance to be fully paralyzed
    if let Some(crate::pokemon::StatusCondition::Paralysis) = pokemon_status {
        let roll = rng.next_outcome("Immobilized by Paralysis Check"); // 0-100
        if roll < 25 {
            return (
                Some(ActionFailureReason::IsParalyzed {
                    pokemon: pokemon_species,
                }),
                commands,
            );
        }
    }

    // Check confusion - 50% chance to hit self instead
    // Confusion ticks at end of turn, but expires when Pokemon tries to act with 0 turns remaining
    for condition in player.active_pokemon_conditions.values() {
        if let PokemonCondition::Confused { turns_remaining } = condition {
            if *turns_remaining == 0 {
                // This is the last turn of confusion - confusion ends, no self-hit check
                commands.push(BattleCommand::ExpirePokemonCondition {
                    target: PlayerTarget::from_index(player_index),
                    condition: condition.clone(),
                });
                // Confusion has ended, so no chance to hit self - action proceeds normally
                break;
            }
            if *turns_remaining > 0 {
                // Confusion continues, so roll for self-hit (don't decrement here - that happens at end of turn)
                let roll = rng.next_outcome("Hit Itself in Confusion Check"); // 1-100
                if roll < 50 {
                    return (
                        Some(ActionFailureReason::IsConfused {
                            pokemon: pokemon_species,
                        }),
                        commands,
                    );
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
                return (
                    Some(ActionFailureReason::MoveFailedToExecute {
                        move_used: *pokemon_move,
                    }),
                    commands,
                );
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
                        return (
                            Some(ActionFailureReason::MoveFailedToExecute {
                                move_used: move_used,
                            }),
                            commands,
                        );
                    }
                }
            }
        }
    }

    (None, commands) // No conditions prevent action
}

/// Calculate commands for a Pokemon switch action
pub fn calculate_switch_commands(
    player_index: usize,
    target_pokemon_index: usize,
    battle_state: &BattleState,
) -> Vec<BattleCommand> {
    let target = PlayerTarget::from_index(player_index);
    let player = &battle_state.players[player_index];

    // Capture the old and new Pokemon info before the state change
    let old_pokemon = player.team[player.active_pokemon_index]
        .as_ref()
        .map(|p| p.species);
    let new_pokemon = player.team[target_pokemon_index]
        .as_ref()
        .map(|p| p.species);

    let mut commands = vec![
        // 1. Command to clear the old state.
        BattleCommand::ClearPlayerState { target },
    ];

    // 2. Emit the switch event with correct old/new Pokemon info
    if let (Some(old), Some(new)) = (old_pokemon, new_pokemon) {
        commands.push(BattleCommand::EmitEvent(BattleEvent::PokemonSwitched {
            player_index,
            old_pokemon: old,
            new_pokemon: new,
        }));
    }

    // 3. Command to perform the switch (disable automatic event emission)
    commands.push(BattleCommand::SwitchPokemon {
        target,
        new_pokemon_index: target_pokemon_index,
    });

    commands
}

/// Calculate commands for a forfeit action
pub fn calculate_forfeit_commands(player_index: usize) -> Vec<BattleCommand> {
    let new_state = if player_index == 0 {
        crate::battle::state::GameState::Player2Win
    } else {
        crate::battle::state::GameState::Player1Win
    };

    vec![
        BattleCommand::SetGameState(new_state),
        BattleCommand::EmitEvent(BattleEvent::PlayerDefeated { player_index }),
        BattleCommand::EmitEvent(BattleEvent::BattleEnded {
            winner: Some(if player_index == 0 { 1 } else { 0 }),
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::state::{BattleState, TurnRng};
    use crate::moves::Move;
    use crate::player::BattlePlayer;
    use crate::pokemon::PokemonInst;
    use crate::species::Species;
    use std::collections::HashMap;

    fn create_test_battle_state() -> BattleState {
        let pokemon1 = PokemonInst::new_for_test(
            Species::Pikachu,
            1,
            0,
            100,
            [15; 6],
            [0; 6],
            [100, 80, 60, 80, 60, 100],
            [const { None }; 4],
            None,
        );

        let pokemon2 = PokemonInst::new_for_test(
            Species::Charmander,
            1,
            0,
            100,
            [15; 6],
            [0; 6],
            [100, 80, 60, 80, 60, 100],
            [const { None }; 4],
            None,
        );

        let player1 = BattlePlayer {
            player_id: "test1".to_string(),
            player_name: "Player 1".to_string(),
            player_type: crate::player::PlayerType::NPC,
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
            player_type: crate::player::PlayerType::NPC,
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
    fn test_calculate_attack_outcome_hit() {
        let state = create_test_battle_state();
        let mut rng = TurnRng::new_for_test(vec![1, 99, 50, 50, 50]); // Hit + no critical hit + damage calculation values

        let commands = calculate_attack_outcome(&state, 0, 1, Move::Tackle, 0, &mut rng);

        // Should have MoveUsed, MoveHit, and DealDamage commands at minimum
        assert!(commands.len() >= 3);

        assert!(matches!(
            commands[0],
            BattleCommand::EmitEvent(BattleEvent::MoveUsed { .. })
        ));
        assert!(matches!(
            commands[1],
            BattleCommand::EmitEvent(BattleEvent::MoveHit { .. })
        ));

        // Should have DealDamage command (last command after any events)
        assert!(
            commands
                .iter()
                .any(|cmd| matches!(cmd, BattleCommand::DealDamage { .. }))
        );

        // May have type effectiveness or critical hit events
    }

    #[test]
    fn test_calculate_attack_outcome_miss() {
        let state = create_test_battle_state();
        let mut rng = TurnRng::new_for_test(vec![100]); // High value should force miss

        let commands = calculate_attack_outcome(&state, 0, 1, Move::Tackle, 0, &mut rng);

        // Should have MoveUsed and MoveMissed events
        assert_eq!(commands.len(), 2);

        assert!(matches!(
            commands[0],
            BattleCommand::EmitEvent(BattleEvent::MoveUsed { .. })
        ));
        assert!(matches!(
            commands[1],
            BattleCommand::EmitEvent(BattleEvent::MoveMissed { .. })
        ));
    }

    #[test]
    fn test_calculate_attack_outcome_no_attacker() {
        let mut state = create_test_battle_state();
        // Remove the attacker's active Pokemon
        state.players[0].team[0] = None;

        let mut rng = TurnRng::new_for_test(vec![50]);

        let commands = calculate_attack_outcome(&state, 0, 1, Move::Tackle, 0, &mut rng);

        // Should fail with PokemonFainted
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            BattleCommand::EmitEvent(BattleEvent::ActionFailed {
                reason: crate::battle::state::ActionFailureReason::PokemonFainted
            })
        ));
    }

    #[test]
    fn test_calculate_attack_outcome_no_defender() {
        let mut state = create_test_battle_state();
        // Remove the defender's active Pokemon
        state.players[1].team[0] = None;

        let mut rng = TurnRng::new_for_test(vec![50]);

        let commands = calculate_attack_outcome(&state, 0, 1, Move::Tackle, 0, &mut rng);

        // Should fail with NoEnemyPresent
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            BattleCommand::EmitEvent(BattleEvent::ActionFailed {
                reason: crate::battle::state::ActionFailureReason::NoEnemyPresent
            })
        ));
    }

    #[test]
    fn test_calculate_attack_outcome_with_substitute() {
        let mut state = create_test_battle_state();
        // Add substitute condition to defender
        state.players[1].add_condition(PokemonCondition::Substitute { hp: 50 });

        let mut rng = TurnRng::new_for_test(vec![1, 99, 50, 50, 50]); // Hit + no critical hit + damage calculation values

        let commands = calculate_attack_outcome(&state, 0, 1, Move::Tackle, 0, &mut rng);

        // Should have MoveUsed, MoveHit, and substitute-related commands
        assert!(commands.len() >= 3);

        assert!(matches!(
            commands[0],
            BattleCommand::EmitEvent(BattleEvent::MoveUsed { .. })
        ));
        assert!(matches!(
            commands[1],
            BattleCommand::EmitEvent(BattleEvent::MoveHit { .. })
        ));

        // Should have a SubstituteDamaged event (substitute absorbed the damage)
        assert!(commands.iter().any(|cmd| matches!(
            cmd,
            BattleCommand::EmitEvent(BattleEvent::SubstituteDamaged { .. })
        )));

        // Should have condition update commands (RemoveCondition and possibly AddCondition if substitute survives)
        assert!(
            commands
                .iter()
                .any(|cmd| matches!(cmd, BattleCommand::RemoveCondition { .. }))
        );
    }

    #[test]
    fn test_calculate_attack_outcome_substitute_destroyed() {
        let mut state = create_test_battle_state();
        // Add weak substitute that will be destroyed by tackle
        state.players[1].add_condition(PokemonCondition::Substitute { hp: 1 });

        let mut rng = TurnRng::new_for_test(vec![1, 99, 50, 50, 50]); // Hit + no critical hit + damage calculation values

        let commands = calculate_attack_outcome(&state, 0, 1, Move::Tackle, 0, &mut rng);

        // Should have substitute removal command (which auto-generates the StatusRemoved event)
        assert!(
            commands
                .iter()
                .any(|cmd| matches!(cmd, BattleCommand::RemoveSpecificCondition { .. }))
        );

        // Should only have RemoveSpecificCondition (no AddCondition since substitute is destroyed)
        let remove_condition_count = commands
            .iter()
            .filter(|cmd| matches!(cmd, BattleCommand::RemoveSpecificCondition { .. }))
            .count();
        let add_condition_count = commands
            .iter()
            .filter(|cmd| matches!(cmd, BattleCommand::AddCondition { .. }))
            .count();

        assert_eq!(remove_condition_count, 1);
        assert_eq!(add_condition_count, 0);
    }
}
