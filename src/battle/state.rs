use crate::battle::conditions::PokemonCondition;
use crate::moves::Move;
use crate::player::{BattlePlayer, PlayerAction, StatType, TeamCondition};
use crate::species::Species;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Copy)]
pub enum GameState {
    WaitingForActions,
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
    TeamConditionExpired {
        player_index: usize,
        condition: TeamCondition,
    },

    // Stat Changes
    StatStageChanged {
        target: Species,
        stat: StatType,
        old_stage: i8,
        new_stage: i8,
    },
    StatChangeBlocked {
        target: Species,
        stat: StatType,
        reason: String,
    },

    // Action Failures
    ActionFailed {
        reason: ActionFailureReason,
    },

    // Money/Ante
    AnteIncreased {
        player_index: usize,
        amount: u32,
        new_total: u32,
    },

    // Battle End
    PlayerDefeated {
        player_index: usize,
    },
    BattleEnded {
        winner: Option<usize>,
    },
}

impl BattleEvent {
    /// Formats the event into a human-readable string using battle context.
    #[allow(dead_code)]
    pub fn format(&self, battle_state: &BattleState) -> String {
        match self {
            BattleEvent::TurnStarted { turn_number } => {
                format!("=== Turn {} ===", turn_number)
            }
            BattleEvent::TurnEnded => {
                "Turn ended.".to_string()
            }
            BattleEvent::PokemonSwitched { player_index, old_pokemon, new_pokemon } => {
                let player_name = &battle_state.players[*player_index].player_name;
                format!("{} recalled {} and sent out {}!", player_name, 
                        self.format_species_name(*old_pokemon), self.format_species_name(*new_pokemon))
            }
            BattleEvent::MoveUsed { player_index, pokemon, move_used } => {
                let player_name = &battle_state.players[*player_index].player_name;
                let pokemon_name = self.format_species_name(*pokemon);
                format!("{}'s {} used {}!", player_name, pokemon_name, self.format_move_name(*move_used))
            }
            BattleEvent::MoveMissed { attacker, .. } => {
                let attacker_name = self.format_species_name(*attacker);
                format!("{}'s attack missed!", attacker_name)
            }
            BattleEvent::MoveHit { .. } => {
                // This event is often followed by damage/effectiveness, so a generic message is often not needed.
                // You could add one if you like, e.g., "The attack hit!".
                "".to_string()
            }
            BattleEvent::CriticalHit { .. } => {
                "A critical hit!".to_string()
            }
            BattleEvent::DamageDealt { target, damage, .. } => {
                let target_name = self.format_species_name(*target);
                format!("{} took {} damage!", target_name, damage)
            }
            BattleEvent::PokemonHealed { target, amount, .. } => {
                let target_name = self.format_species_name(*target);
                format!("{} recovered {} HP!", target_name, amount)
            }
            BattleEvent::PokemonFainted { pokemon, .. } => {
                let pokemon_name = self.format_species_name(*pokemon);
                format!("{} fainted!", pokemon_name)
            }
            BattleEvent::AttackTypeEffectiveness { multiplier } => {
                match *multiplier {
                    m if m > 1.0 => "It's super effective!".to_string(),
                    m if m < 1.0 && m > 0.0 => "It's not very effective...".to_string(),
                    0.0 => "It had no effect!".to_string(),
                    _ => "".to_string(), // Normal effectiveness, no message
                }
            }
            BattleEvent::StatusApplied { target, status } => {
                let target_name = self.format_species_name(*target);
                format!("{} was affected by {}!", target_name, self.format_condition(status))
            }
            BattleEvent::StatusRemoved { target, status } => {
                let target_name = self.format_species_name(*target);
                format!("{} is no longer affected by {}!", target_name, self.format_condition(status))
            }
            BattleEvent::StatusDamage { target, status, damage } => {
                let target_name = self.format_species_name(*target);
                let condition_name = self.format_condition(status);
                format!("{} is hurt by {}! ({} damage)", target_name, condition_name, damage)
            }
            BattleEvent::PokemonStatusApplied { target, status } => {
                let target_name = self.format_species_name(*target);
                format!("{} {}", target_name, self.format_pokemon_status_applied(status))
            }
            BattleEvent::PokemonStatusRemoved { target, status } => {
                let target_name = self.format_species_name(*target);
                format!("{} {}", target_name, self.format_pokemon_status_removed(status))
            }
            BattleEvent::PokemonStatusDamage { target, status, damage, .. } => {
                let target_name = self.format_species_name(*target);
                let status_name = self.format_pokemon_status(status);
                format!("{} is hurt by its {}! ({} damage)", target_name, status_name, damage)
            }
            BattleEvent::ConditionExpired { target, condition } => {
                let target_name = self.format_species_name(*target);
                let condition_name = self.format_condition(condition);
                format!("{}'s {} wore off.", target_name, condition_name)
            }
            BattleEvent::TeamConditionExpired { player_index, condition } => {
                let player_name = &battle_state.players[*player_index].player_name;
                format!("{}'s {} wore off.", player_name, condition)
            }
            BattleEvent::StatStageChanged { target, stat, new_stage, .. } => {
                let target_name = self.format_species_name(*target);
                let stat_name = self.format_stat_type(stat);
                if *new_stage > 6 || *new_stage < -6 { // This indicates a reset to 0 from Haze
                     format!("All stat changes were eliminated!")
                } else if *new_stage > 0 {
                    format!("{}'s {} rose!", target_name, stat_name)
                } else {
                    format!("{}'s {} fell!", target_name, stat_name)
                }
            }
            BattleEvent::StatChangeBlocked { target, .. } => {
                let target_name = self.format_species_name(*target);
                format!("{}'s stats won't go any higher!", target_name)
            }
            BattleEvent::ActionFailed { reason } => {
                self.format_action_failure_reason(reason)
            }
            BattleEvent::AnteIncreased { amount, .. } => {
                format!("Gained ${} from Pay Day!", amount)
            }
            BattleEvent::PlayerDefeated { player_index } => {
                let player_name = &battle_state.players[*player_index].player_name;
                format!("{} is out of usable Pokémon!", player_name)
            }
            BattleEvent::BattleEnded { winner } => {
                match winner {
                    Some(index) => format!("{} has won the battle!", battle_state.players[*index].player_name),
                    None => "The battle ended in a draw!".to_string(),
                }
            }
        }
    }

    // --- Private Helper Functions ---

    fn format_species_name(&self, species: Species) -> String {
        species.name().to_string()
    }
    
    fn format_move_name(&self, move_used: Move) -> String {
        // This could be expanded to format names like "Double-Edge"
        format!("{:?}", move_used)
    }
    
    fn format_condition(&self, condition: &PokemonCondition) -> String {
        // This can be expanded to be more descriptive
        format!("{:?}", condition.get_type())
    }
    
    fn format_pokemon_status(&self, status: &crate::pokemon::StatusCondition) -> String {
        match status {
            crate::pokemon::StatusCondition::Sleep(_) => "sleep".to_string(),
            crate::pokemon::StatusCondition::Poison(_) => "poison".to_string(),
            crate::pokemon::StatusCondition::Burn => "burn".to_string(),
            crate::pokemon::StatusCondition::Freeze => "freeze".to_string(),
            crate::pokemon::StatusCondition::Paralysis => "paralysis".to_string(),
            crate::pokemon::StatusCondition::Faint => "faint".to_string(),
        }
    }

    fn format_pokemon_status_applied(&self, status: &crate::pokemon::StatusCondition) -> String {
        match status {
            crate::pokemon::StatusCondition::Sleep(_) => "fell asleep!".to_string(),
            crate::pokemon::StatusCondition::Poison(_) => "was poisoned!".to_string(),
            crate::pokemon::StatusCondition::Burn => "was burned!".to_string(),
            crate::pokemon::StatusCondition::Freeze => "was frozen solid!".to_string(),
            crate::pokemon::StatusCondition::Paralysis => "is paralyzed! It may be unable to move!".to_string(),
            crate::pokemon::StatusCondition::Faint => "fainted!".to_string(),
        }
    }

    fn format_pokemon_status_removed(&self, status: &crate::pokemon::StatusCondition) -> String {
        match status {
            crate::pokemon::StatusCondition::Sleep(_) => "woke up!".to_string(),
            _ => format!("was cured of its {}!", self.format_pokemon_status(status)),
        }
    }
    
    fn format_stat_type(&self, stat: &StatType) -> String {
        match stat {
            StatType::Attack => "Attack".to_string(),
            StatType::Defense => "Defense".to_string(),
            StatType::Speed => "Speed".to_string(),
            StatType::SpecialAttack => "Special Attack".to_string(),
            StatType::SpecialDefense => "Special Defense".to_string(),
            StatType::Accuracy => "accuracy".to_string(),
            StatType::Evasion => "evasiveness".to_string(),
            StatType::Focus => "critical hit ratio".to_string(),
        }
    }
    
    fn format_action_failure_reason(&self, reason: &ActionFailureReason) -> String {
        match reason {
            ActionFailureReason::IsAsleep => "is fast asleep.".to_string(),
            ActionFailureReason::IsFrozen => "is frozen solid!".to_string(),
            ActionFailureReason::IsExhausted => "must recharge!".to_string(),
            ActionFailureReason::IsParalyzed => "is fully paralyzed!".to_string(),
            ActionFailureReason::IsFlinching => "flinched and couldn't move!".to_string(),
            ActionFailureReason::IsConfused => "is confused!".to_string(),
            ActionFailureReason::IsTrapped => "can't escape!".to_string(),
            _ => "But it failed!".to_string(),
        }
    }
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

    pub fn next_outcome(&mut self, reason: &str) -> u8 {
        if self.index >= self.outcomes.len() {
            // Add the reason to the panic message for better debugging!
            panic!(
                "TurnRng exhausted! Tried to get a value for: '{}'. Need more random values.",
                reason
            );
        }
        let outcome = self.outcomes[self.index];

        // The magic line: Print the consumption event to the console during tests.
        #[cfg(test)]
        println!("[RNG] Consumed {} for: {}", outcome, reason);

        self.index += 1;
        outcome
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
            game_state: GameState::WaitingForActions,
            action_queue: [None, None],
        }
    }
}
