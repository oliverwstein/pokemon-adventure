//! MCP interface functions extracted from main.rs for use in the MCP server
//!
//! This module contains all the display, command handling, and interaction functions
//! that were originally in main.rs, made available as library functions.

use crate::battle::engine::{collect_npc_actions, ready_for_turn_resolution, resolve_turn};
use crate::battle::state::{BattleState, GameState, TurnRng};
use crate::move_data::get_move_data;
use crate::player::{PlayerAction, PlayerType};
use crate::teams;
use crate::{BattlePlayer, Move, Species};

/// Returns formatted text displaying available demo teams
pub fn get_available_teams_display() -> String {
    let team_ids = teams::get_demo_team_ids();
    let mut output = String::from("Available Teams:\n");

    for (i, team_id) in team_ids.iter().enumerate() {
        if let Some(team_info) = teams::get_team_info(team_id) {
            output.push_str(&format!(
                "  {}. {} - {}\n",
                i + 1,
                team_info.name,
                team_info.description
            ));
        }
    }
    output
}

/// Creates a new battle with the specified team choice and returns initial battle text
pub fn create_battle(team_choice: usize) -> Result<(BattleState, String), String> {
    let team_ids = teams::get_demo_team_ids();
    if team_choice == 0 || team_choice > team_ids.len() {
        return Err(format!(
            "Invalid team choice. Please choose 1-{}",
            team_ids.len()
        ));
    }

    let player_team_id = &team_ids[team_choice - 1];
    let player_team_info = teams::get_team_info(player_team_id)
        .ok_or_else(|| format!("Team info not found for {}", player_team_id))?;

    let mut human_player = teams::create_battle_player_from_team(
        player_team_id,
        "human_player".to_string(),
        "Player".to_string(),
    )
    .map_err(|e| format!("Failed to create player team: {}", e))?;
    human_player.player_type = PlayerType::Human;

    let mut npc_player = teams::create_battle_player_from_team(
        "demo_charizard",
        "npc_opponent".to_string(),
        "AI Trainer".to_string(),
    )
    .map_err(|e| format!("Failed to create NPC team: {}", e))?;
    npc_player.player_type = PlayerType::NPC;

    let battle_state = BattleState::new("mcp_battle".to_string(), human_player, npc_player);

    let intro_text = format!(
        "🔥 Welcome to the Pokémon Adventure Battle Engine! 🔥\n\nYou chose the {}!\n\n💥 A wild trainer challenges you to a battle! 💥\nYou sent out {}!\n{} sends out {}!",
        player_team_info.name,
        battle_state.players[0]
            .active_pokemon()
            .map(|p| p.name.as_str())
            .unwrap_or("Unknown"),
        battle_state.players[1].player_name,
        battle_state.players[1]
            .active_pokemon()
            .map(|p| p.name.as_str())
            .unwrap_or("Unknown")
    );

    Ok((battle_state, intro_text))
}

/// Displays the current battle state using the BattleState's Display implementation
pub fn display_battle_status(state: &BattleState) -> String {
    format!("{}", state)
}

/// Displays the full details of the player's active Pokémon
pub fn display_self_status(player: &BattlePlayer) -> String {
    if let Some(pokemon) = player.active_pokemon() {
        format!("--- Your Active Pokémon ---\n{}", pokemon)
    } else {
        "You have no active Pokémon.".to_string()
    }
}

/// Displays a summary of the opponent's status
pub fn display_opponent_status(opponent: &BattlePlayer) -> String {
    format!("--- Opponent's Status ---\n{}", opponent)
}

/// Displays a summary of the player's entire team
pub fn display_team_status(player: &BattlePlayer) -> String {
    let mut output = String::from("--- Your Team ---\n");
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
                output.push_str(&format!(
                    " {}. {}{}{}\n",
                    i + 1,
                    first_line,
                    active_marker,
                    fainted_marker
                ));
            }

            // Print the rest of the Pokemon's details, indented
            for line in lines {
                output.push_str(&format!("    {}\n", line));
            }
            output.push('\n'); // Add a blank line between Pokémon for readability
        }
    }
    output
}

/// Displays the full details of a single benched Pokémon
pub fn display_benched_pokemon_details(index: usize, player: &BattlePlayer) -> String {
    if let Some(Some(pokemon)) = player.team.get(index) {
        let mut output = String::new();
        if index == player.active_pokemon_index {
            output.push_str("This is your active Pokémon.\n");
        }
        output.push_str(&format!("--- Benched Pokémon Details ---\n{}", pokemon));
        output
    } else {
        "No Pokémon at that position in your team.".to_string()
    }
}

/// Handles the "check" command with the given arguments
pub fn handle_check_command(args: &str, battle_state: &BattleState) -> String {
    let parts: Vec<&str> = args.trim().split_whitespace().collect();
    if parts.is_empty() {
        return "What do you want to check? (e.g., 'self', 'opponent', 'team')".to_string();
    }

    match parts[0].to_lowercase().as_str() {
        "self" => display_self_status(&battle_state.players[0]),
        "opponent" => display_opponent_status(&battle_state.players[1]),
        "team" => {
            if parts.len() > 1 {
                if let Ok(index) = parts[1].parse::<usize>() {
                    display_benched_pokemon_details(index - 1, &battle_state.players[0])
                } else {
                    "Invalid team index. Please use a number.".to_string()
                }
            } else {
                display_team_status(&battle_state.players[0])
            }
        }
        _ => "Unknown check command. Use 'self', 'opponent', or 'team'.".to_string(),
    }
}

/// Handles the "lookup move" command for move details
pub fn handle_lookup_move_command(move_name: &str) -> String {
    if move_name.trim().is_empty() {
        return "What move do you want to look up? (e.g., 'tackle')".to_string();
    }

    match move_name.parse::<Move>() {
        Ok(move_enum) => {
            if let Ok(move_data) = get_move_data(move_enum) {
                format!("--- Move Details ---\n{}", move_data)
            } else {
                format!("Could not find details for the move '{}'.", move_name)
            }
        }
        Err(_) => {
            format!("The move '{}' was not found.", move_name)
        }
    }
}

/// Handles the "lookup pokemon" command for species details
pub fn handle_lookup_pokemon_command(species_name: &str) -> String {
    if species_name.trim().is_empty() {
        return "What Pokemon do you want to look up? (e.g., 'Pikachu')".to_string();
    }

    match species_name.parse::<Species>() {
        Ok(species) => {
            format!("--- Pokemon Details ---\n{}", species)
        }
        Err(_) => {
            format!("The Pokemon '{}' was not found.", species_name)
        }
    }
}

/// Executes a move action and returns the battle events as formatted text
pub fn execute_move_action(
    battle_state: &mut BattleState,
    move_name: &str,
) -> Result<String, String> {
    let player = &battle_state.players[0];
    if let Some(active_pokemon) = player.active_pokemon() {
        for (i, move_slot) in active_pokemon.moves.iter().enumerate() {
            if let Some(move_instance) = move_slot {
                if let Ok(move_data) = get_move_data(move_instance.move_) {
                    if move_data.name.eq_ignore_ascii_case(move_name) {
                        let action = PlayerAction::UseMove { move_index: i };
                        return execute_player_action(battle_state, action);
                    }
                }
            }
        }
    }
    Err(format!(
        "'{}' is not a valid move for your active Pokémon.",
        move_name
    ))
}

/// Executes a switch action and returns the battle events as formatted text
pub fn execute_switch_action(
    battle_state: &mut BattleState,
    pokemon_number: usize,
) -> Result<String, String> {
    if pokemon_number == 0 || pokemon_number > 6 {
        return Err("Invalid Pokemon number. Use 1-6.".to_string());
    }

    let team_index = pokemon_number - 1;
    let action = PlayerAction::SwitchPokemon { team_index };

    if let Err(msg) = battle_state.players[0].validate_action(&action) {
        return Err(format!("Invalid switch: {}", msg));
    }

    execute_player_action(battle_state, action)
}

/// Executes a forfeit action
pub fn execute_forfeit_action(battle_state: &mut BattleState) -> Result<String, String> {
    execute_player_action(battle_state, PlayerAction::Forfeit)
}

/// Internal helper to execute a player action and return formatted events
fn execute_player_action(
    battle_state: &mut BattleState,
    action: PlayerAction,
) -> Result<String, String> {
    // Set the player action
    battle_state.action_queue[0] = Some(action);

    // Let the AI act if needed
    if battle_state.action_queue[1].is_none() {
        let npc_actions = collect_npc_actions(battle_state);
        for (player_index, ai_action) in npc_actions {
            battle_state.action_queue[player_index] = Some(ai_action);
        }
    }

    // Resolve the turn if both players have actions
    let mut output = String::new();
    if ready_for_turn_resolution(battle_state) {
        let rng = TurnRng::new_random();
        let event_bus = resolve_turn(battle_state, rng);

        // Format turn events
        for event in event_bus.events() {
            if let Some(formatted_event) = event.format(battle_state) {
                output.push_str(&format!("{}\n", formatted_event));
            }
        }
    }

    // Check for battle end states
    match battle_state.game_state {
        GameState::Player1Win => {
            output.push_str("\n🎉 You won the battle! 🎉\n");
        }
        GameState::Player2Win => {
            output.push_str("\n💀 You lost the battle! 💀\n");
        }
        GameState::Draw => {
            output.push_str("\n🤝 The battle ended in a draw! 🤝\n");
        }
        GameState::WaitingForPlayer1Replacement => {
            output.push_str("\nYour Pokémon fainted! You must switch to a new Pokémon.\n");
        }
        _ => {}
    }

    Ok(output)
}

/// Checks if the battle is over
pub fn is_battle_over(battle_state: &BattleState) -> bool {
    matches!(
        battle_state.game_state,
        GameState::Player1Win | GameState::Player2Win | GameState::Draw
    )
}

/// Checks if the player needs to make a forced replacement
pub fn needs_forced_replacement(battle_state: &BattleState) -> bool {
    matches!(
        battle_state.game_state,
        GameState::WaitingForPlayer1Replacement
    )
}

/// Gets the current battle status as a formatted string
pub fn get_battle_status_summary(battle_state: &BattleState) -> String {
    let mut output = String::new();

    if is_battle_over(battle_state) {
        output.push_str(&match battle_state.game_state {
            GameState::Player1Win => "🎉 Battle Over - You Won! 🎉",
            GameState::Player2Win => "💀 Battle Over - You Lost! 💀",
            GameState::Draw => "🤝 Battle Over - Draw! 🤝",
            _ => "Battle Over",
        });
        output.push('\n');
    } else if needs_forced_replacement(battle_state) {
        output.push_str("⚠️  Your Pokémon fainted! You must switch to continue. ⚠️\n");
    } else {
        output.push_str("⚔️  Battle in Progress ⚔️\n");
    }

    output.push_str(&display_battle_status(battle_state));
    output
}
