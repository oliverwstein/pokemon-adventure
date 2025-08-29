use crate::battle::commands::{BattleCommand, ExecutionError, PlayerTarget};
use crate::pokemon::MoveInstance;
use crate::species::Species;
use crate::{BattleState, Move};

/// Execute experience award command
pub fn execute_award_experience(
    recipients: &[(PlayerTarget, usize, u32)],
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    for &(target, pokemon_index, exp_amount) in recipients {
        let player_index = target.to_index();

        if let Some(pokemon) = state.players[player_index].team[pokemon_index].as_mut() {
            pokemon.add_experience(exp_amount);
        }
    }

    Ok(vec![])
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
    } else {
        return Err(ExecutionError::NoPokemon);
    }

    Ok(vec![])
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
        pokemon.evolve(new_species);
    } else {
        return Err(ExecutionError::NoPokemon);
    }

    Ok(vec![])
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
