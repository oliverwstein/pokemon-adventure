use crate::battle::commands::{BattleCommand, PlayerTarget};
use crate::battle::conditions::{PokemonCondition, PokemonConditionType};
use crate::battle::state::{BattleEvent, BattleState, TurnRng};
use crate::battle::stats::{move_hits, move_is_critical_hit};
use crate::move_data::get_move_data;
use crate::moves::Move;

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

    let move_data = get_move_data(move_used).expect("Move data must exist");

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
        BattleCommand::EmitEvent(BattleEvent::ActionFailed {
            reason: crate::battle::state::ActionFailureReason::PokemonFainted,
        })
    })?;

    let defender_pokemon = defender_player.active_pokemon().ok_or_else(|| {
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
    let move_data = get_move_data(move_used).expect("Move data must exist");
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
    if let Some(special_damage) = crate::battle::stats::calculate_special_attack_damage(
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
    }
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

    if remaining_substitute_hp == 0 {
        // Substitute is destroyed
        commands.push(BattleCommand::RemoveCondition {
            target: PlayerTarget::from_index(defender_index),
            condition_type: PokemonConditionType::Substitute,
        });
        commands.push(BattleCommand::EmitEvent(BattleEvent::StatusRemoved {
            target: defender_pokemon.species,
            status: PokemonCondition::Substitute { hp: substitute_hp },
        }));
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

    // No damage to Pokemon, substitute took it all - emit 0 damage event
    commands.push(BattleCommand::EmitEvent(BattleEvent::DamageDealt {
        target: defender_pokemon.species,
        damage: 0,
        remaining_hp: defender_pokemon.current_hp(),
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

    let move_data = get_move_data(move_used).expect("Move data must exist");
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
        // Initialize move data for tests
        use std::path::Path;
        let data_path = Path::new("data");
        if crate::move_data::initialize_move_data(data_path).is_err() {
            // Skip if move data isn't available
            return;
        }

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
        // Initialize move data for tests
        use std::path::Path;
        let data_path = Path::new("data");
        if crate::move_data::initialize_move_data(data_path).is_err() {
            // Skip if move data isn't available
            return;
        }

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
        // Initialize move data for tests
        use std::path::Path;
        let data_path = Path::new("data");
        if crate::move_data::initialize_move_data(data_path).is_err() {
            // Skip if move data isn't available
            return;
        }

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

        // Should have a DamageDealt event with 0 damage (substitute absorbed it)
        assert!(commands.iter().any(|cmd| matches!(
            cmd,
            BattleCommand::EmitEvent(BattleEvent::DamageDealt { damage: 0, .. })
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
        // Initialize move data for tests
        use std::path::Path;
        let data_path = Path::new("data");
        if crate::move_data::initialize_move_data(data_path).is_err() {
            // Skip if move data isn't available
            return;
        }

        let mut state = create_test_battle_state();
        // Add weak substitute that will be destroyed by tackle
        state.players[1].add_condition(PokemonCondition::Substitute { hp: 1 });

        let mut rng = TurnRng::new_for_test(vec![1, 99, 50, 50, 50]); // Hit + no critical hit + damage calculation values

        let commands = calculate_attack_outcome(&state, 0, 1, Move::Tackle, 0, &mut rng);

        // Should have substitute destruction event
        assert!(commands.iter().any(|cmd| matches!(
            cmd,
            BattleCommand::EmitEvent(BattleEvent::StatusRemoved { .. })
        )));

        // Should only have RemoveCondition (no AddCondition since substitute is destroyed)
        let remove_condition_count = commands
            .iter()
            .filter(|cmd| matches!(cmd, BattleCommand::RemoveCondition { .. }))
            .count();
        let add_condition_count = commands
            .iter()
            .filter(|cmd| matches!(cmd, BattleCommand::AddCondition { .. }))
            .count();

        assert_eq!(remove_condition_count, 1);
        assert_eq!(add_condition_count, 0);
    }
}
