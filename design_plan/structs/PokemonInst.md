# PokemonInst Design

## Overview

`PokemonInst` represents a specific Pokemon instance with individual characteristics like level, stats, moves, and current battle state. This document outlines the current implementation and necessary updates to align with the new status condition system.

## Current PokemonInst Structure

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct PokemonInst {
    pub name: String,                        // Display name (species name if no nickname)
    pub species: Species,                    // Species enum for data lookup
    pub level: u8,                          // Level 1-100
    pub curr_exp: u32,                      // Experience points (single-player progression)
    curr_hp: u16,                           // Current HP (private, accessed via methods)
    pub ivs: [u8; 6],                       // Individual Values: HP, ATK, DEF, SP.ATK, SP.DEF, SPD
    pub evs: [u8; 6],                       // Effort Values: HP, ATK, DEF, SP.ATK, SP.DEF, SPD
    pub stats: CurrentStats,                // Calculated current stats
    pub moves: [Option<MoveInstance>; 4],   // Up to 4 moves with PP tracking
    pub status: Option<PokemonStatus>,    // Major status condition
}
```

## Status System Integration

### PokemonStatus Definition

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PokemonStatus {
    Sleep { turns_remaining: u8 },
    Poison { intensity: u8 },         // 0 = normal, >0 = badly poisoned
    Burn,
    Paralysis,
    Freeze,
    Faint,
}
```

## Component Structures

### CurrentStats
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub struct CurrentStats {
    pub hp: u16,        // Maximum HP
    pub attack: u16,    // Attack stat
    pub defense: u16,   // Defense stat  
    pub sp_attack: u16, // Special Attack stat
    pub sp_defense: u16,// Special Defense stat
    pub speed: u16,     // Speed stat
}
```

**Characteristics**:
- Calculated from base stats, level, IVs, and EVs using Gen 1 formulas
- Recalculated when level, EVs, or species changes (evolution)
- Used for damage calculations and turn order determination

### MoveInstance
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct MoveInstance {
    pub move_: Move,    // Move enum variant
    pub pp: u8,         // Current Power Points remaining
}
```

**Functionality**:
- Tracks PP consumption for individual move usage
- Provides max PP lookup from move data
- Handles PP restoration and usage validation
- Supports special moves like Struggle that don't consume PP

## Key Methods and Behavior

### Creation and Initialization

#### `PokemonInst::new()`
- Creates Pokemon from Species data and level
- Derives moves from learnset (last 4 moves learned by level)
- Calculates stats using Gen 1 formulas
- Initializes with full HP and no status conditions

#### `PokemonInst::new_with_hp()`
- Variant that allows specific starting HP
- Used for creating Pokemon with reduced health
- Validates HP doesn't exceed maximum

#### `PokemonInst::new_for_test()`
- Test-specific constructor with direct stat control
- Bypasses normal stat calculation for predictable testing
- Maintains backward compatibility with existing test suite

### Stat Management

#### `calculate_stats()`
**Gen 1 Stat Formulas**:
- **HP**: `((2 * base + IV + EV/4) * level / 100) + level + 10`
- **Other Stats**: `((2 * base + IV + EV/4) * level / 100) + 5`

#### `recalculate_stats()`
- Updates stats when level, EVs, or species changes
- Preserves HP ratio when max HP increases (except for fainted Pokemon)
- Used by level up, evolution, and EV gain methods

### Battle State Management

#### HP Management
```rust
pub fn current_hp(&self) -> u16           // Get current HP
pub fn max_hp(&self) -> u16              // Get maximum HP from stats
pub fn set_hp(&mut self, hp: u16)        // Set HP with validation
pub fn take_damage(&mut self, damage: u16) -> bool  // Apply damage, returns if fainted
pub fn heal(&mut self, amount: u16)       // Restore HP (doesn't revive)
pub fn revive(&mut self, hp_amount: u16)  // Revive fainted Pokemon
```

### Progression and Development

#### Experience and Leveling
```rust
pub fn add_experience(&mut self, amount: u32) -> Option<u8>  // Returns new level if leveled up
pub fn apply_level_up(&mut self)                            // Increases level and recalculates stats
```

#### Evolution Support
```rust
pub fn evolve(&mut self, new_species: Species)  // Changes species and recalculates stats
```

#### EV Training
```rust
pub fn add_evs(&mut self, ev_gains: [u8; 6])   // Adds EVs with 255/stat and 510 total limits
```

### Move System Integration

#### Move Usage
```rust
pub fn get_move(&mut self, move_index: u8) -> Result<(Move), MoveNotFound>
```

**Special Cases**:
- Struggle and HittingItself bypass PP system
- Returns specific errors for no PP vs move not known
- Handles PP deduction automatically

#### Learnset Integration
```rust
fn derive_moves_from_learnset(learnset: &Learnset, level: u8) -> [Option<MoveInstance>; 4]
```

**Behavior**:
- Scans level-up learnset from level 1 to current level
- Returns last 4 moves learned (most recent moveset)
- Used only during creation when no explicit moves are provided
