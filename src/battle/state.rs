use crate::player::{BattlePlayer, PlayerAction};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GameState {
    WaitingForBothActions,
    TurnInProgress,
    Player1Win,
    Player2Win,
    Draw,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BattleState {
    pub battle_id: String,
    pub players: [BattlePlayer; 2],
    pub turn_number: u32,
    pub game_state: GameState,
    pub action_queue: [Option<PlayerAction>; 2],
    pub turn_log: Vec<String>,
}

impl BattleState {
    pub fn new(id: String, player1: BattlePlayer, player2: BattlePlayer) -> Self {
        Self {
            battle_id: id,
            players: [player1, player2],
            turn_number: 1,
            game_state: GameState::WaitingForBothActions,
            action_queue: [None, None],
            turn_log: Vec::new(),
        }
    }
}