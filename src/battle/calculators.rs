use crate::battle::commands::{BattleCommand, PlayerTarget};
use crate::battle::state::{BattleEvent, BattleState, TurnRng};
use crate::battle::stats::{move_hits, move_is_critical_hit};
use crate::move_data::get_move_data;
use crate::moves::Move;

/// Calculate the outcome of an attack attempt
/// 
/// This function starts with just hit/miss logic as the first "bubble" of purity.
/// Additional logic (damage, effects, etc.) will be added incrementally.
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
    
    // Get the active Pokemon
    let attacker_pokemon = match attacker_player.active_pokemon() {
        Some(pokemon) => pokemon,
        None => {
            // If no active Pokemon, the attack fails
            return vec![BattleCommand::EmitEvent(BattleEvent::ActionFailed {
                reason: crate::battle::state::ActionFailureReason::PokemonFainted,
            })];
        }
    };
    
    let defender_pokemon = match defender_player.active_pokemon() {
        Some(pokemon) => pokemon,
        None => {
            // If no defender, the attack fails
            return vec![BattleCommand::EmitEvent(BattleEvent::ActionFailed {
                reason: crate::battle::state::ActionFailureReason::NoEnemyPresent,
            })];
        }
    };
    
    // First, emit the MoveUsed event (only for first hit in multi-hit sequence)
    if hit_number == 0 {
        commands.push(BattleCommand::EmitEvent(BattleEvent::MoveUsed {
            player_index: attacker_index,
            pokemon: attacker_pokemon.species,
            move_used,
        }));
    }
    
    // Check if the move hits
    let hit_result = move_hits(
        attacker_pokemon,
        defender_pokemon,
        attacker_player,
        defender_player,
        move_used,
        rng,
    );
    
    if hit_result {
        // Move hits - emit hit event
        commands.push(BattleCommand::EmitEvent(BattleEvent::MoveHit {
            attacker: attacker_pokemon.species,
            defender: defender_pokemon.species,
            move_used,
        }));
        
        // Get move data for type effectiveness and damage calculations
        let move_data = get_move_data(move_used).expect("Move data must exist");
        
        // Calculate type effectiveness
        let defender_types = defender_pokemon.get_current_types(defender_player);
        let type_adv_multiplier =
            crate::battle::stats::get_type_effectiveness(move_data.move_type, &defender_types);
        
        // Emit type effectiveness event if significant
        if (type_adv_multiplier - 1.0).abs() > 0.1 {
            commands.push(BattleCommand::EmitEvent(BattleEvent::AttackTypeEffectiveness {
                multiplier: type_adv_multiplier,
            }));
        }
        
        // Calculate damage
        let damage = if let Some(special_damage) =
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
        
        // Handle substitute damage absorption
        if damage > 0 {
            // Check for Substitute protection
            if let Some(substitute_condition) = defender_player
                .active_pokemon_conditions
                .values()
                .find_map(|condition| match condition {
                    crate::player::PokemonCondition::Substitute { hp } => Some(*hp),
                    _ => None,
                })
            {
                // Substitute absorbs the damage
                let substitute_hp = substitute_condition;
                let actual_damage = damage.min(substitute_hp as u16);
                let remaining_substitute_hp = substitute_hp.saturating_sub(actual_damage as u8);

                if remaining_substitute_hp == 0 {
                    // Substitute is destroyed
                    commands.push(BattleCommand::RemoveCondition {
                        target: PlayerTarget::from_index(defender_index),
                        condition_type: crate::battle::commands::PokemonConditionType::Substitute,
                    });
                    commands.push(BattleCommand::EmitEvent(BattleEvent::StatusRemoved {
                        target: defender_pokemon.species,
                        status: crate::player::PokemonCondition::Substitute { hp: substitute_hp },
                    }));
                } else {
                    // Update substitute HP - remove old and add new
                    commands.push(BattleCommand::RemoveCondition {
                        target: PlayerTarget::from_index(defender_index),
                        condition_type: crate::battle::commands::PokemonConditionType::Substitute,
                    });
                    commands.push(BattleCommand::AddCondition {
                        target: PlayerTarget::from_index(defender_index),
                        condition: crate::player::PokemonCondition::Substitute {
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
            } else {
                // No substitute, normal damage to Pokemon
                commands.push(BattleCommand::DealDamage {
                    target: PlayerTarget::from_index(defender_index),
                    amount: damage,
                });
            }
        }
        
        // Handle Counter/Bide/Enraged conditions when damage is dealt and not absorbed by substitute
        if damage > 0 && !defender_player
            .active_pokemon_conditions
            .values()
            .any(|condition| matches!(condition, crate::player::PokemonCondition::Substitute { .. }))
        {
            // Counter Logic (Iteration 7): Retaliate with 2x physical damage (only if defender survives)
            let move_data = get_move_data(move_used).expect("Move data must exist");
            let defender_will_faint = damage >= defender_pokemon.current_hp();
            
            if matches!(move_data.category, crate::move_data::MoveCategory::Physical)
                && defender_player.has_condition(&crate::player::PokemonCondition::Countering { damage: 0 })
                && !defender_will_faint  // Can only counter if defender survives the damage
            {
                let counter_damage = damage * 2;
                commands.push(BattleCommand::DealDamage {
                    target: PlayerTarget::from_index(attacker_index),
                    amount: counter_damage,
                });
            }
            
            // Bide Logic (Iteration 8): Accumulate damage for future release
            if let Some(bide_condition) = defender_player
                .active_pokemon_conditions
                .values()
                .find_map(|condition| match condition {
                    crate::player::PokemonCondition::Biding { turns_remaining, damage: stored_damage } => {
                        Some((*turns_remaining, *stored_damage))
                    },
                    _ => None,
                })
            {
                let (turns_remaining, stored_damage) = bide_condition;
                // Remove old condition
                commands.push(BattleCommand::RemoveCondition {
                    target: PlayerTarget::from_index(defender_index),
                    condition_type: crate::battle::commands::PokemonConditionType::Biding,
                });
                // Add updated condition with accumulated damage
                commands.push(BattleCommand::AddCondition {
                    target: PlayerTarget::from_index(defender_index),
                    condition: crate::player::PokemonCondition::Biding {
                        turns_remaining,
                        damage: stored_damage + damage,
                    },
                });
            }
            
            // Enraged Logic (Iteration 9): Increase attack when hit
            if defender_player.has_condition(&crate::player::PokemonCondition::Enraged) {
                let old_stage = defender_player.get_stat_stage(crate::player::StatType::Attack);
                let new_stage = (old_stage + 1).min(6); // Cap at +6
                
                if old_stage != new_stage {
                    commands.push(BattleCommand::ChangeStatStage {
                        target: PlayerTarget::from_index(defender_index),
                        stat: crate::player::StatType::Attack,
                        delta: 1,
                    });
                    commands.push(BattleCommand::EmitEvent(BattleEvent::StatStageChanged {
                        target: defender_pokemon.species,
                        stat: crate::player::StatType::Attack,
                        old_stage,
                        new_stage,
                    }));
                }
            }
        }
        
        // TODO: In future iterations, add:
        // - Move effects
        // - Status applications
        // - Fainting checks (for normal damage case)
    } else {
        // Move misses - emit miss event
        commands.push(BattleCommand::EmitEvent(BattleEvent::MoveMissed {
            attacker: attacker_pokemon.species,
            defender: defender_pokemon.species,
            move_used,
        }));
    }
    
    commands
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
            team: [Some(pokemon1), const { None }, const { None }, const { None }, const { None }, const { None }],
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
            team: [Some(pokemon2), const { None }, const { None }, const { None }, const { None }, const { None }],
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
        
        assert!(matches!(commands[0], BattleCommand::EmitEvent(BattleEvent::MoveUsed { .. })));
        assert!(matches!(commands[1], BattleCommand::EmitEvent(BattleEvent::MoveHit { .. })));
        
        // Should have DealDamage command (last command after any events)
        assert!(commands.iter().any(|cmd| matches!(cmd, BattleCommand::DealDamage { .. })));
        
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
        
        assert!(matches!(commands[0], BattleCommand::EmitEvent(BattleEvent::MoveUsed { .. })));
        assert!(matches!(commands[1], BattleCommand::EmitEvent(BattleEvent::MoveMissed { .. })));
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
        state.players[1].add_condition(crate::player::PokemonCondition::Substitute { hp: 50 });
        
        let mut rng = TurnRng::new_for_test(vec![1, 99, 50, 50, 50]); // Hit + no critical hit + damage calculation values
        
        let commands = calculate_attack_outcome(&state, 0, 1, Move::Tackle, 0, &mut rng);
        
        // Should have MoveUsed, MoveHit, and substitute-related commands
        assert!(commands.len() >= 3);
        
        assert!(matches!(commands[0], BattleCommand::EmitEvent(BattleEvent::MoveUsed { .. })));
        assert!(matches!(commands[1], BattleCommand::EmitEvent(BattleEvent::MoveHit { .. })));
        
        // Should have a DamageDealt event with 0 damage (substitute absorbed it)
        assert!(commands.iter().any(|cmd| matches!(cmd, BattleCommand::EmitEvent(BattleEvent::DamageDealt { damage: 0, .. }))));
        
        // Should have condition update commands (RemoveCondition and possibly AddCondition if substitute survives)
        assert!(commands.iter().any(|cmd| matches!(cmd, BattleCommand::RemoveCondition { .. })));
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
        state.players[1].add_condition(crate::player::PokemonCondition::Substitute { hp: 1 });
        
        let mut rng = TurnRng::new_for_test(vec![1, 99, 50, 50, 50]); // Hit + no critical hit + damage calculation values
        
        let commands = calculate_attack_outcome(&state, 0, 1, Move::Tackle, 0, &mut rng);
        
        // Should have substitute destruction event
        assert!(commands.iter().any(|cmd| matches!(cmd, BattleCommand::EmitEvent(BattleEvent::StatusRemoved { .. }))));
        
        // Should only have RemoveCondition (no AddCondition since substitute is destroyed)
        let remove_condition_count = commands.iter().filter(|cmd| matches!(cmd, BattleCommand::RemoveCondition { .. })).count();
        let add_condition_count = commands.iter().filter(|cmd| matches!(cmd, BattleCommand::AddCondition { .. })).count();
        
        assert_eq!(remove_condition_count, 1);
        assert_eq!(add_condition_count, 0);
    }
}