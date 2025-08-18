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
    TeamConditionApplied {
        player_index: usize,
        condition: TeamCondition,
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
    /// Returns None for silent events that should not produce user-visible text.
    pub fn format(&self, battle_state: &BattleState) -> Option<String> {
        match self {
            // === Turn Management Events ===
            BattleEvent::TurnStarted { turn_number } => {
                Some(format!("=== Turn {} ===", turn_number))
            }
            BattleEvent::TurnEnded => {
                None // Silent - turn ending is usually obvious from context
            }

            // === Pokemon Switching Events ===
            BattleEvent::PokemonSwitched { player_index, old_pokemon, new_pokemon } => {
                let player_name = &battle_state.players[*player_index].player_name;
                Some(format!("{} recalled {} and sent out {}!", 
                    player_name, 
                    Self::format_species_name(*old_pokemon), 
                    Self::format_species_name(*new_pokemon)
                ))
            }

            // === Move Events ===
            BattleEvent::MoveUsed { player_index, pokemon, move_used } => {
                let player_name = &battle_state.players[*player_index].player_name;
                let pokemon_name = Self::format_species_name(*pokemon);
                Some(format!("{}'s {} used {}!", 
                    player_name, pokemon_name, Self::format_move_name(*move_used)
                ))
            }
            BattleEvent::MoveMissed { attacker, .. } => {
                let attacker_name = Self::format_species_name(*attacker);
                Some(format!("{}'s attack missed!", attacker_name))
            }
            BattleEvent::MoveHit { .. } => {
                None // Silent - hit is usually obvious from damage/effects
            }
            BattleEvent::CriticalHit { .. } => {
                Some("A critical hit!".to_string())
            }

            // === Damage and Healing Events ===
            BattleEvent::DamageDealt { target, damage, .. } => {
                let target_name = Self::format_species_name(*target);
                Some(format!("{} took {} damage!", target_name, damage))
            }
            BattleEvent::PokemonHealed { target, amount, .. } => {
                let target_name = Self::format_species_name(*target);
                Some(format!("{} recovered {} HP!", target_name, amount))
            }
            BattleEvent::PokemonFainted { pokemon, .. } => {
                let pokemon_name = Self::format_species_name(*pokemon);
                Some(format!("{} fainted!", pokemon_name))
            }

            // === Type Effectiveness Events ===
            BattleEvent::AttackTypeEffectiveness { multiplier } => {
                match *multiplier {
                    m if m > 1.0 => Some("It's super effective!".to_string()),
                    m if m < 1.0 && m > 0.0 => Some("It's not very effective...".to_string()),
                    0.0 => Some("It had no effect!".to_string()),
                    _ => None, // Normal effectiveness, no message
                }
            }

            // === Condition Events ===
            BattleEvent::StatusApplied { target, status } => {
                let target_name = Self::format_species_name(*target);
                Some(format!("{} was affected by {}!", 
                    target_name, Self::format_condition(status)
                ))
            }
            BattleEvent::StatusRemoved { target, status } => {
                let target_name = Self::format_species_name(*target);
                Some(format!("{} is no longer affected by {}!", 
                    target_name, Self::format_condition(status)
                ))
            }
            BattleEvent::StatusDamage { target, status, damage } => {
                let target_name = Self::format_species_name(*target);
                let condition_name = Self::format_condition(status);
                Some(format!("{} is hurt by {}! ({} damage)", 
                    target_name, condition_name, damage
                ))
            }
            BattleEvent::ConditionExpired { target, condition } => {
                let target_name = Self::format_species_name(*target);
                let condition_name = Self::format_condition(condition);
                Some(format!("{}'s {} wore off.", target_name, condition_name))
            }

            // === Pokemon Status Events ===
            BattleEvent::PokemonStatusApplied { target, status } => {
                let target_name = Self::format_species_name(*target);
                Some(format!("{} {}", target_name, Self::format_pokemon_status_applied(status)))
            }
            BattleEvent::PokemonStatusRemoved { target, status } => {
                let target_name = Self::format_species_name(*target);
                Some(format!("{} {}", target_name, Self::format_pokemon_status_removed(status)))
            }
            BattleEvent::PokemonStatusDamage { target, status, damage, .. } => {
                let target_name = Self::format_species_name(*target);
                let status_name = Self::format_pokemon_status(status);
                Some(format!("{} is hurt by its {}! ({} damage)", 
                    target_name, status_name, damage
                ))
            }

            // === Team Condition Events ===
            BattleEvent::TeamConditionApplied { player_index, condition } => {
                let player_name = &battle_state.players[*player_index].player_name;
                Some(format!("{}'s {} is now in effect!", player_name, condition))
            }
            BattleEvent::TeamConditionExpired { player_index, condition } => {
                let player_name = &battle_state.players[*player_index].player_name;
                Some(format!("{}'s {} wore off.", player_name, condition))
            }

            // === Stat Change Events ===
            BattleEvent::StatStageChanged { target, stat, new_stage, .. } => {
                let target_name = Self::format_species_name(*target);
                let stat_name = Self::format_stat_type(stat);
                if *new_stage > 6 || *new_stage < -6 { // This indicates a reset to 0 from Haze
                    Some("All stat changes were eliminated!".to_string())
                } else if *new_stage > 0 {
                    Some(format!("{}'s {} rose!", target_name, stat_name))
                } else {
                    Some(format!("{}'s {} fell!", target_name, stat_name))
                }
            }
            BattleEvent::StatChangeBlocked { target, .. } => {
                let target_name = Self::format_species_name(*target);
                Some(format!("{}'s stats won't go any higher!", target_name))
            }

            // === Action Failure Events ===
            BattleEvent::ActionFailed { reason } => {
                Some(Self::format_action_failure_reason(reason))
            }

            // === Battle Economy Events ===
            BattleEvent::AnteIncreased { amount, .. } => {
                Some(format!("Gained ${} from Pay Day!", amount))
            }

            // === Battle End Events ===
            BattleEvent::PlayerDefeated { player_index } => {
                let player_name = &battle_state.players[*player_index].player_name;
                Some(format!("{} is out of usable Pokémon!", player_name))
            }
            BattleEvent::BattleEnded { winner } => {
                match winner {
                    Some(index) => Some(format!("{} has won the battle!", battle_state.players[*index].player_name)),
                    None => Some("The battle ended in a draw!".to_string()),
                }
            }
        }
    }

    // --- Private Helper Functions ---

    fn format_species_name(species: Species) -> String {
        species.name().to_string()
    }
    
    fn format_move_name(move_used: Move) -> String {
        // Convert CamelCase enum variants to human-readable names
        match move_used {
            Move::DoubleEdge => "Double-Edge".to_string(),
            Move::SolarBeam => "Solar Beam".to_string(),
            Move::ThunderWave => "Thunder Wave".to_string(),
            Move::SwordsDance => "Swords Dance".to_string(),
            Move::SelfDestruct => "Self-Destruct".to_string(),
            Move::ViceGrip => "Vice Grip".to_string(),
            Move::PayDay => "Pay Day".to_string(),
            _ => {
                // Convert other moves from CamelCase to Title Case
                let debug_string = format!("{:?}", move_used);
                debug_string.chars()
                    .enumerate()
                    .map(|(i, c)| {
                        if i > 0 && c.is_uppercase() {
                            format!(" {}", c)
                        } else {
                            c.to_string()
                        }
                    })
                    .collect()
            }
        }
    }
    
    fn format_condition(condition: &PokemonCondition) -> String {
        // Convert condition types to human-readable names
        match condition.get_type() {
            crate::battle::conditions::PokemonConditionType::Confused => "confusion".to_string(),
            crate::battle::conditions::PokemonConditionType::Exhausted => "exhaustion".to_string(),
            crate::battle::conditions::PokemonConditionType::Trapped => "trapping".to_string(),
            crate::battle::conditions::PokemonConditionType::Flinched => "flinching".to_string(),
            crate::battle::conditions::PokemonConditionType::Rampaging => "rampage".to_string(),
            crate::battle::conditions::PokemonConditionType::Disabled => "disable".to_string(),
            crate::battle::conditions::PokemonConditionType::Biding => "bide".to_string(),
            crate::battle::conditions::PokemonConditionType::Teleported => "teleportation".to_string(),
            crate::battle::conditions::PokemonConditionType::Countering => "counter stance".to_string(),
            crate::battle::conditions::PokemonConditionType::Charging => "charging".to_string(),
            crate::battle::conditions::PokemonConditionType::Underground => "underground".to_string(),
            crate::battle::conditions::PokemonConditionType::InAir => "in air".to_string(),
            crate::battle::conditions::PokemonConditionType::Substitute => "substitute".to_string(),
            crate::battle::conditions::PokemonConditionType::Seeded => "leech seed".to_string(),
            crate::battle::conditions::PokemonConditionType::Converted => "type conversion".to_string(),
            crate::battle::conditions::PokemonConditionType::Transformed => "transformation".to_string(),
            crate::battle::conditions::PokemonConditionType::Enraged => "rage".to_string(),
        }
    }
    
    fn format_pokemon_status(status: &crate::pokemon::StatusCondition) -> String {
        match status {
            crate::pokemon::StatusCondition::Sleep(_) => "sleep".to_string(),
            crate::pokemon::StatusCondition::Poison(_) => "poison".to_string(),
            crate::pokemon::StatusCondition::Burn => "burn".to_string(),
            crate::pokemon::StatusCondition::Freeze => "freeze".to_string(),
            crate::pokemon::StatusCondition::Paralysis => "paralysis".to_string(),
            crate::pokemon::StatusCondition::Faint => "faint".to_string(),
        }
    }

    fn format_pokemon_status_applied(status: &crate::pokemon::StatusCondition) -> String {
        match status {
            crate::pokemon::StatusCondition::Sleep(_) => "fell asleep!".to_string(),
            crate::pokemon::StatusCondition::Poison(_) => "was poisoned!".to_string(),
            crate::pokemon::StatusCondition::Burn => "was burned!".to_string(),
            crate::pokemon::StatusCondition::Freeze => "was frozen solid!".to_string(),
            crate::pokemon::StatusCondition::Paralysis => "is paralyzed! It may be unable to move!".to_string(),
            crate::pokemon::StatusCondition::Faint => "fainted!".to_string(),
        }
    }

    fn format_pokemon_status_removed(status: &crate::pokemon::StatusCondition) -> String {
        match status {
            crate::pokemon::StatusCondition::Sleep(_) => "woke up!".to_string(),
            _ => format!("was cured of its {}!", Self::format_pokemon_status(status)),
        }
    }
    
    fn format_stat_type(stat: &StatType) -> String {
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
    
    fn format_action_failure_reason(reason: &ActionFailureReason) -> String {
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

#[cfg(test)]
mod event_formatting_tests {
    use super::*;
    use crate::species::Species;
    use crate::moves::Move;
    use crate::player::BattlePlayer;
    use crate::pokemon::{PokemonInst, StatusCondition, get_species_data};

    fn create_test_battle_state() -> BattleState {
        let pikachu_data = get_species_data(Species::Pikachu).expect("Failed to load Pikachu data");
        let charmander_data = get_species_data(Species::Charmander).expect("Failed to load Charmander data");

        let pikachu = PokemonInst::new(Species::Pikachu, &pikachu_data, 25, None, None);
        let charmander = PokemonInst::new(Species::Charmander, &charmander_data, 25, None, None);

        let player1 = BattlePlayer::new("p1".to_string(), "Player 1".to_string(), vec![pikachu]);
        let player2 = BattlePlayer::new("p2".to_string(), "Player 2".to_string(), vec![charmander]);

        BattleState {
            battle_id: "test".to_string(),
            players: [player1, player2],
            turn_number: 1,
            game_state: GameState::TurnInProgress,
            action_queue: [None, None],
        }
    }

    #[test]
    fn test_silent_events_return_none() {
        let battle_state = create_test_battle_state();
        
        // These events should be silent (return None)
        let silent_events = vec![
            BattleEvent::TurnEnded,
            BattleEvent::MoveHit { 
                attacker: Species::Pikachu, 
                defender: Species::Charmander, 
                move_used: Move::Tackle 
            },
            BattleEvent::AttackTypeEffectiveness { multiplier: 1.0 }, // Normal effectiveness
        ];

        for event in silent_events {
            assert!(event.format(&battle_state).is_none(), 
                "Event {:?} should be silent but returned text", event);
        }
    }

    #[test] 
    fn test_formatted_events_return_some() {
        let battle_state = create_test_battle_state();

        let formatted_events = vec![
            BattleEvent::TurnStarted { turn_number: 1 },
            BattleEvent::CriticalHit { 
                attacker: Species::Pikachu, 
                defender: Species::Charmander, 
                move_used: Move::Tackle 
            },
            BattleEvent::AttackTypeEffectiveness { multiplier: 2.0 }, // Super effective
            BattleEvent::PokemonFainted { player_index: 0, pokemon: Species::Pikachu },
        ];

        for event in formatted_events {
            assert!(event.format(&battle_state).is_some(),
                "Event {:?} should return formatted text but returned None", event);
        }
    }

    #[test]
    fn test_move_name_formatting() {
        // Test that move names are properly formatted with spaces and hyphens
        assert_eq!(BattleEvent::format_move_name(Move::ViceGrip), "Vice Grip");
        assert_eq!(BattleEvent::format_move_name(Move::SolarBeam), "Solar Beam");  
        assert_eq!(BattleEvent::format_move_name(Move::ThunderWave), "Thunder Wave");
        assert_eq!(BattleEvent::format_move_name(Move::SelfDestruct), "Self-Destruct");
        
        // Test CamelCase conversion for unlisted moves
        assert_eq!(BattleEvent::format_move_name(Move::Tackle), "Tackle");
        assert_eq!(BattleEvent::format_move_name(Move::QuickAttack), "Quick Attack");
    }

    #[test]
    fn test_status_condition_formatting() {
        // Test Pokemon status formatting
        assert_eq!(BattleEvent::format_pokemon_status_applied(&StatusCondition::Sleep(3)), "fell asleep!");
        assert_eq!(BattleEvent::format_pokemon_status_applied(&StatusCondition::Poison(0)), "was poisoned!");
        assert_eq!(BattleEvent::format_pokemon_status_applied(&StatusCondition::Paralysis), "is paralyzed! It may be unable to move!");
        
        assert_eq!(BattleEvent::format_pokemon_status_removed(&StatusCondition::Sleep(0)), "woke up!");
        assert_eq!(BattleEvent::format_pokemon_status_removed(&StatusCondition::Burn), "was cured of its burn!");
    }

    #[test]
    fn test_event_text_samples() {
        let battle_state = create_test_battle_state();

        // Test a few specific event text outputs
        let turn_event = BattleEvent::TurnStarted { turn_number: 5 };
        assert_eq!(turn_event.format(&battle_state), Some("=== Turn 5 ===".to_string()));

        let crit_event = BattleEvent::CriticalHit { 
            attacker: Species::Pikachu, 
            defender: Species::Charmander, 
            move_used: Move::Tackle 
        };
        assert_eq!(crit_event.format(&battle_state), Some("A critical hit!".to_string()));

        let effectiveness_event = BattleEvent::AttackTypeEffectiveness { multiplier: 0.5 };
        assert_eq!(effectiveness_event.format(&battle_state), Some("It's not very effective...".to_string()));

        let no_effect_event = BattleEvent::AttackTypeEffectiveness { multiplier: 0.0 };
        assert_eq!(no_effect_event.format(&battle_state), Some("It had no effect!".to_string()));
    }

    #[test]
    fn test_event_bus_printing_methods() {
        let mut event_bus = EventBus::new();
        let battle_state = create_test_battle_state();

        // Add some sample events
        event_bus.push(BattleEvent::TurnStarted { turn_number: 1 });
        event_bus.push(BattleEvent::MoveHit { 
            attacker: Species::Pikachu, 
            defender: Species::Charmander, 
            move_used: Move::Tackle 
        });
        event_bus.push(BattleEvent::CriticalHit { 
            attacker: Species::Pikachu, 
            defender: Species::Charmander, 
            move_used: Move::Tackle 
        });

        // Test basic properties
        assert!(!event_bus.is_empty());
        assert_eq!(event_bus.len(), 3);

        // Test printing methods (these would normally print to stdout, but we can't easily capture that in a test)
        // These calls should not panic and should work correctly
        event_bus.print_debug();
        event_bus.print_debug_with_message("Test message:");
        event_bus.print_formatted(&battle_state);
        event_bus.print_formatted_with_message("Formatted test:", &battle_state);

        // Test Display trait
        let display_output = format!("{}", event_bus);
        assert!(display_output.contains("TurnStarted"));
        assert!(display_output.contains("MoveHit"));
        assert!(display_output.contains("CriticalHit"));
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

/// Event bus for collecting and managing battle events.
/// 
/// ## Usage Examples
/// 
/// ```rust,ignore
/// // Basic debug printing (old way)
/// for event in event_bus.events() {
///     println!("  {:?}", event);
/// }
/// 
/// // New convenient methods
/// event_bus.print_debug();                                    // Just print events
/// event_bus.print_debug_with_message("Turn 1 events:");      // With header message
/// event_bus.print_formatted(&battle_state);                  // Human-readable format
/// event_bus.print_formatted_with_message("Battle log:", &battle_state);  // With header
/// 
/// // Using Display trait
/// println!("{}", event_bus);                                  // Print all events
/// ```
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

    /// Print all events in debug format with indentation.
    /// This replaces the common pattern of manually iterating and printing events.
    pub fn print_debug(&self) {
        for event in &self.events {
            println!("  {:?}", event);
        }
    }

    /// Print all events in debug format with a custom prefix message.
    pub fn print_debug_with_message(&self, message: &str) {
        println!("{}", message);
        self.print_debug();
    }

    /// Print all events using their formatted text (when available) along with battle context.
    /// Falls back to debug format for silent events.
    pub fn print_formatted(&self, battle_state: &BattleState) {
        for event in &self.events {
            match event.format(battle_state) {
                Some(formatted) => println!("  {}", formatted),
                None => println!("  {:?} (silent)", event),
            }
        }
    }

    /// Print all events using their formatted text with a custom prefix message.
    pub fn print_formatted_with_message(&self, message: &str, battle_state: &BattleState) {
        println!("{}", message);
        self.print_formatted(battle_state);
    }

    /// Return true if the event bus contains no events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Return the number of events in the bus.
    pub fn len(&self) -> usize {
        self.events.len()
    }
}

impl std::fmt::Display for EventBus {
    /// Format the EventBus for printing. Shows debug format of all events.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for event in &self.events {
            writeln!(f, "  {:?}", event)?;
        }
        Ok(())
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
