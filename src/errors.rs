use crate::moves::Move;
use crate::species::Species;
use std::fmt;

/// Main error type for the Pokemon Adventure battle engine
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleEngineError {
    /// Error related to move data lookup or processing
    MoveData(MoveDataError),
    /// Error related to species data lookup or processing
    SpeciesData(SpeciesDataError),
    /// Error related to invalid battle state
    BattleState(BattleStateError),
    /// Error related to invalid player actions
    Action(ActionError),
}

/// Errors related to move data operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveDataError {
    /// The specified move was not found in the database
    MoveNotFound(Move),
    /// Move reference is invalid or corrupted
    InvalidMoveReference,
    /// Move data is malformed or incomplete
    MalformedData(String),
}

/// Errors related to species data operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeciesDataError {
    /// The specified species was not found in the database
    SpeciesNotFound(Species),
    /// Species reference is invalid or corrupted
    InvalidSpeciesReference,
    /// Species data is malformed or incomplete
    MalformedData(String),
}

/// Errors related to battle state validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleStateError {
    /// No active Pokemon found when one was expected
    NoActivePokemon,
    /// Invalid player index
    InvalidPlayerIndex(usize),
    /// Battle state is in an inconsistent or corrupted state
    InconsistentState(String),
}

/// Errors related to player actions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionError {
    /// Move index is out of bounds
    InvalidMoveIndex(usize),
    /// Pokemon index is out of bounds
    InvalidPokemonIndex(usize),
    /// Action is not valid in the current battle state
    InvalidAction(String),
}

impl fmt::Display for BattleEngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BattleEngineError::MoveData(err) => write!(f, "Move data error: {}", err),
            BattleEngineError::SpeciesData(err) => write!(f, "Species data error: {}", err),
            BattleEngineError::BattleState(err) => write!(f, "Battle state error: {}", err),
            BattleEngineError::Action(err) => write!(f, "Action error: {}", err),
        }
    }
}

impl fmt::Display for MoveDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveDataError::MoveNotFound(move_) => write!(f, "Move not found: {:?}", move_),
            MoveDataError::InvalidMoveReference => write!(f, "Invalid move reference"),
            MoveDataError::MalformedData(details) => write!(f, "Malformed move data: {}", details),
        }
    }
}

impl fmt::Display for SpeciesDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpeciesDataError::SpeciesNotFound(species) => write!(f, "Species not found: {:?}", species),
            SpeciesDataError::InvalidSpeciesReference => write!(f, "Invalid species reference"),
            SpeciesDataError::MalformedData(details) => write!(f, "Malformed species data: {}", details),
        }
    }
}

impl fmt::Display for BattleStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BattleStateError::NoActivePokemon => write!(f, "No active Pokemon found"),
            BattleStateError::InvalidPlayerIndex(index) => write!(f, "Invalid player index: {}", index),
            BattleStateError::InconsistentState(details) => write!(f, "Inconsistent battle state: {}", details),
        }
    }
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionError::InvalidMoveIndex(index) => write!(f, "Invalid move index: {}", index),
            ActionError::InvalidPokemonIndex(index) => write!(f, "Invalid Pokemon index: {}", index),
            ActionError::InvalidAction(details) => write!(f, "Invalid action: {}", details),
        }
    }
}

impl std::error::Error for BattleEngineError {}
impl std::error::Error for MoveDataError {}
impl std::error::Error for SpeciesDataError {}
impl std::error::Error for BattleStateError {}
impl std::error::Error for ActionError {}

impl From<MoveDataError> for BattleEngineError {
    fn from(err: MoveDataError) -> Self {
        BattleEngineError::MoveData(err)
    }
}

impl From<SpeciesDataError> for BattleEngineError {
    fn from(err: SpeciesDataError) -> Self {
        BattleEngineError::SpeciesData(err)
    }
}

impl From<BattleStateError> for BattleEngineError {
    fn from(err: BattleStateError) -> Self {
        BattleEngineError::BattleState(err)
    }
}

impl From<ActionError> for BattleEngineError {
    fn from(err: ActionError) -> Self {
        BattleEngineError::Action(err)
    }
}

/// Type alias for Results using BattleEngineError
pub type BattleResult<T> = Result<T, BattleEngineError>;

/// Type alias for Results using MoveDataError
pub type MoveDataResult<T> = Result<T, MoveDataError>;

/// Type alias for Results using SpeciesDataError  
pub type SpeciesDataResult<T> = Result<T, SpeciesDataError>;