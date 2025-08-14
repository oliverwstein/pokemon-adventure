use crate::moves::Move;
use crate::player::BattlePlayer;
use crate::pokemon::PokemonInst;
use crate::species::Species;
use serde::{Deserialize, Serialize};

/// A predefined team configuration for guest battles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabTeam {
    pub id: String,
    pub name: String,
    pub description: String,
    pub pokemon: Vec<PrefabPokemon>,
}

/// A predefined Pokemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabPokemon {
    pub species: Species,
    pub level: u8,
    pub moves: Vec<Move>,
}

/// Get all available prefab teams for guest battles
pub fn get_prefab_teams() -> Vec<PrefabTeam> {
    vec![
        PrefabTeam {
            id: "venusaur_team".to_string(),
            name: "Venusaur Team".to_string(),
            description: "Elite team featuring Venusaur with diverse type coverage and strategic options".to_string(),
            pokemon: vec![
                PrefabPokemon {
                    species: Species::Venusaur,
                    level: 60,
                    moves: vec![Move::SleepPowder, Move::Solarbeam, Move::PetalDance, Move::Earthquake],
                },
                PrefabPokemon {
                    species: Species::Arcanine,
                    level: 60,
                    moves: vec![Move::RockSlide, Move::Roar, Move::Flamethrower, Move::QuickAttack],
                },
                PrefabPokemon {
                    species: Species::Lapras,
                    level: 60,
                    moves: vec![Move::Blizzard, Move::Surf, Move::Reflect, Move::Substitute],
                },
                PrefabPokemon {
                    species: Species::Nidoking,
                    level: 60,
                    moves: vec![Move::PoisonJab, Move::Submission, Move::Earthquake, Move::Thunderclap],
                },
                PrefabPokemon {
                    species: Species::Hitmonlee,
                    level: 60,
                    moves: vec![Move::HighJumpKick, Move::Submission, Move::MegaKick, Move::FocusEnergy],
                },
                PrefabPokemon {
                    species: Species::Snorlax,
                    level: 60,
                    moves: vec![Move::BodySlam, Move::Counter, Move::Rest, Move::Perplex],
                },
            ],
        },
        PrefabTeam {
            id: "blastoise_team".to_string(),
            name: "Blastoise Team".to_string(),
            description: "Balanced team featuring Blastoise with excellent type diversity and control options".to_string(),
            pokemon: vec![
                PrefabPokemon {
                    species: Species::Blastoise,
                    level: 60,
                    moves: vec![Move::HydroPump, Move::IceBeam, Move::Earthquake, Move::Bide],
                },
                PrefabPokemon {
                    species: Species::Dragonite,
                    level: 60,
                    moves: vec![Move::BodySlam, Move::Earthquake, Move::Thunderclap, Move::Outrage],
                },
                PrefabPokemon {
                    species: Species::Dodrio,
                    level: 60,
                    moves: vec![Move::TriAttack, Move::DrillPeck, Move::Toxic, Move::Whirlwind],
                },
                PrefabPokemon {
                    species: Species::Magneton,
                    level: 60,
                    moves: vec![Move::ChargeBeam, Move::Discharge, Move::ThunderWave, Move::TriAttack],
                },
                PrefabPokemon {
                    species: Species::Exeggutor,
                    level: 60,
                    moves: vec![Move::Perplex, Move::EggBomb, Move::Rest, Move::Hypnosis],
                },
                PrefabPokemon {
                    species: Species::Gengar,
                    level: 60,
                    moves: vec![Move::Hypnosis, Move::DreamEater, Move::ShadowBall, Move::PoisonGas],
                },
            ],
        },
        PrefabTeam {
            id: "charizard_team".to_string(),
            name: "Charizard Team".to_string(),
            description: "Aggressive team featuring Charizard with high offensive potential and versatility".to_string(),
            pokemon: vec![
                PrefabPokemon {
                    species: Species::Charizard,
                    level: 60,
                    moves: vec![Move::Fly, Move::FireSpin, Move::AncientPower, Move::Outrage],
                },
                PrefabPokemon {
                    species: Species::Starmie,
                    level: 60,
                    moves: vec![Move::Perplex, Move::IceBeam, Move::Recover, Move::Bubblebeam],
                },
                PrefabPokemon {
                    species: Species::Raichu,
                    level: 60,
                    moves: vec![Move::Lightning, Move::QuickAttack, Move::Substitute, Move::DoubleTeam],
                },
                PrefabPokemon {
                    species: Species::Machamp,
                    level: 60,
                    moves: vec![Move::RockSlide, Move::Earthquake, Move::Submission, Move::ThunderPunch],
                },
                PrefabPokemon {
                    species: Species::Weezing,
                    level: 60,
                    moves: vec![Move::Explosion, Move::PoisonGas, Move::Toxic, Move::Haze],
                },
                PrefabPokemon {
                    species: Species::Dugtrio,
                    level: 60,
                    moves: vec![Move::Earthquake, Move::RockSlide, Move::Fissure, Move::DoubleTeam],
                },
            ],
        },
    ]
}

/// Get a specific prefab team by ID
pub fn get_prefab_team(team_id: &str) -> Option<PrefabTeam> {
    get_prefab_teams().into_iter().find(|team| team.id == team_id)
}

/// Convert a prefab team into a BattlePlayer for use in battles
pub fn create_battle_player_from_prefab(
    team_id: &str,
    player_id: String,
    player_name: String,
) -> Result<BattlePlayer, String> {
    let prefab_team = get_prefab_team(team_id)
        .ok_or_else(|| format!("Prefab team '{}' not found", team_id))?;

    // Convert prefab Pokemon to actual Pokemon instances
    let mut team_pokemon: Vec<PokemonInst> = Vec::new();
    
    for prefab_pokemon in prefab_team.pokemon.iter() {
        let species_data = crate::pokemon::get_species_data(prefab_pokemon.species)
            .ok_or_else(|| format!("Species data not found for {:?}", prefab_pokemon.species))?;
        
        // Convert Move enum to move instances
        let moves: Vec<Move> = prefab_pokemon.moves.clone();
        
        let pokemon = PokemonInst::new(
            prefab_pokemon.species,
            &species_data,
            prefab_pokemon.level,
            None, // Use default IVs
            Some(moves),
        );
        
        team_pokemon.push(pokemon);
    }
    
    Ok(BattlePlayer::new(player_id, player_name, team_pokemon))
}

/// Generate a random NPC team for battles
pub fn create_random_npc_team(difficulty: &str) -> Result<BattlePlayer, String> {
    let npc_teams = match difficulty {
        "easy" => vec!["venusaur_team"],
        "medium" => vec!["blastoise_team"], 
        "hard" => vec!["charizard_team"],
        _ => vec!["venusaur_team"], // Default to Venusaur team
    };
    
    // For now, just pick the first team of the difficulty
    // TODO: Add randomization when we have more teams per difficulty
    let team_id = npc_teams[0];
    
    create_battle_player_from_prefab(
        team_id,
        "npc".to_string(),
        format!("NPC Trainer ({})", difficulty),
    )
}

/// Validate that all prefab teams are properly configured
pub fn validate_prefab_teams() -> Result<(), String> {
    let teams = get_prefab_teams();
    
    if teams.is_empty() {
        return Err("No prefab teams defined".to_string());
    }
    
    for team in &teams {
        if team.pokemon.is_empty() {
            return Err(format!("Team '{}' has no Pokemon", team.id));
        }
        
        if team.pokemon.len() > 6 {
            return Err(format!("Team '{}' has more than 6 Pokemon", team.id));
        }
        
        for (i, pokemon) in team.pokemon.iter().enumerate() {
            if pokemon.level == 0 || pokemon.level > 100 {
                return Err(format!("Team '{}' Pokemon {} has invalid level {}", team.id, i, pokemon.level));
            }
            
            if pokemon.moves.is_empty() {
                return Err(format!("Team '{}' Pokemon {} has no moves", team.id, i));
            }
            
            if pokemon.moves.len() > 4 {
                return Err(format!("Team '{}' Pokemon {} has more than 4 moves", team.id, i));
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::move_data::initialize_move_data;
    use crate::pokemon::initialize_species_data;
    use std::path::Path;

    #[test]
    fn test_get_prefab_teams() {
        let teams = get_prefab_teams();
        assert!(!teams.is_empty());
        
        // Check that we have the expected teams
        let team_ids: Vec<String> = teams.iter().map(|t| t.id.clone()).collect();
        assert!(team_ids.contains(&"venusaur_team".to_string()));
        assert!(team_ids.contains(&"blastoise_team".to_string()));
        assert!(team_ids.contains(&"charizard_team".to_string()));
    }

    #[test]
    fn test_get_prefab_team() {
        let team = get_prefab_team("venusaur_team");
        assert!(team.is_some());
        
        let team = team.unwrap();
        assert_eq!(team.id, "venusaur_team");
        assert_eq!(team.pokemon.len(), 6);
        assert_eq!(team.pokemon[0].species, Species::Venusaur);
        
        // Test non-existent team
        let team = get_prefab_team("non_existent");
        assert!(team.is_none());
    }

    #[test]
    fn test_create_battle_player_from_prefab() {
        // Initialize required data
        let _ = initialize_move_data(Path::new("data"));
        let _ = initialize_species_data(Path::new("data"));
        
        let result = create_battle_player_from_prefab(
            "venusaur_team",
            "test_player".to_string(),
            "Test Player".to_string(),
        );
        
        assert!(result.is_ok(), "Error: {:?}", result.err());
        
        let player = result.unwrap();
        assert_eq!(player.player_id, "test_player");
        assert_eq!(player.player_name, "Test Player");
        
        // Check that we have 6 Pokemon (venusaur team)
        let team_count = player.team.iter().filter(|p| p.is_some()).count();
        assert_eq!(team_count, 6);
        
        // Check that the first Pokemon is Venusaur
        assert!(player.team[0].is_some());
        let first_pokemon = player.team[0].as_ref().unwrap();
        assert_eq!(first_pokemon.species, Species::Venusaur);
        assert_eq!(first_pokemon.level, 60);
    }

    #[test]
    fn test_create_random_npc_team() {
        let _ = initialize_move_data(Path::new("data"));
        let _ = initialize_species_data(Path::new("data"));
        
        let result = create_random_npc_team("easy");
        assert!(result.is_ok(), "Error: {:?}", result.err());
        
        let npc = result.unwrap();
        assert_eq!(npc.player_id, "npc");
        assert!(npc.player_name.contains("NPC Trainer"));
        
        // Should have at least one Pokemon
        let team_count = npc.team.iter().filter(|p| p.is_some()).count();
        assert!(team_count > 0);
    }

    #[test]
    fn test_validate_prefab_teams() {
        let result = validate_prefab_teams();
        assert!(result.is_ok(), "Prefab team validation failed: {:?}", result);
    }

    #[test]
    fn test_load_all_pokemon_species() {
        // Initialize species data
        let init_result = initialize_species_data(Path::new("data"));
        assert!(init_result.is_ok(), "Failed to initialize species data: {:?}", init_result);
        
        // Test loading all species from 1 to 151
        let mut failed_species = Vec::new();
        
        for i in 1..=151 {
            // Get species by pokedex number
            let species_variants = [
                Species::Bulbasaur, Species::Ivysaur, Species::Venusaur,
                Species::Charmander, Species::Charmeleon, Species::Charizard,
                Species::Squirtle, Species::Wartortle, Species::Blastoise,
                Species::Caterpie, Species::Metapod, Species::Butterfree,
                Species::Weedle, Species::Kakuna, Species::Beedrill,
                Species::Pidgey, Species::Pidgeotto, Species::Pidgeot,
                Species::Rattata, Species::Raticate, Species::Spearow, Species::Fearow,
                Species::Ekans, Species::Arbok, Species::Pikachu, Species::Raichu,
                Species::Sandshrew, Species::Sandslash, Species::NidoranFemale, Species::Nidorina,
                Species::Nidoqueen, Species::NidoranMale, Species::Nidorino, Species::Nidoking,
                Species::Clefairy, Species::Clefable, Species::Vulpix, Species::Ninetales,
                Species::Jigglypuff, Species::Wigglytuff, Species::Zubat, Species::Golbat,
                Species::Oddish, Species::Gloom, Species::Vileplume, Species::Paras,
                Species::Parasect, Species::Venonat, Species::Venomoth, Species::Diglett,
                Species::Dugtrio, Species::Meowth, Species::Persian, Species::Psyduck,
                Species::Golduck, Species::Mankey, Species::Primeape, Species::Growlithe,
                Species::Arcanine, Species::Poliwag, Species::Poliwhirl, Species::Poliwrath,
                Species::Abra, Species::Kadabra, Species::Alakazam, Species::Machop,
                Species::Machoke, Species::Machamp, Species::Bellsprout, Species::Weepinbell,
                Species::Victreebel, Species::Tentacool, Species::Tentacruel, Species::Geodude,
                Species::Graveler, Species::Golem, Species::Ponyta, Species::Rapidash,
                Species::Slowpoke, Species::Slowbro, Species::Magnemite, Species::Magneton,
                Species::Farfetchd, Species::Doduo, Species::Dodrio, Species::Seel,
                Species::Dewgong, Species::Grimer, Species::Muk, Species::Shellder,
                Species::Cloyster, Species::Gastly, Species::Haunter, Species::Gengar,
                Species::Onix, Species::Drowzee, Species::Hypno, Species::Krabby,
                Species::Kingler, Species::Voltorb, Species::Electrode, Species::Exeggcute,
                Species::Exeggutor, Species::Cubone, Species::Marowak, Species::Hitmonlee,
                Species::Hitmonchan, Species::Lickitung, Species::Koffing, Species::Weezing,
                Species::Rhyhorn, Species::Rhydon, Species::Chansey, Species::Tangela,
                Species::Kangaskhan, Species::Horsea, Species::Seadra, Species::Goldeen,
                Species::Seaking, Species::Staryu, Species::Starmie, Species::MrMime,
                Species::Scyther, Species::Jynx, Species::Electabuzz, Species::Magmar,
                Species::Pinsir, Species::Tauros, Species::Magikarp, Species::Gyarados,
                Species::Lapras, Species::Ditto, Species::Eevee, Species::Vaporeon,
                Species::Jolteon, Species::Flareon, Species::Porygon, Species::Omanyte,
                Species::Omastar, Species::Kabuto, Species::Kabutops, Species::Aerodactyl,
                Species::Snorlax, Species::Articuno, Species::Zapdos, Species::Moltres,
                Species::Dratini, Species::Dragonair, Species::Dragonite, Species::Mewtwo,
                Species::Mew
            ];
            
            if let Some(species) = species_variants.get(i - 1) {
                let species_data = crate::pokemon::get_species_data(*species);
                if species_data.is_none() {
                    failed_species.push((i, species.name()));
                }
            }
        }
        
        if !failed_species.is_empty() {
            panic!("Failed to load {} species: {:?}", failed_species.len(), failed_species);
        }
        
        println!("Successfully loaded all 151 Pokemon species");
    }

    #[test] 
    fn test_load_all_moves() {
        // Initialize move data
        let init_result = initialize_move_data(Path::new("data"));
        assert!(init_result.is_ok(), "Failed to initialize move data: {:?}", init_result);
        
        // Test a sampling of moves from each category
        let test_moves = vec![
            Move::Tackle, Move::Scratch, Move::DoubleEdge, Move::HyperBeam,
            Move::Thunderclap, Move::Lightning, Move::Flamethrower, Move::Surf,
            Move::Earthquake, Move::Psychic, Move::SleepPowder, Move::PoisonPowder,
            Move::Hypnosis, Move::ThunderWave, Move::Toxic, Move::Rest,
            Move::Substitute, Move::Reflect, Move::LightScreen, Move::Haze,
            Move::HighJumpKick, Move::Submission, Move::Counter, Move::Bide,
            Move::SelfDestruct, Move::Explosion, Move::PetalDance, Move::Outrage,
            Move::Metronome, Move::Transform, Move::Mimic, Move::DoubleTeam,
            Move::Recover, Move::SoftBoiled, Move::Whirlwind, Move::Roar,
            Move::Teleport, Move::Fly, Move::Dig, Move::Fissure,
            Move::HornDrill, Move::QuickAttack, Move::RockSlide, Move::IceBeam,
            Move::Blizzard, Move::Bubblebeam, Move::AuroraBeam, Move::Solarbeam,
            Move::MegaDrain, Move::LeechSeed, Move::Growth, Move::RazorLeaf,
            Move::PoisonJab, Move::MegaKick, Move::FocusEnergy, Move::Meditate
        ];
        
        let mut failed_moves = Vec::new();
        
        for test_move in test_moves {
            // Try to get move data - this will use the global move data store
            let move_data = crate::move_data::get_move_data(&test_move);
            if move_data.is_none() {
                failed_moves.push(test_move);
            }
        }
        
        if !failed_moves.is_empty() {
            panic!("Failed to load {} moves: {:?}", failed_moves.len(), failed_moves);
        }
        
        println!("Successfully loaded all test moves");
    }
}