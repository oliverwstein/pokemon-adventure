mod moves;
mod pokemon;
mod player;
mod move_data;
mod species;

use std::path::Path;
use pokemon::{PokemonSpecies, PokemonInst, initialize_species_data, get_species_data};
use moves::Move;
use move_data::initialize_move_data;
use species::Species;
use player::{BattlePlayer, StatType};

fn main() {
    let data_path = Path::new("data");
    
    // Initialize global move data first
    if let Err(e) = initialize_move_data(data_path) {
        println!("Error initializing move data: {}", e);
        return;
    }
    
    // Initialize global species data
    if let Err(e) = initialize_species_data(data_path) {
        println!("Error initializing species data: {}", e);
        return;
    }
    
    // Example 1: Load a single Pokemon using Species enum
    if let Some(pikachu) = get_species_data(Species::Pikachu) {
        println!("Loaded Pikachu:");
        println!("  Number: #{}", pikachu.pokedex_number);
        println!("  Types: {:?}", pikachu.types);
        println!("  Base Stats: HP:{} ATK:{} DEF:{} SP.ATK:{} SP.DEF:{} SPD:{}", 
                 pikachu.base_stats.hp, pikachu.base_stats.attack, pikachu.base_stats.defense,
                 pikachu.base_stats.sp_attack, pikachu.base_stats.sp_defense, pikachu.base_stats.speed);
    } else {
        println!("Error loading Pikachu");
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
    
    // Example 4: Create a Pokemon instance using Species enum
    if let Some(pikachu_species) = get_species_data(Species::Pikachu) {
        // Create a level 25 Pikachu with some moves
        let pikachu_moves = vec![
            Move::QuickAttack,
            Move::Thunderclap,
            Move::TailWhip,
            Move::Agility,
        ];
        
        // Example with custom moves
        let my_pikachu = PokemonInst::new(
            Species::Pikachu,
            &pikachu_species,
            25,
            Some([15, 20, 10, 25, 18, 30]), // Custom IVs
            Some(pikachu_moves), // Custom moves
        );
        
        println!("Created Pokemon instance (with custom moves):");
        println!("  Name: {}", my_pikachu.name);
        println!("  Species: {:?} ({})", my_pikachu.species, my_pikachu.species.name());
        println!("  Current Stats: HP:{} ATK:{} DEF:{} SP.ATK:{} SP.DEF:{} SPD:{}", 
                 my_pikachu.curr_stats[0], my_pikachu.curr_stats[1], my_pikachu.curr_stats[2],
                 my_pikachu.curr_stats[3], my_pikachu.curr_stats[4], my_pikachu.curr_stats[5]);
        println!("  Moves:");
        for (i, move_slot) in my_pikachu.moves.iter().enumerate() {
            if let Some(move_inst) = move_slot {
                println!("    {}: {:?} (PP: {})", i + 1, move_inst.move_, move_inst.pp);
            }
        }
        
        println!();
        
        // Example with moves derived from learnset
        let auto_pikachu = PokemonInst::new(
            Species::Pikachu,
            &pikachu_species,
            25,
            None, // Default IVs
            None, // Auto-derive moves from learnset
        );
        
        println!("Created Pokemon instance (with auto-derived moves):");
        println!("  Name: {}", auto_pikachu.name);
        println!("  Current Stats: HP:{} ATK:{} DEF:{} SP.ATK:{} SP.DEF:{} SPD:{}", 
                 auto_pikachu.curr_stats[0], auto_pikachu.curr_stats[1], auto_pikachu.curr_stats[2],
                 auto_pikachu.curr_stats[3], auto_pikachu.curr_stats[4], auto_pikachu.curr_stats[5]);
        println!("  Moves (auto-derived from level {} learnset):", 25);
        for (i, move_slot) in auto_pikachu.moves.iter().enumerate() {
            if let Some(move_inst) = move_slot {
                println!("    {}: {:?} (PP: {})", i + 1, move_inst.move_, move_inst.pp);
            }
        }
    } else {
        println!("Error loading Pikachu species data for Pokemon instance");
    }
    
    println!();
    
    // Example 5: Demonstrate stat stage management
    if let Some(charizard_species) = get_species_data(Species::Charizard) {
        let charizard = PokemonInst::new(Species::Charizard, &charizard_species, 50, None, None);
        let mut player = BattlePlayer::new(
            "trainer_red".to_string(),
            "Red".to_string(),
            vec![charizard],
        );
        
        println!("Stat Stage Management Example:");
        println!("  Initial Attack stage: {}", player.get_stat_stage(StatType::Attack));
        
        // Swords Dance (+2 Attack)
        player.modify_stat_stage(StatType::Attack, 2);
        println!("  After Swords Dance: {}", player.get_stat_stage(StatType::Attack));
        
        // Another Attack boost
        player.modify_stat_stage(StatType::Attack, 1);
        println!("  After another boost: {}", player.get_stat_stage(StatType::Attack));
        
        // Speed reduction
        player.set_stat_stage(StatType::Speed, -1);
        println!("  Speed stage: {}", player.get_stat_stage(StatType::Speed));
        
        println!("  All current stages: {:?}", player.get_all_stat_stages());
        
        // Switching clears stat stages
        if player.team[1].is_some() {
            let _ = player.switch_pokemon(1);
        } else {
            player.clear_stat_stages();
        }
        println!("  After switch/clear - Attack stage: {}", player.get_stat_stage(StatType::Attack));
    }
}