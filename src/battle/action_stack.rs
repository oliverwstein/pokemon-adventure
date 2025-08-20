use std::collections::VecDeque;

use crate::battle::state::BattleState;
use crate::player::PlayerAction;
use crate::{Move, battle::stats::effective_speed, move_data::MoveData};
/// Internal action types for the action stack
/// These represent atomic actions that can be executed during battle resolution
#[derive(Debug, Clone)]
pub enum BattleAction {
    /// Player forfeits the battle
    Forfeit { player_index: usize },

    /// Player switches to a different Pokemon
    Switch {
        player_index: usize,
        target_pokemon_index: usize,
    },

    /// Execute a single hit of a move (for multi-hit moves, multiple actions are pushed)
    AttackHit {
        attacker_index: usize,
        defender_index: usize,
        move_used: Move,
        hit_number: u8, // 0 for single hit, 0,1,2... for multi-hit
    },
}

pub struct ActionStack {
    actions: VecDeque<BattleAction>,
}

// A helper struct local to this implementation detail.
#[derive(Debug, Clone)]
struct ActionPriority {
    action_priority: i8, // Forfeit: 10, Switch: 6, Move: 0
    move_priority: i8,   // Priority from move data (e.g., Quick Attack)
    speed: u16,          // Pokémon's effective speed for tiebreaking
}

impl ActionStack {
    /// Creates a new, empty ActionStack.
    pub fn new() -> Self {
        Self {
            actions: VecDeque::new(),
        }
    }

    /// Builds the initial action stack for a turn based on the queued actions in the BattleState.
    /// This is the primary "smart constructor" for creating an ordered list of turn actions.
    /// It consumes the state of the `action_queue` and produces a ready-to-execute stack.
    pub fn build_initial(battle_state: &BattleState) -> Self {
        // 1. Collect the actions that have been submitted into the queue.
        // It's assumed the queue has been pre-filled by player input, AI, and/or
        // the "End-of-Turn Injection" of forced moves.
        let actions_to_prioritize: Vec<(usize, PlayerAction)> = battle_state
            .action_queue
            .iter()
            .enumerate()
            .filter_map(|(index, action_opt)| {
                action_opt.as_ref().map(|action| (index, action.clone()))
            })
            .collect();

        // 2. Determine the execution order based on game rules (priority, speed).
        // We call our own private helper function for this, keeping the logic encapsulated.
        let action_order = Self::determine_action_order(battle_state, &actions_to_prioritize);

        // 3. Convert the sorted PlayerActions into executable BattleActions and build the stack.
        let mut new_stack = Self::new();
        for (player_index, player_action) in action_order {
            let battle_action = Self::convert_player_action_to_battle_action(
                &player_action,
                player_index,
                battle_state,
            );
            new_stack.push_back(battle_action);
        }

        new_stack
    }

    /// Adds an action to the end of the execution queue.
    pub fn push_back(&mut self, action: BattleAction) {
        self.actions.push_back(action);
    }

    /// Adds an action to the front of the execution queue, to be executed next.
    /// Used for dynamically injected actions like multi-hits or confusion self-damage.
    pub fn push_front(&mut self, action: BattleAction) {
        self.actions.push_front(action);
    }

    /// Removes and returns the next action to be executed from the front of the queue.
    pub fn pop_front(&mut self) -> Option<BattleAction> {
        self.actions.pop_front()
    }

    // --- Private Helper Functions ---
    // These functions are implementation details of `build_initial`.

    /// A private helper that sorts a list of player actions based on priority and speed.
    fn determine_action_order<'a>(
        battle_state: &'a BattleState,
        actions: &'a [(usize, PlayerAction)],
    ) -> Vec<(usize, PlayerAction)> {
        let mut player_priorities = Vec::new();

        // Calculate priority for each player's action.
        for (player_index, action) in actions {
            let priority = Self::calculate_action_priority(*player_index, action, battle_state);
            player_priorities.push((*player_index, action.clone(), priority));
        }

        // Sort by action priority (highest first), then move priority, then speed.
        player_priorities.sort_by(|a, b| {
            let priority_cmp = b.2.action_priority.cmp(&a.2.action_priority);
            if priority_cmp != std::cmp::Ordering::Equal {
                return priority_cmp;
            }

            let move_priority_cmp = b.2.move_priority.cmp(&a.2.move_priority);
            if move_priority_cmp != std::cmp::Ordering::Equal {
                return move_priority_cmp;
            }

            b.2.speed.cmp(&a.2.speed)
        });

        // Return just the sorted (player_index, PlayerAction) tuples.
        player_priorities
            .into_iter()
            .map(|(player_index, action, _)| (player_index, action))
            .collect()
    }

    /// A private helper to calculate the priority values for a single action.
    fn calculate_action_priority(
        player_index: usize,
        action: &PlayerAction,
        battle_state: &BattleState,
    ) -> ActionPriority {
        match action {
            PlayerAction::SwitchPokemon { .. } => {
                ActionPriority {
                    action_priority: 6,
                    move_priority: 0,
                    speed: player_index as u16, // Stable sort for dual switches
                }
            }
            PlayerAction::Forfeit => {
                ActionPriority {
                    action_priority: 10, // Forfeit is highest priority
                    move_priority: 0,
                    speed: 0,
                }
            }
            PlayerAction::UseMove { move_index } => {
                let player = &battle_state.players[player_index];
                let active_pokemon = player.active_pokemon().expect("Active pokemon must exist");

                let move_instance = active_pokemon.moves[*move_index]
                    .as_ref()
                    .expect("Move must exist in queue");

                let move_data =
                    MoveData::get_move_data(move_instance.move_).expect("Move data must exist");

                let speed = effective_speed(active_pokemon, player);

                let move_priority = move_data
                    .effects
                    .iter()
                    .find_map(|effect| match effect {
                        crate::move_data::MoveEffect::Priority(p) => Some(*p),
                        _ => None,
                    })
                    .unwrap_or(0);

                ActionPriority {
                    action_priority: 0, // Moves are lowest action priority
                    move_priority,
                    speed,
                }
            }
        }
    }

    /// A private helper to convert a PlayerAction into an executable BattleAction.
    fn convert_player_action_to_battle_action(
        player_action: &PlayerAction,
        player_index: usize,
        battle_state: &BattleState,
    ) -> BattleAction {
        match player_action {
            PlayerAction::Forfeit => BattleAction::Forfeit { player_index },
            PlayerAction::SwitchPokemon { team_index } => BattleAction::Switch {
                player_index,
                target_pokemon_index: *team_index,
            },
            PlayerAction::UseMove { move_index } => {
                let player = &battle_state.players[player_index];
                let active_pokemon = player
                    .active_pokemon()
                    .expect("Active pokemon should exist");

                // Determine if the move should become Struggle due to 0 PP.
                let final_move = active_pokemon.moves[*move_index]
                    .as_ref()
                    .map(|inst| {
                        if inst.pp > 0 {
                            inst.move_
                        } else {
                            Move::Struggle
                        }
                    })
                    .unwrap_or(Move::Struggle);

                BattleAction::AttackHit {
                    attacker_index: player_index,
                    defender_index: 1 - player_index,
                    move_used: final_move,
                    hit_number: 0,
                }
            }
        }
    }
}
