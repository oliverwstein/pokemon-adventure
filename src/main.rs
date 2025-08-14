mod battle;
mod move_data;
mod moves;
mod player;
mod pokemon;
mod prefab_teams;
mod species;

use move_data::initialize_move_data;
use moves::Move;
use player::BattlePlayer;
use pokemon::{PokemonInst, PokemonSpecies, get_species_data, initialize_species_data};
use species::Species;
use std::path::Path;

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
        println!(
            "  Base Stats: HP:{} ATK:{} DEF:{} SP.ATK:{} SP.DEF:{} SPD:{}",
            pikachu.base_stats.hp,
            pikachu.base_stats.attack,
            pikachu.base_stats.defense,
            pikachu.base_stats.sp_attack,
            pikachu.base_stats.sp_defense,
            pikachu.base_stats.speed
        );
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
                println!(
                    "  Found Charizard via filename lookup: #{}",
                    charizard.pokedex_number
                );
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
            Some(pikachu_moves),            // Custom moves
        );

        println!("Created Pokemon instance (with custom moves):");
        println!("  Name: {}", my_pikachu.name);
        println!(
            "  Species: {:?} ({})",
            my_pikachu.species,
            my_pikachu.species.name()
        );
        println!(
            "  Current Stats: HP:{} ATK:{} DEF:{} SP.ATK:{} SP.DEF:{} SPD:{}",
            my_pikachu.stats.hp,
            my_pikachu.stats.attack,
            my_pikachu.stats.defense,
            my_pikachu.stats.sp_attack,
            my_pikachu.stats.sp_defense,
            my_pikachu.stats.speed,
        );
        println!("  Moves:");
        for (i, move_slot) in my_pikachu.moves.iter().enumerate() {
            if let Some(move_inst) = move_slot {
                println!(
                    "    {}: {:?} (PP: {})",
                    i + 1,
                    move_inst.move_,
                    move_inst.pp
                );
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
        println!(
            "  Current Stats: HP:{} ATK:{} DEF:{} SP.ATK:{} SP.DEF:{} SPD:{}",
            auto_pikachu.stats.hp,
            auto_pikachu.stats.attack,
            auto_pikachu.stats.defense,
            auto_pikachu.stats.sp_attack,
            auto_pikachu.stats.sp_defense,
            auto_pikachu.stats.speed,
        );
        println!("  Moves (auto-derived from level {} learnset):", 25);
        for (i, move_slot) in auto_pikachu.moves.iter().enumerate() {
            if let Some(move_inst) = move_slot {
                println!(
                    "    {}: {:?} (PP: {})",
                    i + 1,
                    move_inst.move_,
                    move_inst.pp
                );
            }
        }
    } else {
        println!("Error loading Pikachu species data for Pokemon instance");
    }

    println!();

    // Example 5: NPC vs NPC Multi-Pokemon Battle Demo
    println!("=== NPC vs NPC Battle Demo ===");
    run_npc_battle_demo();
}

fn run_npc_battle_demo() {
    use battle::runner::BattleRunner;
    
    // Create two trainers with multiple Pokemon each
    let trainer1_team = vec![
        create_demo_pokemon(Species::Pikachu, 25),
        create_demo_pokemon(Species::Charmander, 20),
        create_demo_pokemon(Species::Squirtle, 22),
    ];
    
    let trainer2_team = vec![
        create_demo_pokemon(Species::Bulbasaur, 23),
        create_demo_pokemon(Species::Rattata, 18),
        create_demo_pokemon(Species::Pidgey, 21),
    ];
    
    let player1 = BattlePlayer::new(
        "npc_trainer_1".to_string(),
        "AI Trainer Red".to_string(),
        trainer1_team,
    );
    
    let player2 = BattlePlayer::new(
        "npc_trainer_2".to_string(), 
        "AI Trainer Blue".to_string(),
        trainer2_team,
    );
    
    let mut battle_runner = BattleRunner::new("npc_vs_npc_demo".to_string(), player1, player2);
    
    println!("🔥 Battle begins!");
    let battle_info = battle_runner.get_battle_info();
    println!("  {} sends out {}!", 
             battle_info.players[0].player_name,
             battle_info.players[0].active_pokemon.as_ref().unwrap().name);
    println!("  {} sends out {}!", 
             battle_info.players[1].player_name,
             battle_info.players[1].active_pokemon.as_ref().unwrap().name);
    println!();
    
    let mut execution_count = 0;
    
    // Battle loop - continue until one trainer has no Pokemon left
    while !battle_runner.is_battle_ended() {
        let battle_info = battle_runner.get_battle_info();
        
        // Only print turn header for actual battle turns (not replacements)
        let current_turn = battle_runner.get_turn_number();
        println!("--- Turn {} ---", current_turn);
        
        // Print current Pokemon status
        for player in &battle_info.players {
            if let Some(pokemon) = &player.active_pokemon {
                println!("  {}: {} (HP: {}/{})", 
                         player.player_name,
                         pokemon.name,
                         pokemon.current_hp,
                         pokemon.max_hp);
            }
        }
        println!();
        
        // Auto-generate NPC actions and execute if ready
        match battle_runner.auto_execute_if_ready() {
            Ok(Some(result)) => {
                // Show what actions were chosen based on events
                for event in &result.events {
                    match event {
                        battle::state::BattleEvent::MoveUsed { player_index, pokemon, move_used } => {
                            println!("  {} chooses {:?}!", 
                                     battle_info.players[*player_index].player_name,
                                     move_used);
                        },
                        battle::state::BattleEvent::PokemonSwitched { player_index, new_pokemon, .. } => {
                            println!("  {} switches to {:?}!", 
                                     battle_info.players[*player_index].player_name,
                                     new_pokemon);
                        },
                        _ => {} // Don't print other events here
                    }
                }
                
                // Print ALL events like the tests do
                if !result.events.is_empty() {
                    println!("  Events generated this turn:");
                    for (i, event) in result.events.iter().enumerate() {
                        println!("    {}: {:?}", i + 1, event);
                    }
                    println!();
                }
                
                execution_count += 1;
                
                // Safety check to prevent infinite loops
                if execution_count > 50 {
                    println!("Battle reached execution limit - ending demo");
                    break;
                }
            },
            Ok(None) => {
                println!("Waiting for actions...");
                break;
            },
            Err(e) => {
                println!("Error executing battle: {}", e);
                break;
            }
        }
    }
    
    // Announce the winner
    if let Some(winner_index) = battle_runner.get_winner() {
        let battle_info = battle_runner.get_battle_info();
        println!("🏆 {} wins the battle!", battle_info.players[winner_index].player_name);
    } else if battle_runner.is_battle_ended() {
        println!("🤝 The battle ended in a draw!");
    } else {
        println!("🔚 Battle ended (Execution limit reached)");
    }
    
    println!("Battle completed after {} turn(s).", battle_runner.get_turn_number());
}

fn create_demo_pokemon(species: Species, level: u8) -> PokemonInst {
    let species_data = get_species_data(species).expect("Species data should exist");
    PokemonInst::new(species, &species_data, level, None, None)
}

