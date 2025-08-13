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
        
        // Calculate critical hit for normal (non-special) damage moves
        if crate::battle::stats::calculate_special_attack_damage(
            move_used,
            attacker_pokemon,
            defender_pokemon,
        ).is_none() {
            // This is a normal damage move, check for critical hit
            let is_critical = move_is_critical_hit(attacker_pokemon, attacker_player, move_used, rng);
            
            if is_critical {
                commands.push(BattleCommand::EmitEvent(BattleEvent::CriticalHit {
                    attacker: attacker_pokemon.species,
                    defender: defender_pokemon.species,
                    move_used,
                }));
            }
        }
        
        // TODO: In future iterations, add:
        // - Damage calculation
        // - Move effects
        // - Status applications
        // - Fainting checks
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
        let mut rng = TurnRng::new_for_test(vec![1, 99]); // Hit + no critical hit
        
        let commands = calculate_attack_outcome(&state, 0, 1, Move::Tackle, 0, &mut rng);
        
        // Should have MoveUsed, MoveHit, and possibly type effectiveness events
        assert!(commands.len() >= 2);
        
        assert!(matches!(commands[0], BattleCommand::EmitEvent(BattleEvent::MoveUsed { .. })));
        assert!(matches!(commands[1], BattleCommand::EmitEvent(BattleEvent::MoveHit { .. })));
        
        // May have type effectiveness event if not normal effectiveness
        // May have critical hit event if critical hit occurred
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
}