# Status Conditions and Effects Design

## Overview

The Pokemon battle system uses a three-tier classification for conditions that affect Pokemon:

1. **Major Status** (`PokemonStatus`): Traditional status conditions that prevent or alter normal function
2. **Volatile Conditions** (`PokemonCondition`): Temporary battle-specific conditions with turn counters
3. **Battle Flags** (`PokemonFlag`): State markers that trigger effects but are not "cured"

This separation clarifies the distinction between conditions that can be healed/cured versus state flags that are simply removed when their trigger conditions end.

## Major Status Conditions (PokemonStatus)

Major status conditions follow the classic "one status per Pokemon" rule and persist until cured or the Pokemon faints.

### Status Definitions

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PokemonStatus {
    Sleep { turns_remaining: u8 },
    Poison { intensity: u8 },
    Burn,
    Paralysis,
    Freeze,
    Faint,
}
```

### Status Mechanics

#### `Sleep { turns_remaining: u8 }`
**Duration**: 1-7 turns (random on application)
**Effects**:
- Prevents move execution (StatusPreventedAction)
- No damage per turn
- Cannot be applied if Pokemon already has a major status
- Decremented *By StatusPreventedAction*, not at end of turn.
**Automatic Cure Conditions**:
- Using a move when turns_remaining == 0 (natural awakening)

#### `Poison { intensity: u8 }`
**Duration**: Until cured or Pokemon faints
**Effects**:
- **Normal Poison** (intensity = 0): Deals 1/8 max HP damage at end of turn
- **Badly Poisoned** (intensity > 0): Deals (intensity) / 16 max HP damage at end of turn
- Intensity increases by 1 each time damage is dealt if intensity is not zero.
- No move prevention
**Automatic Cure Conditions**:
- None

#### `Burn`  
**Duration**: Until cured or Pokemon faints
**Effects**:
- Deals 1/8 max HP damage at end of turn
- Reduces physical Attack damage by 50%
- No move prevention
**Automatic Cure Conditions**:
- None

#### `Paralysis`
**Duration**: Until cured or Pokemon faints
**Effects**:
- 25% chance to prevent move execution (StatusPreventedAction)
- Reduces Speed by 50% for turn order calculation
**Automatic Cure Conditions**:
- None

#### `Freeze`
**Duration**: Until cured or Pokemon faints or randomly defrosts
**Effects**:
- Prevents move execution completely (StatusPreventedAction)
- No damage per turn
**Automatic Cure Conditions**:
- 25% chance of defrosting each time it might prevent a move

#### `Faint`
**Duration**: Until revived
**Effects**:
- Pokemon cannot participate in battle
- Triggers forced replacement
**Automatic Cure Conditions**:
- None

### Status Application Rules

1. **Mutual Exclusivity**: Only one major status per Pokemon
2. **Override Priority**: New status applications fail if Pokemon already has major status
3. **Faint Override**: Faint status can override any other status
4. **Type Immunity**: Fire-types cannot be burned, Electric-types cannot be paralyzed, Ghost-types cannot be put to sleep, Ice-types cannot be frozen, Poison-types cannot be poisoned

## Volatile Conditions (PokemonCondition)

Volatile conditions are battle-specific effects with turn-based duration tracking. Conditions only remain as long as the pokemon is the active pokemon in the battle. 


### Turn-Based Conditions

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PokemonCondition {
    // Turn-based conditions (5 conditions with turns_remaining)
    Confused { turns_remaining: u8 },    // 1-4 turns
    Trapped { turns_remaining: u8 },     // 2-5 turns  
    Rampaging { turns_remaining: u8 },   // 2-3 turns (Thrash, Petal Dance)
    Disabled { turns_remaining: u8, move_index: u8 }, // 1-8 turns
    Biding { turns_remaining: u8, accumulated_damage: u16 }, // 2-3 turns
}
```

#### `Confused { turns_remaining: u8 }`
**Duration**: 2-5 turns (random on application)
**Effects**:
- 50% chance to hit self instead of using intended move
- Self-damage uses 40 power physical typeless attack
- `turns_remaining` decrements when the check is made to prevent action, regardless of the outcome
- Confusion falls off when `turns_remaining == 0` during the check is made to prevent action


#### `Trapped { turns_remaining: u8 }`
**Duration**: 2-5 turns (random on application)  
**Effects**:
- Prevents switching (blocks DoSwitch command conversion)
- Takes 1/16 max HP damage per turn
- Applied by Wrap, Bind, Clamp, Fire Spin
- `turns_remaining` decrements at end-of-turn, when damage is dealt, and is removed when `turns_remaining == 0`

#### `Rampaging { turns_remaining: u8 }`
**Duration**: 2-3 turns (random on application)
**Effects**:
- Forces use of same move (Continue command)
- Prevents switching or using items
- Applied by Thrash, Petal Dance, Outrage
- Prevents sleep
- If disrupted by paralysis or confusion, removed immediately.
- Decremented by re-application of the effect.
- If it is removed when `turns_remaining == 0`, applies Confused for 1-4 turns

#### `Disabled { turns_remaining: u8, move_index: u8 }`
**Duration**: 1-8 turns (random on application)
**Effects**:
- Specified move cannot be selected (command validation failure)
- Other moves remain available
- Applied by Disable move
- `turns_remaining` decrements at end-of-turn and is removed when `turns_remaining == 0`

#### `Biding { turns_remaining: u8, accumulated_damage: u16 }`
**Duration**: 2 turns (fixed on application)
**Effects**:
- Forces inaction while accumulating damage received
- When ends, deals 2x accumulated damage to opponent
- Cannot switch or use other moves during accumulation
- Decremented by re-application of the effect.
- Removed by Sleep

### Condition Application Rules

1. **Stackable**: Multiple volatile conditions can coexist (Though Bide and Rampaging are logically exclusive)
2. **Duration Tracking**: Each condition tracks its own turn counter

## Battle Flags (PokemonFlag)

Battle flags represent state markers that trigger specific behaviors but are not "cured" - they are simply removed when their triggering conditions end.

### Temporary State Flags

```rust
#[derive(Debug, Clone, PartialEq)]  
pub enum PokemonFlag {
    // Execution state flags
    Exhausted,        // Must recharge (Hyper Beam)
    Underground,      // Semi-invulnerable (Dig)
    InAir,           // Semi-invulnerable (Fly)  
    Charging,        // Preparing two-turn move (Solar Beam)
    // Battle effect flags  
    Flinched,        // Skip next move (one-turn effect)
    Seeded,          // Leech Seed effect (until Pokemon switches)
    // Transform flags (persistent until switch)
    Enraged,         // Increased critical hit rate
}

pub enum SpecialFlags {
    Converted { new_type: PokemonType },       // Type changed by Conversion
    Transformed { new_species: PokemonSpecies },     // Stats/type copied by Transform
    Mimicked {move_index: u8},        // Moveset modified by Mimic
    Substituted { hp: u16},     // Substitute active
    Countering { damage: u16 },      // Counter effect active (one turn)
}
```

### Flag Mechanics

#### Execution State Flags
- **`Exhausted`**: Removed by RequestBattleCommands after generating Continue{Recharge} command
- **`Underground/InAir/Charging`**: Removed when Prepare executes the strike.

#### One-Turn Effect Flags
- **`Flinched`**: Applied (silently) on Strikes, prevents moves for the rest of the turn, automatically removed at end of turn.
- **`Countering`**: Applied when Counter is used, removed at end of turn, dealing 2x physical damage reflection.

#### Persistent Effect Flags  
- **`Seeded`**: Drains HP each turn. No standard end condition.
- **`Enraged`**: Causes pokemon's Atk to increase each time it is damaged by a strike until last_move changes.
- **`Converted/Transformed/Mimic`**: Modify state. No standard end condition.
- **`Substitute`**: Consumes 25% of max HP on use, creates a substitute that absorbs all effects and damage until hp is zero.

### Flag vs Condition Distinction

**Flags are NOT cured**:
- Removed only when their trigger condition ends
- No "cure" moves or items affect them
- State markers rather than afflictions

**Conditions CAN be cured**:
- Have specific cure conditions beyond natural expiration
- Considered negative afflictions in most cases
- Can be removed by healing moves, items, or abilities
- Bide is, admittedly, weird.

### Status Effect Integration

Status effects modify battle calculations throughout the system:

- **Burn**: Reduces physical attack by 50%
- **Paralysis**: 50% speed reduction
- **Sleep/Freeze/Paralysis**: Trigger StatusPreventedAction during command conversion
- **Confusion**: 50% chance triggers self-damage instead of intended move

## Integration with Battle System Architecture

### Storage Location Design

**Major Status on PokemonInst**:
```rust
// In PokemonInst
pub status: Option<PokemonStatus>  // Sleep, Poison, Burn, Paralysis, Freeze, Faint
```
- Stored with Pokemon instance for persistence
- Survives battle context changes and switching
- Affects Pokemon's fundamental state and abilities

**Volatile State on BattleState**:
```rust
// In BattleState struct
pub active_conditions: [PokemonConditionSet; 2]  // Per-player condition sets
pub simple_flags: [PokemonFlagSet; 2]           // Per-player simple flags
pub special_flags: [SpecialFlagSet; 2]          // Per-player complex flags
```
- Battle-specific temporary state
- Resets when Pokemon switches or battle ends
- Optimized storage with direct field access instead of HashMaps

### Condition Storage Structures

```rust
pub struct PokemonConditionSet {
    pub confused: Option<u8>,                 // turns_remaining
    pub trapped: Option<u8>,                  // turns_remaining  
    pub rampaging: Option<u8>,                // turns_remaining
    pub disabled: Option<(u8, u8)>,          // (move_index, turns_remaining)
    pub biding: Option<(u8, u16)>,           // (turns_remaining, accumulated_damage)
}

pub struct PokemonFlagSet {
    pub exhausted: bool,     // Must recharge (Hyper Beam)
    pub underground: bool,   // Semi-invulnerable (Dig)
    pub in_air: bool,        // Semi-invulnerable (Fly)  
    pub charging: bool,      // Preparing two-turn move (Solar Beam)
    pub flinched: bool,      // Skip next move (one-turn effect)
    pub seeded: bool,        // Leech Seed effect (until Pokemon switches)
    pub enraged: bool,       // Gains Atk when hit
    pub blinked: bool,       // Can't be hit until end-of-turn

}

pub struct SpecialFlagSet {
    pub converted: Option<PokemonType>,  // Type changed by Conversion
    pub transformed: Option<Species>,    // Species copied by Transform
    pub substituted: Option<u16>,        // substitute HP remaining
    pub countering: Option<u16>,         // damage to counter
}
```

### Access Patterns

**Status Checking** (from PokemonInst):
```rust
// Check major status on Pokemon
let pokemon = battle.get_active_pokemon(player_index);
if matches!(pokemon.status, Some(PokemonStatus::Sleep { .. })) {
    // Handle sleep prevention
}
```

**Condition Checking** (from BattleState):
```rust
// Check volatile conditions on BattleState
let conditions = &battle.battle_state.active_conditions[player_index];
if conditions.confused.is_some() {
    // Handle confusion check
}

// Check simple flags
if battle.battle_state.simple_flags[player_index].flinched {
    // Handle flinch prevention
}
```

**Command Validation Integration**:
```rust
// Check if move is disabled
if let Some((disabled_index, _)) = battle.battle_state.active_conditions[player_index].disabled {
    if disabled_index == move_index {
        return CommandValidationResult::Invalid("Move is disabled");
    }
}

// Check if trapped
if battle.battle_state.active_conditions[player_index].trapped.is_some() {
    return CommandValidationResult::Prevented;
}
```

### Turn Processing

**Status Progression** (PokemonInst):
- Sleep turn counter decremented by StatusPreventedAction
- Poison intensity increased each turn for badly poisoned
- Status damage calculated from Pokemon's own methods

**Condition Progression** (BattleState):
- Trapped/Disabled: Decremented at end-of-turn
- Confused: Decremented when prevention check is made
- Rampaging/Biding: Decremented by re-application of effect
- One-turn flags (Flinched, Countering): Cleared at end-of-turn

## Transformation Effects: Converted, Transformed, and Mimic

### Overview

Conversion, Transform, and Mimic represent the most complex status effects in the battle system due to their ability to fundamentally alter how Pokemon data is interpreted during battle. These effects are stored as special flags in BattleState and require careful integration with species data lookup and move system management.

### Converted Flag

```rust
// In SpecialFlagSet
pub converted: Option<PokemonType>  // Type changed by Conversion
```

**Mechanism**: Conversion changes the Pokemon's type to match its first move's type.

**Implementation**:
- Stores the new type in `SpecialFlagSet.converted`
- Overrides species type data during battle calculations
- Simple single-value storage, no complex state management required

**Key Design Decision**: Conversion takes priority over Transform for type determination, because Transform overrides the pokemon's *species*, while Conversion overrides the pokemon's *type*.

### Transformed Flag

```rust
// In SpecialFlagSet
pub transformed: Option<Species>  // Species copied by Transform
```

**Mechanism**: Transform copies the target's species, base stats, types, and moveset with current PP.

**What Transform Copies**:
- **Species Identity**: For species data lookup
- **Base Stats**: Recalculated using transformed species base stats + user's level/IVs/EVs
- **Types**: From transformed species data (unless overridden by Conversion)
- **Moveset**: Copied to temporary move slots with current PP
- **Stat Stages**: Copied from target at time of transformation

**What Transform Does NOT Copy**:
- Level, IVs, EVs (uses original Pokemon's values)
- Current HP (retains original HP total)
- Major status conditions (Sleep, Poison, etc.)
- Volatile conditions and flags (Confusion, etc.)

**Implementation Strategy**:
```rust
impl Battle {
    pub fn apply_transform(&mut self, user_player: u8, target_player: u8) {
        let target_pokemon = self.get_active_pokemon(target_player);
        let target_species = target_pokemon.species;
        
        // 1. Set Transform flag
        self.battle_state.special_flags[user_player].transformed = Some(target_species);
        
        // 2. Copy stat stages from target
        let target_stat_stages = self.battle_state.stat_stages[target_player];
        self.battle_state.stat_stages[user_player] = target_stat_stages;
        
        // 3. Copy moveset to temporary moves
        self.copy_moveset_to_temp(user_player, target_player);
    }
    
    pub fn get_current_stats(&self, player_index: u8) -> CurrentStats {
        let pokemon = self.get_active_pokemon(player_index);
        
        if let Some(transformed_species) = self.battle_state.special_flags[player_index].transformed {
            // Use transformed species base stats with original Pokemon's level/IVs/EVs
            let species_data = get_species_data(transformed_species).unwrap();
            let base_stats = PokemonInst::calculate_stats(
                &species_data.base_stats,
                pokemon.level,
                &pokemon.ivs,
                &pokemon.evs
            );
            
            // Apply current stat stage modifications
            self.apply_stat_stage_multipliers(base_stats, player_index)
        } else {
            // Normal case: use original Pokemon stats
            let base_stats = pokemon.stats;
            self.apply_stat_stage_multipliers(base_stats, player_index)
        }
    }
}
```

**Architecture Benefit**: By storing only the target species and calculating everything else on-demand, Transform avoids complex state management while providing full transformation functionality.

### Temporary Move System (Transform and Mimic)

```rust
// In BattleState
pub temporary_moves: [TempMoveSet; 2]

pub struct TempMoveSet {
    pub moves: [Option<TempMoveInstance>; 4],
}

pub struct TempMoveInstance {
    pub move_: Move,    // The temporary move
    pub pp: u8,         // Current PP for this move
}
```

**Transform Move Copying**:
- Copies all 4 moves from target Pokemon
- PP values copied as-is from target's current PP
- Completely replaces user's original moveset during battle
- Original moveset restored when Pokemon switches

**Mimic Move Learning**:
- Copies single move from target's last used move
- Learned move gets 5 PP regardless of original PP
- Replaces one of the user's existing moves (specified by move index)
- Learned move persists until Pokemon switches

**Move Access Integration**:
```rust
impl Battle {
    pub fn get_available_moves(&self, player_index: u8) -> [Option<(Move, u8)>; 4] {
        let temp_moves = &self.battle_state.temporary_moves[player_index];
        
        // Check if any temporary moves exist
        if temp_moves.moves.iter().any(|m| m.is_some()) {
            // Use temporary moveset (Transform or Mimic modifications)
            temp_moves.moves.map(|temp_move| {
                temp_move.map(|tm| (tm.move_, tm.pp))
            })
        } else {
            // Use original Pokemon moveset
            let pokemon = self.get_active_pokemon(player_index);
            pokemon.moves.map(|move_inst| {
                move_inst.map(|mi| (mi.move_, mi.pp))
            })
        }
    }
    
    pub fn use_move(&mut self, player_index: u8, move_: Move) -> Result<(), UseMoveError> {
        // Check temporary moves first
        let temp_moves = &mut self.battle_state.temporary_moves[player_index];
        for temp_move_slot in &mut temp_moves.moves {
            if let Some(temp_move) = temp_move_slot {
                if temp_move.move_ == move_ {
                    if temp_move.pp > 0 {
                        temp_move.pp -= 1;
                        return Ok(());
                    } else {
                        return Err(UseMoveError::NoPPRemaining);
                    }
                }
            }
        }
        
        // Fall back to original Pokemon moveset
        let pokemon = self.get_active_pokemon_mut(player_index);
        pokemon.use_move(move_)
    }
}
```

### Architectural Advantages

**Clean Separation**:
- Base Pokemon data remains unmodified
- Battle system handles all transformation logic
- No circular dependencies between Pokemon and Player structures

**On-Demand Calculation**:
- Stats calculated when needed using appropriate base stats
- Types determined by checking flags in priority order
- Moves accessed through unified interface that checks temporary slots first

**Efficient Storage**:
- Transform stores only target species (4-8 bytes)
- Conversion stores only new type (1-4 bytes)  
- Temporary moves use fixed-size arrays, no dynamic allocation

**Type Safety**:
- Species enum prevents invalid transformation targets
- Move enum ensures only valid moves in temporary slots
- Clear flag precedence prevents conflicting transformations

**Performance**:
- Direct field access instead of HashMap lookups
- Cache-friendly data layout with related flags grouped together
- Minimal branching in hot paths (type/stat calculation)

### Edge Cases and Interactions

**Transform + Conversion**:
- Conversion type takes precedence over Transform type
- Both flags can coexist, Conversion checked first

**Multiple Transforms**:
- New Transform overwrites previous Transform
- Stat stages copied from current target, not accumulated

**Switch-Out Behavior**:
- All transformation flags cleared when Pokemon switches
- Original moveset and stats automatically restored
- Temporary moves cleared, PP changes to temporary moves lost

**Struggle Integration**:
- If all temporary moves have 0 PP, Pokemon uses Struggle
- Struggle bypasses temporary move system entirely
- Original moveset PP not affected by temporary move usage

This transformation system elegantly handles the most complex status effects while maintaining clean architecture and optimal performance through the BattleState design.

## Design Principles

### Clear Categorization
- **Major Status**: Traditional Pokemon status conditions with mutual exclusivity
- **Volatile Conditions**: Turn-based battle conditions that can stack
- **Battle Flags**: State markers removed by condition changes, not curing


### Type Safety
- Enum-based condition types prevent invalid condition construction
- Clear flag vs condition distinction eliminates cure/removal confusion

