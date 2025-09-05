# Move Data Design

## Overview

The legacy "pure-data" model, where a move is a flat collection of properties and effects, is being replaced by a more expressive **scripting model**.

The fundamental shift is to treat a move's definition not as a list of static properties, but as a **sequential, ordered script of instructions**. These instructions (`Strike`, `Passive`, `MultiHit`, `Prepare`) are interpreted by the battle engine's FSM to generate the appropriate `BattleAction`s. This approach provides granular control over the timing, sequence, and dependencies of a move's various effects, moving complex logic from the engine's code into human-readable data files.

## Core Principles

*   **Explicit over Implicit:** The data structure unambiguously declares the nature and timing of each effect. The engine no longer needs to infer intent from properties like `MoveCategory`.
*   **Data-Driven Logic:** The RON files become the single source of truth for a move's behavior. New and complex moves can be created by simply writing a new "script" without changing the engine's core execution code.
*   **Sequential Execution:** The `script` field is an ordered `Vec`, and the engine will process its instructions in the sequence provided. This allows for precise control over complex moves.
*   **Contingent vs. Guaranteed Effects:** The design creates a clear distinction between effects that are contingent on a successful hit (defined within a `Strike`'s `effects`) and effects that are guaranteed to happen (defined as a separate `Passive` instruction in the main sequence).
*   **Composability:** Complex moves are built by composing simple, reusable instructions and traits, leading to a more robust and maintainable system.

## Data Structure Definition

The core `MoveData` struct is simplified to contain a name, PP, priority, and a script of instructions.

```rust
// In data/moves/schemas.rs
/// Defines the complete behavior of a single Pokémon move.
pub struct MoveData {
    pub name: String,
    pub max_pp: u8,
    pub priority: i8,
    /// The ordered script of instructions executed when the move is used.
    pub script: Vec<Instruction>,
}

/// A single high-level instruction in a move's script.
pub enum Instruction {
    /// An instruction for a single offensive strike.
    Strike{data: StrikeData},
    /// An instruction for a guaranteed, non-striking effect.
    Passive{effect: PassiveEffect},
    /// An instruction to generate a variable number of strikes.
    MultiHit {
        min_hits: u8,
        continuation_chance: u8,
        /// The template for the strike to be repeated.
        strike: StrikeData,
    },
    /// An instruction for two-turn moves that require preparation.
    Prepare {
        flag: PokemonFlag,  // InAir, Underground, Charging
        /// The strike to execute after preparation is complete.
        strike: StrikeData,
    },
}

/// A reusable component holding all data for a single offensive strike.
#[derive(Clone)]
pub struct StrikeData {
    pub move_type: PokemonType,
    pub power: u8,
    pub accuracy: u8, // 0-100.
    pub category: DamageCategory,
    /// The list of contingent effects that may trigger if this strike hits.
    pub effects: Vec<StrikeEffect>,
}


#[derive(Debug, Serialize, Deserialize)]
pub enum DamageCategory {Physical, Special, Other}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {User, Target}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BattleStat {
    Atk,
    Def,
    SpAtk,
    SpDef,
    Spe,
    Acc,
    Eva,
    Crit,
}

pub enum StrikeEffect {
    // Major Status (stored on PokemonInst)
    ApplyStatus { target: Target, status: PokemonStatusType, percent_chance: u8 },
    RemoveStatus { target: Target, status: PokemonStatusType, percent_chance: u8 },
    CureStatus{ target: Target, status: PokemonStatusType},
    // Volatile Conditions (stored on BattleState)
    ApplyCondition { target: Target, condition: PokemonConditionType, percent_chance: u8 },
    RemoveCondition { target: Target, condition: PokemonConditionType, percent_chance: u8 },
    
    // Battle Flags (stored on BattleState)
    ApplyFlag { target: Target, flag: PokemonFlagType },
    RemoveFlag { target: Target, flag: PokemonFlagType },
    
    StatChange { target: Target, stat: BattleStat, delta: i8, chance: u8 },
    Transform { target: Target },

    // Damage Modifiers
    Drain { percent: u8 },
    Recoil { percent: u8 },
    CritRatio {level: u8},
    PartialIgnoreDefense { chance: u8 },

    // Special Damage Rules
    PercentHpDamage { percentage: u8 },
    FixedDamage { damage: u16 },
    LevelDamage,
    Lethal,

    // Miscellany
    SureHit,
    Reckless { self_damage_percent: u8 },
    RequiresStatus{ target: Target, status: PokemonStatusType},
    
    // Special Transform Effect (can miss, blocked by Substitute)
    Transform { target: Target },
    
}

pub enum PassiveEffect {
    StatChange { stat: BattleStat, delta: i8, chance: u8 },
    Heal { percent: u8 },
    Rest,  // Heal to full HP + apply Sleep status
    CureStatus { status: PokemonStatusType },
    ClearStatus,  // Remove any major status
    ClearAllStatChanges { chance: u8 },
    
    // Team Conditions (stored on BattleState)
    ApplyTeamCondition { condition: TeamConditionType },
    
    // Special Effects
    Conversion,  // Change type to match first move
    Substitute { hp_cost_percent: u8 },  // Create substitute using HP
    Counter,  // Set up counter damage reflection
    MirrorMove,  // Copy and use opponent's last move
    Mimic,   // Copy and add the opponent's last move to the user's moveset temporarily
    Metronome,  // Randomly select and execute any move
    Bide,  // Begin damage accumulation for 2-3 turns
    
    // Utility Effects
    Flicker { chance: u8 },  // Teleport/evasion effect
    Suicide,  // User faints (Explosion, Self-Destruct)

    AnteUp { chance: u8 },
}

```

## Execution Model

1.  **Dispatch:** When a `DoMove` action executes, it reads the `script` list from the chosen move's `MoveData`. It iterates through this list and pushes the corresponding `BattleAction` (`StrikeAction` or `PassiveAction`) for each instruction onto the action stack in **reverse order**. For `MultiHit`, it runs the hit-generation logic and pushes multiple `StrikeAction`s.

2.  **Strike Execution:** When a `StrikeAction` executes, it first performs its accuracy check.
    *   **On Hit:** It calculates damage and then processes its `effects` list, generating and pushing the final Direct Effect Actions (`Damage`, `Heal`, `ApplyStatus`, etc.).
    *   **On Miss:** It pushes a single `Miss` action. The `Miss` action is responsible for checking the original strike's `effects` for any on-miss effects (e.g., `Reckless`).

3.  **Passive Execution:** When a `PassiveAction` executes, it processes its `effects` list and pushes the corresponding `Direct Effect Actions`. There is no accuracy check. Note that the Passive versions of Strike effects never include a Target field, because the Strike/Passive distinction is not Damaging/Status but Offensive/Self-Directed. PassiveEffects impact the field overall, or the user, but never another target.

4.  **Effect Storage:** Effects are applied to different storage locations based on their type:
    *   **Major Status** (`ApplyStatus`, `RemoveStatus`, `ClearStatus`): Stored on `PokemonInst.status` for persistence across battle contexts
    *   **Volatile Conditions** (`ApplyCondition`, `RemoveCondition`): Stored on `BattleState.active_conditions` for temporary battle state
    *   **Battle Flags** (`ApplyFlag`, `RemoveFlag`): Stored on `BattleState.simple_flags` and `BattleState.special_flags` for battle-specific state markers
    *   **Team Conditions** (`ApplyTeamCondition`): Stored on `BattleState.team_conditions` for team-wide effects
    *   **Stat Stages** (`StatChange`): Stored on `BattleState.stat_stages` for temporary battle modifications

## Examples in Practice

### Simple Strike: `Absorb`

A single `Strike` instruction with a contingent `Drain` effect. The healing only happens if the attack lands.

```ron
MoveData(
    name: "Absorb",
    max_pp: 40,
    script: [
        Strike(
            move_type: Grass,
            power: 40,
            accuracy: 100,
            category: Special,
            effects: [
                Drain(percent: 50),
            ],
        ),
    ],
)
```

### Simple Passive: `Acid Armor`

A single `Passive` instruction that modifies the user's stats. It cannot miss.

```ron
// data/moves/acid-armor.ron
MoveData(
    name: "Acid Armor",
    max_pp: 20,
    script: [
        Passive(StatChange(stat: Def, delta: 2, chance: 100)),
    ],
)
```

### Sequential vs. Contingent Effects: `Overheat` vs. `Ancient Power`

**`Overheat`** has a guaranteed stat drop that happens *after* the attack resolves, hit or miss. This is scripted as a sequence of two instructions.

```ron
// data/moves/overheat.ron
MoveData(
    name: "Overheat",
    max_pp: 5,
    script: [
        Strike(
            move_type: Fire,
            power: 130,
            accuracy: 90,
            category: Special,
            effects: [],
        ),
        Passive(StatChange(stat: SpAtk, delta: -2, chance: 100)),
    ],
)
```

**`Ancient Power`** has a *chance* for a stat boost that only occurs if the attack hits. This is scripted as a single `Strike` with contingent effects.

```ron
// data/moves/ancient-power.ron
MoveData(
    name: "Ancient Power",
    max_pp: 5,
    script: [
        Strike(
            move_type: Rock,
            power: 60,
            accuracy: 100,
            category: Special,
            effects: [
                StatChange(target: User, stat: Atk, delta: 1, chance: 10),
                StatChange(target: User, stat: Def, delta: 1, chance: 10),
                StatChange(target: User, stat: SpAtk, delta: 1, chance: 10),
                StatChange(target: User, stat: SpDef, delta: 1, chance: 10),
                StatChange(target: User, stat: Spe, delta: 1, chance: 10)],
        ),
    ],
)
```

### Multi-Strike: `Tri Attack`

Scripted as a sequence of three distinct `Strike` instructions. Each has its own accuracy check and potential secondary effect.

```ron
// data/moves/tri-attack.ron
MoveData(
    name: "Tri Attack",
    max_pp: 15,
    script: [
        Strike(data: (
            move_type: Fire, 
            power: 40, 
            accuracy: 100, 
            category: Special, 
            effects: [ApplyStatus(target: Target, status: Burn, percent_chance: 10)]
        )),
        Strike(data: (
            move_type: Electric, 
            power: 40, 
            accuracy: 100, 
            category: Special, 
            effects: [ApplyStatus(target: Target, status: Paralysis, percent_chance: 10)]
        )),
        Strike(data: (
            move_type: Ice, 
            power: 40, 
            accuracy: 100, 
            category: Special, 
            effects: [ApplyStatus(target: Target, status: Freeze, percent_chance: 10)]
        )),
    ],
)
```

### Multi-Hit: `Fury Attack`

Uses the `MultiHit` instruction, which the engine will unroll into a variable number of `StrikeAction`s at runtime.

```ron
// data/moves/fury-attack.ron
MoveData(
    name: "Fury Attack",
    max_pp: 30,
    script: [
        MultiHit(
            min_hits: 2,
            continuation_chance: 50,
            strike: (
                move_type: Normal,
                power: 15,
                accuracy: 90,
                category: Physical,
                effects: [],
            ),
        ),
    ],
)
```

### Multi-Turn Moves and Special Conditions: Fly, SolarBeam, Bide, and Counter
These are tricky, because they rely on a mix of hard-coding, while the new move design has a certain amount of scripting. 
They are also tricky because they all involve forced actions. The current system is meant to reflect the scripting/coding boundary by noting how they interact with control conditions. 

Fly and Solar Beam use Prepare, which applies a flag to the user if they don't have the flag, and removes the flag + applies the effects of the move if they do have it.

Bide uses a simple Passive, which applies the Bide volatile condition, then does nothing until Bide's turns remaining hits zero, at which point the doubled damage is unleashed. As Bide's effect has to be hard-coded anyway, there's nothing else to say.

Counter is handled slightly differently than in the regular games--now, instead of having negative priority, it has very high priority, and applies a one-turn special flag (Countering) that records the physical damage the pokemon receives and does twice as much damage back to the opponent at the beginning of the end-of-turn. Like Bide, this can be handled with a simple Passive.

```ron
MoveData(
    name: "Fly",
    max_pp: 15,
    script: [
        Prepare(
            flag: InAir,
            strike: (
                move_type: Flying,
                power: 90,
                accuracy: 95,
                category: Physical,
                effects: [],
            ),
        ),
    ],
)

MoveData(
    name: "Solar Beam",
    max_pp: 10,
    script: [
        Prepare(
            flag: Charging,
            strike: (
                move_type: Grass,
                power: 150,
                accuracy: 100,
                category: Special,
                effects: [ApplyStatus(target: Target, status: Burn, percent_chance: 50)],
            ),
        ),
    ],
)

MoveData(
    name: "Bide",
    max_pp: 20,
    script: [
        Passive(Bide),
    ],
)

MoveData(
    name: "Counter",
    max_pp: 15,
    priority: 2,
    script: [
        Passive(Counter),
    ],
)

MoveData(
    name: "Transform",
    max_pp: 10,
    script: [
        Strike(data: (
            move_type: Typeless,
            power: 0,
            accuracy: 255,
            category: Other,
            effects: [Transform(target: Target)],
        )),
    ],
)
```

## Benefits of this Approach

*   **Clarity:** The move's logic and timing are relatively clear.
*   **Maintainability:** Engine logic is generic and stable. Move balancing and bug fixes can often be done entirely within the RON files.
*   **Extensibility:** Adding new, complex moves becomes a matter of composing existing instructions and traits, or adding new, small, self-contained traits.
*   **Power:** This model can accurately represent nearly any move from the official games, including those with complex sequential or contingent logic, without requiring custom one-off code in the engine.