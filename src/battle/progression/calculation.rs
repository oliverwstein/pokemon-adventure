use crate::battle::commands::{BattleCommand, PlayerTarget};
use crate::battle::state::BattleType;
use crate::progression::RewardCalculator;
use crate::species::Species;
use crate::BattleState;

/// Calculate all progression commands that should be executed when a Pokemon faints
/// This is the main integration point between battle system and progression system
pub fn calculate_progression_commands(
    fainted_target: PlayerTarget, // Which player's Pokemon fainted
    fainted_species: Species,     // Species of fainted Pokemon
    battle_state: &BattleState,   // Complete battle state for context
) -> Vec<BattleCommand> {
    // Only award progression rewards for appropriate battle types
    // Tournament battles explicitly have "no EXP or other rewards"
    match battle_state.battle_type {
        BattleType::Tournament => return Vec::new(),
        BattleType::Trainer | BattleType::Wild | BattleType::Safari => {
            // These battle types award experience and progression rewards
        }
    }

    let mut commands = Vec::new();
    let calculator = RewardCalculator;

    // Get the index of the Pokemon that fainted
    let fainted_player_index = fainted_target.to_index();
    let fainted_pokemon_index = battle_state.players[fainted_player_index].active_pokemon_index;

    // Get base experience and EV yield for the fainted Pokemon
    let base_exp = match calculator.calculate_base_exp(fainted_species) {
        Ok(exp) => exp,
        Err(_) => return commands, // Skip if species data unavailable
    };

    let ev_yield = match calculator.calculate_ev_yield(fainted_species) {
        Ok(yield_) => yield_,
        Err(_) => return commands, // Skip if species data unavailable
    };

    // Get participants from the participation tracker
    let participants = battle_state
        .participation_tracker
        .get_participants_against(fainted_player_index, fainted_pokemon_index);

    // Calculate experience share among participants
    let exp_per_participant = if participants.is_empty() {
        0
    } else {
        base_exp / participants.len() as u32
    };

    // Award experience and EVs to each participant
    let opposing_player = PlayerTarget::from_index(1 - fainted_player_index);
    let mut experience_recipients = Vec::new();

    for &participant_index in &participants {
        // Check if the participant is still alive and not at max level
        if let Some(pokemon) =
            battle_state.players[1 - fainted_player_index].team[participant_index].as_ref()
        {
            if pokemon.current_hp() > 0 && pokemon.level < 100 {
                experience_recipients.push((
                    opposing_player,
                    participant_index,
                    exp_per_participant,
                ));

                // Award EVs to this participant
                let ev_stats = [
                    ev_yield.hp,
                    ev_yield.attack,
                    ev_yield.defense,
                    ev_yield.special_attack,
                    ev_yield.special_defense,
                    ev_yield.speed,
                ];
                commands.push(BattleCommand::DistributeEffortValues {
                    target: opposing_player,
                    pokemon_index: participant_index,
                    stats: ev_stats,
                });
            }
        }
    }

    // Add the experience award command if there are valid recipients
    if !experience_recipients.is_empty() {
        commands.push(BattleCommand::AwardExperience {
            recipients: experience_recipients,
        });

        // For each participant, check if they should level up, evolve, or learn moves
        for &participant_index in &participants {
            if let Some(pokemon) =
                battle_state.players[1 - fainted_player_index].team[participant_index].as_ref()
            {
                if pokemon.current_hp() > 0 && pokemon.level < 100 {
                    // Calculate new experience and potential level ups
                    let new_exp = pokemon.curr_exp + exp_per_participant;
                    let species_data = match crate::get_species_data(pokemon.species) {
                        Ok(data) => data,
                        Err(_) => continue,
                    };

                    let new_level = species_data
                        .experience_group
                        .calculate_level_from_exp(new_exp);

                    // Generate level up commands for each level gained
                    for level in (pokemon.level + 1)..=new_level {
                        commands.push(BattleCommand::LevelUpPokemon {
                            target: opposing_player,
                            pokemon_index: participant_index,
                        });

                        // Check for moves learned at this level
                        if let Ok(moves) = calculator.moves_learned_at_level(pokemon.species, level)
                        {
                            for move_ in moves {
                                commands.push(BattleCommand::LearnMove {
                                    target: opposing_player,
                                    pokemon_index: participant_index,
                                    move_,
                                    replace_index: None, // Let the execution logic decide
                                });
                            }
                        }

                        // Check for evolution at this level
                        if let Ok(Some(new_species)) =
                            calculator.should_evolve(pokemon.species, level)
                        {
                            commands.push(BattleCommand::EvolvePokemon {
                                target: opposing_player,
                                pokemon_index: participant_index,
                                new_species,
                            });
                        }
                    }
                }
            }
        }
    }

    commands
}
