use crate::battle::conditions::PokemonCondition;
use crate::errors::{MoveDataError, MoveDataResult};
use crate::moves::Move;
use crate::pokemon::PokemonType;
use serde::{Deserialize, Serialize};

// helper modules
mod damage_effects;
mod format_description;
mod special_effects;
mod stat_effects;
mod status_effects;
// Include the compiled move data
include!(concat!(env!("OUT_DIR"), "/generated_data.rs"));

// Re-export move-related types from the schema crate
pub use pokemon_adventure_schema::{MoveCategory, StatType, StatusType, Target};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MoveEffect {
    // Basic effects
    Flinch(u8),   // chance %
    Burn(u8),     // chance %
    Freeze(u8),   // chance %
    Paralyze(u8), // chance %
    Poison(u8),   // chance %
    Sedate(u8),   // chance % (sleep)
    Confuse(u8),  // chance %

    // Stat changes
    StatChange(Target, StatType, i8, u8), // target, stat, stages, chance %
    RaiseAllStats(u8),                    // chance %

    // Damage modifiers
    Recoil(u8),     // % of damage dealt
    Drain(u8),      // % of damage healed
    Crit(u8),       // increased crit ratio
    IgnoreDef(u8),  // chance % to ignore defense
    SuperFang(u8),  // chance % to halve HP
    SetDamage(u16), // fixed damage
    LevelDamage,    // damage = user level

    // Multi-hit
    MultiHit(u8, u8), // min hits, % chance of continuation

    // Status and conditions
    Trap(u8),     // chance % to trap
    Exhaust(u8),  // chance % to exhaust (skip next turn)
    Priority(i8), // move priority modifier
    ChargeUp,     // charge for 1 turn
    InAir,        // go in air (avoid ground moves)
    Underground,  // go underground
    Teleport(u8), // chance % to teleport away

    // Special mechanics
    OHKO,         // one-hit KO
    Explode,      // user faints
    Reckless(u8), // recoil if miss, chance %
    Transform,    // copy target's appearance/stats
    Conversion,   // change user's type
    Disable(u8),  // disable target's last move, chance %
    Counter,      // return double physical damage
    MirrorMove,   // copy target's last move
    Metronome,    // random move
    Substitute,   // create substitute with 25% HP
    Rest(u8),     // sleep for X turns, full heal
    Bide(u8),     // store damage for X turns
    Rage(u8),     // chance % to enter rage mode
    Rampage,      // rampage

    // Field effects
    Haze(u8), // remove all stat changes, chance %
    SetTeamCondition(crate::player::TeamCondition, u8),
    Seed(u8),  // leech seed effect, chance %
    Nightmare, // only works on sleeping targets

    // Utility
    Heal(u8),                       // heal % of max HP
    CureStatus(Target, StatusType), // cure specific status
    Ante(u8), // percent chance to gain money equal to 2x level (Pay Day effect)
}

/// Context information needed for move effect calculations
#[derive(Debug, Clone)]
pub struct EffectContext {
    pub attacker_index: usize,
    pub defender_index: usize,
    pub move_used: crate::moves::Move,
}

/// Result of applying a move effect, controlling execution flow
#[derive(Debug, Clone)]
pub enum EffectResult {
    /// Apply commands and continue with normal attack execution
    Continue(Vec<crate::battle::commands::BattleCommand>),
    /// Apply commands and skip normal attack execution
    Skip(Vec<crate::battle::commands::BattleCommand>),
}

impl EffectContext {
    pub fn new(
        attacker_index: usize,
        defender_index: usize,
        move_used: crate::moves::Move,
    ) -> Self {
        Self {
            attacker_index,
            defender_index,
            move_used,
        }
    }

    pub fn target_index(&self, target: &Target) -> usize {
        match target {
            Target::User => self.attacker_index,
            Target::Target => self.defender_index,
        }
    }
}

impl MoveEffect {
    /// Apply this effect to the battle state, returning commands and execution control
    pub fn apply(
        &self,
        context: &EffectContext,
        state: &crate::battle::state::BattleState,
        rng: &mut crate::battle::state::TurnRng,
    ) -> EffectResult {
        let defender_has_substitute = state.players[context.defender_index]
            .active_pokemon_conditions
            .values()
            .any(|condition| matches!(condition, PokemonCondition::Substitute { .. }));

        // 2. If so, ask the effect if it's blocked and return early if it is.
        if defender_has_substitute && self.is_blocked_by_substitute() {
            return EffectResult::Continue(Vec::new()); // Effect is nullified but continue with attack.
        }

        // Handle all effects through unified apply system
        match self {
            // Special moves that may skip attack execution
            MoveEffect::InAir => self.apply_in_air_special(context, state),
            MoveEffect::Teleport(_) => self.apply_teleport_special(context, state),
            MoveEffect::ChargeUp => self.apply_charge_up_special(context, state),
            MoveEffect::Underground => self.apply_underground_special(context, state),
            MoveEffect::Transform => self.apply_transform_special(context, state),
            MoveEffect::Conversion => self.apply_conversion_special(context, state),
            MoveEffect::Substitute => self.apply_substitute_special(context, state),
            MoveEffect::Counter => self.apply_counter_special(context, state),
            MoveEffect::Bide(turns) => self.apply_bide_special(*turns, context, state),
            MoveEffect::MirrorMove => self.apply_mirror_move_special(context, state),
            MoveEffect::Rest(sleep_turns) => self.apply_rest_special(*sleep_turns, context, state),
            MoveEffect::Metronome => self.apply_metronome_special(context, state, rng),

            // Special moves that continue with attack execution
            MoveEffect::Rampage => self.apply_rampage_special(context, state, rng),
            MoveEffect::Rage(_) => self.apply_rage_special(context, state),
            MoveEffect::Explode => self.apply_explode_special(context, state),

            // Regular effects that always continue with attack execution
            MoveEffect::Burn(chance) => {
                EffectResult::Continue(self.apply_burn_effect(*chance, context, state, rng))
            }
            MoveEffect::Paralyze(chance) => {
                EffectResult::Continue(self.apply_paralyze_effect(*chance, context, state, rng))
            }
            MoveEffect::Freeze(chance) => {
                EffectResult::Continue(self.apply_freeze_effect(*chance, context, state, rng))
            }
            MoveEffect::Poison(chance) => {
                EffectResult::Continue(self.apply_poison_effect(*chance, context, state, rng))
            }
            MoveEffect::Sedate(chance) => {
                EffectResult::Continue(self.apply_sedate_effect(*chance, context, state, rng))
            }
            MoveEffect::Flinch(chance) => {
                EffectResult::Continue(self.apply_flinch_effect(*chance, context, state, rng))
            }
            MoveEffect::Confuse(chance) => {
                EffectResult::Continue(self.apply_confuse_effect(*chance, context, state, rng))
            }
            MoveEffect::Trap(chance) => {
                EffectResult::Continue(self.apply_trap_effect(*chance, context, state, rng))
            }
            MoveEffect::Seed(chance) => {
                EffectResult::Continue(self.apply_seed_effect(*chance, context, state, rng))
            }
            MoveEffect::Exhaust(chance) => {
                EffectResult::Continue(self.apply_exhaust_effect(*chance, context, state, rng))
            }
            MoveEffect::StatChange(target, stat, stages, chance) => EffectResult::Continue(
                self.apply_stat_change_effect(target, stat, *stages, *chance, context, state, rng),
            ),
            MoveEffect::RaiseAllStats(chance) => EffectResult::Continue(
                self.apply_raise_all_stats_effect(*chance, context, state, rng),
            ),
            MoveEffect::Heal(percentage) => {
                EffectResult::Continue(self.apply_heal_effect(*percentage, context, state))
            }
            MoveEffect::Haze(chance) => {
                EffectResult::Continue(self.apply_haze_effect(*chance, context, state, rng))
            }
            MoveEffect::CureStatus(target, status_type) => EffectResult::Continue(
                self.apply_cure_status_effect(target, status_type, context, state),
            ),
            MoveEffect::SetTeamCondition(condition, turns) => {
                EffectResult::Continue(self.apply_team_condition_effect(condition, *turns, context))
            }
            MoveEffect::Ante(chance) => {
                EffectResult::Continue(self.apply_ante_effect(*chance, context, state, rng))
            }
            MoveEffect::Recoil(_) | MoveEffect::Drain(_) => {
                // Damage-based effects are handled separately in apply_damage_based_effects
                EffectResult::Continue(Vec::new())
            }
            MoveEffect::Reckless(_) => {
                // Miss-based effects are handled separately in apply_miss_based_effects
                EffectResult::Continue(Vec::new())
            }
            _ => {
                // For effects not yet migrated, return empty command list
                EffectResult::Continue(Vec::new())
            }
        }
    }

    pub fn is_blocked_by_substitute(&self) -> bool {
        use crate::move_data::{MoveEffect, Target};

        match self {
            // --- EFFECTS THAT BYPASS SUBSTITUTE ---

            // Effects that explicitly target the user.
            MoveEffect::Heal(_)
            | MoveEffect::Exhaust(_)
            | MoveEffect::RaiseAllStats(_)
            | MoveEffect::Rest(_)
            | MoveEffect::Rage(_)
            | MoveEffect::Substitute
            | MoveEffect::Transform
            | MoveEffect::Conversion
            | MoveEffect::Counter
            | MoveEffect::Bide(_)
            | MoveEffect::Explode
            | MoveEffect::Reckless(_)
            | MoveEffect::MirrorMove
            | MoveEffect::Metronome => false,

            // Damage modifiers that affect the user's calculation, not the target.
            MoveEffect::Recoil(_)
            | MoveEffect::Drain(_)
            | MoveEffect::Crit(_)
            | MoveEffect::IgnoreDef(_)
            | MoveEffect::Priority(_)
            | MoveEffect::MultiHit(_, _) => false,

            // Field effects or team conditions that affect the user's side.
            MoveEffect::Haze(_) | MoveEffect::SetTeamCondition(..) => false,

            // Conditional effects: blocked only if they target the opponent.
            MoveEffect::StatChange(target, ..) => matches!(target, Target::Target),
            MoveEffect::CureStatus(target, ..) => matches!(target, Target::Target),

            // --- EFFECTS THAT ARE BLOCKED BY SUBSTITUTE ---

            // All other effects are assumed to target the opponent and are blocked by default.
            // This includes all primary status conditions (Burn, Flinch, etc.),
            // stat-lowering effects on the target, and other debilitating conditions.
            _ => true,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MoveData {
    pub name: String,
    pub move_type: PokemonType,
    pub power: Option<u8>, // None for no damage moves
    pub category: MoveCategory,
    pub accuracy: Option<u8>, // None for sure-hit moves
    pub max_pp: u8,
    pub effects: Vec<MoveEffect>,
}

impl MoveData {
    /// Get move data for a specific move from the compiled data
    pub fn get_move_data(move_: Move) -> MoveDataResult<MoveData> {
        // Handle special hardcoded moves first
        match move_ {
            Move::HittingItself => {
                Ok(MoveData {
                    name: "Hit Itself".to_string(),
                    move_type: PokemonType::Typeless,
                    power: Some(40),
                    category: MoveCategory::Physical,
                    accuracy: None, // Always hits
                    max_pp: 0,      // Not a real move, no PP
                    effects: vec![],
                })
            }
            Move::Struggle => {
                Ok(MoveData {
                    name: "Struggle".to_string(),
                    move_type: PokemonType::Typeless,
                    power: Some(50),
                    category: MoveCategory::Physical,
                    accuracy: Some(90),
                    max_pp: 0,                             // Not a real move, no PP
                    effects: vec![MoveEffect::Recoil(25)], // 25% recoil of damage dealt
                })
            }
            _ => {
                // For regular moves, get from compiled data
                get_compiled_move_data()
                    .get(&move_)
                    .cloned()
                    .ok_or(MoveDataError::MoveNotFound(move_))
            }
        }
    }

    /// Get move data for a specific move from the compiled data (legacy version that panics)
    /// This function is deprecated - use get_move_data() instead
    #[deprecated(note = "Use get_move_data() instead for proper error handling")]
    pub fn get_move_data_unchecked(move_: Move) -> Option<MoveData> {
        Self::get_move_data(move_).ok()
    }

    pub fn get_move_max_pp(move_: Move) -> MoveDataResult<u8> {
        Self::get_move_data(move_).map(|data| data.max_pp)
    }

    /// Get move max PP with fallback (legacy version that uses fallback)
    /// This function is deprecated - use get_move_max_pp() instead
    #[deprecated(note = "Use get_move_max_pp() instead for proper error handling")]
    pub fn get_move_max_pp_with_fallback(move_: Move) -> u8 {
        Self::get_move_max_pp(move_).unwrap_or(30) // Default fallback
    }
}

// Helper function to parse Move enum from string
// FromStr implementation moved to schema crate to avoid orphan rule violations
/*
impl std::str::FromStr for Move {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_uppercase().replace([' ', '-', '_'], "");

        match normalized.as_str() {
            "POUND" => Ok(Move::Pound),
            "DOUBLESLAP" => Ok(Move::Doubleslap),
            "PAYDAY" => Ok(Move::PayDay),
            "SCRATCH" => Ok(Move::Scratch),
            "GUILLOTINE" => Ok(Move::Guillotine),
            "SWORDSDANCE" => Ok(Move::SwordsDance),
            "CUT" => Ok(Move::Cut),
            "BIND" => Ok(Move::Bind),
            "SLAM" => Ok(Move::Slam),
            "STOMP" => Ok(Move::Stomp),
            "HEADBUTT" => Ok(Move::Headbutt),
            "HORNATTACK" => Ok(Move::HornAttack),
            "FURYATTACK" => Ok(Move::FuryAttack),
            "HORNDRILL" => Ok(Move::HornDrill),
            "TACKLE" => Ok(Move::Tackle),
            "BODYSLAM" => Ok(Move::BodySlam),
            "WRAP" => Ok(Move::Wrap),
            "HARDEN" => Ok(Move::Harden),
            "TAKEDOWN" => Ok(Move::TakeDown),
            "THRASH" => Ok(Move::Thrash),
            "DOUBLEEDGE" => Ok(Move::DoubleEdge),
            "TAILWHIP" => Ok(Move::TailWhip),
            "LEER" => Ok(Move::Leer),
            "BITE" => Ok(Move::Bite),
            "GROWL" => Ok(Move::Growl),
            "ROAR" => Ok(Move::Roar),
            "SING" => Ok(Move::Sing),
            "SUPERSONIC" => Ok(Move::Supersonic),
            "SONICBOOM" => Ok(Move::SonicBoom),
            "DISABLE" => Ok(Move::Disable),
            "AGILITY" => Ok(Move::Agility),
            "QUICKATTACK" => Ok(Move::QuickAttack),
            "RAGE" => Ok(Move::Rage),
            "MIMIC" => Ok(Move::Mimic),
            "SCREECH" => Ok(Move::Screech),
            "DOUBLETEAM" => Ok(Move::DoubleTeam),
            "RECOVER" => Ok(Move::Recover),
            "MINIMIZE" => Ok(Move::Minimize),
            "WITHDRAW" => Ok(Move::Withdraw),
            "DEFENSECURL" => Ok(Move::DefenseCurl),
            "BARRIER" => Ok(Move::Barrier),
            "FOCUSENERGY" => Ok(Move::FocusEnergy),
            "BIDE" => Ok(Move::Bide),
            "METRONOME" => Ok(Move::Metronome),
            "MIRRORMOVE" => Ok(Move::MirrorMove),
            "SELFDESTRUCT" => Ok(Move::SelfDestruct),
            "CLAMP" => Ok(Move::Clamp),
            "SWIFT" => Ok(Move::Swift),
            "SPIKECANNON" => Ok(Move::SpikeCannon),
            "CONSTRICT" => Ok(Move::Constrict),
            "SOFTBOILED" => Ok(Move::SoftBoiled),
            "GLARE" => Ok(Move::Glare),
            "TRANSFORM" => Ok(Move::Transform),
            "EXPLOSION" => Ok(Move::Explosion),
            "FURYSWIPES" => Ok(Move::FurySwipes),
            "REST" => Ok(Move::Rest),
            "HYPERFANG" => Ok(Move::HyperFang),
            "SHARPEN" => Ok(Move::Sharpen),
            "CONVERSION" => Ok(Move::Conversion),
            "TRIATTACK" => Ok(Move::TriAttack),
            "SUPERFANG" => Ok(Move::SuperFang),
            "SLASH" => Ok(Move::Slash),
            "SUBSTITUTE" => Ok(Move::Substitute),
            "HYPERBEAM" => Ok(Move::HyperBeam),
            "KARATECHOP" => Ok(Move::KarateChop),
            "COMETPUNCH" => Ok(Move::CometPunch),
            "MEGAPUNCH" => Ok(Move::MegaPunch),
            "KOPUNCH" => Ok(Move::KOPunch),
            "DOUBLEKICK" => Ok(Move::DoubleKick),
            "MEGAKICK" => Ok(Move::MegaKick),
            "JUMPKICK" => Ok(Move::JumpKick),
            "ROLLINGKICK" => Ok(Move::RollingKick),
            "SUBMISSION" => Ok(Move::Submission),
            "LOWKICK" => Ok(Move::LowKick),
            "COUNTER" => Ok(Move::Counter),
            "SEISMICTOSS" => Ok(Move::SeismicToss),
            "STRENGTH" => Ok(Move::Strength),
            "MEDITATE" => Ok(Move::Meditate),
            "HIGHJUMPKICK" => Ok(Move::HighJumpKick),
            "BARRAGE" => Ok(Move::Barrage),
            "DIZZYPUNCH" => Ok(Move::DizzyPunch),
            "RAZORWIND" => Ok(Move::RazorWind),
            "GUST" => Ok(Move::Gust),
            "WINGATTACK" => Ok(Move::WingAttack),
            "WHIRLWIND" => Ok(Move::Whirlwind),
            "FLY" => Ok(Move::Fly),
            "PECK" => Ok(Move::Peck),
            "DRILLPECK" => Ok(Move::DrillPeck),
            "SKYATTACK" => Ok(Move::SkyAttack),
            "VICEGRIP" => Ok(Move::ViceGrip),
            "ROCKTHROW" => Ok(Move::RockThrow),
            "SKULLBASH" => Ok(Move::SkullBash),
            "ROCKSLIDE" => Ok(Move::RockSlide),
            "ANCIENTPOWER" => Ok(Move::AncientPower),
            "SANDATTACK" => Ok(Move::SandAttack),
            "EARTHQUAKE" => Ok(Move::Earthquake),
            "FISSURE" => Ok(Move::Fissure),
            "DIG" => Ok(Move::Dig),
            "BONECLUB" => Ok(Move::BoneClub),
            "BONEMERANG" => Ok(Move::Bonemerang),
            "POISONSTING" => Ok(Move::PoisonSting),
            "TWINEEDLE" => Ok(Move::Twineedle),
            "ACID" => Ok(Move::Acid),
            "TOXIC" => Ok(Move::Toxic),
            "HAZE" => Ok(Move::Haze),
            "SMOG" => Ok(Move::Smog),
            "SLUDGE" => Ok(Move::Sludge),
            "POISONJAB" => Ok(Move::PoisonJab),
            "POISONGAS" => Ok(Move::PoisonGas),
            "ACIDARMOR" => Ok(Move::AcidArmor),
            "PINMISSILE" => Ok(Move::PinMissile),
            "SILVERWIND" => Ok(Move::SilverWind),
            "STRINGSHOT" => Ok(Move::StringShot),
            "LEECHLIFE" => Ok(Move::LeechLife),
            "FIREPUNCH" => Ok(Move::FirePunch),
            "BLAZEKICK" => Ok(Move::BlazeKick),
            "FIREFANG" => Ok(Move::FireFang),
            "EMBER" => Ok(Move::Ember),
            "FLAMETHROWER" => Ok(Move::Flamethrower),
            "WILLOWISP" => Ok(Move::WillOWisp),
            "FIRESPIN" => Ok(Move::FireSpin),
            "SMOKESCREEN" => Ok(Move::Smokescreen),
            "FIREBLAST" => Ok(Move::FireBlast),
            "MIST" => Ok(Move::Mist),
            "WATERGUN" => Ok(Move::WaterGun),
            "HYDROPUMP" => Ok(Move::HydroPump),
            "SURF" => Ok(Move::Surf),
            "BUBBLEBEAM" => Ok(Move::Bubblebeam),
            "WATERFALL" => Ok(Move::Waterfall),
            "BUBBLE" => Ok(Move::Bubble),
            "SPLASH" => Ok(Move::Splash),
            "BUBBLEHAMMER" => Ok(Move::Bubblehammer),
            "VINEWHIP" => Ok(Move::VineWhip),
            "ABSORB" => Ok(Move::Absorb),
            "MEGADRAIN" => Ok(Move::MegaDrain),
            "GIGADRAIN" => Ok(Move::GigaDrain),
            "LEECHSEED" => Ok(Move::LeechSeed),
            "GROWTH" => Ok(Move::Growth),
            "RAZORLEAF" => Ok(Move::RazorLeaf),
            "SOLARBEAM" => Ok(Move::SolarBeam),
            "POISONPOWDER" => Ok(Move::PoisonPowder),
            "STUNSPORE" => Ok(Move::StunSpore),
            "SLEEPPOWDER" => Ok(Move::SleepPowder),
            "PETALDANCE" => Ok(Move::PetalDance),
            "SPORE" => Ok(Move::Spore),
            "EGGBOMB" => Ok(Move::EggBomb),
            "ICEPUNCH" => Ok(Move::IcePunch),
            "ICEBEAM" => Ok(Move::IceBeam),
            "BLIZZARD" => Ok(Move::Blizzard),
            "AURORABEAM" => Ok(Move::AuroraBeam),
            "THUNDERPUNCH" => Ok(Move::ThunderPunch),
            "SHOCK" => Ok(Move::Shock),
            "DISCHARGE" => Ok(Move::Discharge),
            "THUNDERWAVE" => Ok(Move::ThunderWave),
            "THUNDERCLAP" => Ok(Move::Thunderclap),
            "CHARGEBEAM" => Ok(Move::ChargeBeam),
            "LIGHTNING" => Ok(Move::Lightning),
            "FLASH" => Ok(Move::Flash),
            "CONFUSION" => Ok(Move::Confusion),
            "PSYBEAM" => Ok(Move::Psybeam),
            "PERPLEX" => Ok(Move::Perplex),
            "HYPNOSIS" => Ok(Move::Hypnosis),
            "TELEPORT" => Ok(Move::Teleport),
            "CONFUSERAY" => Ok(Move::ConfuseRay),
            "LIGHTSCREEN" => Ok(Move::LightScreen),
            "REFLECT" => Ok(Move::Reflect),
            "AMNESIA" => Ok(Move::Amnesia),
            "KINESIS" => Ok(Move::Kinesis),
            "PSYWAVE" => Ok(Move::Psywave),
            "DREAMEATER" => Ok(Move::DreamEater),
            "LOVELYKISS" => Ok(Move::LovelyKiss),
            "NIGHTSHADE" => Ok(Move::NightShade),
            "LICK" => Ok(Move::Lick),
            "SHADOWBALL" => Ok(Move::ShadowBall),
            "OUTRAGE" => Ok(Move::Outrage),
            "DRAGONRAGE" => Ok(Move::DragonRage),
            "STRUGGLE" => Ok(Move::Struggle),
            "HITITSELF" => Ok(Move::HittingItself),
            _ => Err(format!("Unknown move: {}", s)),
        }
    }
}
*/
