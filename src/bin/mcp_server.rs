//! Pokemon Adventure MCP Server
//! 
//! A Model Context Protocol server that exposes the Pokemon Adventure battle engine
//! for LLM interaction through natural language responses.

use std::io::{self, BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};

use pokemon_adventure::battle::state::BattleState;
use pokemon_adventure::mcp_interface::*;
use serde_json::{json, Value};

/// Shared battle state that persists across MCP tool calls
type SharedBattleState = Arc<Mutex<Option<BattleState>>>;

struct McpServer {
    battle_state: SharedBattleState,
}

impl McpServer {
    fn new() -> Self {
        Self {
            battle_state: Arc::new(Mutex::new(None)),
        }
    }

    fn handle_request(&self, method: &str, params: &Value) -> Value {
        match method {
            "initialize" => {
                json!({
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "pokemon-adventure",
                        "version": "1.0.0"
                    }
                })
            }
            "tools/list" => {
                json!({
                    "tools": [
                        {
                            "name": "start_battle",
                            "description": "Start a new Pokemon battle with the selected team",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "team_choice": {
                                        "type": "number",
                                        "description": "Team number to choose (1-3)"
                                    }
                                },
                                "required": ["team_choice"]
                            }
                        },
                        {
                            "name": "get_available_teams",
                            "description": "List all available Pokemon teams",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "get_battle_state",
                            "description": "Get the current battle state and status",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "use_move",
                            "description": "Use a Pokemon move in battle",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "move_name": {
                                        "type": "string",
                                        "description": "Name of the move to use"
                                    }
                                },
                                "required": ["move_name"]
                            }
                        },
                        {
                            "name": "switch_pokemon",
                            "description": "Switch to a different Pokemon on your team",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "pokemon_number": {
                                        "type": "number",
                                        "description": "Pokemon number to switch to (1-6)"
                                    }
                                },
                                "required": ["pokemon_number"]
                            }
                        },
                        {
                            "name": "check",
                            "description": "Check status of self, opponent, or team",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "target": {
                                        "type": "string",
                                        "description": "What to check: 'self', 'opponent', 'team', or 'team <number>'"
                                    }
                                },
                                "required": ["target"]
                            }
                        },
                        {
                            "name": "lookup_move",
                            "description": "Look up detailed information about a Pokemon move",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "move_name": {
                                        "type": "string",
                                        "description": "Name of the move to look up"
                                    }
                                },
                                "required": ["move_name"]
                            }
                        },
                        {
                            "name": "lookup_pokemon",
                            "description": "Look up detailed information about a Pokemon species",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "species_name": {
                                        "type": "string",
                                        "description": "Name of the Pokemon species to look up"
                                    }
                                },
                                "required": ["species_name"]
                            }
                        },
                        {
                            "name": "forfeit_battle",
                            "description": "Forfeit the current battle",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        }
                    ]
                })
            }
            "tools/call" => {
                let tool_name = params["name"].as_str().unwrap_or("");
                let args = &params["arguments"];
                self.handle_tool_call(tool_name, args)
            }
            _ => {
                json!({
                    "error": {
                        "code": -32601,
                        "message": "Method not found"
                    }
                })
            }
        }
    }

    fn handle_tool_call(&self, tool_name: &str, args: &Value) -> Value {
        let result = match tool_name {
            "start_battle" => {
                let team_choice = args["team_choice"].as_u64().unwrap_or(1) as usize;
                match create_battle(team_choice) {
                    Ok((new_battle_state, intro_text)) => {
                        *self.battle_state.lock().unwrap() = Some(new_battle_state);
                        json!({
                            "content": [{"type": "text", "text": intro_text}]
                        })
                    }
                    Err(e) => json!({
                        "content": [{"type": "text", "text": format!("Error: {}", e)}]
                    })
                }
            }
            "get_available_teams" => {
                let teams = get_available_teams_display();
                json!({
                    "content": [{"type": "text", "text": teams}]
                })
            }
            "get_battle_state" => {
                let text = match self.battle_state.lock().unwrap().as_ref() {
                    Some(state) => get_battle_status_summary(state),
                    None => "No battle is currently active. Use 'start_battle' to begin.".to_string(),
                };
                json!({
                    "content": [{"type": "text", "text": text}]
                })
            }
            "use_move" => {
                let move_name = args["move_name"].as_str().unwrap_or("");
                let text = match self.battle_state.lock().unwrap().as_mut() {
                    Some(state) => {
                        match execute_move_action(state, move_name) {
                            Ok(result) => result,
                            Err(e) => format!("Error: {}", e),
                        }
                    }
                    None => "No battle is currently active. Use 'start_battle' to begin.".to_string(),
                };
                json!({
                    "content": [{"type": "text", "text": text}]
                })
            }
            "switch_pokemon" => {
                let pokemon_number = args["pokemon_number"].as_u64().unwrap_or(1) as usize;
                let text = match self.battle_state.lock().unwrap().as_mut() {
                    Some(state) => {
                        match execute_switch_action(state, pokemon_number) {
                            Ok(result) => result,
                            Err(e) => format!("Error: {}", e),
                        }
                    }
                    None => "No battle is currently active. Use 'start_battle' to begin.".to_string(),
                };
                json!({
                    "content": [{"type": "text", "text": text}]
                })
            }
            "check" => {
                let target = args["target"].as_str().unwrap_or("self");
                let text = match self.battle_state.lock().unwrap().as_ref() {
                    Some(state) => handle_check_command(target, state),
                    None => "No battle is currently active. Use 'start_battle' to begin.".to_string(),
                };
                json!({
                    "content": [{"type": "text", "text": text}]
                })
            }
            "lookup_move" => {
                let move_name = args["move_name"].as_str().unwrap_or("");
                let text = handle_lookup_move_command(move_name);
                json!({
                    "content": [{"type": "text", "text": text}]
                })
            }
            "lookup_pokemon" => {
                let species_name = args["species_name"].as_str().unwrap_or("");
                let text = handle_lookup_pokemon_command(species_name);
                json!({
                    "content": [{"type": "text", "text": text}]
                })
            }
            "forfeit_battle" => {
                let text = match self.battle_state.lock().unwrap().as_mut() {
                    Some(state) => {
                        match execute_forfeit_action(state) {
                            Ok(result) => result,
                            Err(e) => format!("Error: {}", e),
                        }
                    }
                    None => "No battle is currently active. Use 'start_battle' to begin.".to_string(),
                };
                json!({
                    "content": [{"type": "text", "text": text}]
                })
            }
            _ => json!({
                "content": [{"type": "text", "text": format!("Unknown tool: {}", tool_name)}]
            })
        };

        result
    }

    fn run(&self) -> io::Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let reader = BufReader::new(stdin.lock());

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            // Parse the JSON-RPC request
            let request: Value = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(_) => continue,
            };

            let id = request["id"].clone();
            let method = request["method"].as_str().unwrap_or("");
            let params = &request["params"];

            // Handle the request and create response
            let result = self.handle_request(method, params);
            
            let response = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            });

            // Send response
            writeln!(stdout, "{}", response)?;
            stdout.flush()?;
        }

        Ok(())
    }
}

fn main() -> io::Result<()> {
    let server = McpServer::new();
    server.run()
}