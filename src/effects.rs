#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Target,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stat {
    Atk,
    Def,
    SpAtk,
    SpecDef,
    Spe,
    Acc,
    Eva,
    Crit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCondition {
    Paralysis,
    Sleep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectType {
    Physical,
    Special,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MoveEffect {
    // Status effects with chance
    Flinch(u8),
    Burn(u8),
    Paralyze(u8),
    Poison(u8),
    Freeze(u8),
    Sedate(u8),
    Confuse(u8),
    Exhaust(u8),

    // Stat changes: StatChange(target, stat, amount, chance)
    StatChange(Target, Stat, i8, u8),

    // Multi-hit moves: MultiHit(min_hits, max_hits)
    MultiHit(u8, u8),

    // Damage modifications
    Recoil(u8),
    Drain(u8),
    Heal(u8),
    SetDamage(u8),
    SuperFang(u8),
    LevelDamage,
    Crit(u8),
    IgnoreDef(u8),

    // Trapping and binding
    Trap(u8),
    Ante(u8),

    // Special mechanics
    OHKO,
    Priority(i8),
    SureHit,
    ChargeUp,
    Charge,
    Teleport(u8),
    RaiseAllStats(u8),
    Rampage(StatusCondition),
    Rage(u8),
    MirrorMove,
    Metronome,
    Transform,
    Explode,
    Bide(u8),
    Substitute,
    Conversion,
    Counter,
    Haze(u8),
    Mist,
    Reflect(ReflectType),
    Seed(u8),
    CureStatus(Target, StatusCondition),
    Rest(u8),
    Nightmare,
    Reckless(u8),
    Disable(u8),

    // No parameters
    Struggle,
}