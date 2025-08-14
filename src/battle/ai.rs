//! A module for defining AI behaviors for battle opponents.

use ordered_float::OrderedFloat;
use rand::seq::IndexedRandom;

use crate::battle::state::BattleState;
use crate::battle::turn_orchestrator::get_valid_actions;
use crate::move_data::{get_move_data, MoveCategory};
use crate::player::{PlayerAction};

/// A trait for any system that can decide on a battle action.
/// This provides a common interface for different AI difficulties or strategies.
pub trait Behavior {
    /// Inspects the battle state and decides on the next action for the given player.
    fn decide_action(&self, player_index: usize, battle_state: &BattleState) -> PlayerAction;
}

pub struct ScoringAI;

impl ScoringAI {
    pub fn new() -> Self {
        Self
    }

    /// The core scoring logic. Assigns a floating-point value to a given action.
    fn score_action(
        &self,
        action: &PlayerAction,
        player_index: usize,
        state: &BattleState,
    ) -> f32 {
        let opponent_index = 1 - player_index;

        match action {
            PlayerAction::UseMove { move_index } => {
                self.score_move(*move_index, player_index, opponent_index, state)
            }
            PlayerAction::SwitchPokemon { team_index } => {
                self.score_switch(*team_index, player_index, opponent_index, state)
            }
            PlayerAction::Forfeit => -1000.0, // Never choose to forfeit unless it's the only option.
        }
    }

    fn score_move(
        &self,
        move_index: usize,
        player_index: usize,
        opponent_index: usize,
        state: &BattleState,
    ) -> f32 {
        let player = &state.players[player_index];
        let opponent = &state.players[opponent_index];
        let attacker = player.active_pokemon().unwrap();
        let defender = match opponent.active_pokemon() {
            Some(p) => p,
            None => return 0.0, // Cannot score if there is no target.
        };

        let move_instance = attacker.moves[move_index].as_ref().unwrap();
        let move_data = get_move_data(move_instance.move_).unwrap();

        // --- Step 1: Calculate the Core Damage Score ---
        // This score is based on the move's potential to deal direct damage.
        // For non-damaging moves, this starts at 0.
        let mut damage_score = 0.0;
        if matches!(move_data.category, MoveCategory::Physical | MoveCategory::Special) {
            // Start with the move's base power.
            let base_power = move_data.power.unwrap_or(0) as f32;

            // Factor in Type Effectiveness. This is the most critical multiplier.
            let defender_types = defender.get_current_types(opponent);
            let effectiveness =
                crate::battle::stats::get_type_effectiveness(move_data.move_type, &defender_types) as f32;
            
            // If the opponent is immune, this is a terrible move.
            if effectiveness < 0.1 {
                return -1.0;
            }

            // Factor in STAB (Same-Type Attack Bonus).
            let attacker_types = attacker.get_current_types(player);
            let stab_multiplier = if attacker_types.contains(&move_data.move_type) { 1.5 } else { 1.0 };
            
            // Factor in the attacker's normalized effective power.
            let effective_stat = crate::battle::stats::effective_attack(attacker, player, move_instance.move_);
            let level_scalar = (attacker.level as f32 * 2.0).max(1.0);
            let normalized_power = effective_stat as f32 / level_scalar;

            damage_score = base_power * effectiveness * stab_multiplier * normalized_power;
        }

        // --- Step 2: Calculate the Utility Score ---
        // This score is based on beneficial secondary effects, regardless of move category.
        let mut utility_score = 0.0;
        for effect in &move_data.effects {
            match effect {
                // Self-buffs are valuable if the stat isn't maxed out.
                crate::move_data::MoveEffect::StatChange(target, stat_type, stages, chance) 
                if *target == crate::move_data::Target::User && *stages > 0 => {
                    let current_stage = player.get_stat_stage(stat_type.clone().into());
                    if current_stage < 6 {
                        let potential_gain = 1.0 - (current_stage as f32 / 6.0); // Value diminishes as stat rises
                        utility_score += 20.0 * (*stages as f32) * potential_gain * (*chance as f32 / 100.0);
                    }
                }
                // Opponent debuffs are valuable if the stat isn't minimized.
                crate::move_data::MoveEffect::StatChange(target, stat_type, stages, chance) 
                if *target == crate::move_data::Target::Target && *stages < 0 => {
                    let opponent_stage = opponent.get_stat_stage(stat_type.clone().into());
                    if opponent_stage > -6 {
                        utility_score += 15.0 * (stages.abs() as f32) * (*chance as f32 / 100.0);
                    }
                }
                // Inflicting a status is very valuable, but only if the opponent is healthy.
                crate::move_data::MoveEffect::Sedate(chance) 
                | crate::move_data::MoveEffect::Paralyze(chance) 
                | crate::move_data::MoveEffect::Poison(chance) 
                | crate::move_data::MoveEffect::Burn(chance) 
                | crate::move_data::MoveEffect::Freeze(chance) => {
                    if defender.status.is_none() {
                        utility_score += 45.0 * (*chance as f32 / 100.0);
                    }
                }
                // Flinching is also good.
                crate::move_data::MoveEffect::Flinch(chance) => {
                    utility_score += 30.0 * (*chance as f32 / 100.0);
                }
                _ => {} // Other effects can be added here (e.g., Heal, LeechSeed)
            }
        }

        // --- Step 3: Combine Scores and Apply Final Modifiers ---
        let mut final_score = damage_score + utility_score;

        // Don't use a Status move if it has no utility (e.g., trying to boost a maxed stat).
        if move_data.category == MoveCategory::Status && utility_score < 1.0 {
            return -1.0;
        }

        // Factor in accuracy for any move that targets the opponent.
        if move_data.category != MoveCategory::Status {
            let accuracy = move_data.accuracy.unwrap_or(101); // Give a slight edge to sure-hit moves
            final_score *= accuracy as f32 / 100.0;
        }

        // Add a small random factor to break ties and prevent repetitive loops.
        let random_factor = 1.0 + (rand::random::<f32>() * 0.1 - 0.05); // +/- 5%
        final_score *= random_factor;

        final_score
    }
    
    fn score_switch(
        &self,
        _team_index: usize,
        _player_index: usize,
        _opponent_index: usize,
        _state: &BattleState,
    ) -> f32 {
        // A small, positive baseline score. It's better than doing nothing or using
        // a move that's immune, but worse than almost any decent damaging move.
        let base_score = 1.0;

        // Add a tiny random value to break ties if multiple switch options exist.
        // This ensures that if the AI decides to switch, it won't always pick the
        // first Pokémon in its party list.
        let random_tiebreaker = rand::random::<f32>() * 0.1; // A value between 0.0 and 0.1

        base_score + random_tiebreaker
    }
}

impl Behavior for ScoringAI {
    fn decide_action(&self, player_index: usize, battle_state: &BattleState) -> PlayerAction {
        // 1. Get the pre-validated list of all possible actions.
        // This call is now simpler as it doesn't return a Result.
        let valid_actions = get_valid_actions(battle_state, player_index);
        // Failsafe: If the list is empty, it means the player has lost (e.g., must switch
        // but has no valid Pokémon). The only logical action is to do nothing, which the
        // engine will interpret correctly. Forfeiting is a good, explicit choice here.
        if valid_actions.is_empty() {
            return PlayerAction::Forfeit;
        }

        // If there's only one choice, just take it.
        if valid_actions.len() == 1 {
            // .clone() is necessary because `valid_actions` owns the data.
            return valid_actions[0].clone();
        }

        // 2. Score each action.
        let scored_actions: Vec<(PlayerAction, f32)> = valid_actions
            .into_iter()
            .map(|action| {
                let score = self.score_action(&action, player_index, battle_state);
                (action, score)
            })
            .collect();

        // 3. Find the maximum score among all actions.
        let max_score = scored_actions
            .iter()
            .map(|(_, score)| OrderedFloat(*score))
            .max()
            .unwrap() // Safe because we checked for an empty list.
            .0;

        // 4. Filter for all actions that have a score close to the maximum.
        let best_actions: Vec<PlayerAction> = scored_actions
            .into_iter()
            .filter(|(_, score)| (*score - max_score).abs() < 0.01) // Check for floating point equality
            .map(|(action, _)| action)
            .collect();

        // 5. Choose one of the best actions randomly to prevent predictability.
        let mut rng = rand::rng();
        best_actions.choose(&mut rng).unwrap().clone()
    }

}