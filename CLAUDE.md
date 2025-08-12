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