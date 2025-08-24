//! Pokemon Adventure Battle Engine
//!
//! A comprehensive Pokemon battle system with authentic Generation 1 mechanics
//! and modern software engineering practices. Designed for serverless deployment
//! with compile-time data optimization.

pub mod battle;
pub mod errors;
pub mod mcp_interface;
pub mod move_data;
pub mod moves;
pub mod player;
pub mod pokemon;
pub mod prefab_teams;
pub mod species;

// Re-export commonly used types for convenience
pub use battle::engine::{collect_npc_actions, ready_for_turn_resolution, resolve_turn};
pub use battle::state::{BattleEvent, BattleState, GameState};
pub use errors::{
    BattleEngineError, BattleResult, MoveDataError, MoveDataResult, SpeciesDataError,
    SpeciesDataResult,
};
pub use moves::Move;
pub use player::{BattlePlayer, PlayerAction, StatType};
pub use pokemon::{get_species_data, PokemonInst};
pub use species::Species;
