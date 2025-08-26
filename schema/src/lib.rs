// Pokemon Adventure Schema - Shared type definitions
// This crate contains all the core enums and types that are shared between
// the main pokemon-adventure crate and its build script, enabling the use of
// postcard for efficient serialization.

// Re-export core enums
pub use move_types::*;
pub use moves::*;
pub use pokemon_types::*;
pub use species::*;

// Re-export data structures
pub use move_data::*;
pub use species_data::*;
pub use battle_data::*;

pub mod move_types;
pub mod moves;
pub mod pokemon_types;
pub mod species;
pub mod move_data;
pub mod species_data;
pub mod battle_data;
