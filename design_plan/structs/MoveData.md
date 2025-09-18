# Move Data Design

## Overview

The `MoveData` struct defines the complete behavior of Pokemon moves using a **modified legacy approach** that balances simplicity with comprehensive effect coverage. This design prioritizes clear timing semantics and atomic execution within the coarse-grained ActionStack architecture.

## Design Philosophy

### Why Not Complex Scripting?

The rejected scripting approach (see `RejectedMoveData.md`) attempted to treat moves as sequential instruction lists with granular actions like `StrikeAction` and `PassiveAction`. This created several problems:

- **Timing Fragility**: Complex instruction sequencing made move behavior unpredictable
- **Action Explosion**: Every effect became a separate action, creating a "super-enum" problem
- **Maintenance Overhead**: Move logic spread across multiple instruction types and effect handlers
- **Execution Complexity**: Multi-layer indirection between instructions, actions, and actual effects

### The Modified Legacy Approach

Instead, we use a **flat effect list** with **implicit timing based on effect type**. This approach:

- **Leverages Proven Design**: Based on the working legacy system with targeted improvements
- **Natural Timing**: Most effects have obvious timing (damage calculation, on-hit, automatic)
- **Atomic Execution**: All effects processed within `DoMove`'s single action execution
- **Clear Semantics**: Effect behavior is obvious from the effect type
- **Comprehensive Coverage**: Supports all Pokemon move mechanics with explicit timing control

## Core Structure

```rust
pub struct MoveData {
    pub name: String,
    pub move_type: PokemonType,
    pub power: Option<u8>,        // None for non-damaging moves
    pub accuracy: Option<u8>,     // None for sure-hit moves  
    pub max_pp: u8,
    pub priority: i8,             // Turn order priority (-7 to +5)
    pub category: MoveCategory,   // Physical, Special, or Status
    pub effects: Vec<MoveEffect>, // Ordered list of move effects
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveCategory {
    Physical,  // Uses Attack vs Defense
    Special,   // Uses Special Attack vs Special Defense  
    Status,    // Non-damaging moves
}
```

## Effect Timing System

### Implicit Timing Categories

Effects are processed in a specific order during `DoMove` execution, with timing determined by effect type:

1. **Damage Calculation Effects**: Modify how damage is calculated (before accuracy check)
2. **Accuracy/Hit Resolution**: Determine if move hits
3. **On-Hit Effects**: Only execute if move hits target
4. **On-Damage Effects**: Only execute if damage was actually dealt
5. **Multi-Turn Effects**: Set up or resolve state-dependent behavior
6. **Automatic Effects**: Always execute regardless of hit/miss
7. **Miss-Only Effects**: Only execute if move missed

### Explicit Timing Control

Only one category requires explicit timing control:

- **`OnHit(SecondaryEffect)`**: Effects that only trigger when move hits
- **`Automatic(GuaranteedEffect)`**: Effects that always happen after move execution

This solves the primary ambiguity in the legacy system (e.g., Ancient Power's stat boost only on hit vs Overheat's stat drop regardless of hit/miss).

## Complete Effect System

```rust
pub enum MoveEffect {
    // Damage calculation effects (applied during damage calculation)
    FixedDamage(u16),           // Sonic Boom (20), Dragon Rage (40)
    LevelDamage,                // Seismic Toss, Night Shade
    PercentHpDamage(u8),        // Super Fang (50% of current HP)
    CritRatio(u8),              // Increased crit ratio (Slash, Karate Chop)
    PartialIgnoreDefense(u8),   // % chance to ignore defense
    MultiHit(u8, u8),           // min hits, % continuation chance
    Lethal,                     // OHKO moves (Fissure, Horn Drill) 
    SureHit,                    // Never misses (Swift)
    
    // On-damage effects (only apply if damage is dealt)
    Drain(u8),                  // % of damage healed to user (Absorb, Giga Drain)
    Recoil(u8),                 // % of damage dealt as self-damage (Take Down)
    
    // Multi-turn/condition effects (state-dependent behavior)  
    ApplyFlag(Target, PokemonFlagType), // Set battle flags (Charging, InAir, Underground)
    RemoveFlag(Target, PokemonFlagType), // Remove battle flags
    Bide,                       // Begin damage accumulation for 2-3 turns
    
    // Miss-only effects
    Reckless(u8),              // Self-damage % on miss (Jump Kick, Hi Jump Kick)
    
    // On-hit effects (only apply if move hits)
    OnHit(SecondaryEffect),
    
    // Automatic effects (always happen after move execution) 
    Automatic(GuaranteedEffect),
}

pub enum SecondaryEffect {
    // Major Status conditions (stored on PokemonInst)
    ApplyStatus(Target, PokemonStatusType, u8),     // Burn, paralysis, etc. with chance
    RemoveStatus(Target, PokemonStatusType, u8),    // Remove status with chance
    CureStatus(Target, PokemonStatusType),          // Remove status (100% chance)
    
    // Volatile conditions (stored on BattleState) 
    ApplyCondition(Target, PokemonConditionType, u8), // Confusion, trap, etc.
    RemoveCondition(Target, PokemonConditionType, u8), // Remove condition
    
    // Battle flags (stored on BattleState)
    ApplyFlag(Target, PokemonFlagType),             // Flinch, seeded, etc.
    RemoveFlag(Target, PokemonFlagType),            // Remove flags
    
    // Stat modifications (on-hit only)
    StatChange(Target, BattleStat, i8, u8),        // Target, stat, stages, chance
    
    // Special mechanics (on-hit only)
    Transform(Target),                              // Transform (can miss/be blocked)
    RequiresStatus(Target, PokemonStatusType),     // Only works if target has status (Dream Eater)
}

pub enum GuaranteedEffect {
    // Self-effects (always happen)
    StatChange(Target, BattleStat, i8, u8),        // Overheat (-2 SpAtk), Swords Dance (+2 Atk)
    Heal(u8),                                      // Recover, Roost (% of max HP)
    Rest,                                          // Full heal + sleep status
    CureStatus(Target, PokemonStatusType),         // Aromatherapy, Heal Bell
    ClearStatus(Target),                           // Remove any major status
    ClearAllStatChanges(u8),                       // Haze, Clear Smog
    Suicide,                                       // Explosion, Self-Destruct
    
    // Team/field effects  
    ApplyTeamCondition(TeamConditionType),         // Reflect, Light Screen, Mist
    
    // Special mechanics (guaranteed to attempt)
    Conversion,                                    // Change type to match first move
    Substitute(u8),                               // Create substitute with % HP cost  
    Counter,                                      // Set up counter reflection
    MirrorMove,                                   // Copy opponent's last move
    Mimic,                                        // Temporarily learn opponent's last move
    Metronome,                                    // Randomly execute any move
    Flicker(u8),                                  // Teleport/evasion effect
    AnteUp(u8),                                   // Pay Day (scatter coins)
}
```

## Target System

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    User,      // Effect applies to the move user
    Target,    // Effect applies to the move target
}
```

Effects specify their target explicitly, enabling moves that affect the user (Swords Dance), the opponent (Thunder Wave), or conditionally either (some Counter-type moves).

## Example Move Definitions

### Simple Damaging Move: Tackle
```rust
MoveData {
    name: "Tackle".to_string(),
    move_type: PokemonType::Normal,
    power: Some(40),
    accuracy: Some(100), 
    max_pp: 35,
    priority: 0,
    category: MoveCategory::Physical,
    effects: vec![], // No additional effects
}
```

### Move with On-Hit Effect: Thunder Punch
```rust  
MoveData {
    name: "Thunder Punch".to_string(),
    move_type: PokemonType::Electric,
    power: Some(75),
    accuracy: Some(100),
    max_pp: 15, 
    priority: 0,
    category: MoveCategory::Physical,
    effects: vec![
        MoveEffect::OnHit(SecondaryEffect::ApplyStatus(
            Target::Target, 
            PokemonStatusType::Paralysis, 
            10 // 10% chance
        ))
    ],
}
```

### Move with Guaranteed Effect: Overheat
```rust
MoveData {
    name: "Overheat".to_string(),
    move_type: PokemonType::Fire,
    power: Some(130),
    accuracy: Some(90),
    max_pp: 5,
    priority: 0, 
    category: MoveCategory::Special,
    effects: vec![
        MoveEffect::Automatic(GuaranteedEffect::StatChange(
            Target::User,
            BattleStat::SpAtk, 
            -2,  // -2 stages
            100  // Always happens
        ))
    ],
}
```

### Multi-Hit Move: Fury Swipes
```rust
MoveData {
    name: "Fury Swipes".to_string(),
    move_type: PokemonType::Normal,
    power: Some(18),
    accuracy: Some(80),
    max_pp: 15,
    priority: 0,
    category: MoveCategory::Physical, 
    effects: vec![
        MoveEffect::MultiHit(2, 50) // 2-5 hits, 50% continuation chance
    ],
}
```

### Complex Move: Absorb  
```rust
MoveData {
    name: "Absorb".to_string(),
    move_type: PokemonType::Grass,
    power: Some(20),
    accuracy: Some(100),
    max_pp: 25,
    priority: 0,
    category: MoveCategory::Special,
    effects: vec![
        MoveEffect::Drain(50) // Heal 50% of damage dealt
    ],
}
```

### Multi-Turn Move: Solar Beam
```rust
MoveData {
    name: "Solar Beam".to_string(), 
    move_type: PokemonType::Grass,
    power: Some(120),
    accuracy: Some(100),
    max_pp: 10,
    priority: 0,
    category: MoveCategory::Special,
    effects: vec![
        MoveEffect::ApplyFlag(Target::User, PokemonFlagType::Charging)
    ],
}
```

## Execution Flow

### DoMove Processing Order

When `DoMove` executes, effects are processed in this order:

1. **Check State-Dependent Effects**: Multi-turn moves check flags (Charging, InAir, etc.)
2. **Process Damage Calculation Effects**: Modify power, accuracy, hit count
3. **Resolve Accuracy**: Determine if move hits (considering SureHit)  
4. **Calculate Damage**: Apply damage calculation effects, deal damage
5. **Apply On-Damage Effects**: Drain, Recoil (only if damage dealt)
6. **Apply On-Hit Effects**: Secondary effects (only if move hit)
7. **Apply Automatic Effects**: Guaranteed effects (always happen)  
8. **Apply Miss Effects**: Reckless (only if move missed)
9. **Check for Knockouts**: Push HandleKnockout if needed

### Effect Storage Locations

Effects modify different parts of the battle state:

- **Major Status**: Stored on `PokemonInst.status` (persistent across battles)
- **Volatile Conditions**: Stored on `BattleState.active_conditions` (battle-specific)
- **Battle Flags**: Stored on `BattleState.simple_flags` (temporary battle markers)
- **Stat Stages**: Stored on `BattleState.stat_stages` (temporary stat modifications)
- **Team Conditions**: Stored on `BattleState.team_conditions` (team-wide effects)

## Benefits

### Compared to Legacy System
- **Explicit Timing**: OnHit vs Automatic removes ambiguity
- **Priority Support**: Added priority field for turn order
- **Comprehensive Effects**: Covers all Pokemon move mechanics
- **Better Target Control**: Explicit Target enum for effect targeting

### Compared to Scripting System  
- **Simplicity**: Single effect list instead of complex instruction sequences
- **Performance**: No action stack overhead for individual effects
- **Maintainability**: Move logic contained in single structure
- **Predictability**: Effect timing is clear and consistent  
- **Atomic Execution**: All effects processed within single DoMove action

This design provides the flexibility needed for Pokemon's complex move system while maintaining the simplicity and performance of the proven legacy approach.