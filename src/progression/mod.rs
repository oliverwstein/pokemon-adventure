pub mod evolution;
pub mod moves;
pub mod participation;
pub mod rewards;

// Re-exports for backward compatibility
pub use participation::BattleParticipationTracker;
pub use rewards::{EvYield, RewardCalculator};
