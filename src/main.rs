use std::io::{self, Write};

use pokemon_adventure::battle::engine::{
    collect_npc_actions, ready_for_turn_resolution, resolve_turn,
};
use pokemon_adventure::battle::state::{BattleState, EventBus, GameState, TurnRng};
use pokemon_adventure::move_data::MoveData;
use pokemon_adventure::player::{PlayerAction, PlayerType};
use pokemon_adventure::prefab_teams::{self, PrefabTeam};
use pokemon_adventure::{BattlePlayer, Move};

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
    // Use the BattleState's Display trait for the final status.
    println!("{}", battle_state);
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
            println!("\nYour Pokémon fainted! You must switch.");
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
            // Display the entire battle state using our new Display trait.
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
                "  use <move name>        - Use one of your Pokémon's moves (e.g., 'use tackle')."
            );
            println!(
                "  switch to <team_num>   - Switch to a Pokémon on your team (e.g., 'switch to 2')."
            );
            println!("  check self             - View your active Pokémon's details and moves.");
            println!("  check opponent         - View the opponent's active Pokémon's details.");
            println!("  check team             - View a summary of your team.");
            println!("  check team <team_num>    - View a benched Pokémon's details.");
            println!(
                "  lookup <move name>     - View the details of a specific move (e.g., 'lookup flamethrower')."
            );
            println!("  quit / forfeit         - Give up the battle.");
            println!("------------------------");
            continue;
        }
        if command == "check" {
            handle_check_command(args, battle_state);
            continue;
        }
        if command == "lookup" {
            handle_lookup_command(args);
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
                            // Fetch move data to compare names
                            if let Ok(move_data) = MoveData::get_move_data(move_instance.move_) {
                                if move_data.name.eq_ignore_ascii_case(&move_name) {
                                    return PlayerAction::UseMove { move_index: i };
                                }
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
                            if let Err(msg) = battle_state.players[0].validate_action(&action) {
                                println!("Invalid switch: {}", msg);
                            } else {
                                return action;
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

// --- Rewritten Display Functions ---

/// Displays the entire battle state. Replaces the old two-line summary.
fn display_battle_status(state: &BattleState) {
    println!("\n{}", state);
}

/// Displays the full details of the player's active Pokémon using its Display trait.
fn display_self_status(player: &BattlePlayer) {
    if let Some(pokemon) = player.active_pokemon() {
        println!("\n--- Your Active Pokémon ---");
        println!("{}", pokemon);
    } else {
        println!("You have no active Pokémon.");
    }
}

/// Sub-parser for the "lookup" command.
fn handle_lookup_command(args: &[&str]) {
    if args.is_empty() {
        println!("What move do you want to look up? (e.g., 'lookup tackle')");
        return;
    }

    // Join all arguments to handle multi-word move names like "swords dance"
    let move_name = args.join(" ");

    // Use the FromStr implementation for the Move enum to parse the string.
    match move_name.parse::<Move>() {
        Ok(move_enum) => {
            // The parse was successful, now get the associated data.
            if let Ok(move_data) = MoveData::get_move_data(move_enum) {
                // We found the data, so print it using its Display implementation.
                println!("\n--- Move Details ---");
                println!("{}", move_data);
            } else {
                // This is an unlikely edge case where a Move variant exists
                // but has no entry in the data map.
                println!("Could not find details for the move '{}'.", move_name);
            }
        }
        Err(_) => {
            // The string did not match any known Move variants.
            println!("The move '{}' was not found.", move_name);
        }
    }
}

/// Displays a summary of the opponent's status using the BattlePlayer's Display trait.
fn display_opponent_status(opponent: &BattlePlayer) {
    println!("\n--- Opponent's Status ---");
    println!("{}", opponent);
}

/// Displays a summary of the player's entire team.
fn display_team_status(player: &BattlePlayer) {
    println!("\n--- Your Team ---");
    for (i, pokemon_opt) in player.team.iter().enumerate() {
        if let Some(pokemon) = pokemon_opt {
            let pokemon_display = format!("{:#}", pokemon);
            let mut lines = pokemon_display.lines();

            // Print the first line with contextual markers
            if let Some(first_line) = lines.next() {
                let active_marker = if i == player.active_pokemon_index {
                    " (Active)"
                } else {
                    ""
                };
                let fainted_marker = if pokemon.is_fainted() {
                    " (Fainted)"
                } else {
                    ""
                };
                println!(
                    " {}. {}{}{}",
                    i + 1,
                    first_line,
                    active_marker,
                    fainted_marker
                );
            }

            // Print the rest of the Pokemon's details, indented
            for line in lines {
                println!("    {}", line);
            }
            println!(); // Add a blank line between Pokémon for readability
        }
    }
}

/// Displays the full details of a single benched Pokémon using its Display trait.
fn display_benched_pokemon_details(index: usize, player: &BattlePlayer) {
    if let Some(Some(pokemon)) = player.team.get(index) {
        if index == player.active_pokemon_index {
            println!("\nThis is your active Pokémon.");
        }
        println!("\n--- Benched Pokémon Details ---");
        println!("{}", pokemon);
    } else {
        println!("No Pokémon at that position in your team.");
    }
}

/// Prints formatted events from the event bus.
fn print_turn_events(event_bus: &EventBus, battle_state: &BattleState) {
    println!();
    for event in event_bus.events() {
        if let Some(formatted_event) = event.format(battle_state) {
            println!("{}", formatted_event);
        }
    }
}
