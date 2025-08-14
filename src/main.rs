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

fn main() {
    // Initialize global move data (no-op since data is compiled in)
    if let Err(e) = initialize_move_data(&std::path::Path::new(".")) {
        println!("Error initializing move data: {}", e);
        return;
    }

    // Initialize global species data (no-op since data is compiled in)
    if let Err(e) = initialize_species_data(&std::path::Path::new(".")) {
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
    match PokemonSpecies::load_all(&std::path::Path::new(".")) {
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
    match PokemonSpecies::create_species_map(&std::path::Path::new(".")) {
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
    run_npc_battle_demo_without_runner();
}

fn run_npc_battle_demo_without_runner() {
    use battle::state::{BattleState, GameState, TurnRng};
    use battle::turn_orchestrator::{
        collect_player_actions, ready_for_turn_resolution, resolve_turn,
    };

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
    println!(
        "  {} sends out {}!",
        battle_state.players[0].player_name,
        battle_state.players[0].active_pokemon().unwrap().name
    );
    println!(
        "  {} sends out {}!",
        battle_state.players[1].player_name,
        battle_state.players[1].active_pokemon().unwrap().name
    );
    println!();

    let mut execution_count = 0;

    // Battle loop - continue until one trainer has no Pokemon left
    while !matches!(
        battle_state.game_state,
        GameState::Player1Win | GameState::Player2Win | GameState::Draw
    ) {
        println!("--- Turn {} ---", battle_state.turn_number);

        // Print current Pokemon status
        for player in &battle_state.players {
            if let Some(pokemon) = player.active_pokemon() {
                println!(
                    "  {}: {} (HP: {}/{})",
                    player.player_name,
                    pokemon.name,
                    pokemon.current_hp(),
                    pokemon.max_hp()
                );
            }
        }
        println!();

        // Auto-generate NPC actions
        if let Err(e) = collect_player_actions(&mut battle_state) {
            println!("Error generating actions: {}", e);
            break;
        }

        // Execute the game tick loop - keep resolving turns until waiting for input
        while ready_for_turn_resolution(&battle_state) {
            let rng = TurnRng::new_random();
            let event_bus = resolve_turn(&mut battle_state, rng);
            let events = event_bus.events();

            // Show what actions were chosen based on events
            for event in events {
                match event {
                    battle::state::BattleEvent::MoveUsed {
                        player_index,
                        pokemon: _,
                        move_used,
                    } => {
                        println!(
                            "  {} chooses {:?}!",
                            battle_state.players[*player_index].player_name, move_used
                        );
                    }
                    battle::state::BattleEvent::PokemonSwitched {
                        player_index,
                        new_pokemon,
                        ..
                    } => {
                        println!(
                            "  {} switches to {:?}!",
                            battle_state.players[*player_index].player_name, new_pokemon
                        );
                    }
                    _ => {} // Don't print other events here
                }
            }

            // Print ALL events like the tests do
            if !events.is_empty() {
                println!("  Events generated this turn:");
                for (i, event) in events.iter().enumerate() {
                    println!("    {}: {:?}", i + 1, event);
                }
                println!();
            }

            execution_count += 1;

            // Safety check to prevent infinite loops
            if execution_count > 50 {
                println!("Battle reached execution limit - ending demo");
                return;
            }

            // Check if battle ended
            if matches!(
                battle_state.game_state,
                GameState::Player1Win | GameState::Player2Win | GameState::Draw
            ) {
                break;
            }
        }
    }

    // Announce the winner
    match battle_state.game_state {
        GameState::Player1Win => {
            println!(
                "🏆 {} wins the battle!",
                battle_state.players[0].player_name
            );
        }
        GameState::Player2Win => {
            println!(
                "🏆 {} wins the battle!",
                battle_state.players[1].player_name
            );
        }
        GameState::Draw => {
            println!("🤝 The battle ended in a draw!");
        }
        _ => {
            println!("🔚 Battle ended (Execution limit reached)");
        }
    }

    println!(
        "Battle completed after {} turn(s).",
        battle_state.turn_number
    );
}

fn create_demo_pokemon(species: Species, level: u8) -> PokemonInst {
    let species_data = get_species_data(species).expect("Species data should exist");
    PokemonInst::new(species, &species_data, level, None, None)
}
