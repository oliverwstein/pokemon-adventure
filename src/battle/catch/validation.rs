use crate::battle::state::{BattleState, BattleType};
use crate::species::Species;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CatchError {
    /// Catch attempts not allowed in this battle type
    InvalidBattleType { battle_type: BattleType },
    /// No active Pokemon on opponent's side to catch
    NoTargetPokemon,
    /// Player's team is full (6 Pokemon already)
    TeamFull,
    /// Target Pokemon is already fainted
    TargetFainted { pokemon: Species },
}

/// Check if catch attempts are allowed based on battle type
pub fn is_catch_allowed(battle_type: BattleType) -> bool {
    matches!(battle_type, BattleType::Wild | BattleType::Safari)
}

/// Validate if a catch attempt can be made and return the target species if valid
pub fn can_attempt_catch(
    battle_state: &BattleState,
    player_index: usize,
) -> Result<Species, CatchError> {
    // Check battle type
    if !is_catch_allowed(battle_state.battle_type) {
        return Err(CatchError::InvalidBattleType {
            battle_type: battle_state.battle_type,
        });
    }

    // Check if player's team is full
    let player = &battle_state.players[player_index];
    let team_count = player.team.iter().flatten().count();
    if team_count >= 6 {
        return Err(CatchError::TeamFull);
    }

    // Get opponent's active Pokemon
    let opponent_index = 1 - player_index;
    let opponent = &battle_state.players[opponent_index];

    match opponent.active_pokemon() {
        Some(target_pokemon) => {
            if target_pokemon.is_fainted() {
                Err(CatchError::TargetFainted {
                    pokemon: target_pokemon.species,
                })
            } else {
                Ok(target_pokemon.species)
            }
        }
        None => Err(CatchError::NoTargetPokemon),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::state::BattleState;
    use crate::player::{BattlePlayer, PlayerType};
    use crate::pokemon::{get_species_data, PokemonInst};
    use crate::species::Species;

    fn create_test_battle_state(
        battle_type: BattleType,
        player1_team_size: usize,
        opponent_fainted: bool,
    ) -> BattleState {
        // Create player 1 with variable team size
        let mut player1_team = vec![];
        for _i in 0..player1_team_size {
            let species_data = get_species_data(Species::Pikachu).unwrap();
            let pokemon = PokemonInst::new(Species::Pikachu, &species_data, 25, None, None);
            player1_team.push(pokemon);
        }

        // Create opponent with one Pokemon
        let opponent_species_data = get_species_data(Species::Charmander).unwrap();
        let mut opponent_pokemon =
            PokemonInst::new(Species::Charmander, &opponent_species_data, 25, None, None);

        if opponent_fainted {
            opponent_pokemon.take_damage(opponent_pokemon.current_hp()); // Faint the Pokemon
        }

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
    fn test_is_catch_allowed() {
        assert!(is_catch_allowed(BattleType::Wild));
        assert!(is_catch_allowed(BattleType::Safari));
        assert!(!is_catch_allowed(BattleType::Trainer));
        assert!(!is_catch_allowed(BattleType::Tournament));
    }

    #[test]
    fn test_can_attempt_catch_success() {
        let battle_state = create_test_battle_state(BattleType::Wild, 1, false);
        let result = can_attempt_catch(&battle_state, 0);
        assert_eq!(result, Ok(Species::Charmander));
    }

    #[test]
    fn test_can_attempt_catch_invalid_battle_type() {
        let battle_state = create_test_battle_state(BattleType::Trainer, 1, false);
        let result = can_attempt_catch(&battle_state, 0);
        assert_eq!(
            result,
            Err(CatchError::InvalidBattleType {
                battle_type: BattleType::Trainer
            })
        );
    }

    #[test]
    fn test_can_attempt_catch_team_full() {
        let battle_state = create_test_battle_state(BattleType::Wild, 6, false);
        let result = can_attempt_catch(&battle_state, 0);
        assert_eq!(result, Err(CatchError::TeamFull));
    }

    #[test]
    fn test_can_attempt_catch_target_fainted() {
        let battle_state = create_test_battle_state(BattleType::Wild, 1, true);
        let result = can_attempt_catch(&battle_state, 0);
        assert_eq!(
            result,
            Err(CatchError::TargetFainted {
                pokemon: Species::Charmander
            })
        );
    }
}
