use crate::battle::commands::{BattleCommand, ExecutionError, PlayerTarget};
use crate::pokemon::MoveInstance;
use crate::species::Species;
use crate::{BattleState, Move};

/// Execute experience award command
pub fn execute_award_experience(
    recipients: &[(PlayerTarget, usize, u32)],
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    let mut additional_commands = Vec::new();

    for &(target, pokemon_index, exp_amount) in recipients {
        let player_index = target.to_index();

        if let Some(pokemon) = state.players[player_index].team[pokemon_index].as_mut() {
            let old_level = pokemon.level;

            if let Some(new_level) = pokemon.add_experience(exp_amount) {
                // Generate level-up commands in ascending order (lower levels first)
                // The command execution system will reverse and process LIFO,
                // ensuring 15→16 executes before 16→17, etc.
                for _level in (old_level + 1)..=new_level {
                    additional_commands.push(BattleCommand::LevelUpPokemon {
                        target,
                        pokemon_index,
                    });
                }
            }
        }
    }

    Ok(additional_commands)
}

/// Execute level up command
pub fn execute_level_up_pokemon(
    target: PlayerTarget,
    pokemon_index: usize,
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    let player_index = target.to_index();

    if let Some(pokemon) = state.players[player_index].team[pokemon_index].as_mut() {
        pokemon.apply_level_up();
        let current_level = pokemon.level;
        let mut additional_commands = Vec::new();

        let calculator = crate::progression::RewardCalculator;

        // First, check for moves learned at this level (before evolution)
        if let Ok(moves) = calculator.moves_learned_at_level(pokemon.species, current_level) {
            for move_ in moves {
                additional_commands.push(BattleCommand::LearnMove {
                    target,
                    pokemon_index,
                    move_,
                    replace_index: None, // Let execute_learn_move handle the choice
                });
            }
        }

        // Then, check for evolution at this level (after moves)
        if let Ok(Some(new_species)) = calculator.should_evolve(pokemon) {
            additional_commands.push(BattleCommand::EvolvePokemon {
                target,
                pokemon_index,
                new_species,
            });
        }

        Ok(additional_commands)
    } else {
        Err(ExecutionError::NoPokemon)
    }
}

/// Execute learn move command
pub fn execute_learn_move(
    target: PlayerTarget,
    pokemon_index: usize,
    move_: Move,
    replace_index: Option<usize>,
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    let player_index = target.to_index();

    if let Some(pokemon) = state.players[player_index].team[pokemon_index].as_mut() {
        let new_move = MoveInstance::new(move_);

        match replace_index {
            Some(index) => {
                if index >= 4 {
                    return Err(ExecutionError::InvalidMoveIndex);
                }
                pokemon.moves[index] = Some(new_move);
            }
            None => {
                // Find the first empty slot
                let empty_slot = pokemon.moves.iter_mut().find(|slot| slot.is_none());
                match empty_slot {
                    Some(slot) => *slot = Some(new_move),
                    None => {
                        // No empty slot, replace the last move as default
                        pokemon.moves[3] = Some(new_move);
                    }
                }
            }
        }
    } else {
        return Err(ExecutionError::NoPokemon);
    }

    Ok(vec![])
}

/// Execute evolution command
pub fn execute_evolve_pokemon(
    target: PlayerTarget,
    pokemon_index: usize,
    new_species: Species,
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    let player_index = target.to_index();

    if let Some(pokemon) = state.players[player_index].team[pokemon_index].as_mut() {
        let current_level = pokemon.level;
        pokemon.evolve(new_species);

        let mut additional_commands = Vec::new();
        let calculator = crate::progression::RewardCalculator;

        // Check if the newly evolved Pokemon learns any moves at this level
        if let Ok(moves) = calculator.moves_learned_at_level(new_species, current_level) {
            for move_ in moves {
                additional_commands.push(BattleCommand::LearnMove {
                    target,
                    pokemon_index,
                    move_,
                    replace_index: None, // Let execute_learn_move handle the choice
                });
            }
        }

        Ok(additional_commands)
    } else {
        Err(ExecutionError::NoPokemon)
    }
}

/// Execute effort values distribution command
pub fn execute_distribute_effort_values(
    target: PlayerTarget,
    pokemon_index: usize,
    stats: [u8; 6],
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    let player_index = target.to_index();

    if let Some(pokemon) = state.players[player_index].team[pokemon_index].as_mut() {
        pokemon.add_evs(stats);
    } else {
        return Err(ExecutionError::NoPokemon);
    }

    Ok(vec![])
}
