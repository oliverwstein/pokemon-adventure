//! Pokemon Adventure MCP Server - Proper SDK Implementation
//!
//! A Model Context Protocol server using the official Rust SDK (rmcp)
//! that exposes the Pokemon Adventure battle engine for LLM interaction.

use std::borrow::Cow;
use std::future::Future;

use pokemon_adventure::mcp_interface::*;
use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{ErrorData as McpError, *},
    schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt,
};
use serde::Deserialize;
use tokio::io::{stdin, stdout};

#[derive(Debug, Clone)]
pub struct PokemonAdventureService {
    tool_router: ToolRouter<PokemonAdventureService>,
    battle_state:
        std::sync::Arc<std::sync::Mutex<Option<pokemon_adventure::battle::state::BattleState>>>,
}

// Tool request structures
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StartBattleRequest {
    #[schemars(description = "Team number to choose (1-3)")]
    pub team_choice: u8,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UseMoveRequest {
    #[schemars(description = "Name of the move to use")]
    pub move_name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SwitchPokemonRequest {
    #[schemars(description = "Pokemon number to switch to (1-6)")]
    pub pokemon_number: u8,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckRequest {
    #[schemars(description = "What to check: 'self', 'opponent', 'team', or 'team <number>'")]
    pub target: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LookupMoveRequest {
    #[schemars(description = "Name of the move to look up")]
    pub move_name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LookupPokemonRequest {
    #[schemars(description = "Name of the Pokemon species to look up")]
    pub species_name: String,
}

#[tool_router]
impl PokemonAdventureService {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            battle_state: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    #[tool(description = "List all available pre-built Pokemon teams")]
    async fn get_available_teams(&self) -> Result<CallToolResult, McpError> {
        let teams = get_available_teams_display();
        Ok(CallToolResult::success(vec![Content::text(teams)]))
    }

    #[tool(description = "Start a new Pokemon battle with the selected team")]
    async fn start_battle(
        &self,
        Parameters(request): Parameters<StartBattleRequest>,
    ) -> Result<CallToolResult, McpError> {
        match create_battle(request.team_choice as usize) {
            Ok((new_battle_state, intro_text)) => {
                *self.battle_state.lock().unwrap() = Some(new_battle_state);
                Ok(CallToolResult::success(vec![Content::text(intro_text)]))
            }
            Err(e) => Err(McpError {
                code: ErrorCode(-32603),
                message: Cow::from(format!("Error starting battle: {}", e)),
                data: None,
            }),
        }
    }

    #[tool(description = "Get the current battle state and status")]
    async fn get_battle_state(&self) -> Result<CallToolResult, McpError> {
        let text = match self.battle_state.lock().unwrap().as_ref() {
            Some(state) => get_battle_status_summary(state),
            None => "No battle is currently active. Use 'start_battle' to begin.".to_string(),
        };
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Use a Pokemon move in battle")]
    async fn use_move(
        &self,
        Parameters(request): Parameters<UseMoveRequest>,
    ) -> Result<CallToolResult, McpError> {
        let text = match self.battle_state.lock().unwrap().as_mut() {
            Some(state) => match execute_move_action(state, &request.move_name) {
                Ok(result) => result,
                Err(e) => format!("Error: {}", e),
            },
            None => "No battle is currently active. Use 'start_battle' to begin.".to_string(),
        };
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Switch to a different Pokemon on your team")]
    async fn switch_pokemon(
        &self,
        Parameters(request): Parameters<SwitchPokemonRequest>,
    ) -> Result<CallToolResult, McpError> {
        let text = match self.battle_state.lock().unwrap().as_mut() {
            Some(state) => match execute_switch_action(state, request.pokemon_number as usize) {
                Ok(result) => result,
                Err(e) => format!("Error: {}", e),
            },
            None => "No battle is currently active. Use 'start_battle' to begin.".to_string(),
        };
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Check status of self, opponent, or team")]
    async fn check(
        &self,
        Parameters(request): Parameters<CheckRequest>,
    ) -> Result<CallToolResult, McpError> {
        let text = match self.battle_state.lock().unwrap().as_ref() {
            Some(state) => handle_check_command(&request.target, state),
            None => "No battle is currently active. Use 'start_battle' to begin.".to_string(),
        };
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Look up detailed information about a Pokemon move")]
    async fn lookup_move(
        &self,
        Parameters(request): Parameters<LookupMoveRequest>,
    ) -> Result<CallToolResult, McpError> {
        let text = handle_lookup_move_command(&request.move_name);
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Look up detailed information about a Pokemon species")]
    async fn lookup_pokemon(
        &self,
        Parameters(request): Parameters<LookupPokemonRequest>,
    ) -> Result<CallToolResult, McpError> {
        let text = handle_lookup_pokemon_command(&request.species_name);
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Forfeit the current battle")]
    async fn forfeit_battle(&self) -> Result<CallToolResult, McpError> {
        let text = match self.battle_state.lock().unwrap().as_mut() {
            Some(state) => match execute_forfeit_action(state) {
                Ok(result) => result,
                Err(e) => format!("Error: {}", e),
            },
            None => "No battle is currently active. Use 'start_battle' to begin.".to_string(),
        };
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

#[tool_handler]
impl ServerHandler for PokemonAdventureService {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Pokemon Adventure MCP Server (SDK version) starting...");

    let service = PokemonAdventureService::new();
    let transport = (stdin(), stdout());

    eprintln!("Starting MCP server with transport...");
    let server = service.serve(transport).await?;

    eprintln!("Server running, waiting for shutdown...");
    let quit_reason = server.waiting().await?;

    eprintln!("Pokemon Adventure MCP Server exiting: {:?}", quit_reason);
    Ok(())
}
