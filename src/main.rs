mod moves;
mod pokemon;
mod player;
mod move_data;

use std::path::Path;
use pokemon::{PokemonSpecies, PokemonInst};
use moves::Move;

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
            
            // Test filename-based lookup
            let charizard_found = if let Some(charizard) = species_map.get("CHARIZARD") {
                println!("  Found Charizard via filename lookup: #{}", charizard.pokedex_number);
                true
            } else {
                false
            };
            
            let mr_mime_found = if let Some(mr_mime) = species_map.get("MR_MIME") {
                println!("  Found Mr. Mime: #{}", mr_mime.pokedex_number);
                true
            } else {
                false
            };
            
            // If either lookup failed, print all available species names
            if !charizard_found || !mr_mime_found {
                println!("  Available species names in map:");
                let mut names: Vec<_> = species_map.keys().collect();
                names.sort();
                for name in names {
                    println!("    {}", name);
                }
            }
        }
        Err(e) => println!("Error creating species map: {}", e),
    }
    
    println!();
    
    // Example 4: Create a Pokemon instance
    match PokemonSpecies::create_species_map(data_path) {
        Ok(species_map) => {
            if let Some(pikachu_species) = species_map.get("PIKACHU") {
                // Create a level 25 Pikachu with some moves
                let pikachu_moves = vec![
                    Move::QuickAttack,
                    Move::Thunderclap,
                    Move::TailWhip,
                    Move::Agility,
                ];
                
                // Example with custom moves
                let my_pikachu = PokemonInst::new(
                    "PIKACHU".to_string(),
                    pikachu_species,
                    25,
                    Some(vec![15, 20, 10, 25, 18, 30]), // Custom IVs
                    Some(pikachu_moves), // Custom moves
                );
                
                println!("Created Pokemon instance (with custom moves):");
                println!("  Name: {}", my_pikachu.name);
                println!("  Species: {}", my_pikachu.species);
                println!("  Current Stats: HP:{} ATK:{} DEF:{} SP.ATK:{} SP.DEF:{} SPD:{}", 
                         my_pikachu.curr_stats[0], my_pikachu.curr_stats[1], my_pikachu.curr_stats[2],
                         my_pikachu.curr_stats[3], my_pikachu.curr_stats[4], my_pikachu.curr_stats[5]);
                println!("  Moves:");
                for (i, move_inst) in my_pikachu.moves.iter().enumerate() {
                    println!("    {}: {:?} (PP: {})", i + 1, move_inst.move_, move_inst.pp);
                }
                
                println!();
                
                // Example with moves derived from learnset
                let auto_pikachu = PokemonInst::new(
                    "PIKACHU".to_string(),
                    pikachu_species,
                    25,
                    None, // Random IVs
                    None, // Auto-derive moves from learnset
                );
                
                println!("Created Pokemon instance (with auto-derived moves):");
                println!("  Name: {}", auto_pikachu.name);
                println!("  Current Stats: HP:{} ATK:{} DEF:{} SP.ATK:{} SP.DEF:{} SPD:{}", 
                         auto_pikachu.curr_stats[0], auto_pikachu.curr_stats[1], auto_pikachu.curr_stats[2],
                         auto_pikachu.curr_stats[3], auto_pikachu.curr_stats[4], auto_pikachu.curr_stats[5]);
                println!("  Moves (auto-derived from level {} learnset):", 25);
                for (i, move_inst) in auto_pikachu.moves.iter().enumerate() {
                    println!("    {}: {:?} (PP: {})", i + 1, move_inst.move_, move_inst.pp);
                }
            }
        }
        Err(e) => println!("Error creating species map for Pokemon instance: {}", e),
    }
}