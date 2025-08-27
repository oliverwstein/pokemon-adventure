// In: src/lib.rs

//! Pokemon Adventure Battle Engine
//!
//! A comprehensive Pokemon battle system with authentic Generation 1 mechanics
//! and modern software engineering practices. Designed for serverless deployment
//! with compile-time data optimization.

// --- MODULE DECLARATIONS ---
// This declares the module hierarchy for the crate.
pub mod battle;
pub mod errors;
pub mod mcp_interface;
pub mod move_data;
pub mod player;
pub mod pokemon;
pub mod species;
pub mod teams;

// --- PUBLIC API RE-EXPORTS ---
// This section defines the public-facing API of the `pokemon-adventure` crate,
// making it easy for users to import the most important types directly.

// --- From the `schema` crate ---
// Re-export all core data definitions and static enums.
pub use schema::{
    // Supporting Types & Enums
    BaseStats,
    EvolutionData,
    EvolutionMethod,
    Item,
    Learnset,
    // Core Enums
    Move,
    MoveCategory,
    // Core Data Structs
    MoveData,
    PokemonSpecies,
    PokemonType,
    Species,
    StatType,
    StatusType,
    Target,
    TeamCondition,
};

// --- From this crate's modules (`src/`) ---

// Core battle engine functions and state.
pub use battle::engine::{collect_npc_actions, ready_for_turn_resolution, resolve_turn};
pub use battle::state::{BattleEvent, BattleState, GameState};

// Core runtime types for a battle.
pub use player::{BattlePlayer, PlayerAction, PlayerType};
pub use pokemon::{PokemonInst, StatusCondition};

// Primary data access functions.
pub use move_data::get_move_data;
pub use pokemon::get_species_data;

// Crate-specific error and result types.
pub use errors::{
    ActionError, BattleEngineError, BattleResult, BattleStateError, MoveDataError, MoveDataResult,
    SpeciesDataError, SpeciesDataResult,
};
