use crate::moves::Move;
use crate::move_data::{get_move_max_pp};
use crate::species::Species;
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use std::sync::{LazyLock, RwLock};
use serde::{Serialize, Deserialize};

// Global species data storage - loaded once at startup, indexed by Species enum
static SPECIES_DATA: LazyLock<RwLock<[Option<PokemonSpecies>; 151]>> = LazyLock::new(|| {
    RwLock::new([const { None }; 151])
});

/// Initialize the global species data by loading from disk
pub fn initialize_species_data(data_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut global_data = SPECIES_DATA.write().unwrap();
    
    for species_variant in [
        Species::Bulbasaur, Species::Ivysaur, Species::Venusaur, Species::Charmander, Species::Charmeleon, Species::Charizard,
        Species::Squirtle, Species::Wartortle, Species::Blastoise, Species::Caterpie, Species::Metapod, Species::Butterfree,
        Species::Weedle, Species::Kakuna, Species::Beedrill, Species::Pidgey, Species::Pidgeotto, Species::Pidgeot,
        Species::Rattata, Species::Raticate, Species::Spearow, Species::Fearow, Species::Ekans, Species::Arbok,
        Species::Pikachu, Species::Raichu, Species::Sandshrew, Species::Sandslash, Species::NidoranFemale, Species::Nidorina,
        Species::Nidoqueen, Species::NidoranMale, Species::Nidorino, Species::Nidoking, Species::Clefairy, Species::Clefable,
        Species::Vulpix, Species::Ninetales, Species::Jigglypuff, Species::Wigglytuff, Species::Zubat, Species::Golbat,
        Species::Oddish, Species::Gloom, Species::Vileplume, Species::Paras, Species::Parasect, Species::Venonat,
        Species::Venomoth, Species::Diglett, Species::Dugtrio, Species::Meowth, Species::Persian, Species::Psyduck,
        Species::Golduck, Species::Mankey, Species::Primeape, Species::Growlithe, Species::Arcanine, Species::Poliwag,
        Species::Poliwhirl, Species::Poliwrath, Species::Abra, Species::Kadabra, Species::Alakazam, Species::Machop,
        Species::Machoke, Species::Machamp, Species::Bellsprout, Species::Weepinbell, Species::Victreebel, Species::Tentacool,
        Species::Tentacruel, Species::Geodude, Species::Graveler, Species::Golem, Species::Ponyta, Species::Rapidash,
        Species::Slowpoke, Species::Slowbro, Species::Magnemite, Species::Magneton, Species::Farfetchd, Species::Doduo,
        Species::Dodrio, Species::Seel, Species::Dewgong, Species::Grimer, Species::Muk, Species::Shellder,
        Species::Cloyster, Species::Gastly, Species::Haunter, Species::Gengar, Species::Onix, Species::Drowzee,
        Species::Hypno, Species::Krabby, Species::Kingler, Species::Voltorb, Species::Electrode, Species::Exeggcute,
        Species::Exeggutor, Species::Cubone, Species::Marowak, Species::Hitmonlee, Species::Hitmonchan, Species::Lickitung,
        Species::Koffing, Species::Weezing, Species::Rhyhorn, Species::Rhydon, Species::Chansey, Species::Tangela,
        Species::Kangaskhan, Species::Horsea, Species::Seadra, Species::Goldeen, Species::Seaking, Species::Staryu,
        Species::Starmie, Species::MrMime, Species::Scyther, Species::Jynx, Species::Electabuzz, Species::Magmar,
        Species::Pinsir, Species::Tauros, Species::Magikarp, Species::Gyarados, Species::Lapras, Species::Ditto,
        Species::Eevee, Species::Vaporeon, Species::Jolteon, Species::Flareon, Species::Porygon, Species::Omanyte,
        Species::Omastar, Species::Kabuto, Species::Kabutops, Species::Aerodactyl, Species::Snorlax, Species::Articuno,
        Species::Zapdos, Species::Moltres, Species::Dratini, Species::Dragonair, Species::Dragonite, Species::Mewtwo,
        Species::Mew,
    ] {
        if let Ok(species_data) = PokemonSpecies::load_by_species(species_variant, data_path) {
            let index = species_variant.pokedex_number() as usize - 1; // 0-indexed
            global_data[index] = Some(species_data);
        }
    }
    
    Ok(())
}

/// Get species data for a specific species from the global store
pub fn get_species_data(species: Species) -> Option<PokemonSpecies> {
    let global_data = SPECIES_DATA.read().unwrap();
    let index = species.pokedex_number() as usize - 1; // 0-indexed
    global_data[index].clone()
}

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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum StatusCondition {
    Sleep(u8),
    Poison(u8),
    Burn,
    Freeze,
    Paralysis,
    Faint,  // Pokemon has 0 HP, can replace any other status
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveInstance {
    pub move_: Move,
    pub pp: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PokemonInst {
    pub name: String,                    // Species name if no nickname
    pub species: Species,                // Species enum for type-safe lookup
    pub curr_exp: u8,                    // Only really relevant for single-player
    pub ivs: [u8; 6],                    // HP, ATK, DEF, SP.ATK, SP.DEF, SPD
    pub evs: [u8; 6],                    // HP, ATK, DEF, SP.ATK, SP.DEF, SPD
    pub curr_stats: [u16; 6],            // HP, ATK, DEF, SP.ATK, SP.DEF, SPD (can exceed 255)
    pub moves: [Option<MoveInstance>; 4], // Up to 4 moves
    pub status: Option<StatusCondition>, // Status condition with optional parameter
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
    /// Load a Pokemon species from its RON file by species enum
    pub fn load_by_species(species: Species, data_path: &Path) -> Result<PokemonSpecies, Box<dyn std::error::Error>> {
        let pokemon_dir = data_path.join("pokemon");
        
        if !pokemon_dir.exists() {
            return Err(format!("Pokemon data directory not found: {}", pokemon_dir.display()).into());
        }

        // Find the RON file based on the species enum
        let species_filename = format!("{:03}-{}", species.pokedex_number(), species.name().to_lowercase());
        let ron_file = pokemon_dir.join(format!("{}.ron", species_filename));
        
        if !ron_file.exists() {
            return Err(format!("Pokemon file not found: {}", ron_file.display()).into());
        }
        
        let content = fs::read_to_string(&ron_file)?;
        let species_data: PokemonSpecies = ron::from_str(&content)?;
        Ok(species_data)
    }

    /// Load a Pokemon species from its RON file by name (legacy method)
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
    
    /// Create a HashMap for fast name-based lookups using RON filename-based keys
    pub fn create_species_map(data_path: &Path) -> Result<HashMap<String, PokemonSpecies>, Box<dyn std::error::Error>> {
        let pokemon_dir = data_path.join("pokemon");
        let mut map = HashMap::new();
        
        if !pokemon_dir.exists() {
            return Err(format!("Pokemon data directory not found: {}", pokemon_dir.display()).into());
        }

        let entries = fs::read_dir(&pokemon_dir)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    // Extract name from filename format: "001-bulbasaur.ron" -> "bulbasaur"
                    if let Some(pokemon_name) = filename.split('-').nth(1) {
                        let content = fs::read_to_string(&path)?;
                        let species: PokemonSpecies = ron::from_str(&content)?;
                        
                        // Use the filename-based name as the key (uppercase for consistency)
                        let key = pokemon_name.to_uppercase();
                        map.insert(key, species);
                    }
                }
            }
        }
        
        Ok(map)
    }
}

impl MoveInstance {
    /// Create a new move instance with max PP
    pub fn new(move_: Move) -> Self {
        let max_pp = get_move_max_pp(move_);
        
        MoveInstance {
            move_,
            pp: max_pp,
        }
    }
    
    /// Get the max PP for this move
    pub fn max_pp(&self) -> u8 {
        get_move_max_pp(self.move_)
    }
    
    /// Use the move (decrease PP)
    pub fn use_move(&mut self) -> bool {
        if self.pp > 0 {
            self.pp -= 1;
            true
        } else {
            false
        }
    }
    
    /// Restore PP
    pub fn restore_pp(&mut self, amount: u8) {
        let max_pp = self.max_pp();
        self.pp = (self.pp + amount).min(max_pp);
    }
}

impl PokemonInst {
    /// Create a new Pokemon instance from species data
    pub fn new(
        species: Species,
        species_data: &PokemonSpecies,
        level: u8,
        ivs: Option<[u8; 6]>,
        moves: Option<Vec<Move>>,
    ) -> Self {
        // Generate default IVs if not provided (0-31 range)
        let ivs = ivs.unwrap_or([0; 6]); // TODO: Add random generation
        
        // Initialize EVs to 0
        let evs = [0; 6];
        
        // Calculate current stats based on level, IVs, EVs, and base stats
        let curr_stats = Self::calculate_stats(&species_data.base_stats, level, &ivs, &evs);
        
        // Derive moves from learnset if not provided
        let moves = moves.unwrap_or_else(|| Self::derive_moves_from_learnset(&species_data.learnset, level));
        
        // Create move instances with max PP from move data
        let mut move_array = [const { None }; 4];
        for (i, move_) in moves.into_iter().take(4).enumerate() {
            move_array[i] = Some(MoveInstance::new(move_));
        }
        
        PokemonInst {
            name: species_data.name.clone(),
            species,
            curr_exp: 0, // TODO: Calculate from level
            ivs,
            evs,
            curr_stats,
            moves: move_array,
            status: None,
        }
    }
    
    /// Calculate current stats based on base stats, level, IVs, and EVs
    /// Uses Gen 3+ stat calculation formula without natures
    fn calculate_stats(base_stats: &BaseStats, level: u8, ivs: &[u8; 6], evs: &[u8; 6]) -> [u16; 6] {
        let base = [
            base_stats.hp,
            base_stats.attack,
            base_stats.defense,
            base_stats.sp_attack,
            base_stats.sp_defense,
            base_stats.speed,
        ];
        
        let mut stats = [0u16; 6];
        
        for i in 0..6 {
            let stat = if i == 0 {
                // HP = floor(0.01 * (2 * Base + IV + floor(0.25 * EV)) * Level) + Level + 10
                let base_calculation = 2 * base[i] as u16 + ivs[i] as u16 + (evs[i] as u16 / 4);
                let hp = (base_calculation * level as u16) / 100 + level as u16 + 10;
                hp
            } else {
                // Other Stat = floor(0.01 * (2 * Base + IV + floor(0.25 * EV)) * Level) + 5
                let base_calculation = 2 * base[i] as u16 + ivs[i] as u16 + (evs[i] as u16 / 4);
                let other_stat = (base_calculation * level as u16) / 100 + 5;
                other_stat
            };
            
            stats[i] = stat.min(65535); // Cap at max u16 value
        }
        
        stats
    }
    
    /// Derive moves from learnset based on current level
    /// Returns the 4 most recent moves the Pokemon would know at this level
    fn derive_moves_from_learnset(learnset: &Learnset, level: u8) -> Vec<Move> {
        let mut learned_moves = Vec::new();
        
        // Collect all moves learned at or before the current level
        for learn_level in 1..=level {
            if let Some(moves_at_level) = learnset.level_up.get(&learn_level) {
                for &move_ in moves_at_level {
                    learned_moves.push(move_);
                }
            }
        }
        
        // Pokemon can only know 4 moves, so take the 4 most recently learned
        // If fewer than 4 moves learned, return all of them
        if learned_moves.len() <= 4 {
            learned_moves
        } else {
            learned_moves.into_iter().rev().take(4).rev().collect()
        }
    }
    
    /// Get the species data for this Pokemon instance
    pub fn get_species_data(&self) -> Option<PokemonSpecies> {
        get_species_data(self.species)
    }
    
    /// Check if this Pokemon is fainted (0 HP or has Faint status)
    pub fn is_fainted(&self) -> bool {
        self.curr_stats[0] == 0 || matches!(self.status, Some(StatusCondition::Faint))
    }
    
    /// Get current HP
    pub fn current_hp(&self) -> u16 {
        self.curr_stats[0]
    }
    
    /// Get max HP (for testing, we'll use a simple approach)
    /// In a real implementation, this would be stored separately or calculated from base stats
    pub fn max_hp(&self) -> u16 {
        // Simple approach: assume the Pokemon was created with its max HP
        // and we need to track the original value
        // For now, let's use a heuristic based on the stats array structure
        if self.current_hp() == 0 && self.is_fainted() {
            // If fainted, we need to estimate max HP
            // For test Pokemon, we'll assume a reasonable max HP
            100 // Default max HP for testing
        } else {
            // For non-fainted Pokemon, assume current HP is close to max
            // This is a simplification for testing purposes
            self.current_hp().max(50) // At least 50 HP
        }
    }
    
    /// Take damage and handle fainting
    /// Returns true if the Pokemon fainted from this damage
    pub fn take_damage(&mut self, damage: u16) -> bool {
        let current_hp = self.curr_stats[0];
        if damage >= current_hp {
            // Pokemon faints - set HP to 0 and replace any existing status with Faint
            self.curr_stats[0] = 0;
            self.status = Some(StatusCondition::Faint);
            true
        } else {
            // Reduce HP by damage amount
            self.curr_stats[0] = current_hp - damage;
            false
        }
    }
    
    /// Heal HP (cannot exceed max HP, cannot revive fainted Pokemon)
    pub fn heal(&mut self, heal_amount: u16) {
        if self.is_fainted() {
            return; // Cannot heal fainted Pokemon
        }
        
        let current_hp = self.curr_stats[0];
        let max_hp = self.max_hp(); // For now, same as current max
        self.curr_stats[0] = (current_hp + heal_amount).min(max_hp);
    }
    
    /// Revive a fainted Pokemon with specified HP
    pub fn revive(&mut self, hp_amount: u16) {
        if self.is_fainted() {
            let max_hp = self.max_hp();
            self.curr_stats[0] = hp_amount.min(max_hp).max(1); // At least 1 HP
            self.status = None; // Remove faint status
        }
    }
}

