use crate::battle::catch::{
    calculate_catch_rate, can_attempt_catch, roll_catch_success, CatchError,
};
use crate::battle::commands::BattleCommand;
use crate::battle::state::{BattleEvent, BattleState, CatchFailureReason, TurnRng};
use crate::species::Species;

/// Calculate commands for a catch attempt
/// This follows the Command-Execution pattern by returning commands to be executed
pub fn calculate_catch_commands(
    player_index: usize,
    target_species: Species,
    battle_state: &BattleState,
    rng: &mut TurnRng,
) -> Vec<BattleCommand> {
    let mut commands = vec![];

    // First, validate the catch attempt
    match can_attempt_catch(battle_state, player_index) {
        Ok(validated_species) => {
            // Double check that the validated species matches what we're trying to catch
            if validated_species != target_species {
                commands.push(BattleCommand::EmitEvent(BattleEvent::CatchFailed {
                    player_index,
                    pokemon: target_species,
                    reason: CatchFailureReason::NoTargetPokemon,
                }));
                return commands;
            }

            // Get the target Pokemon for catch rate calculation
            let opponent_index = 1 - player_index;
            let opponent = &battle_state.players[opponent_index];

            if let Some(target_pokemon) = opponent.active_pokemon() {
                // Calculate catch rate
                let catch_rate = calculate_catch_rate(target_pokemon, 1.0); // 1.0 for regular Pokeball

                // Emit catch attempted event
                commands.push(BattleCommand::EmitEvent(BattleEvent::CatchAttempted {
                    player_index,
                    pokemon: target_species,
                    catch_rate,
                }));

                // Roll for success
                if roll_catch_success(catch_rate, rng) {
                    // Success! Add the Pokemon to the player's team
                    commands.push(BattleCommand::AttemptCatch {
                        player_index,
                        target_pokemon: target_species,
                    });
                    // Note: BattleEvent::CatchSucceeded is emitted by the command's emit_events()
                } else {
                    // Failed catch
                    commands.push(BattleCommand::EmitEvent(BattleEvent::CatchFailed {
                        player_index,
                        pokemon: target_species,
                        reason: CatchFailureReason::RollFailed { catch_rate },
                    }));
                }
            } else {
                // No target Pokemon
                commands.push(BattleCommand::EmitEvent(BattleEvent::CatchFailed {
                    player_index,
                    pokemon: target_species,
                    reason: CatchFailureReason::NoTargetPokemon,
                }));
            }
        }
        Err(error) => {
            // Convert CatchError to CatchFailureReason and emit failure event
            let failure_reason = match error {
                CatchError::InvalidBattleType { battle_type } => {
                    CatchFailureReason::InvalidBattleType { battle_type }
                }
                CatchError::NoTargetPokemon => CatchFailureReason::NoTargetPokemon,
                CatchError::TeamFull => CatchFailureReason::TeamFull,
                CatchError::TargetFainted { pokemon } => {
                    CatchFailureReason::TargetFainted { pokemon }
                }
            };

            commands.push(BattleCommand::EmitEvent(BattleEvent::CatchFailed {
                player_index,
                pokemon: target_species,
                reason: failure_reason,
            }));
        }
    }

    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::state::{BattleState, BattleType};
    use crate::player::{BattlePlayer, PlayerType};
    use crate::pokemon::{get_species_data, PokemonInst};
    use crate::species::Species;

    fn create_test_battle_state(battle_type: BattleType, player1_team_size: usize) -> BattleState {
        // Create player 1 with variable team size
        let mut player1_team = vec![];
        for _i in 0..player1_team_size {
            let species_data = get_species_data(Species::Pikachu).unwrap();
            let pokemon = PokemonInst::new(Species::Pikachu, &species_data, 25, None, None);
            player1_team.push(pokemon);
        }

        // Create opponent with one Pokemon
        let opponent_species_data = get_species_data(Species::Charmander).unwrap();
        let opponent_pokemon =
            PokemonInst::new(Species::Charmander, &opponent_species_data, 25, None, None);

        let player1 = BattlePlayer::new_with_player_type(
            "p1".to_string(),
            "Player 1".to_string(),
            player1_team,
            PlayerType::Human,
        );
        let opponent = BattlePlayer::new_with_player_type(
            "wild".to_string(),
            "Wild Pokemon".to_string(),
            vec![opponent_pokemon],
            PlayerType::NPC,
        );

        let mut battle_state = BattleState::new("test".to_string(), player1, opponent);
        battle_state.battle_type = battle_type;
        battle_state
    }

    #[test]
    fn test_catch_commands_invalid_battle_type() {
        let battle_state = create_test_battle_state(BattleType::Trainer, 1);
        let mut rng = TurnRng::new_for_test(vec![100]); // High roll, would succeed if allowed

        let commands = calculate_catch_commands(0, Species::Charmander, &battle_state, &mut rng);

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            BattleCommand::EmitEvent(BattleEvent::CatchFailed { reason, .. }) => {
                assert!(matches!(
                    reason,
                    CatchFailureReason::InvalidBattleType { .. }
                ));
            }
            _ => panic!("Expected CatchFailed event"),
        }
    }

    #[test]
    fn test_catch_commands_team_full() {
        let battle_state = create_test_battle_state(BattleType::Wild, 6);
        let mut rng = TurnRng::new_for_test(vec![100]);

        let commands = calculate_catch_commands(0, Species::Charmander, &battle_state, &mut rng);

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            BattleCommand::EmitEvent(BattleEvent::CatchFailed { reason, .. }) => {
                assert!(matches!(reason, CatchFailureReason::TeamFull));
            }
            _ => panic!("Expected CatchFailed event"),
        }
    }

    #[test]
    fn test_catch_commands_success() {
        let battle_state = create_test_battle_state(BattleType::Wild, 1);
        let mut rng = TurnRng::new_for_test(vec![1]); // Very low roll, should succeed

        let commands = calculate_catch_commands(0, Species::Charmander, &battle_state, &mut rng);

        // Should have: CatchAttempted event, then AttemptCatch command
        assert_eq!(commands.len(), 2);

        match &commands[0] {
            BattleCommand::EmitEvent(BattleEvent::CatchAttempted {
                player_index,
                pokemon,
                ..
            }) => {
                assert_eq!(*player_index, 0);
                assert_eq!(*pokemon, Species::Charmander);
            }
            _ => panic!("Expected CatchAttempted event"),
        }

        match &commands[1] {
            BattleCommand::AttemptCatch {
                player_index,
                target_pokemon,
            } => {
                assert_eq!(*player_index, 0);
                assert_eq!(*target_pokemon, Species::Charmander);
            }
            _ => panic!("Expected AttemptCatch command"),
        }
    }

    #[test]
    fn test_catch_commands_failure() {
        let battle_state = create_test_battle_state(BattleType::Wild, 1);
        let mut rng = TurnRng::new_for_test(vec![255]); // Max roll, should fail

        let commands = calculate_catch_commands(0, Species::Charmander, &battle_state, &mut rng);

        // Should have: CatchAttempted event, then CatchFailed event
        assert_eq!(commands.len(), 2);

        match &commands[0] {
            BattleCommand::EmitEvent(BattleEvent::CatchAttempted { .. }) => (),
            _ => panic!("Expected CatchAttempted event"),
        }

        match &commands[1] {
            BattleCommand::EmitEvent(BattleEvent::CatchFailed { reason, .. }) => {
                assert!(matches!(reason, CatchFailureReason::RollFailed { .. }));
            }
            _ => panic!("Expected CatchFailed event"),
        }
    }
}
