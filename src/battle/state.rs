use crate::moves::Move;
use crate::player::{BattlePlayer, PlayerAction, PokemonCondition, StatType};
use crate::species::Species;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GameState {
    WaitingForBothActions,
    TurnInProgress,
    WaitingForPlayer1Replacement, // Player 1 needs to send out a new Pokemon after faint
    WaitingForPlayer2Replacement, // Player 2 needs to send out a new Pokemon after faint
    WaitingForBothReplacements,   // Both players need to send out new Pokemon after faints
    Player1Win,
    Player2Win,
    Draw,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BattleEvent {
    // Turn Management
    TurnStarted {
        turn_number: u32,
    },
    TurnEnded,

    // Pokemon Actions
    PokemonSwitched {
        player_index: usize,
        old_pokemon: Species,
        new_pokemon: Species,
    },
    MoveUsed {
        player_index: usize,
        pokemon: Species,
        move_used: Move,
    },
    MoveMissed {
        attacker: Species,
        defender: Species,
        move_used: Move,
    },
    MoveHit {
        attacker: Species,
        defender: Species,
        move_used: Move,
    },
    CriticalHit {
        attacker: Species,
        defender: Species,
        move_used: Move,
    },
    DamageDealt {
        target: Species,
        damage: u16,
        remaining_hp: u16,
    },
    PokemonHealed {
        target: Species,
        amount: u16,
        new_hp: u16,
    },
    PokemonFainted {
        player_index: usize,
        pokemon: Species,
    },
    AttackTypeEffectiveness {
        multiplier: f64,
    },
    // Status Effects
    StatusApplied {
        target: Species,
        status: PokemonCondition,
    },
    StatusRemoved {
        target: Species,
        status: PokemonCondition,
    },
    StatusDamage {
        target: Species,
        status: PokemonCondition,
        damage: u16,
    },

    // Pokemon Status Conditions (Sleep, Poison, Burn, etc.)
    PokemonStatusApplied {
        target: Species,
        status: crate::pokemon::StatusCondition,
    },
    PokemonStatusRemoved {
        target: Species,
        status: crate::pokemon::StatusCondition,
    },
    PokemonStatusDamage {
        target: Species,
        status: crate::pokemon::StatusCondition,
        damage: u16,
        remaining_hp: u16,
    },

    // Active Condition Updates
    ConditionExpired {
        target: Species,
        condition: PokemonCondition,
    },

    // Stat Changes
    StatStageChanged {
        target: Species,
        stat: StatType,
        old_stage: i8,
        new_stage: i8,
    },

    // Action Failures
    ActionFailed {
        reason: ActionFailureReason,
    },

    // Battle End
    PlayerDefeated {
        player_index: usize,
    },
    BattleEnded {
        winner: Option<usize>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ActionFailureReason {
    IsAsleep,
    IsFrozen,
    IsExhausted,
    IsParalyzed,
    IsFlinching,
    IsConfused,
    IsTrapped,
    NoEnemyPresent, // When opponent-targeting move can't execute (e.g., opponent fainted, only self-targeting moves allowed)
    NoPPRemaining,
    PokemonFainted, // When the acting Pokemon or target is fainted
    MoveFailedToExecute,
}

#[derive(Debug, Clone)]
pub struct EventBus {
    events: Vec<BattleEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn push(&mut self, event: BattleEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[BattleEvent] {
        &self.events
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

#[derive(Debug, Clone)]
pub struct TurnRng {
    outcomes: Vec<u8>,
    index: usize,
}

impl TurnRng {
    pub fn new_for_test(outcomes: Vec<u8>) -> Self {
        Self { outcomes, index: 0 }
    }

    pub fn new_random() -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        // Pre-generate a reasonable number of random values for a turn
        let outcomes: Vec<u8> = (0..100).map(|_| rng.random_range(1..=100)).collect();
        Self { outcomes, index: 0 }
    }

    pub fn next_outcome(&mut self) -> u8 {
        if self.index >= self.outcomes.len() {
            panic!("TurnRng exhausted! Need more random values for this turn.");
        }
        let outcome = self.outcomes[self.index];
        self.index += 1;
        outcome
    }

    pub fn peek_outcome(&self) -> u8 {
        if self.index >= self.outcomes.len() {
            panic!("TurnRng exhausted! Need more random values for this turn.");
        }
        self.outcomes[self.index]
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BattleState {
    pub battle_id: String,
    pub players: [BattlePlayer; 2],
    pub turn_number: u32,
    pub game_state: GameState,
    pub action_queue: [Option<PlayerAction>; 2],
}

impl BattleState {
    pub fn new(id: String, player1: BattlePlayer, player2: BattlePlayer) -> Self {
        Self {
            battle_id: id,
            players: [player1, player2],
            turn_number: 1,
            game_state: GameState::WaitingForBothActions,
            action_queue: [None, None],
        }
    }
}
