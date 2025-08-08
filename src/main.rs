mod moves;
mod pokemon;

use std::path::Path;
use pokemon::PokemonSpecies;

fn main() {
    let data_path = Path::new("data");
    
    // Example 1: Load a single Pokemon by name
    match PokemonSpecies::load_by_name("pikachu", data_path) {
        Ok(pikachu) => {
            println!("Loaded Pikachu:");
            println!("  Number: #{}", pikachu.pokedex_number);
            println!("  Types: {:?}", pikachu.types);
            println!("  Base Stats: HP:{} ATK:{} DEF:{} SP.ATK:{} SP.DEF:{} SPD:{}", 
                     pikachu.base_stats.hp, pikachu.base_stats.attack, pikachu.base_stats.defense,
                     pikachu.base_stats.sp_attack, pikachu.base_stats.sp_defense, pikachu.base_stats.speed);
        }
        Err(e) => println!("Error loading Pikachu: {}", e),
    }
    
    println!();
    
    // Example 2: Load all Pokemon and show count
    match PokemonSpecies::load_all(data_path) {
        Ok(all_species) => {
            println!("Loaded {} Pokemon species", all_species.len());
            
            // Show first and last
            if let (Some(first), Some(last)) = (all_species.first(), all_species.last()) {
                println!("  First: #{:03} {}", first.pokedex_number, first.name);
                println!("  Last:  #{:03} {}", last.pokedex_number, last.name);
            }
        }
        Err(e) => println!("Error loading all Pokemon: {}", e),
    }
    
    println!();
    
    // Example 3: Create species map for fast lookups
    match PokemonSpecies::create_species_map(data_path) {
        Ok(species_map) => {
            println!("Created species map with {} entries", species_map.len());
            
            // Test case-insensitive lookup
            if let Some(charizard) = species_map.get("charizard") {
                println!("  Found Charizard via lowercase lookup: #{}", charizard.pokedex_number);
            }
            
            if let Some(mr_mime) = species_map.get("mr-mime") {
                println!("  Found Mr. Mime: #{}", mr_mime.pokedex_number);
            }
        }
        Err(e) => println!("Error creating species map: {}", e),
    }
}