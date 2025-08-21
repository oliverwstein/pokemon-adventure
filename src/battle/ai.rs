//! A module for defining AI behaviors for battle opponents.

use crate::battle::state::BattleState;
use crate::errors::BattleResult;
use crate::move_data::{MoveCategory, MoveData};
use crate::player::PlayerAction;

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
    fn score_action(&self, action: &PlayerAction, player_index: usize, state: &BattleState) -> f32 {
        let opponent_index = 1 - player_index;

        match action {
            PlayerAction::UseMove { move_index } => {
                self.score_move(*move_index, player_index, opponent_index, state).unwrap_or(0.0)
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
    ) -> BattleResult<f32> {
        let player = &state.players[player_index];
        let opponent = &state.players[opponent_index];
        let attacker = player.active_pokemon().unwrap();
        let defender = match opponent.active_pokemon() {
            Some(p) => p,
            None => return Ok(0.0), // Cannot score if there is no target.
        };

        let move_instance = attacker.moves[move_index].as_ref().unwrap();
        let move_data = MoveData::get_move_data(move_instance.move_)?;

        // --- Step 1: Calculate the Core Damage Score ---
        // This score is based on the move's potential to deal direct damage.
        // For non-damaging moves, this starts at 0.
        let mut damage_score = 0.0;
        if matches!(
            move_data.category,
            MoveCategory::Physical | MoveCategory::Special
        ) {
            // Start with the move's base power.
            let base_power = move_data.power.unwrap_or(0) as f32;

            // Factor in Type Effectiveness. This is the most critical multiplier.
            let defender_types = defender.get_current_types(opponent);
            let effectiveness =
                crate::battle::stats::get_type_effectiveness(move_data.move_type, &defender_types)
                    as f32;

            // If the opponent is immune, this is a terrible move.
            if effectiveness < 0.1 {
                return Ok(-1.0);
            }

            // Factor in STAB (Same-Type Attack Bonus).
            let attacker_types = attacker.get_current_types(player);
            let stab_multiplier = if attacker_types.contains(&move_data.move_type) {
                1.5
            } else {
                1.0
            };

            // Factor in the attacker's normalized effective power.
            let effective_stat =
                crate::battle::stats::effective_attack(attacker, player, move_instance.move_).unwrap_or(0);
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
                    if *target == crate::move_data::Target::User && *stages > 0 =>
                {
                    let current_stage = player.get_stat_stage(stat_type.clone().into());
                    if current_stage < 6 {
                        let potential_gain = 1.0 - (current_stage as f32 / 6.0); // Value diminishes as stat rises
                        utility_score +=
                            20.0 * (*stages as f32) * potential_gain * (*chance as f32 / 100.0);
                    }
                }
                // Opponent debuffs are valuable if the stat isn't minimized.
                crate::move_data::MoveEffect::StatChange(target, stat_type, stages, chance)
                    if *target == crate::move_data::Target::Target && *stages < 0 =>
                {
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
            return Ok(-1.0);
        }

        // Factor in accuracy for any move that targets the opponent.
        if move_data.category != MoveCategory::Status {
            let accuracy = move_data.accuracy.unwrap_or(101); // Give a slight edge to sure-hit moves
            final_score *= accuracy as f32 / 100.0;
        }

        // Add a small random factor to break ties and prevent repetitive loops.
        let random_factor = 1.0 + (rand::random::<f32>() * 0.1 - 0.05); // +/- 5%
        final_score *= random_factor;

        Ok(final_score)
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
        let player = &battle_state.players[player_index];

        // --- Phase 1: Handle Forced Replacements ---
        // If the game state requires a replacement, the only valid actions are switches.
        // The AI must choose the best Pokémon to send in.
        let is_replacement_phase = match battle_state.game_state {
            crate::battle::state::GameState::WaitingForPlayer1Replacement => player_index == 0,
            crate::battle::state::GameState::WaitingForPlayer2Replacement => player_index == 1,
            crate::battle::state::GameState::WaitingForBothReplacements => true,
            _ => false,
        };

        if is_replacement_phase {
            let valid_switches = player.get_valid_switches();

            // If there are no valid switches, the player has lost. Forfeit is the only option.
            if valid_switches.is_empty() {
                return PlayerAction::Forfeit;
            }

            // Score only the switch actions and pick the best one.
            return valid_switches
                .into_iter()
                .max_by_key(|action| {
                    let score = self.score_action(action, player_index, battle_state);
                    ordered_float::OrderedFloat(score)
                })
                .unwrap_or(PlayerAction::Forfeit); // Failsafe
        }

        // --- Phase 2: Standard Turn Strategic Decision ---
        // The AI must now decide between attacking and switching.

        // 2a. Get and score all possible moves.
        let valid_moves = player.get_valid_moves();
        let best_move = valid_moves
            .into_iter()
            .map(|action| {
                let score = self.score_action(&action, player_index, battle_state);
                (action, score)
            })
            .max_by_key(|(_, score)| ordered_float::OrderedFloat(*score));

        // 2b. Get and score all possible switches.
        let valid_switches = player.get_valid_switches();
        let best_switch = valid_switches
            .into_iter()
            .map(|action| {
                let score = self.score_action(&action, player_index, battle_state);
                (action, score)
            })
            .max_by_key(|(_, score)| ordered_float::OrderedFloat(*score));

        // 2c. Compare the best options.
        match (best_move, best_switch) {
            // If both attacking and switching are possible...
            (Some((move_action, move_score)), Some((switch_action, switch_score))) => {
                // ...compare their scores to make the strategic choice.
                // You could add a bias here, e.g., `if switch_score > move_score + 10.0`,
                // to make the AI less likely to switch frivolously.
                if switch_score > move_score {
                    switch_action
                } else {
                    move_action
                }
            }
            // If only attacking is possible...
            (Some((move_action, _)), None) => move_action,
            // If only switching is possible...
            (None, Some((switch_action, _))) => switch_action,
            // If no moves or switches are valid, the only option is to forfeit.
            (None, None) => PlayerAction::Forfeit,
        }
    }
}
