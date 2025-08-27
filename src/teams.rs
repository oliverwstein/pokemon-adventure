use crate::player::BattlePlayer;
use crate::pokemon::PokemonInst;
use crate::species::Species;
use schema::Move;
use std::collections::HashMap;
use std::sync::LazyLock;

// Include the generated team data
include!(concat!(env!("OUT_DIR"), "/generated_data.rs"));

// Lazy-loaded team data
static TEAM_DATA: LazyLock<HashMap<String, TeamTemplate>> =
    LazyLock::new(|| get_compiled_team_data());

/// Create a Pokemon from a template, using either specified moves or learnset moves
pub fn create_pokemon_from_template(template: &PokemonTemplate) -> Result<PokemonInst, String> {
    match &template.moves {
        Some(specified_moves) => {
            // Use specified moves (demo team style)
            {
                let species_data = crate::get_species_data(template.species).map_err(|e| {
                    format!(
                        "Failed to get species data for {:?}: {}",
                        template.species, e
                    )
                })?;
                Ok(PokemonInst::new(
                    template.species,
                    &species_data,
                    template.level,
                    None,
                    Some(specified_moves.clone()),
                ))
            }
        }
        None => {
            // Use learnset moves up to this level
            let learnset_moves = get_moves_learned_by_level(template.species, template.level)?;
            {
                let species_data = crate::get_species_data(template.species).map_err(|e| {
                    format!(
                        "Failed to get species data for {:?}: {}",
                        template.species, e
                    )
                })?;
                Ok(PokemonInst::new(
                    template.species,
                    &species_data,
                    template.level,
                    None,
                    Some(learnset_moves),
                ))
            }
        }
    }
}

/// Get moves that a Pokemon would naturally learn by a given level
fn get_moves_learned_by_level(species: Species, level: u8) -> Result<Vec<Move>, String> {
    // Get the species data to access learnset
    let species_data = crate::get_species_data(species)
        .map_err(|e| format!("Failed to get species data for {:?}: {}", species, e))?;

    let mut learned_moves = Vec::new();

    // Collect moves learned by this level
    for (learn_level, moves_at_level) in &species_data.learnset.level_up {
        if *learn_level <= level {
            learned_moves.extend(moves_at_level.iter().copied());
        }
    }

    // Pokemon can only know 4 moves, so take the last 4 moves learned
    if learned_moves.len() > 4 {
        learned_moves = learned_moves.into_iter().rev().take(4).rev().collect();
    }

    // If no moves learned by this level, give a basic move
    if learned_moves.is_empty() {
        learned_moves.push(Move::Tackle); // Default move
    }

    Ok(learned_moves)
}

/// Create a team of Pokemon from a team template
pub fn create_team_from_template(team_id: &str) -> Option<Vec<PokemonInst>> {
    TEAM_DATA.get(team_id).map(|team| {
        team.pokemon
            .iter()
            .filter_map(|template| create_pokemon_from_template(template).ok())
            .collect()
    })
}

/// Get all available team IDs
pub fn get_available_team_ids() -> Vec<String> {
    TEAM_DATA.keys().cloned().collect()
}

/// Get team information without creating Pokemon instances
pub fn get_team_info(team_id: &str) -> Option<&TeamTemplate> {
    TEAM_DATA.get(team_id)
}

/// Convert a team template into a BattlePlayer for use in battles
pub fn create_battle_player_from_team(
    team_id: &str,
    player_id: String,
    player_name: String,
) -> Result<BattlePlayer, String> {
    let team_pokemon = create_team_from_template(team_id)
        .ok_or_else(|| format!("Team '{}' not found", team_id))?;

    if team_pokemon.is_empty() {
        return Err(format!("Team '{}' has no valid Pokemon", team_id));
    }

    Ok(BattlePlayer::new(player_id, player_name, team_pokemon))
}

// Maintain compatibility with existing prefab_teams.rs functions
pub fn get_venusaur_team() -> Vec<PokemonInst> {
    create_team_from_template("demo_venusaur").expect("Demo Venusaur team not found")
}

pub fn get_blastoise_team() -> Vec<PokemonInst> {
    create_team_from_template("demo_blastoise").expect("Demo Blastoise team not found")
}

pub fn get_charizard_team() -> Vec<PokemonInst> {
    create_team_from_template("demo_charizard").expect("Demo Charizard team not found")
}

/// Get all available demo teams (balanced options)
pub fn get_demo_team_ids() -> Vec<String> {
    vec![
        "demo_venusaur".to_string(),
        "demo_blastoise".to_string(),
        "demo_charizard".to_string(),
    ]
}

/// Create a random balanced NPC team for battles
pub fn create_random_npc_team(selection: &str) -> Result<BattlePlayer, String> {
    let team_id = match selection {
        "venusaur" => "demo_venusaur",
        "blastoise" => "demo_blastoise",
        "charizard" => "demo_charizard",
        "random" => {
            // Pick a random demo team - for now, just pick first. TODO: Add actual randomization
            "demo_venusaur"
        }
        _ => "demo_venusaur", // Default to Venusaur team
    };

    create_battle_player_from_team(
        team_id,
        "npc".to_string(),
        format!("NPC Trainer ({})", selection),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_loading() {
        let team_ids = get_available_team_ids();
        assert!(!team_ids.is_empty(), "Should have at least one team");

        // Check for demo teams
        assert!(team_ids.contains(&"demo_venusaur".to_string()));
        assert!(team_ids.contains(&"demo_blastoise".to_string()));
        assert!(team_ids.contains(&"demo_charizard".to_string()));
    }

    #[test]
    fn test_demo_teams_are_balanced() {
        let demo_teams = get_demo_team_ids();
        assert_eq!(demo_teams.len(), 3, "Should have exactly 3 demo teams");

        // All demo teams should be level 60 and have 6 Pokemon
        for team_id in demo_teams {
            let team_info = get_team_info(&team_id).expect("Demo team should exist");
            assert_eq!(
                team_info.pokemon.len(),
                6,
                "Demo teams should have 6 Pokemon"
            );

            // All Pokemon should be level 60
            for pokemon in &team_info.pokemon {
                assert_eq!(pokemon.level, 60, "Demo team Pokemon should be level 60");
            }
        }
    }

    #[test]
    fn test_create_team_from_template() {
        let venusaur_team = create_team_from_template("demo_venusaur");
        assert!(venusaur_team.is_some());

        let team = venusaur_team.unwrap();
        assert_eq!(team.len(), 6);
        assert_eq!(team[0].species, Species::Venusaur);
        assert_eq!(team[0].level, 60);
    }

    #[test]
    fn test_create_battle_player_from_team() {
        let result = create_battle_player_from_team(
            "demo_venusaur",
            "test_player".to_string(),
            "Test Player".to_string(),
        );

        assert!(result.is_ok(), "Error: {:?}", result.err());

        let player = result.unwrap();
        assert_eq!(player.player_id, "test_player");
        assert_eq!(player.player_name, "Test Player");

        // Check that we have 6 Pokemon
        let team_count = player.team.iter().filter(|p| p.is_some()).count();
        assert_eq!(team_count, 6);

        // Check that the first Pokemon is Venusaur
        assert!(player.team[0].is_some());
        let first_pokemon = player.team[0].as_ref().unwrap();
        assert_eq!(first_pokemon.species, Species::Venusaur);
        assert_eq!(first_pokemon.level, 60);
    }

    #[test]
    fn test_compatibility_functions() {
        let venusaur_team = get_venusaur_team();
        assert_eq!(venusaur_team.len(), 6);
        assert_eq!(venusaur_team[0].species, Species::Venusaur);

        let blastoise_team = get_blastoise_team();
        assert_eq!(blastoise_team.len(), 6);
        assert_eq!(blastoise_team[0].species, Species::Blastoise);

        let charizard_team = get_charizard_team();
        assert_eq!(charizard_team.len(), 6);
        assert_eq!(charizard_team[0].species, Species::Charizard);
    }
}
