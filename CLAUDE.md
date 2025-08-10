# Pokemon Adventure Battle System

## Project Overview

This is a Rust implementation of a Pokemon text adventure battle system featuring Generation 1 mechanics with some custom enhancements. The system implements authentic Pokemon battle calculations, status effects, move mechanics, and turn-based gameplay.

## Architecture

### Core Components

- **Battle System** (`src/battle/`): Turn-based battle orchestration, action resolution, and event management
- **Pokemon System** (`src/pokemon.rs`): Pokemon instances, species data, HP/status management
- **Move System** (`src/moves.rs`, `src/move_data.rs`): Move definitions and effect implementations
- **Player System** (`src/player.rs`): Battle players, stat stages, and active conditions
- **Species System** (`src/species.rs`): Pokemon species enumeration

### Key Design Patterns

- **Event-Driven Architecture**: All battle actions generate events through `EventBus`
- **Action Stack Pattern**: Dynamic action injection using `VecDeque` for multi-hit moves and complex sequences
- **Deterministic RNG**: `TurnRng` oracle pattern for reproducible battle outcomes
- **Global Data Stores**: Thread-safe lazy-loaded Pokemon species and move data using `LazyLock<RwLock<>>`
- **Type Safety**: Heavy use of enums (`Species`, `Move`, `PokemonType`) for compile-time correctness

## Current Implementation Status

### ✅ Fully Implemented
- **Core Battle Loop**: Turn orchestration, action collection, resolution
- **Pokemon Mechanics**: HP management, fainting, species data loading
- **Move System**: 214+ move definitions, PP usage, basic move effects
- **Damage Calculation**: Authentic Gen 1 formulas with STAB, critical hits, type effectiveness
- **Status Effects**: Paralysis (speed reduction), Burn (attack halving), Fainting
- **Critical Hits**: Pokemon-accurate rates with Focus Energy support
- **Multi-Hit Moves**: Probabilistic continuation system (Fury Attack, Spike Cannon, etc.)
- **Type System**: Complete type chart with immunity, resistance, and weakness
- **Action Priority**: Switch actions > Move priority > Speed-based turn order
- **Forced Switching**: End-of-turn Pokemon replacement after fainting
- **Special Damage**: OHKO moves, fixed damage (Sonic Boom), level-based damage (Seismic Toss), percentage damage (Super Fang)

### 🚧 Partially Implemented
- **Move Effects System**: Framework exists, only 6/39 MoveEffect variants implemented
- **Status System**: Basic conditions exist but no damage/timing mechanics
- **End-of-Turn Phase**: Exists but only handles replacement checking
- **Active Conditions**: 17 condition types defined but not processed

### ❌ Not Yet Implemented
- **Status Damage**: Poison/burn damage during end-of-turn
- **Most Move Effects**: 33/39 MoveEffect variants need implementation
- **Active Condition Processing**: Confusion, flinch, trap, etc.
- **Two-Turn Moves**: Solar Beam, Skull Bash, Fly, Dig charging mechanics
- **Field Effects**: Reflect, Light Screen, Mist team-wide effects

## Data Format

The system uses RON (Rusty Object Notation) for data files:

- **Pokemon Species**: `data/pokemon/001-bulbasaur.ron`
- **Move Data**: `data/moves/tackle.ron` 
- **Structured Data**: Type-safe deserialization with serde

## Testing Strategy

- **Unit Tests**: Individual component testing (stats, critical hits, type effectiveness)
- **Integration Tests**: Full battle scenario testing
- **Deterministic Testing**: RNG oracle allows reproducible test outcomes

## Battle Flow

1. **Action Collection**: Players submit moves/switches
2. **Priority Resolution**: Sort actions by priority/speed
3. **Action Execution**: Process moves through action stack
4. **Effect Resolution**: Apply damage, status, and secondary effects
5. **End-of-Turn**: Status damage, condition updates, replacement checks
6. **State Transition**: Win condition checking, next turn setup

## Development Priorities

See `BATTLE_TODO.md` for detailed implementation roadmap:

1. **End-of-turn effects** (status damage, timers)
2. **Basic move effects** (status infliction, stat changes)
3. **Active conditions** (confusion, flinch, trap mechanics)
4. **Advanced move mechanics** (two-turn moves, field effects)

## Technical Highlights

- **Memory Safety**: Full Rust ownership model, no unsafe code
- **Concurrency Ready**: Thread-safe global data stores
- **Extensible Design**: Easy addition of new moves, species, and effects
- **Performance Focused**: Minimal allocations during battle resolution
- **Type-Safe**: Compile-time prevention of invalid Pokemon/move combinations

## Custom Features

- **Teleported Condition**: Custom semi-invulnerable state
- **Enhanced Multi-Hit**: Probabilistic continuation beyond guaranteed hits
- **Comprehensive Logging**: Detailed battle event tracking

This system provides a solid foundation for Pokemon battle mechanics with room for expansion into advanced features like abilities, held items, and additional generations.