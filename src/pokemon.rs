use crate::moves::Move;
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Type {
    Normal,
    Fighting,
    Flying,
    Poison,
    Ground,
    Rock,
    Bug,
    Ghost,
    Fire,
    Water,
    Grass,
    Electric,
    Psychic,
    Ice,
    Dragon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Item {
    // Evolution stones
    FireStone,
    WaterStone,
    ThunderStone,
    LeafStone,
    MoonStone,
    // Add more items as needed
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StatusCondition {
    Sleep(u8),
    Poison(u8),
    Burn,
    Freeze,
    Paralysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseStats {
    pub hp: u8,
    pub attack: u8,
    pub defense: u8,
    pub sp_attack: u8,
    pub sp_defense: u8,
    pub speed: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Learnset {
    pub level_up: HashMap<u8, Vec<Move>>,  // level -> moves learned at that level
    pub signature: Option<Move>,           // Evolution line signature move
    pub can_learn: Vec<Move>,              // Moves learnable through tutoring/witnessing
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvolutionMethod {
    Level(u8),
    Item(Item),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionData {
    pub evolves_into: String,  // Pokemon name
    pub method: EvolutionMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PokemonSpecies {
    pub pokedex_number: u16,
    pub name: String,
    pub types: Vec<Type>,
    pub base_stats: BaseStats,
    pub learnset: Learnset,
    pub catch_rate: u8,
    pub base_exp: u16,
    pub description: String,
    pub evolution_data: Option<EvolutionData>,
}

impl Learnset {
    pub fn learns_at_level(&self, level: u8) -> Option<&Vec<Move>> {
        self.level_up.get(&level)
    }
    
    pub fn can_learn_move(&self, move_: Move) -> bool {
        // Check if move is in signature, level-up, or can_learn lists
        if self.signature == Some(move_) {
            return true;
        }
        
        for moves in self.level_up.values() {
            if moves.contains(&move_) {
                return true;
            }
        }
        
        self.can_learn.contains(&move_)
    }
}

impl PokemonSpecies {
    /// Load a Pokemon species from its RON file by name
    /// Name should be lowercase (e.g., "bulbasaur", "mr-mime")
    pub fn load_by_name(name: &str, data_path: &Path) -> Result<PokemonSpecies, Box<dyn std::error::Error>> {
        // Find the RON file that matches this Pokemon name
        let pokemon_dir = data_path.join("pokemon");
        
        if !pokemon_dir.exists() {
            return Err(format!("Pokemon data directory not found: {}", pokemon_dir.display()).into());
        }

        // Read all .ron files in the pokemon directory
        let entries = fs::read_dir(&pokemon_dir)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                // Check if this file matches our Pokemon name
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    // Extract name from filename format: "001-bulbasaur.ron" -> "bulbasaur"
                    if let Some(pokemon_name) = filename.split('-').nth(1) {
                        if pokemon_name.eq_ignore_ascii_case(name) {
                            // Found matching file, load it
                            let content = fs::read_to_string(&path)?;
                            let species: PokemonSpecies = ron::from_str(&content)?;
                            return Ok(species);
                        }
                    }
                }
            }
        }
        
        Err(format!("Pokemon '{}' not found in data directory", name).into())
    }
    
    /// Load all Pokemon species from RON files in the data directory
    pub fn load_all(data_path: &Path) -> Result<Vec<PokemonSpecies>, Box<dyn std::error::Error>> {
        let pokemon_dir = data_path.join("pokemon");
        let mut species = Vec::new();
        
        if !pokemon_dir.exists() {
            return Err(format!("Pokemon data directory not found: {}", pokemon_dir.display()).into());
        }

        let entries = fs::read_dir(&pokemon_dir)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                let content = fs::read_to_string(&path)?;
                let pokemon: PokemonSpecies = ron::from_str(&content)?;
                species.push(pokemon);
            }
        }
        
        // Sort by pokedex number
        species.sort_by(|a, b| a.pokedex_number.cmp(&b.pokedex_number));
        
        Ok(species)
    }
    
    /// Create a HashMap for fast name-based lookups
    pub fn create_species_map(data_path: &Path) -> Result<HashMap<String, PokemonSpecies>, Box<dyn std::error::Error>> {
        let all_species = Self::load_all(data_path)?;
        let mut map = HashMap::new();
        
        for species in all_species {
            // Store both the exact name and a lowercase version for case-insensitive lookup
            let lowercase_name = species.name.to_lowercase();
            map.insert(species.name.clone(), species.clone());
            map.insert(lowercase_name, species);
        }
        
        Ok(map)
    }
}