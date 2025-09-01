## Reference Structs 

These structs 
```rust

pub enum MoveAttribute {
    // Status and condition modification
    ApplyStatus(Target, Status, u8)         // Who to target, the StatusType, and the chance %
    ApplyCondition(Target, Condition, u8)   // Who to target, what to apply, and the chance %
    CureStatus(Target, Status),             // Who to target, what to remove
    CureCondition(Target, Condition),       // Who to target, what to remove

    // Stat changes
    StatChange(Target, CombatStat, i8, u8), // target, stat, stages, chance %
    RaiseAllStats(u8),                    // chance %
    ResetAllStats(u8),                    // remove all stat changes for both players, chance %

    // Damage modifiers
    Recoil(u8),     // % of damage dealt
    Drain(u8),      // % of damage healed
    Crit(u8),       // increased crit ratio
    IgnoreDef(u8),  // chance % to ignore defense

    // Fixed damage
    SetDamage(u16), // fixed damage
    LevelDamage,    // damage = user level
    HalfDamage,     // damage = half of target's current hp

    // Multi-hit
    MultiHit(u8, u8), // min hits, % chance of continuation afterwards (Max 7)

    // Funky Rules
    Priority(i8), // move priority modifier
    ChargeUp,     // charge for 1 turn
    InAir,        // fly up high
    Underground,  // go underground
    Haunting,     // only works on sleeping targets.

    // Special mechanics
    Lethal,       // one-hit KO
    Suicidal,     // user faints
    Reckless(u8), // recoil if miss, % of HP lost
    Transform,    // copy target's appearance/stats
    Conversion,   // change user's type
    Disable(u8),  // disable target's last move, chance %
    Counter,      // return double physical damage
    MirrorMove,   // copy target's last move
    Metronome,    // use random move
    Substitute,   // create substitute with 25% HP 
    Rest(u8),     // sleep for X turns, full heal
    Bide(u8),     // store damage for X turns
    Rage(u8),     // chance % to enter rage mode
    Rampage,      // rampage (the Thrash effect)

    // Field effects
    SetTeamCondition(TeamCondition, u8),

    // Utility
    Heal(u8),  // heal % of max HP
    Ante(u8), // percent chance to gain money equal to 2x level (Pay Day effect)
}

pub enum CombatStat {
    Atk,
    Def,
    SpAtk,
    SpDef,
    Spe,
    Acc,
    Eva,
    Crit,
}

pub enum Status {
    Sleep,
    Poison,
    Burn,
    Freeze,
    Paralysis,
}

pub enum Condition {
    Flinched,
    Confused {
        turns_remaining: u8,
    }, // Counts down each turn
    Seeded,
    Underground,
    InAir,
    Teleported,
    Enraged,
    Exhausted, // Prevents acting for one turn
    Trapped {
        turns_remaining: u8,
    },
    Charging,
    Rampaging {
        turns_remaining: u8,
    },
    Converted {
        pokemon_type: PokemonType,
    },
    Disabled {
        pokemon_move: Move,
        turns_remaining: u8,
    }, // Counts down each turn
    Substitute {
        hp: u8,
    },
    Biding {
        turns_remaining: u8,
        damage: u16,
    },
    Countering {
        damage: u16,
    },
}

```
