# Pokemon Species Design

## Overview

The Pokemon species system provides static, compile-time data for all Pokemon species. This data is immutable and shared across all Pokemon instances of the same species. The design emphasizes performance through static references and comprehensive data coverage for authentic Pokemon mechanics.

## Core Architecture

### Static Data Design
- **Immutable**: Species data never changes during runtime
- **Shared References**: All PokemonInst objects reference the same static species data
- **Compile-Time**: Species data embedded in binary via build system processing
- **Zero-Copy Access**: Direct references to static data, no cloning required

## PokemonSpecies Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PokemonSpecies {
    pub pokedex_number: u16,            // Official Pokedex number (001-151)
    pub name: String,                   // Species name (e.g., "Bulbasaur")
    pub types: Vec<PokemonType>,        // Primary/secondary types
    pub base_stats: BaseStats,          // Base stat values for calculation
    pub learnset: Learnset,            // Moves learnable by this species
    pub catch_rate: u8,                // Probability modifier for capture
    pub base_exp: u16,                 // Base experience yield when defeated
    pub experience_group: ExperienceGroup, // Leveling curve category
    pub description: String,           // Pokedex description
    pub evolution_data: Option<EvolutionData>, // Evolution requirements if applicable
}

impl PokemonSpecies {
      pub fn calculate_stat(
          base_stats: &BaseStats,
          stat: StatType,
          level: u8,
          iv: u8,
          ev: u8
      ) -> u16 {
          match stat {
              StatType::HP => {
                  let stat_base = 2 * base_stats.hp as u16 + iv as u16 + (ev as u16 / 4);
                  (stat_base * level as u16) / 100 + level as u16 + 10
              }
              StatType::Attack => {
                  let stat_base = 2 * base_stats.attack as u16 + iv as u16 + (ev as u16 / 4);
                  (stat_base * level as u16) / 100 + 5
              }
              // ... other stats
          }
      }
  }
```

## Component Systems

### Base Stats System

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseStats {
    pub hp: u8,         // Hit Points base
    pub attack: u8,     // Physical Attack base
    pub defense: u8,    // Physical Defense base
    pub sp_attack: u8,  // Special Attack base
    pub sp_defense: u8, // Special Defense base
    pub speed: u8,      // Speed base
}

impl BaseStats {
    pub fn total(&self) -> u16  // Base stat total for species comparison
}
```

**Usage**: Base stats combined with level, IVs, and EVs to calculate final Pokemon stats

### Learnset System

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Learnset {
    pub level_up: HashMap<u8, Vec<Move>>,  // Level -> moves learned at that level
    pub signature: Option<Move>,           // Special signature move for evolution line
    pub can_learn: Vec<Move>,             // Additional moves via tutoring/witnessing
}

impl Learnset {
    pub fn learns_at_level(&self, level: u8) -> Option<&Vec<Move>>
    pub fn can_learn_move(&self, move_: Move) -> bool
}
```

**Key Features**:
- **Level-Up Learning**: Maps specific levels to moves learned naturally
- **Signature Moves**: Special moves associated with evolution lines
- **Extended Learning**: Moves available through non-level methods
- **Compatibility Checking**: Validates if species can learn specific moves

### Experience Group System

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ExperienceGroup {
    Fast,        // Quick leveling (0.8x multiplier)
    MediumFast,  // Standard leveling (1.0x multiplier)
    MediumSlow,  // Moderate leveling (1.2x multiplier)  
    Slow,        // Slow leveling (1.4x multiplier)
    Fluctuating, // Variable with sine wave modulation
    Erratic,     // Irregular with dampened sine wave
}
```

**Experience Formula**: Unified cubic formula with optional sine wave modulation:
```
Total EXP = A × n³ + B × n² × sin(C × n)
```

**Methods**:
```rust
pub fn exp_for_level(self, level: u8) -> u32        // Experience required for level
pub fn calculate_level_from_exp(self, total_exp: u32) -> u8  // Level from total experience
pub fn can_level_up(self, current_level: u8, total_exp: u32) -> bool  // Level up check
```

### Evolution System

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvolutionMethod {
    Level(u8),      // Evolve at specific level
    Item(Item),     // Evolve using evolution stone/item
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionData {
    pub evolves_into: Species,      // Target species
    pub method: EvolutionMethod,    // Evolution requirement
}
```

**Evolution Items**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Item {
    FireStone, WaterStone, ThunderStone, LeafStone, MoonStone,
    // Extensible for additional evolution items
}
```

## Data Access Patterns

### Species Data Lookup
```rust
// From pokemon.rs
pub fn get_species_data(species: Species) -> SpeciesDataResult<&'static PokemonSpecies>
```

**Characteristics**:
- Returns static reference to species data
- No copying or allocation required
- Compile-time embedded data via build system
- Type-safe species enum prevents invalid lookups

### Move Data Integration
Species learnsets reference the global move database:
- Move enums provide type-safe move references
- Move data (PP, power, effects) accessed separately via move database
- Learnsets only store which moves are learnable, not move details

## Build System Integration

### RON File Processing
Species data stored in human-readable RON files:
```ron
// data/pokemon/001-bulbasaur.ron
PokemonSpecies(
    pokedex_number: 1,
    name: "Bulbasaur",
    types: [Grass, Poison],
    base_stats: (
        hp: 45,
        attack: 49,
        defense: 49,
        sp_attack: 65,
        sp_defense: 65,
        speed: 45,
    ),
    experience_group: MediumSlow,
    learnset: (
        level_up: {
            1: [Tackle, Growl],
            7: [LeechSeed],
            13: [VineWhip],
            19: [PoisonPowder],
            21: [StunSpore],
            25: [MegaDrain],
            27: [Growth],
            31: [SleepPowder],
            
        },
        signature: None,
        can_learn: [
            SwordsDance, Toxic, BodySlam, TakeDown, DoubleEdge,
            Rage, Absorb, RazorLeaf, GigaDrain, SolarBeam, Mimic, DoubleTeam,
            Reflect, Bide, Rest, Substitute, Cut, Bind, Headbutt,
            Stomp, Roar, Bite, AncientPower, Earthquake, PoisonGas, 
            PetalDance, Spore
        ],
    ),
    catch_rate: 45,
    base_exp: 64,
    description: "A strange seed was planted on its back at birth. The plant sprouts and grows with this Pokémon.",
    evolution_data: Some((
        evolves_into: Ivysaur,
        method: Level(16),
    )),
)
```

### Compile-Time Embedding
- Build system processes RON files into Rust arrays
- Species data embedded as static arrays in binary
- Postcard serialization for efficient storage
- Zero runtime file I/O or parsing required

## Usage Patterns

### Species-Based Pokemon Creation
```rust
let species = Species::Bulbasaur;
let species_data = get_species_data(species)?;
let pokemon = PokemonInst::new(species, species_data, level, ivs, moves);
```

### Stat Calculation Integration
```rust
// Base stats used in PokemonInst stat calculation (only during construction)
let stats = PokemonInst::calculate_stats(&species_data.base_stats, level, &ivs, &evs);
```

### Move Learning Validation
```rust
// Check if Pokemon can learn specific move
if species_data.learnset.can_learn_move(Move::SolarBeam) {
    // Allow move learning
}
```

### Evolution Checking
```rust
// Check evolution requirements
if let Some(evolution_data) = &species_data.evolution_data {
    match evolution_data.method {
        EvolutionMethod::Level(required_level) => {
            if pokemon.level >= required_level {
                // Can evolve
            }
        }
        EvolutionMethod::Item(required_item) => {
            // Check if player has required item
        }
    }
}
```

## Performance Characteristics

### Memory Efficiency
- **Single Instance**: One copy of each species data in memory
- **Reference Sharing**: All Pokemon instances share species data references
- **Compact Storage**: Postcard serialization minimizes binary size
- **Cache Friendly**: Static data has optimal memory locality

### Access Performance
- **O(1) Lookup**: Direct array indexing by species enum
- **Zero Allocation**: Static references, no copying required
- **Branch Prediction**: Enum-based dispatch optimizes hot paths
- **Compile-Time Optimization**: All species data known at compile time

## Design Principles

### Immutability
- Species data never changes during gameplay
- Modifications require recompilation (appropriate for game balance)
- Thread-safe by default due to immutability

### Type Safety
- Species enum prevents invalid species references
- Move enums in learnsets prevent invalid move assignments
- Compile-time validation of all species/move relationships

### Extensibility
- New species added via RON files without code changes
- Evolution items extensible through Item enum
- Experience groups support new leveling curves
- Learnset system handles complex move learning patterns

### Separation of Concerns
- Species data purely descriptive, no game logic
- Combat calculations use species data as input
- Evolution logic references species data but doesn't modify it
- Clear boundary between static data and dynamic Pokemon instances

This species system provides the foundational data layer for all Pokemon mechanics while maintaining optimal performance through static references and compile-time optimization.