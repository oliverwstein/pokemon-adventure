mod battle;
mod move_data;
mod moves;
mod player;
mod pokemon;
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
    use battle::state::{BattleState, GameState, TurnRng};
    use battle::turn_orchestrator::{collect_player_actions, resolve_turn};
    
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
    
    let mut battle_state = BattleState::new("npc_vs_npc_demo".to_string(), player1, player2);
    
    println!("🔥 Battle begins!");
    println!("  {} sends out {}!", 
             battle_state.players[0].player_name,
             battle_state.players[0].active_pokemon().unwrap().name);
    println!("  {} sends out {}!", 
             battle_state.players[1].player_name,
             battle_state.players[1].active_pokemon().unwrap().name);
    println!();
    
    let mut turn_number = 1;
    let mut rng_values = generate_battle_rng(); // Generate enough RNG for the whole battle
    let mut rng_index = 0;
    
    // Battle loop - continue until one trainer has no Pokemon left
    while !matches!(battle_state.game_state, GameState::Player1Win | GameState::Player2Win | GameState::Draw) {
        println!("--- Turn {} ---", turn_number);
        
        // Print current Pokemon status
        for (i, player) in battle_state.players.iter().enumerate() {
            if let Some(pokemon) = player.active_pokemon() {
                println!("  {}: {} (HP: {}/{})", 
                         player.player_name,
                         pokemon.name,
                         pokemon.current_hp(),
                         pokemon.stats.hp);
            }
        }
        println!();
        
        // Generate NPC actions using the deterministic AI
        if let Err(e) = collect_player_actions(&mut battle_state) {
            println!("Error collecting actions: {}", e);
            break;
        }
        
        // Show what actions were chosen
        for (i, action) in battle_state.action_queue.iter().enumerate() {
            if let Some(action) = action {
                match action {
                    player::PlayerAction::UseMove { move_index } => {
                        if let Some(pokemon) = battle_state.players[i].active_pokemon() {
                            if let Some(move_inst) = &pokemon.moves[*move_index] {
                                println!("  {} chooses {:?}!", 
                                         battle_state.players[i].player_name,
                                         move_inst.move_);
                            }
                        }
                    },
                    player::PlayerAction::SwitchPokemon { team_index } => {
                        if let Some(pokemon) = &battle_state.players[i].team[*team_index] {
                            println!("  {} switches to {}!", 
                                     battle_state.players[i].player_name,
                                     pokemon.name);
                        }
                    },
                    _ => println!("  {} takes an action", battle_state.players[i].player_name),
                }
            }
        }
        println!();
        
        // Execute the turn
        let turn_rng = TurnRng::new_for_test(
            rng_values[rng_index..rng_index + 10].to_vec()
        );
        rng_index += 10;
        
        let event_bus = resolve_turn(&mut battle_state, turn_rng);
        
        // Print ALL events like the tests do
        println!("  Events generated this turn:");
        for (i, event) in event_bus.events().iter().enumerate() {
            println!("    {}: {:?}", i + 1, event);
        }
        
        turn_number += 1;
        
        // Safety check to prevent infinite loops
        if turn_number > 50 {
            println!("Battle reached turn limit - ending demo");
            break;
        }
        
        println!();
    }
    
    // Announce the winner
    match battle_state.game_state {
        GameState::Player1Win => {
            println!("🏆 {} wins the battle!", battle_state.players[0].player_name);
        },
        GameState::Player2Win => {
            println!("🏆 {} wins the battle!", battle_state.players[1].player_name);
        },
        GameState::Draw => {
            println!("🤝 The battle ended in a draw!");
        },
        _ => {
            println!("🔚 Battle ended (Turn limit reached)");
        }
    }
    
    println!("Battle completed after {} turns.", turn_number - 1);
}

fn create_demo_pokemon(species: Species, level: u8) -> PokemonInst {
    let species_data = get_species_data(species).expect("Species data should exist");
    PokemonInst::new(species, &species_data, level, None, None)
}

fn generate_battle_rng() -> Vec<u8> {
    // Generate enough random values for a full battle
    // Pattern: moderate hit chances, some crits, some misses
    vec![
        75, 60, 45, 80, 95, 30, 85, 50, 70, 40,  // Turn 1
        65, 90, 55, 25, 88, 35, 92, 48, 73, 62,  // Turn 2
        58, 82, 43, 97, 67, 28, 91, 54, 76, 39,  // Turn 3
        71, 46, 83, 59, 94, 37, 86, 52, 68, 41,  // Turn 4
        64, 89, 56, 26, 87, 33, 93, 49, 74, 63,  // Turn 5
        // Continue pattern for more turns...
        77, 42, 84, 57, 96, 29, 90, 51, 69, 38,  // Turn 6
        66, 81, 44, 98, 65, 27, 89, 53, 75, 61,  // Turn 7
        72, 47, 85, 58, 95, 36, 87, 50, 67, 40,  // Turn 8
        63, 88, 55, 24, 86, 32, 92, 48, 73, 62,  // Turn 9
        59, 83, 41, 99, 68, 25, 91, 54, 76, 37,  // Turn 10
        // Repeat and extend for longer battles
        75, 60, 45, 80, 95, 30, 85, 50, 70, 40,  
        65, 90, 55, 25, 88, 35, 92, 48, 73, 62,  
        58, 82, 43, 97, 67, 28, 91, 54, 76, 39,  
        71, 46, 83, 59, 94, 37, 86, 52, 68, 41,  
        64, 89, 56, 26, 87, 33, 93, 49, 74, 63,  
        77, 42, 84, 57, 96, 29, 90, 51, 69, 38,  
        66, 81, 44, 98, 65, 27, 89, 53, 75, 61,  
        72, 47, 85, 58, 95, 36, 87, 50, 67, 40,  
        63, 88, 55, 24, 86, 32, 92, 48, 73, 62,  
        59, 83, 41, 99, 68, 25, 91, 54, 76, 37,  
        75, 60, 45, 80, 95, 30, 85, 50, 70, 40,  
        65, 90, 55, 25, 88, 35, 92, 48, 73, 62,  
        58, 82, 43, 97, 67, 28, 91, 54, 76, 39,  
        71, 46, 83, 59, 94, 37, 86, 52, 68, 41,  
        64, 89, 56, 26, 87, 33, 93, 49, 74, 63,  
    ]
}
