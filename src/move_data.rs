use crate::moves::Move;
use crate::pokemon::PokemonType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{LazyLock, RwLock};

// Global move data storage - loaded once at startup
static MOVE_DATA: LazyLock<RwLock<HashMap<Move, MoveData>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Initialize the global move data by loading from disk
pub fn initialize_move_data(data_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let move_data_map = MoveData::load_all(data_path)?;
    let mut global_data = MOVE_DATA.write().unwrap();
    *global_data = move_data_map;
    Ok(())
}

/// Get move data for a specific move from the global store
pub fn get_move_data(move_: Move) -> Option<MoveData> {
    let global_data = MOVE_DATA.read().unwrap();
    global_data.get(&move_).cloned()
}

/// Get max PP for a specific move
pub fn get_move_max_pp(move_: Move) -> u8 {
    get_move_data(move_).map(|data| data.max_pp).unwrap_or(30) // Default fallback
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveCategory {
    Physical,
    Special,
    Other,
    Status,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatType {
    Hp,
    Atk,
    Def,
    SpAtk,
    SpDef,
    Spe,
    Acc,
    Eva,
    Crit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    User,
    Target,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReflectType {
    Physical,
    Special,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RampageEndCondition {
    Confuse,
    Exhaust,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusType {
    Sleep,
    Poison,
    Burn,
    Freeze,
    Paralysis,
}

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
    OHKO,                         // one-hit KO
    Explode,                      // user faints
    Reckless(u8),                 // recoil if miss, chance %
    Transform,                    // copy target's appearance/stats
    Conversion,                   // change user's type
    Disable(u8),                  // disable target's last move, chance %
    Counter,                      // return double physical damage
    MirrorMove,                   // copy target's last move
    Metronome,                    // random move
    Substitute,                   // create substitute with 25% HP
    Rest(u8),                     // sleep for X turns, full heal
    Bide(u8),                     // store damage for X turns
    Rage(u8),                     // chance % to enter rage mode
    Rampage(RampageEndCondition), // rampage with end condition

    // Field effects
    Haze(u8),             // remove all stat changes, chance %
    Reflect(ReflectType), // reduce physical/special damage
    Mist,                 // prevent stat reduction
    Seed(u8),             // leech seed effect, chance %
    Nightmare,            // only works on sleeping targets

    // Utility
    Heal(u8),                       // heal % of max HP
    CureStatus(Target, StatusType), // cure specific status
    Ante(u8),                       // percent chance to gain money equal to 2x level (Pay Day effect)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Load move data from RON files in the data directory
    pub fn load_all(
        data_path: &Path,
    ) -> Result<HashMap<Move, MoveData>, Box<dyn std::error::Error>> {
        let moves_dir = data_path.join("moves");

        let mut move_map = HashMap::new();
        let hitting_itself_data = MoveData {
            name: "Hit Itself".to_string(),
            move_type: PokemonType::Typeless,
            power: Some(40),
            category: MoveCategory::Physical,
            accuracy: None, // Always hits
            max_pp: 0,      // Not a real move, no PP
            effects: vec![],
        };
        move_map.insert(Move::HittingItself, hitting_itself_data);

        // Add Struggle.
        // It has fixed data and recoil. Recoil is 25% of damage dealt here.
        // Note: In some game generations, recoil is 1/4 of the user's max HP.
        let struggle_data = MoveData {
            name: "Struggle".to_string(),
            move_type: PokemonType::Typeless,
            power: Some(50),
            category: MoveCategory::Physical,
            accuracy: Some(90),
            max_pp: 0,                             // Not a real move, no PP
            effects: vec![MoveEffect::Recoil(25)], // 25% recoil of damage dealt
        };
        move_map.insert(Move::Struggle, struggle_data);

        if !moves_dir.exists() {
            return Err(format!("Moves data directory not found: {}", moves_dir.display()).into());
        }

        let entries = fs::read_dir(&moves_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    let content = fs::read_to_string(&path)?;
                    let move_data: MoveData = ron::from_str(&content)?;

                    // Parse move name from filename to get the Move enum variant
                    if let Ok(move_enum) = filename.parse::<Move>() {
                        move_map.insert(move_enum, move_data);
                    }
                }
            }
        }

        Ok(move_map)
    }

    /// Get move data for a specific move
    pub fn get_move_data(move_: Move, move_map: &HashMap<Move, MoveData>) -> Option<&MoveData> {
        move_map.get(&move_)
    }
}

// Helper function to parse Move enum from string
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
            "KOPUNCH" => Ok(Move::KoPunch),
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
            "VICEGRIP" => Ok(Move::Vicegrip),
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
            "SOLARBEAM" => Ok(Move::Solarbeam),
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
            "PSYCHIC" => Ok(Move::Psychic),
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
