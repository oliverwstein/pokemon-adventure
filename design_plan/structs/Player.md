# Player Struct Design

## Overview

The `Player` struct represents the persistent data for a battle participant that exists outside of any specific battle context. This includes identity, team roster, and accumulated resources, but excludes battle-specific state like active Pokemon index or temporary conditions.

## Player Struct Definition

```rust
pub struct Player {
    pub player_id: String,
    pub player_name: String,
    pub player_type: PlayerType,
    pub team: [Option<PokemonInst>; 6],
    pub ante: u32,
}
```

## Field Descriptions

### `player_id: String`
**Purpose**: Unique identifier for the player across all contexts
**Usage Examples**:
- Human players: User ID from authentication system
- AI players: `"AI_Basic"`, `"AI_Advanced"`  
- Trainer NPCs: `"YoungsterJoey"`, `"BrockGymLeader"`
**Characteristics**:
- Must be unique within any given battle
- Persists across multiple battles
- Used for logging, save data, and player identification

### `player_name: String`
**Purpose**: Human-readable display name for the player
**Usage Examples**:
- Human players: User-chosen display name
- AI players: `"Computer"`, `"AI Opponent"`
- Trainer NPCs: `"Youngster Joey"`, `"Brock"`
**Characteristics**:
- Used for battle UI and event messages
- May not be unique (multiple "Youngster" trainers allowed)
- Separate from ID to allow display name changes

### `player_type: PlayerType`
**Purpose**: Categorizes the type of player for AI behavior and battle rules
```rust
pub enum PlayerType {
    Human,
    Npc,
}

```
**Usage**:
- Distinguishes human players from NPCs for input handling
- Human players require external input (MCP interface, CLI)
- NPC players receive commands from AI systems selected by BattleRunner

### `team: [Option<PokemonInst>; 6]`
**Purpose**: The player's roster of up to 6 Pokemon instances
**Structure**: Fixed-size array where `None` indicates empty slots
**Characteristics**:
- Persistent across battles (HP, status, experience carry forward)
- Mutable during battle (Pokemon can faint, gain experience, evolve)
- Source data for creating battle participants
- Supports partial teams (fewer than 6 Pokemon)

### `ante: u32`
**Purpose**: Money/prize amount accumulated by the player
**Usage**:
- Increased by Pay Day move during battle
- Prize money from winning battles
- Persistent resource that carries between battles
- Used for purchasing items, services in broader game context

## PlayerType Detailed Behavior

### `PlayerType::Human`
- Requires input from external interface (MCP, CLI, API)
- Battle pauses waiting for human decision
- No automatic move selection or AI behavior
- Full access to all battle options and information

### `PlayerType::Npc`
- Automated decision making handled by AI systems
- AI selection determined by BattleRunner based on battle type:
  - **ScoringAI**: Sophisticated move scoring for trainer battles
  - **RandomAI**: Random move selection for wild Pokemon
  - **SafariAI**: Binary choice (do nothing/run) for Safari encounters
- AI behavior independent of Player struct - same NPC can use different AIs in different battle contexts

## Integration with Schema System

### Build System Compatibility
The `Player` struct must be constructible during build-time for:
- Predefined trainer rosters loaded from RON files
- Demo teams converted to player objects
- Test scenarios with specific player configurations

### RON Team File Integration
```ron
// data/teams/trainers/brock.ron
Player(
    player_id: "BrockGymLeader",
    player_name: "Brock", 
    player_type: Npc,
    team: [
        Some(PokemonInst( /* Geodude data */ )),
        Some(PokemonInst( /* Onix data */ )),
        None, None, None, None,
    ],
    ante: 0,
)
```

### Schema Struct Requirement
The `Player` struct and all referenced types (`PokemonInst`, `PlayerType`, etc.) must be defined in the schema crate for build system access:

```rust
// In data/schema/src/lib.rs
pub struct Player { /* ... */ }
pub struct PokemonInst { /* ... */ }  
pub enum PlayerType { /* ... */ }
```

## Separation from Battle State

### What Player Does NOT Include
The `Player` struct explicitly excludes battle-specific state:
- Active Pokemon index (which Pokemon is currently battling)
- Team conditions (Reflect, Light Screen, Mist)
- Stat stage modifications (+2 Attack, -1 Defense, etc.)
- Last move used in battle
- Active Pokemon's volatile conditions (Confused, Trapped, etc.)

### Rationale for Exclusion
- **Battle state is temporary**: Should reset between different battles
- **Multiple battle contexts**: Same player might participate in different battle types
- **Clean persistence**: Only meaningful long-term data stored in `Player`
- **Clear ownership**: Battle struct manages battle-specific state
