#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Move {
    // Normal Type
    Pound,
    Doubleslap,
    PayDay,
    Scratch,
    Guillotine,
    SwordsDance,
    Cut,
    Bind,
    Slam,
    Stomp,
    Headbutt,
    HornAttack,
    FuryAttack,
    HornDrill,
    Tackle,
    BodySlam,
    Wrap,
    Harden,
    TakeDown,
    Thrash,
    DoubleEdge,
    TailWhip,
    Leer,
    Bite,
    Growl,
    Roar,
    Sing,
    Supersonic,
    SonicBoom,
    Disable,
    Agility,
    QuickAttack,
    Rage,
    Mimic,
    Screech,
    DoubleTeam,
    Recover,
    Minimize,
    Withdraw,
    DefenseCurl,
    Barrier,
    FocusEnergy,
    Bide,
    Metronome,
    MirrorMove,
    SelfDestruct,
    Clamp,
    Swift,
    SpikeCannon,
    Constrict,
    SoftBoiled,
    Glare,
    Transform,
    Explosion,
    FurySwipes,
    Rest,
    HyperFang,
    Sharpen,
    Conversion,
    TriAttack,
    SuperFang,
    Slash,
    Substitute,
    HyperBeam,

    // Fighting Type
    KarateChop,
    CometPunch,
    MegaPunch,
    KoPunch,
    DoubleKick,
    MegaKick,
    JumpKick,
    RollingKick,
    Submission,
    LowKick,
    Counter,
    SeismicToss,
    Strength,
    Meditate,
    HighJumpKick,
    Barrage,
    DizzyPunch,

    // Flying Type
    RazorWind,
    Gust,
    WingAttack,
    Whirlwind,
    Fly,
    Peck,
    DrillPeck,
    SkyAttack,

    // Rock Type
    Vicegrip,
    RockThrow,
    SkullBash,
    RockSlide,
    AncientPower,

    // Ground Type
    SandAttack,
    Earthquake,
    Fissure,
    Dig,
    BoneClub,
    Bonemerang,

    // Poison Type
    PoisonSting,
    Twineedle,
    Acid,
    Toxic,
    Haze,
    Smog,
    Sludge,
    PoisonJab,
    PoisonGas,
    AcidArmor,

    // Bug Type
    PinMissile,
    SilverWind,
    StringShot,
    LeechLife,

    // Fire Type
    FirePunch,
    BlazeKick,
    FireFang,
    Ember,
    Flamethrower,
    WillOWisp,
    FireSpin,
    Smokescreen,
    FireBlast,

    // Water Type
    Mist,
    WaterGun,
    HydroPump,
    Surf,
    Bubblebeam,
    Waterfall,
    Bubble,
    Splash,
    Bubblehammer,

    // Grass Type
    VineWhip,
    Absorb,
    MegaDrain,
    GigaDrain,
    LeechSeed,
    Growth,
    RazorLeaf,
    Solarbeam,
    PoisonPowder,
    StunSpore,
    SleepPowder,
    PetalDance,
    Spore,
    EggBomb,

    // Ice Type
    IcePunch,
    IceBeam,
    Blizzard,
    AuroraBeam,

    // Electric Type
    ThunderPunch,
    Shock,
    Discharge,
    ThunderWave,
    Thunderclap,
    ChargeBeam,
    Lightning,
    Flash,

    // Psychic Type
    Confusion,
    Psybeam,
    Perplex,
    Hypnosis,
    Teleport,
    ConfuseRay,
    LightScreen,
    Reflect,
    Amnesia,
    Kinesis,
    Psychic,
    Psywave,
    DreamEater,
    LovelyKiss,

    // Ghost Type
    NightShade,
    Lick,
    ShadowBall,

    // Dragon Type
    Outrage,
    DragonRage,

    // Typeless
    Struggle,
}

#[derive(Debug, Clone)]
pub struct MoveInstance {
    pub move_: Move,
    pub pp: u8,
}
impl MoveInstance {
    /// Create a new move instance with max PP
    pub fn new(move_: Move) -> Self {
        MoveInstance {
            move_,
            pp: 30, // TODO: Get actual max PP from move data
        }
    }
    
    /// Use the move (decrease PP)
    pub fn use_move(&mut self) -> bool {
        if self.pp > 0 {
            self.pp -= 1;
            true
        } else {
            false
        }
    }
    
    /// Restore PP
    pub fn restore_pp(&mut self, amount: u8) {
        self.pp = (self.pp + amount).min(30); // TODO: Use actual max PP
    }
}