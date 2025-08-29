use crate::{battle::commands::PlayerTarget, species::Species, BattleState};

/// Errors that can occur during progression validation
#[derive(Debug, Clone, PartialEq)]
pub enum ProgressionError {
    NoPokemon {
        player_index: usize,
        pokemon_index: usize,
    },
    PokemonFainted {
        player_index: usize,
        pokemon: Species,
    },
    MaxLevel {
        pokemon: Species,
        level: u8,
    },
    InvalidIndices {
        player_index: usize,
        pokemon_index: usize,
    },
}

/// Validate that a Pokemon can receive experience in battle
pub fn can_award_experience_in_battle(
    target: PlayerTarget,
    pokemon_index: usize,
    state: &BattleState,
) -> Result<(), ProgressionError> {
    let player_index = target.to_index();

    if player_index >= 2 || pokemon_index >= 6 {
        return Err(ProgressionError::InvalidIndices {
            player_index,
            pokemon_index,
        });
    }

    match state.players[player_index].team[pokemon_index].as_ref() {
        Some(pokemon) => {
            if pokemon.current_hp() == 0 {
                Err(ProgressionError::PokemonFainted {
                    player_index,
                    pokemon: pokemon.species,
                })
            } else if pokemon.level >= 100 {
                Err(ProgressionError::MaxLevel {
                    pokemon: pokemon.species,
                    level: pokemon.level,
                })
            } else {
                Ok(())
            }
        }
        None => Err(ProgressionError::NoPokemon {
            player_index,
            pokemon_index,
        }),
    }
}

/// Validate that a Pokemon can level up in battle
pub fn can_level_up_in_battle(
    target: PlayerTarget,
    pokemon_index: usize,
    state: &BattleState,
) -> Result<(), ProgressionError> {
    // Same validation as experience award
    can_award_experience_in_battle(target, pokemon_index, state)
}

/// Validate that a Pokemon can learn a move in battle
pub fn can_learn_move_in_battle(
    target: PlayerTarget,
    pokemon_index: usize,
    state: &BattleState,
) -> Result<(), ProgressionError> {
    let player_index = target.to_index();

    if player_index >= 2 || pokemon_index >= 6 {
        return Err(ProgressionError::InvalidIndices {
            player_index,
            pokemon_index,
        });
    }

    match state.players[player_index].team[pokemon_index].as_ref() {
        Some(_pokemon) => {
            // Pokemon exists, can learn moves (no additional restrictions for now)
            Ok(())
        }
        None => Err(ProgressionError::NoPokemon {
            player_index,
            pokemon_index,
        }),
    }
}

/// Validate that a Pokemon can evolve in battle
pub fn can_evolve_in_battle(
    target: PlayerTarget,
    pokemon_index: usize,
    state: &BattleState,
) -> Result<(), ProgressionError> {
    // Same validation as move learning
    can_learn_move_in_battle(target, pokemon_index, state)
}
