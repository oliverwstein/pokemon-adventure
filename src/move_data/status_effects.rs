use super::*;

impl MoveEffect {
    /// Apply burn status effect
    pub(super) fn apply_burn_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        let target_player = &state.players[context.defender_index];
        if let Some(target_pokemon) = target_player.active_pokemon() {
            // Fire-type Pokemon are immune to burn
            let target_types = target_pokemon.get_current_types(target_player);
            if target_types.contains(&crate::pokemon::PokemonType::Fire) {
                return commands; // No RNG check, no burn application
            }

            if rng.next_outcome("Apply Burn Check") <= chance {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: crate::pokemon::StatusCondition::Burn,
                    });
                    // Event will be automatically emitted by the command system
                }
            }
        }

        commands
    }

    /// Apply paralyze status effect  
    pub(super) fn apply_paralyze_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        let target_player = &state.players[context.defender_index];
        if let Some(target_pokemon) = target_player.active_pokemon() {
            // Electric-type Pokemon are immune to paralysis
            let target_types = target_pokemon.get_current_types(target_player);
            if target_types.contains(&crate::pokemon::PokemonType::Electric) {
                return commands; // No RNG check, no paralysis application
            }

            if rng.next_outcome("Apply Paralysis Check") <= chance {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: crate::pokemon::StatusCondition::Paralysis,
                    });
                    // Event will be automatically emitted by the command system
                }
            }
        }

        commands
    }

    /// Apply freeze status effect
    pub(super) fn apply_freeze_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        let target_player = &state.players[context.defender_index];
        if let Some(target_pokemon) = target_player.active_pokemon() {
            // Ice-type Pokemon are immune to freeze
            let target_types = target_pokemon.get_current_types(target_player);
            if target_types.contains(&crate::pokemon::PokemonType::Ice) {
                return commands; // No RNG check, no freeze application
            }

            if rng.next_outcome("Apply Freeze Check") <= chance {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: crate::pokemon::StatusCondition::Freeze,
                    });
                    // Event will be automatically emitted by the command system
                }
            }
        }

        commands
    }

    /// Apply poison status effect
    pub(super) fn apply_poison_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        let target_player = &state.players[context.defender_index];
        if let Some(target_pokemon) = target_player.active_pokemon() {
            // Poison-type Pokemon are immune to poison
            let target_types = target_pokemon.get_current_types(target_player);
            if target_types.contains(&crate::pokemon::PokemonType::Poison) {
                return commands; // No RNG check, no poison application
            }

            if rng.next_outcome("Apply Poison Check") <= chance {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: crate::pokemon::StatusCondition::Poison(0),
                    });
                    // Event will be automatically emitted by the command system
                }
            }
        }

        commands
    }

    /// Apply sedate (sleep) status effect
    pub(super) fn apply_sedate_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        let target_player = &state.players[context.defender_index];
        if let Some(target_pokemon) = target_player.active_pokemon() {
            // Ghost-type Pokemon are immune to sleep
            let target_types = target_pokemon.get_current_types(target_player);
            if target_types.contains(&crate::pokemon::PokemonType::Ghost) {
                return commands; // No RNG check, no sleep application
            }

            if rng.next_outcome("Apply Sedate Check") <= chance {
                // Only apply if Pokemon has no status
                if target_pokemon.status.is_none() {
                    // Sleep for 1-3 turns (random)
                    let sleep_turns = (rng.next_outcome("Generate Sleep Duration") % 3) + 1; // 1, 2, or 3 turns
                    let sleep_status = crate::pokemon::StatusCondition::Sleep(sleep_turns);

                    commands.push(BattleCommand::SetPokemonStatus {
                        target: PlayerTarget::from_index(context.defender_index),
                        status: sleep_status,
                    });
                    // Event will be automatically emitted by the command system
                }
            }
        }

        commands
    }

    /// Apply flinch condition effect
    pub(super) fn apply_flinch_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        if rng.next_outcome("Apply Flinch Effect") <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(_) = target_player.active_pokemon() {
                let condition = crate::battle::conditions::PokemonCondition::Flinched;

                commands.push(BattleCommand::AddCondition {
                    target: PlayerTarget::from_index(context.defender_index),
                    condition: condition.clone(),
                });
            }
        }

        commands
    }

    /// Apply confuse condition effect
    pub(super) fn apply_confuse_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        if rng.next_outcome("Apply Confuse Effect") <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(_) = target_player.active_pokemon() {
                // Confuse for 1-4 turns (random)
                let confuse_turns = (rng.next_outcome("Generate Confusion Duration") % 4) + 1;
                let condition = crate::battle::conditions::PokemonCondition::Confused {
                    turns_remaining: confuse_turns,
                };

                commands.push(BattleCommand::AddCondition {
                    target: PlayerTarget::from_index(context.defender_index),
                    condition: condition.clone(),
                });
            }
        }

        commands
    }

    /// Apply trap condition effect
    pub(super) fn apply_trap_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        if rng.next_outcome("Apply Trap Check") <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(_) = target_player.active_pokemon() {
                // Trap for 2-5 turns (random)
                let trap_turns = (rng.next_outcome("Generate Trap Duration") % 4) + 2;
                let condition = crate::battle::conditions::PokemonCondition::Trapped {
                    turns_remaining: trap_turns,
                };

                commands.push(BattleCommand::AddCondition {
                    target: PlayerTarget::from_index(context.defender_index),
                    condition: condition.clone(),
                });
            }
        }

        commands
    }

    /// Apply seed condition effect
    pub(super) fn apply_seed_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        if rng.next_outcome("Apply Seeded Effect") <= chance {
            let target_player = &state.players[context.defender_index];
            if let Some(_) = target_player.active_pokemon() {
                let condition = crate::battle::conditions::PokemonCondition::Seeded;

                commands.push(BattleCommand::AddCondition {
                    target: PlayerTarget::from_index(context.defender_index),
                    condition: condition.clone(),
                });
            }
        }

        commands
    }

    /// Apply exhaust condition effect (targets user, not opponent)
    pub(super) fn apply_exhaust_effect(
        &self,
        chance: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        if rng.next_outcome("Apply Exhaust Check") <= chance {
            let attacker_player = &state.players[context.attacker_index];
            if let Some(_) = attacker_player.active_pokemon() {
                let condition = crate::battle::conditions::PokemonCondition::Exhausted {
                    turns_remaining: 2, // Decremented same turn, so start at 2
                };

                commands.push(BattleCommand::AddCondition {
                    target: PlayerTarget::from_index(context.attacker_index),
                    condition: condition.clone(),
                });
            }
        }

        commands
    }

    /// Apply cure status effect
    pub(super) fn apply_cure_status_effect(
        &self,
        target: &Target,
        status_type: &StatusType,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let mut commands = Vec::new();

        let target_index = context.target_index(target);
        let target_player = &state.players[target_index];

        if let Some(target_pokemon) = target_player.active_pokemon() {
            // Check if the Pokemon has the status condition we want to cure
            let status_to_cure = match (&target_pokemon.status, status_type) {
                (Some(status @ crate::pokemon::StatusCondition::Sleep(_)), StatusType::Sleep) => Some(*status),
                (Some(status @ crate::pokemon::StatusCondition::Poison(_)), StatusType::Poison) => Some(*status),
                (Some(status @ crate::pokemon::StatusCondition::Burn), StatusType::Burn) => Some(*status),
                (Some(status @ crate::pokemon::StatusCondition::Freeze), StatusType::Freeze) => Some(*status),
                (Some(status @ crate::pokemon::StatusCondition::Paralysis), StatusType::Paralysis) => Some(*status),
                _ => None,
            };

            if let Some(status) = status_to_cure {
                commands.push(BattleCommand::CurePokemonStatus {
                    target: PlayerTarget::from_index(target_index),
                    status,
                });
            }
        }

        commands
    }
}