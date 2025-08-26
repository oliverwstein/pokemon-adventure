use crate::errors::{MoveDataError, MoveDataResult};
use crate::pokemon::PokemonType;
use schema::Move;
// Include the compiled move data
include!(concat!(env!("OUT_DIR"), "/generated_data.rs"));

// Re-export move-related types from the schema crate
pub use schema::{MoveCategory, StatType, StatusType, Target, MoveData, MoveEffect};

pub fn get_move_data(move_: Move) -> MoveDataResult<MoveData> {
    // Handle special hardcoded moves first
    match move_ {
        Move::HittingItself => Ok(MoveData {
            name: "Hit Itself".to_string(),
            move_type: PokemonType::Typeless,
            power: Some(40),
            category: MoveCategory::Physical,
            accuracy: None, // Always hits
            max_pp: 0,      // Not a real move, no PP
            effects: vec![],
        }),
        Move::Struggle => Ok(MoveData {
            name: "Struggle".to_string(),
            move_type: PokemonType::Typeless,
            power: Some(50),
            category: MoveCategory::Physical,
            accuracy: Some(90),
            max_pp: 0,                             // Not a real move, no PP
            effects: vec![MoveEffect::Recoil(25)], // 25% recoil of damage dealt
        }),
        _ => {
            // For regular moves, get from the compiled data warehouse
            get_compiled_move_data()
                .get(&move_)
                .cloned()
                .ok_or(MoveDataError::MoveNotFound(move_))
        }
    }
}

pub fn get_move_max_pp(move_: Move) -> MoveDataResult<u8> {
    get_move_data(move_).map(|data| data.max_pp)
}