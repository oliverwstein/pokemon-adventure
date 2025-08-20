use super::*;

impl MoveEffect {
    /// Apply heal effect (targets user)
    pub(super) fn apply_heal_effect(
        &self,
        percentage: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        let attacker_player = &state.players[context.attacker_index];
        if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
            let max_hp = attacker_pokemon.max_hp();
            let current_hp = attacker_pokemon.current_hp();

            // Don't heal if already at full HP or fainted
            if current_hp > 0 && current_hp < max_hp {
                let heal_amount = (max_hp * (percentage as u16)) / 100;
                if heal_amount > 0 {
                    commands.push(BattleCommand::HealPokemon {
                        target: PlayerTarget::from_index(context.attacker_index),
                        amount: heal_amount,
                    });
                }
            }
        }

        commands
    }

    /// Apply ante effect (Pay Day)
    pub(super) fn apply_ante_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::state::BattleEvent;

        let mut commands = Vec::new();

        if rng.next_outcome("Apply Ante Check") <= chance {
            let attacker_player = &state.players[context.attacker_index];
            if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
                let pokemon_level = attacker_pokemon.level as u32;
                let ante_amount = pokemon_level * 2;

                // We need the defender's current ante to create the event correctly
                let defender_player = &state.players[context.defender_index];
                let new_total = defender_player.get_ante() + ante_amount;

                // Command to add the ante
                commands.push(BattleCommand::AddAnte {
                    target: PlayerTarget::from_index(context.defender_index),
                    amount: ante_amount,
                });

                // Command to emit the event
                // TODO: The AddAnte BattleCommand should automatically emit this.
                commands.push(BattleCommand::EmitEvent(BattleEvent::AnteIncreased {
                    player_index: context.defender_index,
                    amount: ante_amount,
                    new_total,
                }));
            }
        }

        commands
    }

    /// If this effect is MultiHit, calculates if another hit should be queued and returns the command.
    /// This function assumes preconditions (like the defender not fainting) have already been checked by the caller.
    pub fn apply_multi_hit_continuation(
        &self,
        context: &EffectContext,
        rng: &mut crate::battle::state::TurnRng,
        hit_number: u8,
    ) -> Option<crate::battle::commands::BattleCommand> {
        use crate::battle::action_stack::BattleAction;
        use crate::battle::commands::BattleCommand;

        // This logic only applies if the effect is actually a MultiHit variant.
        if let MoveEffect::MultiHit(guaranteed_hits, continuation_chance) = self {
            let next_hit_number = hit_number + 1;

            // Determine if the next hit should be queued.
            let should_queue_next_hit = if next_hit_number <= *guaranteed_hits {
                // We are still within the guaranteed number of hits.
                true
            } else {
                // Past the guaranteed hits, so roll for continuation.
                // Assuming a max of 7 hits for any sequence.
                // This is a change from a max of 5 because we allow each hit to miss indepedently
                next_hit_number <= 7
                    && rng.next_outcome("Multi-Hit Continuation Check") <= *continuation_chance
            };

            if should_queue_next_hit {
                // Return a command to push the next hit onto the action stack.
                return Some(BattleCommand::PushAction(BattleAction::AttackHit {
                    attacker_index: context.attacker_index,
                    defender_index: context.defender_index,
                    move_used: context.move_used,
                    hit_number: next_hit_number,
                }));
            }
        }

        // If not a multi-hit effect or if the continuation roll fails, return None.
        None
    }

    /// Apply damage-based effects that require the damage amount
    pub fn apply_damage_based_effects(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        damage_dealt: u16,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        let mut commands = Vec::new();

        // Only process if damage was actually dealt
        if damage_dealt == 0 {
            return commands;
        }

        match self {
            MoveEffect::Recoil(percentage) => {
                commands.extend(self.apply_recoil_effect(*percentage, context, damage_dealt));
            }
            MoveEffect::Drain(percentage) => {
                commands.extend(self.apply_drain_effect(*percentage, context, state, damage_dealt));
            }
            _ => {
                // Not a damage-based effect
            }
        }

        commands
    }

    /// Apply recoil effect (attacker takes damage)
    fn apply_recoil_effect(
        &self,
        percentage: u8,
        context: &EffectContext,
        damage_dealt: u16,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        let recoil_damage = (damage_dealt * (percentage as u16)) / 100;
        if recoil_damage > 0 {
            commands.push(BattleCommand::DealDamage {
                target: PlayerTarget::from_index(context.attacker_index),
                amount: recoil_damage,
            });
        }

        commands
    }

    /// Apply drain effect (attacker heals based on damage)
    fn apply_drain_effect(
        &self,
        percentage: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        damage_dealt: u16,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        let heal_amount = (damage_dealt * (percentage as u16)) / 100;
        if heal_amount > 0 {
            let attacker_player = &state.players[context.attacker_index];
            if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
                let current_hp = attacker_pokemon.current_hp();
                let max_hp = attacker_pokemon.max_hp();

                // Only heal if not at full HP or fainted
                if current_hp > 0 && current_hp < max_hp {
                    commands.push(BattleCommand::HealPokemon {
                        target: PlayerTarget::from_index(context.attacker_index),
                        amount: heal_amount,
                    });
                }
            }
        }

        commands
    }

    /// Apply miss-based effects that trigger when a move misses
    pub fn apply_miss_based_effects(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        let mut commands = Vec::new();

        match self {
            MoveEffect::Reckless(percentage) => {
                commands.extend(self.apply_reckless_effect(*percentage, context, state));
            }
            _ => {
                // Not a miss-based effect
            }
        }

        commands
    }

    /// Apply reckless effect (attacker takes damage based on max HP when move misses)
    fn apply_reckless_effect(
        &self,
        percentage: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        let attacker_player = &state.players[context.attacker_index];
        if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
            let max_hp = attacker_pokemon.max_hp();
            let recoil_damage = (max_hp * (percentage as u16)) / 100;

            if recoil_damage > 0 {
                commands.push(BattleCommand::DealDamage {
                    target: PlayerTarget::from_index(context.attacker_index),
                    amount: recoil_damage,
                });
            }
        }

        commands
    }
}

impl MoveData {
    /// Apply all damage-based effects for this move
    pub fn apply_damage_based_effects(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        damage_dealt: u16,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        let mut all_commands = Vec::new();

        for effect in &self.effects {
            let effect_commands = effect.apply_damage_based_effects(context, state, damage_dealt);
            all_commands.extend(effect_commands);
        }

        all_commands
    }

    /// Apply all miss-based effects for this move
    pub fn apply_miss_based_effects(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        let mut all_commands = Vec::new();

        for effect in &self.effects {
            let effect_commands = effect.apply_miss_based_effects(context, state);
            all_commands.extend(effect_commands);
        }

        all_commands
    }
}
