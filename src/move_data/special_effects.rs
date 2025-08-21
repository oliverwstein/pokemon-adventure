use super::*;

impl MoveEffect {
    /// Apply InAir effect (Fly move pattern)
    pub(super) fn apply_in_air_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::{PokemonCondition, PokemonConditionType};

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        // If already in air, this is the second turn - clear condition and proceed with normal attack
        if attacker_player.has_condition_type(PokemonConditionType::InAir) {
            let commands = vec![BattleCommand::RemoveCondition {
                target: attacker_target,
                condition_type: PokemonConditionType::InAir,
            }];
            return EffectResult::Continue(commands);
        }

        // First turn - apply condition and skip normal attack
        if let Some(_) = attacker_player.active_pokemon() {
            let condition = PokemonCondition::InAir;
            let commands = vec![BattleCommand::AddCondition {
                target: attacker_target,
                condition: condition.clone(),
            }];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Teleport effect
    pub(super) fn apply_teleport_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;

        let attacker_player = &state.players[context.attacker_index];
        if let Some(_) = attacker_player.active_pokemon() {
            let condition = PokemonCondition::Teleported;
            let commands = vec![BattleCommand::AddCondition {
                target: PlayerTarget::from_index(context.attacker_index),
                condition: condition.clone(),
            }];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply ChargeUp effect (Solar Beam pattern)
    pub(super) fn apply_charge_up_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::{PokemonCondition, PokemonConditionType};

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        // If already charging, this is the second turn - clear condition and proceed with normal attack
        if attacker_player.has_condition_type(PokemonConditionType::Charging) {
            let commands = vec![BattleCommand::RemoveCondition {
                target: attacker_target,
                condition_type: PokemonConditionType::Charging,
            }];
            return EffectResult::Continue(commands);
        }

        // First turn - apply condition and skip normal attack
        if let Some(_) = attacker_player.active_pokemon() {
            let condition = PokemonCondition::Charging;
            let commands = vec![BattleCommand::AddCondition {
                target: attacker_target,
                condition: condition.clone(),
            }];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Underground effect (Dig move pattern)
    pub(super) fn apply_underground_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::{PokemonCondition, PokemonConditionType};

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        // If already underground, this is the second turn - clear condition and proceed with normal attack
        if attacker_player.has_condition_type(PokemonConditionType::Underground) {
            let commands = vec![BattleCommand::RemoveCondition {
                target: attacker_target,
                condition_type: PokemonConditionType::Underground,
            }];
            return EffectResult::Continue(commands);
        }

        // First turn - apply condition and skip normal attack
        if let Some(_) = attacker_player.active_pokemon() {
            let condition = PokemonCondition::Underground;
            let commands = vec![BattleCommand::AddCondition {
                target: attacker_target,
                condition: condition.clone(),
            }];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Transform effect
    pub(super) fn apply_transform_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;

        let attacker_player = &state.players[context.attacker_index];
        let defender_player = &state.players[context.defender_index];

        if let (Some(_), Some(target_pokemon)) = (
            attacker_player.active_pokemon(),
            defender_player.active_pokemon().cloned(),
        ) {
            let condition = PokemonCondition::Transformed {
                target: target_pokemon,
            };
            let commands = vec![BattleCommand::AddCondition {
                target: PlayerTarget::from_index(context.attacker_index),
                condition: condition.clone(),
            }];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Conversion effect
    pub(super) fn apply_conversion_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;

        let attacker_player = &state.players[context.attacker_index];
        let defender_player = &state.players[context.defender_index];

        if let (Some(_), Some(target_type)) = (
            attacker_player.active_pokemon(),
            defender_player
                .active_pokemon()
                .map(|target_pokemon| target_pokemon.get_current_types(defender_player))
                .and_then(|types| types.into_iter().next()), // Take first type
        ) {
            let condition = PokemonCondition::Converted {
                pokemon_type: target_type,
            };
            let commands = vec![BattleCommand::AddCondition {
                target: PlayerTarget::from_index(context.attacker_index),
                condition: condition.clone(),
            }];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Substitute effect
    pub(super) fn apply_substitute_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;

        let attacker_player = &state.players[context.attacker_index];
        if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
            // Substitute uses 25% of max HP
            let substitute_hp = (attacker_pokemon.max_hp() / 4).max(1) as u8;
            let condition = PokemonCondition::Substitute { hp: substitute_hp };

            let commands = vec![BattleCommand::AddCondition {
                target: PlayerTarget::from_index(context.attacker_index),
                condition: condition.clone(),
            }];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Counter effect
    pub(super) fn apply_counter_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;

        let attacker_player = &state.players[context.attacker_index];
        if let Some(_) = attacker_player.active_pokemon() {
            let condition = PokemonCondition::Countering { damage: 0 };
            let commands = vec![BattleCommand::AddCondition {
                target: PlayerTarget::from_index(context.attacker_index),
                condition: condition.clone(),
            }];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Rampage effect
    pub(super) fn apply_rampage_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        if let Some(_) = attacker_player.active_pokemon() {
            // Check if Pokemon is already rampaging
            if let Some(PokemonCondition::Rampaging { turns_remaining }) = attacker_player
                .active_pokemon_conditions
                .values()
                .find(|c| matches!(c, PokemonCondition::Rampaging { .. }))
            {
                if *turns_remaining > 0 {
                    // Still rampaging, don't apply again, just continue with attack
                    return EffectResult::Continue(Vec::new());
                } else {
                    // Rampage ending, apply confusion
                    let confusion_condition = PokemonCondition::Confused { turns_remaining: 2 };
                    let commands = vec![BattleCommand::AddCondition {
                        target: attacker_target,
                        condition: confusion_condition.clone(),
                    }];
                    return EffectResult::Continue(commands);
                }
            }

            // Not rampaging yet, apply the condition
            let turns = if rng.next_outcome("Generate Rampage Duration") <= 50 {
                1
            } else {
                2
            };
            let condition = PokemonCondition::Rampaging {
                turns_remaining: turns,
            };
            let commands = vec![BattleCommand::AddCondition {
                target: attacker_target,
                condition: condition.clone(),
            }];
            return EffectResult::Continue(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Rage effect
    pub(super) fn apply_rage_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;

        let attacker_player = &state.players[context.attacker_index];
        if let Some(_) = attacker_player.active_pokemon() {
            let condition = PokemonCondition::Enraged;
            let commands = vec![BattleCommand::AddCondition {
                target: PlayerTarget::from_index(context.attacker_index),
                condition: condition.clone(),
            }];
            return EffectResult::Continue(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Explode effect
    pub(super) fn apply_explode_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let attacker_player = &state.players[context.attacker_index];
        if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
            let commands = vec![BattleCommand::DealDamage {
                target: PlayerTarget::from_index(context.attacker_index),
                amount: attacker_pokemon.current_hp(),
            }];
            return EffectResult::Continue(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Bide effect (complex state machine)
    pub(super) fn apply_bide_special(
        &self,
        turns: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};
        use crate::battle::conditions::PokemonCondition;

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);
        let defender_target = PlayerTarget::from_index(context.defender_index);

        if let Some((turns_remaining, stored_damage)) = attacker_player
            .active_pokemon_conditions
            .values()
            .find_map(|c| match c {
                PokemonCondition::Biding {
                    turns_remaining,
                    damage,
                } => Some((turns_remaining, damage)),
                _ => None,
            })
        {
            if *turns_remaining < 1 {
                // Last turn of Bide - execute stored damage
                let damage_to_deal = (*stored_damage * 2).max(1);
                let commands = vec![BattleCommand::DealDamage {
                    target: defender_target,
                    amount: damage_to_deal,
                }];
                return EffectResult::Skip(commands);
            } else {
                // Still Biding, skip turn
                return EffectResult::Skip(Vec::new());
            }
        } else {
            // Start a new Bide
            if attacker_player.active_pokemon().is_some() {
                let condition = PokemonCondition::Biding {
                    turns_remaining: turns,
                    damage: 0,
                };
                let commands = vec![BattleCommand::AddCondition {
                    target: attacker_target,
                    condition: condition.clone(),
                }];
                return EffectResult::Skip(commands);
            }
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply Rest effect (heal, clear conditions, apply sleep)
    pub(super) fn apply_rest_special(
        &self,
        sleep_turns: u8,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        let attacker_player = &state.players[context.attacker_index];
        let attacker_target = PlayerTarget::from_index(context.attacker_index);

        if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
            let max_hp = attacker_pokemon.max_hp();
            let current_hp = attacker_pokemon.current_hp();
            let mut commands = Vec::new();

            if current_hp < max_hp {
                commands.push(BattleCommand::HealPokemon {
                    target: attacker_target,
                    amount: max_hp - current_hp,
                });
            }

            // Cure any existing status first, then apply Sleep
            if let Some(existing_status) = attacker_pokemon.status {
                commands.push(BattleCommand::CurePokemonStatus {
                    target: attacker_target,
                    status: existing_status,
                });
            }

            commands.push(BattleCommand::SetPokemonStatus {
                target: attacker_target,
                status: crate::pokemon::StatusCondition::Sleep(sleep_turns),
            });

            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply MirrorMove effect (mirrors opponent's last move)
    pub(super) fn apply_mirror_move_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
    ) -> EffectResult {
        use crate::battle::action_stack::BattleAction;
        use crate::battle::commands::BattleCommand;
        use crate::battle::state::{ActionFailureReason, BattleEvent};

        let defender_player = &state.players[context.defender_index];
        if let Some(mirrored_move) = defender_player.last_move {
            if mirrored_move == crate::moves::Move::MirrorMove {
                let commands = vec![BattleCommand::EmitEvent(BattleEvent::ActionFailed {
                    reason: ActionFailureReason::MoveFailedToExecute,
                })];
                return EffectResult::Skip(commands);
            }

            let attacker_player = &state.players[context.attacker_index];
            let move_used_event = if let Some(attacker_pokemon) = attacker_player.active_pokemon() {
                BattleCommand::EmitEvent(BattleEvent::MoveUsed {
                    player_index: context.attacker_index,
                    pokemon: attacker_pokemon.species,
                    move_used: mirrored_move,
                })
            } else {
                BattleCommand::EmitEvent(BattleEvent::ActionFailed {
                    reason: ActionFailureReason::PokemonFainted,
                })
            };

            let mirrored_action = BattleAction::AttackHit {
                attacker_index: context.attacker_index,
                defender_index: context.defender_index,
                move_used: mirrored_move,
                hit_number: 1, // Must be >0 to avoid using PP
            };

            return EffectResult::Skip(vec![
                move_used_event,
                BattleCommand::PushAction(mirrored_action),
            ]);
        }

        EffectResult::Skip(vec![BattleCommand::EmitEvent(BattleEvent::ActionFailed {
            reason: ActionFailureReason::MoveFailedToExecute,
        })])
    }

    /// Apply Metronome effect (randomly selects and executes a move)
    pub(super) fn apply_metronome_special(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> EffectResult {
        use crate::battle::action_stack::BattleAction;
        use crate::battle::commands::BattleCommand;
        use crate::battle::state::BattleEvent;
        use crate::moves::Move;

        let all_moves: &[Move] = &[
            Move::Pound,
            Move::Doubleslap,
            Move::PayDay,
            Move::Scratch,
            Move::Guillotine,
            Move::SwordsDance,
            Move::Cut,
            Move::Bind,
            Move::Slam,
            Move::Stomp,
            Move::Headbutt,
            Move::HornAttack,
            Move::FuryAttack,
            Move::HornDrill,
            Move::Tackle,
            Move::BodySlam,
            Move::Wrap,
            Move::Harden,
            Move::TakeDown,
            Move::Thrash,
            Move::DoubleEdge,
            Move::TailWhip,
            Move::Leer,
            Move::Bite,
            Move::Growl,
            Move::Roar,
            Move::Sing,
            Move::Supersonic,
            Move::SonicBoom,
            Move::Disable,
            Move::Agility,
            Move::QuickAttack,
            Move::Rage,
            Move::Mimic,
            Move::Screech,
            Move::DoubleTeam,
            Move::Recover,
            Move::Minimize,
            Move::Withdraw,
            Move::DefenseCurl,
            Move::Barrier,
            Move::FocusEnergy,
            Move::Bide,
            Move::MirrorMove,
            Move::SelfDestruct,
            Move::Clamp,
            Move::Swift,
            Move::SpikeCannon,
            Move::Constrict,
            Move::SoftBoiled,
            Move::Glare,
            Move::Transform,
            Move::Explosion,
            Move::FurySwipes,
            Move::Rest,
            Move::HyperFang,
            Move::Sharpen,
            Move::Conversion,
            Move::TriAttack,
            Move::SuperFang,
            Move::Slash,
            Move::Substitute,
            Move::HyperBeam,
            Move::KarateChop,
            Move::CometPunch,
            Move::MegaPunch,
            Move::KOPunch,
            Move::DoubleKick,
            Move::MegaKick,
            Move::JumpKick,
            Move::RollingKick,
            Move::Submission,
            Move::LowKick,
            Move::Counter,
            Move::SeismicToss,
            Move::Strength,
            Move::Meditate,
            Move::HighJumpKick,
            Move::Barrage,
            Move::DizzyPunch,
            Move::RazorWind,
            Move::Gust,
            Move::WingAttack,
            Move::Whirlwind,
            Move::Fly,
            Move::Peck,
            Move::DrillPeck,
            Move::SkyAttack,
            Move::ViceGrip,
            Move::RockThrow,
            Move::SkullBash,
            Move::RockSlide,
            Move::AncientPower,
            Move::SandAttack,
            Move::Earthquake,
            Move::Fissure,
            Move::Dig,
            Move::BoneClub,
            Move::Bonemerang,
            Move::PoisonSting,
            Move::Twineedle,
            Move::Acid,
            Move::Toxic,
            Move::Haze,
            Move::Smog,
            Move::Sludge,
            Move::PoisonJab,
            Move::PoisonGas,
            Move::AcidArmor,
            Move::PinMissile,
            Move::SilverWind,
            Move::StringShot,
            Move::LeechLife,
            Move::FirePunch,
            Move::BlazeKick,
            Move::FireFang,
            Move::Ember,
            Move::Flamethrower,
            Move::WillOWisp,
            Move::FireSpin,
            Move::Smokescreen,
            Move::FireBlast,
            Move::Mist,
            Move::WaterGun,
            Move::HydroPump,
            Move::Surf,
            Move::Bubblebeam,
            Move::Waterfall,
            Move::Bubble,
            Move::Splash,
            Move::Bubblehammer,
            Move::VineWhip,
            Move::Absorb,
            Move::MegaDrain,
            Move::GigaDrain,
            Move::LeechSeed,
            Move::Growth,
            Move::RazorLeaf,
            Move::SolarBeam,
            Move::PoisonPowder,
            Move::StunSpore,
            Move::SleepPowder,
            Move::PetalDance,
            Move::Spore,
            Move::EggBomb,
            Move::IcePunch,
            Move::IceBeam,
            Move::Blizzard,
            Move::AuroraBeam,
            Move::ThunderPunch,
            Move::Shock,
            Move::Discharge,
            Move::ThunderWave,
            Move::Thunderclap,
            Move::ChargeBeam,
            Move::Lightning,
            Move::Flash,
            Move::Confusion,
            Move::Psybeam,
            Move::Perplex,
            Move::Hypnosis,
            Move::Teleport,
            Move::ConfuseRay,
            Move::LightScreen,
            Move::Reflect,
            Move::Amnesia,
            Move::Kinesis,
            Move::Psywave,
            Move::DreamEater,
            Move::LovelyKiss,
            Move::NightShade,
            Move::Lick,
            Move::ShadowBall,
            Move::Outrage,
            Move::DragonRage,
        ];

        let random_index =
            (rng.next_outcome("Generate Metronome Move Select") as usize) % all_moves.len();
        let selected_move = all_moves[random_index];

        let attacker_player = &state.players[context.attacker_index];
        if let Some(pokemon_species) = attacker_player.active_pokemon().map(|p| p.species) {
            let metronome_action = BattleAction::AttackHit {
                attacker_index: context.attacker_index,
                defender_index: context.defender_index,
                move_used: selected_move,
                hit_number: 1, // Must be >0 to avoid using PP
            };

            let commands = vec![
                BattleCommand::EmitEvent(BattleEvent::MoveUsed {
                    player_index: context.attacker_index,
                    pokemon: pokemon_species,
                    move_used: selected_move,
                }),
                BattleCommand::PushAction(metronome_action),
            ];
            return EffectResult::Skip(commands);
        }

        EffectResult::Continue(Vec::new())
    }

    /// Apply team condition effect
    pub(super) fn apply_team_condition_effect(
        &self,
        condition: &crate::player::TeamCondition,
        turns: u8,
        context: &EffectContext,
    ) -> Vec<crate::battle::commands::BattleCommand> {
        use crate::battle::commands::{BattleCommand, PlayerTarget};

        vec![BattleCommand::AddTeamCondition {
            target: PlayerTarget::from_index(context.attacker_index),
            condition: *condition,
            turns,
        }]
    }
}
