// Pokemon Adventure Schema - Shared type definitions
// This crate contains all the core enums and types that are shared between
// the main pokemon-adventure crate and its build script, enabling the use of
// postcard for efficient serialization.

// Re-export the main types
pub use species::*;
pub use moves::*;
pub use pokemon_types::*;
pub use move_types::*;

pub mod species;
pub mod moves;
pub mod pokemon_types;
pub mod move_types;