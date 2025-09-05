# Battle State Design

## Overview

The `BattleState` struct provides efficient storage for all temporary, battle-specific state that doesn't belong in the persistent `Player` or `PokemonInst` structures. This design prioritizes memory efficiency and cache performance by using tight enum-based storage instead of HashMaps.

## Core Architecture

### Battle Structure Integration

```rust
pub struct Battle {
    pub players: Vec<Player>,        // Persistent player data
    pub battle_state: BattleState,   // All temporary battle state
    // ... other battle fields
}

pub struct BattleState {
    // All per-player state stored in arrays [player_0, player_1]
    pub active_pokemon_indices: [u8; 2],
    pub team_conditions: [TeamConditionSet; 2],
    pub stat_stages: [StatStageSet; 2], 
    pub last_moves: [Option<Move>; 2],
    pub active_conditions: [PokemonConditionSet; 2],
    pub simple_flags: [PokemonFlagSet; 2],
    pub special_flags: [SpecialFlagSet; 2],
    pub temporary_moves: [TempMoveSet; 2],

    // Shared data
    pub scattered_coins: u16,
}
```

## Component Structures

### Simple Binary Flags

```rust

pub enum SimpleFlagType {
    Exhausted,
    Underground,
    InAir,
    Charging,
    Flinched,
    Seeded,
    Enraged,
    Blinked,
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
```

**Characteristics**:
- 8 boolean flags stored as individual fields
- Zero overhead compared to 8 separate bool variables
- Direct field access, no hashing or lookup required
- Cache-friendly: all flags fit in single cache line

### Complex Flags with Data

```rust
pub struct SpecialFlagSet {
    pub converted: Option<PokemonType>,  // Type changed by Conversion
    pub transformed: Option<Species>,    // Species copied by Transform
    pub substituted: Option<u16>,       // substitute HP remaining
    pub countering: Option<u16>,        // damage to counter
}
```

**Design Notes**:
- Each complex effect stored as `Option<T>` for presence + data
- Transform stores only `Species` enum, not full `PokemonSpecies` data
- Counter stores damage amount for end-of-turn reflection

### Volatile Conditions

```rust

pub enum PokemonConditionType {
    Confused,
    Trapped,
    Rampaging,
    Disabled, // Note: The move index would be a parameter on the action
    Biding,
}

pub struct PokemonConditionSet {
    pub confused: Option<u8>,                 // turns_remaining
    pub trapped: Option<u8>,                  // turns_remaining  
    pub rampaging: Option<u8>,                // turns_remaining
    pub disabled: Option<(u8, u8)>,        // (move_index, turns_remaining)
    pub biding: Option<(u8, u16)>,            // (turns_remaining, accumulated_damage)
}
```

**Condition-Specific Data**:
- **Confused/Trapped/Rampaging**: Simple turn counters
- **Disabled**: Tracks which move is disabled + duration
- **Biding**: Tracks both duration and accumulated damage for release

### Team Conditions

```rust
pub struct TeamConditionSet {
    pub reflect: Option<u8>,      // turns_remaining
    pub light_screen: Option<u8>, // turns_remaining
    pub mist: Option<u8>,         // turns_remaining
}
```

**Team-Wide Effects**:
- **Reflect**: 50% physical damage reduction
- **Light Screen**: 50% special damage reduction  
- **Mist**: Prevents stat stage reductions
- All conditions track remaining turn duration

### Stat Stage Modifications

```rust
pub struct StatStageSet {
    pub attack: i8,      // -6 to +6
    pub defense: i8,     // -6 to +6
    pub sp_attack: i8,   // -6 to +6
    pub sp_defense: i8,  // -6 to +6
    pub speed: i8,       // -6 to +6
    pub accuracy: i8,    // -6 to +6
    pub evasion: i8,     // -6 to +6
}

impl Default for StatStageSet {
    fn default() -> Self {
        Self {
            attack: 0, defense: 0, sp_attack: 0, sp_defense: 0,
            speed: 0, accuracy: 0, evasion: 0,
        }
    }
}
```

**Stat Stage Rules**:
- Each stat can be modified -6 to +6 stages
- 0 represents unmodified base value
- Positive values increase effectiveness
- Negative values decrease effectiveness
- Reset when Pokemon switches out

### Temporary Move System

```rust
pub struct TempMoveSet {
    pub moves: [Option<TempMoveInstance>; 4],
}

pub struct TempMoveInstance {
    pub move_: Move,    // The temporary move
    pub pp: u8,         // Current PP for this move
}
```

**Temporary Move Mechanics**:
- **Transform**: Copies all 4 moves from target Pokemon with current PP
- **Mimic**: Adds single move to temporary set with 5 PP
- **Slot Management**: Transform overwrites all slots, Mimic uses first available slot
- **PP Tracking**: Independent PP counter for each temporary move

## Access Pattern Implementation

### Flag Access Methods

```rust
impl BattleState {
    pub fn is_simple_flag_set(&self, player_index: u8, flag: PokemonFlag) -> bool {
        let flags = &self.simple_flags[player_index];
        match flag {
            PokemonFlag::Exhausted => flags.exhausted,
            PokemonFlag::Underground => flags.underground,
            PokemonFlag::InAir => flags.in_air,
            PokemonFlag::Charging => flags.charging,
            PokemonFlag::Flinched => flags.flinched,
            PokemonFlag::Seeded => flags.seeded,
            PokemonFlag::Enraged => flags.enraged,
        }
    }
    
    pub fn set_simple_flag(&mut self, player_index: u8, flag: PokemonFlag, value: bool) {
        let flags = &mut self.simple_flags[player_index];
        match flag {
            PokemonFlag::Exhausted => flags.exhausted = value,
            PokemonFlag::Underground => flags.underground = value,
            PokemonFlag::InAir => flags.in_air = value,
            PokemonFlag::Charging => flags.charging = value,
            PokemonFlag::Flinched => flags.flinched = value,
            PokemonFlag::Seeded => flags.seeded = value,
            PokemonFlag::Enraged => flags.enraged = value,
        }
    }
}
```

### Special Flag Access Methods

```rust
impl BattleState {
    pub fn get_transformed_species(&self, player_index: u8) -> Option<Species> {
        self.special_flags[player_index].transformed
    }
    
    pub fn set_transform(&mut self, player_index: u8, target_species: Species) {
        self.special_flags[player_index].transformed = Some(target_species);
    }
    
    pub fn get_converted_type(&self, player_index: u8) -> Option<PokemonType> {
        self.special_flags[player_index].converted
    }
}
```

### Stat Stage Management

```rust
impl BattleState {
    pub fn modify_stat_stage(&mut self, player_index: u8, stat: StatType, delta: i8) {
        let stages = &mut self.stat_stages[player_index];
        match stat {
            StatType::Attack => stages.attack = (stages.attack + delta).clamp(-6, 6),
            StatType::Defense => stages.defense = (stages.defense + delta).clamp(-6, 6),
            StatType::SpAttack => stages.sp_attack = (stages.sp_attack + delta).clamp(-6, 6),
            StatType::SpDefense => stages.sp_defense = (stages.sp_defense + delta).clamp(-6, 6),
            StatType::Speed => stages.speed = (stages.speed + delta).clamp(-6, 6),
            StatType::Accuracy => stages.accuracy = (stages.accuracy + delta).clamp(-6, 6),
            StatType::Evasion => stages.evasion = (stages.evasion + delta).clamp(-6, 6),
        }
    }
    
    pub fn get_stat_stage(&self, player_index: u8, stat: StatType) -> i8 {
        let stages = &self.stat_stages[player_index];
        match stat {
            StatType::Attack => stages.attack,
            StatType::Defense => stages.defense,
            StatType::SpAttack => stages.sp_attack,
            StatType::SpDefense => stages.sp_defense,
            StatType::Speed => stages.speed,
            StatType::Accuracy => stages.accuracy,
            StatType::Evasion => stages.evasion,
        }
    }
}
```

## Transform Integration

### Current Stats Calculation (Transform Integration)

```rust
impl Battle {
    pub fn get_current_stats(&self, player_index: u8) -> CurrentStats {
        let pokemon = self.get_active_pokemon(player_index);
        
        // Check for Transform effect
        if let Some(target_species) = self.battle_state.get_transformed_species(player_index) {
            // Use target species base stats with current Pokemon's level/IVs/EVs
            let target_species_data = get_species_data(target_species)?;
            PokemonInst::calculate_stats(
                &target_species_data.base_stats,
                pokemon.level,
                &pokemon.ivs,
                &pokemon.evs
            )
        } else {
            // Normal case: use Pokemon's original stats
            pokemon.stats
        }
    }
    
    pub fn get_current_stat(&self, player_index: u8, stat: StatType) -> u16 {
        let pokemon = self.get_active_pokemon(player_index);
        
        if let Some(transformed_species) = self.battle_state.special_flags[player_index].transformed {
            if stat == StatType::HP {
                // HP never affected by Transform - use original
                pokemon.stats.hp
            } else {
                // Calculate stat using transformed species base stats + original Pokemon's level/IVs/EVs
                let species_data = get_species_data(transformed_species).unwrap();
                PokemonSpecies::calculate_stat(
                    &species_data.base_stats, 
                    stat, 
                    pokemon.level, 
                    pokemon.ivs[stat.index()], 
                    pokemon.evs[stat.index()]
                )
            }
        } else {
            // Normal case - use Pokemon's stored current stats
            pokemon.stats.get_stat(stat)
        }
    }
    
    pub fn get_effective_stat(&self, player_index: u8, stat: StatType) -> u16 {
        // Get current stat (accounting for Transform)
        let current_stat = self.get_current_stat(player_index, stat);
        
        // Apply stat stage modifications
        let stage = self.battle_state.get_stat_stage(player_index, stat);
        self.apply_stat_stage_multiplier(current_stat, stage)
    }
    
    pub fn get_effective_species(&self, player_index: u8) -> Species {
        self.battle_state.get_transformed_species(player_index)
            .unwrap_or_else(|| self.get_active_pokemon(player_index).species)
    }
    
    pub fn get_effective_types(&self, player_index: u8) -> Vec<PokemonType> {
        // Check for Conversion first (overrides Transform)
        if let Some(converted_type) = self.battle_state.get_converted_type(player_index) {
            return vec![converted_type];
        }
        
        // Check for Transform
        if let Some(transformed_species) = self.battle_state.get_transformed_species(player_index) {
            let species_data = get_species_data(transformed_species)?;
            return species_data.types.clone();
        }
        
        // Normal case: use original Pokemon types
        let pokemon = self.get_active_pokemon(player_index);
        let species_data = get_species_data(pokemon.species)?;
        species_data.types.clone()
    }
}
```

## Performance Benefits

### Memory Efficiency
- **Compact Storage**: All flags fit in small structs, no HashMap overhead
- **Cache Locality**: Related battle state packed together in arrays
- **Zero Fragmentation**: Fixed-size structures, no dynamic allocation
- **Predictable Layout**: Array indexing instead of hash table lookups

### Access Speed
- **O(1) Access**: Direct field access, no hashing required
- **Branch Prediction**: Simple match statements compile to jump tables
- **Minimal Indirection**: Direct array indexing + field access
- **Hot Path Optimization**: Frequently accessed flags in simple boolean fields

### Type Safety
- **Compile-Time Validation**: Cannot set invalid flag combinations
- **Exhaustive Matching**: Compiler ensures all cases handled
- **Clear Ownership**: Battle owns all temporary state, no shared references
- **Immutable Patterns**: Read-only access methods prevent accidental mutation

This tight BattleState design provides efficient storage and access for all battle-specific state while maintaining clear separation from persistent Player and PokemonInst data.