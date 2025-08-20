use std::io::{self, Write};

use pokemon_adventure::battle::engine::{
    collect_npc_actions, ready_for_turn_resolution, resolve_turn,
};
use pokemon_adventure::battle::state::{BattleState, EventBus, GameState, TurnRng};
use pokemon_adventure::player::{PlayerAction, PlayerType};
use pokemon_adventure::prefab_teams::{self, PrefabTeam};
use pokemon_adventure::{BattlePlayer, PokemonInst};
/// The main entry point for the text-based battle game.
fn main() {
    println!("🔥 Welcome to the Pokémon Adventure Battle Engine! 🔥");

    // --- Battle Setup ---
    let player_team = select_player_team();
    let mut human_player = prefab_teams::create_battle_player_from_prefab(
        &player_team.id,
        "human_player".to_string(),
        "Player".to_string(),
    )
    .expect("Failed to create player team.");
    human_player.player_type = PlayerType::Human;

    // For this demo, the NPC always uses the Charizard team.
    let mut npc_player = prefab_teams::create_battle_player_from_prefab(
        "charizard_team",
        "npc_opponent".to_string(),
        "AI Trainer".to_string(),
    )
    .expect("Failed to create NPC team.");
    npc_player.player_type = PlayerType::NPC;

    let mut battle_state = BattleState::new(
        "text_adventure_battle".to_string(),
        human_player,
        npc_player,
    );

    println!("\n💥 A wild trainer challenges you to a battle! 💥");
    println!(
        "You sent out {}!",
        battle_state.players[0].active_pokemon().unwrap().name
    );
    println!(
        "{} sends out {}!",
        battle_state.players[1].player_name,
        battle_state.players[1].active_pokemon().unwrap().name
    );

    // --- Main Game Loop ---
    run_game_loop(&mut battle_state);

    // --- Battle Conclusion ---
    println!("\n--- Battle Over! ---");
    match battle_state.game_state {
        GameState::Player1Win => println!("🏆 You are victorious! 🏆"),
        GameState::Player2Win => println!("💔 You were defeated... 💔"),
        GameState::Draw => println!("🤝 The battle ended in a draw! 🤝"),
        _ => println!("The battle ended unexpectedly."),
    }
}

/// Runs the main interactive game loop until the battle concludes.
fn run_game_loop(battle_state: &mut BattleState) {
    loop {
        // Check for terminal states first.
        if matches!(
            battle_state.game_state,
            GameState::Player1Win | GameState::Player2Win | GameState::Draw
        ) {
            break;
        }

        // Handle forced player actions, like switching after a faint.
        if let GameState::WaitingForPlayer1Replacement = battle_state.game_state {
            println!("\nYour Pokémon fainted!");
            display_team_status(&battle_state.players[0]);
            let action = get_player_action(battle_state, true); // `true` forces switch-only actions
            battle_state.action_queue[0] = Some(action);
        }

        // Standard turn flow.
        if ready_for_turn_resolution(battle_state) {
            // Both players have queued actions, resolve the turn.
            let rng = TurnRng::new_random();
            let event_bus = resolve_turn(battle_state, rng);
            print_turn_events(&event_bus, battle_state);
        } else if battle_state.action_queue[0].is_none() {
            // It's the human player's turn to act.
            println!("\n--- Turn {} ---", battle_state.turn_number);
            display_battle_status(battle_state);
            let action = get_player_action(battle_state, false);
            battle_state.action_queue[0] = Some(action);
        }

        // Let the AI act if its slot is empty.
        if battle_state.action_queue[1].is_none() {
            let npc_actions = collect_npc_actions(battle_state);
            for (player_index, action) in npc_actions {
                battle_state.action_queue[player_index] = Some(action);
            }
        }
    }
}

/// Prompts the human player to select a prefab team.
fn select_player_team() -> PrefabTeam {
    let teams = prefab_teams::get_prefab_teams();
    println!("\nPlease choose your team:");
    for (i, team) in teams.iter().enumerate() {
        println!("  {}. {} - {}", i + 1, team.name, team.description);
    }

    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();

        match choice.trim().parse::<usize>() {
            Ok(n) if n > 0 && n <= teams.len() => {
                return teams[n - 1].clone();
            }
            _ => println!(
                "Invalid selection. Please enter a number from 1 to {}.",
                teams.len()
            ),
        }
    }
}

/// Handles the user input loop to get a valid player action.
fn get_player_action(battle_state: &BattleState, switch_only: bool) -> PlayerAction {
    loop {
        print!("\nWhat will you do? (Type 'help' for commands)\n> ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let command = parts[0].to_lowercase();
        let args = &parts[1..];

        // --- Handle Informational Commands (don't consume a turn) ---
        if command == "help" {
            println!("--- Available Commands ---");
            println!(
                "  use <move name>      - Use one of your Pokémon's moves (e.g., 'use tackle')."
            );
            println!(
                "  switch to <team_num> - Switch to a Pokémon on your team (e.g., 'switch to 2')."
            );
            println!("  check self           - View your active Pokémon's details and moves.");
            println!("  check opponent       - View the opponent's active Pokémon's details.");
            println!("  check team           - View a summary of your team.");
            println!("  check team <team_num>  - View a benched Pokémon's details.");
            println!("  quit / forfeit       - Give up the battle.");
            println!("------------------------");
            continue;
        }
        if command == "check" {
            handle_check_command(args, battle_state);
            continue;
        }

        // --- Handle Action Commands (consume a turn) ---
        if switch_only && command != "switch" {
            println!("You must switch to a new Pokémon!");
            continue;
        }

        match command.as_str() {
            "use" => {
                let move_name = args.join(" ");
                let player = &battle_state.players[0];
                if let Some(active_pokemon) = player.active_pokemon() {
                    for (i, move_slot) in active_pokemon.moves.iter().enumerate() {
                        if let Some(move_instance) = move_slot {
                            let formatted_move_name =
                                format!("{:?}", move_instance.move_).replace('_', " ");
                            if formatted_move_name.eq_ignore_ascii_case(&move_name) {
                                return PlayerAction::UseMove { move_index: i };
                            }
                        }
                    }
                }
                println!(
                    "'{}' is not a valid move for your active Pokémon.",
                    move_name
                );
            }
            "switch" => {
                if args.len() == 2 && args[0] == "to" {
                    if let Ok(index) = args[1].parse::<usize>() {
                        if index > 0 && index <= 6 {
                            let team_index = index - 1; // Convert to 0-based index
                            let action = PlayerAction::SwitchPokemon { team_index };
                            if battle_state.players[0].validate_action(&action).is_ok() {
                                return action;
                            } else {
                                println!(
                                    "Invalid switch: {}",
                                    battle_state.players[0]
                                        .validate_action(&action)
                                        .unwrap_err()
                                );
                            }
                        }
                    }
                }
                println!("Invalid switch command. Use 'switch to <number>'.");
            }
            "quit" | "forfeit" => return PlayerAction::Forfeit,
            _ => println!("Unknown command. Type 'help' to see a list of commands."),
        }
    }
}

/// Sub-parser for the "check" command.
fn handle_check_command(args: &[&str], battle_state: &BattleState) {
    if args.is_empty() {
        println!("What do you want to check? (e.g., 'check self', 'check opponent', 'check team')");
        return;
    }
    match args[0].to_lowercase().as_str() {
        "self" => display_self_status(&battle_state.players[0]),
        "opponent" => display_opponent_status(&battle_state.players[1]),
        "team" => {
            if args.len() > 1 {
                if let Ok(index) = args[1].parse::<usize>() {
                    display_benched_pokemon_details(index - 1, &battle_state.players[0]);
                } else {
                    println!("Invalid team index. Please use a number.");
                }
            } else {
                display_team_status(&battle_state.players[0]);
            }
        }
        _ => println!("Unknown check command. Use 'self', 'opponent', or 'team'."),
    }
}

// --- Display Functions ---

fn display_hp_bar(pokemon: &PokemonInst) -> String {
    let percent = (pokemon.current_hp() as f32 / pokemon.max_hp() as f32) * 100.0;
    let filled_count = (percent / 10.0).round() as usize;
    let empty_count = 10 - filled_count;
    format!(
        "[{}{}] {}/{}",
        "█".repeat(filled_count),
        " ".repeat(empty_count),
        pokemon.current_hp(),
        pokemon.max_hp()
    )
}

fn display_battle_status(state: &BattleState) {
    let player_pokemon = state.players[0].active_pokemon().unwrap();
    let opponent_pokemon = state.players[1].active_pokemon().unwrap();

    println!(
        "  Opponent: {} {} {}",
        opponent_pokemon.name,
        display_hp_bar(opponent_pokemon),
        opponent_pokemon
            .status
            .map_or("".to_string(), |s| format!("{:?}", s))
    );
    println!(
        "      Your: {} {} {}",
        player_pokemon.name,
        display_hp_bar(player_pokemon),
        player_pokemon
            .status
            .map_or("".to_string(), |s| format!("{:?}", s))
    );
}

fn display_self_status(player: &BattlePlayer) {
    if let Some(pokemon) = player.active_pokemon() {
        println!(
            "\n--- Your Pokémon: {} (Lvl {}) ---",
            pokemon.name, pokemon.level
        );
        println!("  HP: {}", display_hp_bar(pokemon));
        if let Some(status) = pokemon.status {
            println!("  Status: {:?}", status);
        }
        println!("  Moves:");
        for (i, move_slot) in pokemon.moves.iter().enumerate() {
            if let Some(mv) = move_slot {
                println!(
                    "    {}. {:?} (PP: {}/{})",
                    i + 1,
                    mv.move_,
                    mv.pp,
                    mv.max_pp()
                );
            }
        }
    }
}

fn display_opponent_status(opponent: &BattlePlayer) {
    if let Some(pokemon) = opponent.active_pokemon() {
        println!(
            "\n--- Opponent's Pokémon: {} (Lvl {}) ---",
            pokemon.name, pokemon.level
        );
        println!("  HP: {}", display_hp_bar(pokemon));
        if let Some(status) = pokemon.status {
            println!("  Status: {:?}", status);
        }
        if !opponent.active_pokemon_conditions.is_empty() {
            println!(
                "  Active Conditions: {:?}",
                opponent
                    .active_pokemon_conditions
                    .keys()
                    .collect::<Vec<_>>()
            );
        }
        let total_pokemon = opponent.team.iter().filter(|p| p.is_some()).count();
        let remaining_pokemon = opponent
            .team
            .iter()
            .filter(|p| p.as_ref().map_or(false, |pk| !pk.is_fainted()))
            .count();
        println!(
            "  Team: {}/{} Pokémon remaining",
            remaining_pokemon, total_pokemon
        );
    }
}

fn display_team_status(player: &BattlePlayer) {
    println!("\n--- Your Team ---");
    for (i, pokemon_opt) in player.team.iter().enumerate() {
        if let Some(pokemon) = pokemon_opt {
            let active_marker = if i == player.active_pokemon_index {
                "(Active)"
            } else {
                ""
            };
            let fainted_marker = if pokemon.is_fainted() {
                "(Fainted)"
            } else {
                ""
            };
            println!(
                "  {}. {} (Lvl {}) {} {} {}",
                i + 1,
                pokemon.name,
                pokemon.level,
                display_hp_bar(pokemon),
                pokemon
                    .status
                    .map_or("".to_string(), |s| format!("{:?}", s)),
                active_marker.to_string() + fainted_marker
            );
        }
    }
}

fn display_benched_pokemon_details(index: usize, player: &BattlePlayer) {
    if let Some(Some(pokemon)) = player.team.get(index) {
        if index == player.active_pokemon_index {
            println!("This Pokémon is already active. Use 'check self' instead.");
            return;
        }
        println!(
            "\n--- Benched: {} (Lvl {}) ---",
            pokemon.name, pokemon.level
        );
        println!("  HP: {}", display_hp_bar(pokemon));
        if let Some(status) = pokemon.status {
            println!("  Status: {:?}", status);
        }
        println!("  Types: {:?}", pokemon.get_current_types(player));
        println!("  Moves:");
        for (i, move_slot) in pokemon.moves.iter().enumerate() {
            if let Some(mv) = move_slot {
                println!(
                    "    {}. {:?} (PP: {}/{})",
                    i + 1,
                    mv.move_,
                    mv.pp,
                    mv.max_pp()
                );
            }
        }
    } else {
        println!("No Pokémon at that position in your team.");
    }
}

fn print_turn_events(event_bus: &EventBus, battle_state: &BattleState) {
    println!();
    for event in event_bus.events() {
        if let Some(formatted_event) = event.format(battle_state) {
            println!("{}", formatted_event);
        }
    }
}
