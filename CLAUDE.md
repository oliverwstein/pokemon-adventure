# Pokemon Adventure Battle System

## Project Overview

This is a comprehensive Rust implementation of a Pokemon text adventure battle system featuring authentic Generation 1 mechanics with custom enhancements. The system implements accurate Pokemon battle calculations, status effects, move mechanics, and turn-based gameplay with excellent software engineering practices.

## Architecture

### Core Components

- **Battle System** (`src/battle/`): Turn-based battle orchestration, action resolution, and event management
  - `state.rs`: Battle state management, event system, and game state tracking
  - `turn_orchestrator.rs`: Action execution, priority resolution, and turn flow control
  - `stats.rs`: Damage calculations, critical hits, and stat modifications
- **Pokemon System** (`src/pokemon.rs`): Pokemon instances, species data, HP/status management
- **Move System** (`src/moves.rs`, `src/move_data.rs`): Move definitions and effect implementations
- **Player System** (`src/player.rs`): Battle players, stat stages, and active conditions
- **Species System** (`src/species.rs`): Pokemon species enumeration with 151 Gen 1 Pokemon

### Key Design Patterns

- **Event-Driven Architecture**: All battle actions generate events through `EventBus` for comprehensive logging
- **Action Stack Pattern**: Dynamic action injection using `VecDeque` for multi-hit moves and complex sequences
- **Deterministic RNG**: `TurnRng` oracle pattern for reproducible battle outcomes and testing
- **Global Data Stores**: Thread-safe lazy-loaded Pokemon species and move data using `LazyLock<RwLock<>>`
- **Type Safety**: Heavy use of enums (`Species`, `Move`, `PokemonType`) for compile-time correctness

## Data Format & Content

The system uses RON (Rusty Object Notation) for human-readable data files:

- **Pokemon Species**: 151 complete Gen 1 Pokemon in `data/pokemon/` (e.g., `001-bulbasaur.ron`)
  - Base stats, types, learnsets, evolution data, catch rates
- **Move Data**: 150+ moves in `data/moves/` (e.g., `tackle.ron`)
  - Power, accuracy, PP, type, category, and complex effects
- **Structured Data**: Type-safe deserialization with serde

## Testing Strategy

- **Comprehensive Test Suite**: 119 tests covering all battle mechanics
- **Unit Tests**: Individual component testing (stats, critical hits, type effectiveness)
- **Integration Tests**: Full battle scenario testing with complex interactions
- **Deterministic Testing**: RNG oracle allows reproducible test outcomes
- **Edge Case Coverage**: Fainting, action prevention, status interactions, multi-hit moves

## Battle Flow

1. **Action Collection**: Players submit moves/switches
2. **Priority Resolution**: Sort actions by priority/speed
3. **Action Execution**: Process moves through action stack
4. **Effect Resolution**: Apply damage, status, and secondary effects
5. **End-of-Turn**: Status damage, condition updates, replacement checks
6. **State Transition**: Win condition checking, next turn setup

## Battle Mechanics Implementation

### Authentic Gen 1 Features
- **Damage Calculation**: Accurate Gen 1 damage formulas with type effectiveness
- **Critical Hit System**: Speed-based critical hit rates and damage multipliers
- **Status Conditions**: Sleep, poison, burn, paralysis, freeze with proper mechanics
- **Stat Stages**: ±6 stat modification system with accurate multipliers
- **Type Effectiveness**: Complete type chart with proper damage multipliers

### Advanced Battle Features
- **Team Conditions**: Reflect (physical damage reduction), Light Screen (special damage reduction), Mist (stat change protection)
- **Active Conditions**: Multi-turn effects like Leech Seed, binding moves, and custom states
- **Action Prevention**: Sleep, paralysis, confusion affecting move execution
- **Priority System**: Move priority and speed-based turn order resolution
- **Multi-Hit Moves**: Complex probabilistic multi-hit logic with proper damage distribution

### Complex Move Effects
- **Two-Turn Moves**: Dig, Fly with semi-invulnerable states
- **Recoil Moves**: Self-damage on successful hits
- **Drain Moves**: HP recovery based on damage dealt
- **Status Moves**: Stat modifications, status infliction, team condition application
- **Special Mechanics**: Bide (damage storage), Counter (retaliation), Transform, Metronome

## Technical Highlights

- **Memory Safety**: Full Rust ownership model, no unsafe code
- **Concurrency Ready**: Thread-safe global data stores with proper synchronization
- **Extensible Design**: Easy addition of new moves, species, and effects through enum expansion
- **Performance Focused**: Minimal allocations during battle resolution, efficient data structures
- **Type-Safe**: Compile-time prevention of invalid Pokemon/move combinations
- **Deterministic Testing**: Reproducible battle outcomes for comprehensive test coverage

## Custom Features

- **Teleported Condition**: Custom semi-invulnerable state for moves like Teleport
- **Enhanced Multi-Hit**: Probabilistic continuation beyond guaranteed hits for moves like Pin Missile
- **Comprehensive Event Logging**: Detailed battle event tracking through EventBus system
- **Action Stack Architecture**: Dynamic action injection for complex move sequences and effects

## Development & Testing

- **119 Comprehensive Tests**: Full coverage of battle mechanics, edge cases, and interactions
- **Integration Test Examples**: Complete battle scenarios demonstrating system capabilities
- **Deterministic RNG**: Predictable random outcomes for reliable testing
- **Modular Architecture**: Clean separation of concerns enabling easy maintenance and extension

This system provides a robust, well-tested foundation for Pokemon battle mechanics with authentic Gen 1 accuracy and room for expansion into advanced features like abilities, held items, and additional generations.