# Pokemon Adventure Battle System Engine

## Project Overview

This is a comprehensive Rust implementation of a Pokemon text adventure battle system featuring authentic Generation 1 mechanics with custom enhancements. The engine provides the core battle mechanics, Pokemon data management, and turn-based combat logic that powers the Pokemon Adventure API. Built with excellent software engineering practices, it implements accurate Pokemon battle calculations, status effects, move mechanics, and deterministic gameplay suitable for both interactive and automated systems.

## Architecture

### Core Components

- **Battle System** (`src/battle/`): Pure Command-Execution architecture with complete separation of calculation and mutation
  - `state.rs`: Battle state management, event system, and game state tracking with `BattleState` core struct
  - `engine.rs`: Action execution orchestration, priority resolution, and turn flow control (execution layer)
  - `calculators.rs`: Pure battle logic calculations that return command lists (calculation layer)
  - `commands.rs`: Command definitions and execution logic with automatic event generation
  - `stats.rs`: Damage calculations, critical hits, stat modifications, and type effectiveness
  - `conditions.rs`: Pokemon and team condition definitions with pure calculation methods
- **Pokemon System** (`src/pokemon.rs`): Pokemon instances, species data, HP/status management, and move learning
- **Move System** (`src/moves.rs`, `src/move_data.rs`): 150+ move definitions with complex effect implementations
  - Category system: Physical, Special, Status moves with authentic Gen 1 mechanics
  - Secondary effects: Status infliction, stat changes, healing, protection, and unique mechanics
  - Multi-turn moves: Charging attacks, binding moves, semi-invulnerable states
- **Player System** (`src/player.rs`): Battle participants, team management, stat stages, and active conditions
  - `BattlePlayer`: Core player structure with 6-Pokemon teams and battle state
  - Stat stages: ±6 modification system with accurate multipliers
  - Team conditions: Reflect, Light Screen, Mist for advanced strategy
- **Species System** (`src/species.rs`): Complete Gen 1 Pokemon roster with 151 species enumeration
- **Team System** (`src/teams.rs`): Flexible RON-based team management with build-time compilation
  - Demo teams: Venusaur, Blastoise, Charizard with strategic movesets (level 60, balanced)
  - Support for both specified movesets and learnset-based auto-generation
  - Extensible team creation system using RON data files

### Key Design Patterns

- **Command-Execution Architecture**: Complete separation between calculation (`calculators.rs`) and execution (`engine.rs`)
  - **Pure Calculation Functions**: All game logic functions return `Vec<BattleCommand>` without mutating state
  - **Centralized Execution**: Only `resolve_turn()` and command executors mutate `BattleState`
  - **Automatic Event Generation**: Commands emit their own events, eliminating manual event management
- **Event-Driven Architecture**: All battle actions generate events through `EventBus` for comprehensive logging and replay capability
- **Action Stack Pattern**: Dynamic action injection using `VecDeque` for multi-hit moves and complex sequences
- **Deterministic RNG**: `TurnRng` oracle pattern for reproducible battle outcomes and comprehensive testing
- **Global Data Stores**: Thread-safe lazy-loaded Pokemon species and move data using `LazyLock<RwLock<>>` with compile-time optimization
- **Type Safety**: Heavy use of enums (`Species`, `Move`, `PokemonType`, `StatusCondition`) for compile-time correctness
- **Atomic Commands**: Single-responsibility commands for status changes, damage, healing, and condition management
- **Compile-Time Data**: `build.rs` script processes RON data files into Rust code for zero-runtime overhead
- **HashMap Optimization**: Type-safe condition keys with proper `Hash`/`Eq` implementations for O(1) lookups

## Data Format & Content

The system uses RON (Rusty Object Notation) for human-readable data files with compile-time optimization:

### Pokemon Species Data (`data/pokemon/`)
- **151 Complete Gen 1 Pokemon**: From Bulbasaur (#001) to Mew (#151)
- **Comprehensive Stats**: Base HP, Attack, Defense, Special Attack, Special Defense, Speed
- **Type Information**: Primary and secondary types with full type chart integration
- **Move Learning**: Level-up learnsets, TM/HM compatibility, and move tutoring
- **Evolution Data**: Evolution methods, levels, items, and stone requirements
- **Game Mechanics**: Catch rates, base experience, growth rates, and sprite data
- **Example Structure**: `001-bulbasaur.ron` contains complete Bulbasaur data

### Move Database (`data/moves/`)
- **150+ Moves**: Complete Gen 1 moveset with authentic mechanics
- **Move Categories**: Physical, Special, and Status moves with proper damage calculation
- **Accuracy & Power**: Exact values matching original games including 100% accuracy moves
- **PP System**: Power Points with maximum values and PP Up enhancement support
- **Complex Effects**: Multi-stage effects, condition application, stat modifications
- **Priority System**: Move priority values for speed-based turn order resolution
- **Target Selection**: Self, opponent, user's team, opponent's team targeting modes

### Team Data (`data/teams/`)
- **RON Team Templates**: Human-readable team definitions with flexible move specification
- **Demo Teams**: Pre-balanced teams in `data/teams/demo/` (Venusaur, Blastoise, Charizard)
  - Level 60 Pokemon with curated movesets for balanced competitive play
  - Exact preservation of strategic move combinations from original system
- **Flexible Move System**: Teams can specify exact moves or use auto-generated learnset moves
  - `moves: Some([SleepPowder, SolarBeam, PetalDance, Earthquake])` - specified moves
  - `moves: None` - automatically use the last 4 moves learned by level from learnset
- **Extensible Structure**: Easy addition of new teams through RON file creation
- **Recursive Processing**: Build system processes team files in subdirectories for organization

### Compile-Time Optimization
- **Build Script Integration**: `build.rs` processes all RON files (Pokemon, moves, teams) during compilation
- **Generated Code**: Creates Rust source with embedded data structures for all game data
- **Postcard Serialization**: Binary serialization of team, move, and species data for minimal runtime overhead
- **Zero Runtime Cost**: No file I/O or parsing during battle execution
- **Type Safety**: Compile-time verification of all Pokemon/move/team references
- **Hot Path Optimization**: Critical battle data pre-computed and inlined

## Testing Strategy

### Comprehensive Test Coverage
- **All Tests Passing**: Complete test suite with 100% success rate
- **Unit Testing**: Individual component validation (stats, critical hits, type effectiveness)
- **Integration Testing**: Full battle scenario testing with complex multi-turn interactions
- **Deterministic Testing**: `TurnRng` oracle pattern enables reproducible test outcomes
- **Edge Case Coverage**: Fainting sequences, action prevention, status interactions, multi-hit moves

### Test Categories
- **Battle Mechanics**: Core combat system validation
  - Damage calculation accuracy with type effectiveness multipliers
  - Critical hit mechanics with speed-based probability
  - Status effect application, duration, and interaction
  - Priority system with speed tiebreakers and move precedence
- **Pokemon Management**: Instance creation, stat calculation, status tracking
  - HP management with maximum bounds and damage overflow
  - Stat stage modifications with ±6 limits and multipliers
  - Move PP tracking with maximum values and depletion
  - Team switching mechanics and active Pokemon management
- **Move Effects**: Complex move behavior validation
  - Multi-hit moves with probabilistic continuation (Pin Missile, Fury Swipes)
  - Two-turn moves with charging phases (Solar Beam, Dig, Fly)
  - Status moves with various effects (Sleep Powder, Thunder Wave)
  - Recoil moves with self-damage calculation (Take Down, Double-Edge)
- **Advanced Scenarios**: Complex battle state interactions
  - Team conditions with duration tracking (Reflect, Light Screen, Mist)
  - Active conditions with turn-based effects (Leech Seed, Bind)
  - Forced actions and action prevention (sleep, paralysis, charging)
  - Win condition detection and battle termination

### Testing Infrastructure
- **Deterministic RNG**: Predictable random number generation for reproducible tests
- **Battle State Snapshots**: Complete battle state capture for regression testing
- **Event Validation**: Comprehensive event generation testing for battle logging
- **Performance Benchmarks**: Execution time validation for critical battle paths

### Test Structure and Organization
The testing framework uses modern Rust testing practices for maintainability and clarity:

- **RSTest Framework**: Leverages `rstest` crate for parametric testing with `#[case]` attributes
  - Enables comprehensive scenario testing with descriptive case names
  - Reduces code duplication through parameterized test functions
  - Example: `test_two_turn_moves` covers SolarBeam, Fly, and Dig with single test function
- **Common Test Utilities** (`src/battle/tests/common.rs`):
  - **TestPokemonBuilder**: Fluent builder pattern for creating test Pokemon instances
    - `.new(Species::Pikachu, 25)` - species and level
    - `.with_moves(vec![Move::Tackle])` - custom movesets
    - `.with_status(StatusCondition::Burn)` - status conditions
    - `.with_hp(50)` - specific HP values (capped to max HP)
  - **Battle Creation**: `create_test_battle()` for standard 1v1 scenarios
  - **Predictable RNG**: `predictable_rng()` provides consistent random values for deterministic testing
- **Test Organization by Feature**: Tests grouped by battle mechanics in separate files
  - `test_fainting.rs` - Pokemon fainting mechanics and revival
  - `test_special_moves.rs` - Multi-turn moves, Mirror Move, Transform
  - `test_condition_damage.rs` - Status damage, Leech Seed, binding moves
  - `test_action_prevention.rs` - Sleep, paralysis, confusion mechanics
  - Each file contains both unit tests (individual Pokemon methods) and integration tests (full battles)
- **Event-Driven Assertions**: Tests validate battle outcomes through event inspection
  - `event_bus.events().iter().any(|e| matches!(e, BattleEvent::PokemonFainted { .. }))` 
  - Comprehensive event logging ensures all battle actions are properly recorded
- **Debug Output Integration**: `event_bus.print_debug_with_message()` for test debugging
  - Provides detailed event traces for complex battle scenarios
  - Facilitates rapid diagnosis of test failures and battle flow issues

## Battle Flow

### Turn-Based Combat Cycle
1. **Action Collection**: Players submit moves, switches, or forfeit actions
   - Input validation ensures only legal actions are accepted
   - Action queue populated with player and AI submissions
   - Forced actions (charging moves, binding effects) automatically queued

2. **Priority Resolution**: Sort actions by move priority and Pokemon speed
   - Higher priority moves execute first (Quick Attack, Agility effects)
   - Speed tiebreakers for moves with identical priority values
   - Switch actions always have highest priority for immediate execution

3. **Action Execution**: Process moves through pure command generation
   - **Calculate Phase**: Pure functions generate command lists without state mutation
   - **Execute Phase**: Commands applied to state with automatic event emission
   - Multi-hit moves spawn additional actions in sequence
   - Action stack allows complex move combinations and interruptions

4. **Effect Resolution**: Apply calculated commands to battle state
   - **Command Processing**: Atomic commands execute state changes
   - **Automatic Events**: Commands generate appropriate battle events
   - **Status Effects**: Pure calculation of damage, healing, and condition changes

5. **End-of-Turn Processing**: Status damage, condition updates, replacement checks
   - Status damage application (burn, poison) with turn counting
   - Active condition processing (Leech Seed, binding moves)
   - Team condition expiration tracking (Reflect, Light Screen)
   - Forced Pokemon replacement for fainted Pokemon

6. **State Transition**: Win condition checking, next turn setup
   - Battle completion detection (all Pokemon fainted, forfeit)
   - Game state updates and turn counter increment
   - Action queue reset and preparation for next turn cycle

### Command-Execution Architecture
- **Pure Calculation Layer**: Functions in `calculators.rs` return command lists without state mutation
- **Centralized Execution**: Only command executors in `engine.rs` and `commands.rs` modify state
- **Automatic Event Generation**: Commands emit events via `BattleCommand::emit_events()`
- **Atomic Operations**: Single-responsibility commands for damage, healing, status changes
- **Dynamic Action Injection**: Complex moves inject additional actions mid-execution through command lists
- **State Consistency**: Battle state mutations centralized in command execution layer

## Battle Mechanics Implementation

### Authentic Gen 1 Damage System
- **Core Damage Formula**: `((2 * Level + 10) / 250) * (Attack / Defense) * Base Power + 2`
  - Level-based scaling matching original game mechanics
  - Attack/Defense ratio with proper stat stage modifications
  - Base power integration from move database
  - Critical hit multiplier application (2x damage)
- **Type Effectiveness Chart**: Complete 15-type system with authentic multipliers
  - Super effective: 2.0x damage (Fire vs. Grass, Water vs. Rock)
  - Not very effective: 0.5x damage (Water vs. Fire, Electric vs. Ground)
  - No effect: 0.0x damage (Normal vs. Ghost, Ground vs. Flying)
  - Same-type attack bonus (STAB): 1.5x damage for matching Pokemon/move types
- **Critical Hit Mechanics**: Speed-based critical hit determination
  - Base critical hit rate based on Pokemon species speed stat
  - High critical hit moves (Slash, Karate Chop) with increased probability
  - Critical hits ignore negative stat stage modifications
- **Random Damage Variance**: 85%-100% damage range for battle unpredictability

### Status Condition System
- **Sleep**: Prevents move execution for 1-7 turns with gradual awakening probability
- **Poison**: Deals 1/8 max HP damage per turn with stackable effects
- **Burn**: Reduces physical attack damage by 50% + 1/16 max HP damage per turn
- **Paralysis**: 25% chance to prevent move execution + 50% speed reduction
- **Freeze**: Complete immobilization until thawed by Fire-type move or chance
- **Status Priority**: Only one major status per Pokemon with proper override rules

### Stat Stage Modification System
- **Six-Stage Range**: -6 to +6 modifications for all core battle stats
- **Multiplier Table**: Authentic Gen 1 stat stage multipliers
  - +6 stages: 4.0x stat value (maximum boost)
  - +3 stages: 2.5x stat value (significant boost)
  - +1 stage: 1.5x stat value (minor boost)
  - -1 stage: 0.67x stat value (minor reduction)
  - -3 stages: 0.4x stat value (significant reduction)
  - -6 stages: 0.25x stat value (maximum reduction)
- **Stat Categories**: Attack, Defense, Special Attack, Special Defense, Speed modifications
- **Move Integration**: Moves like Swords Dance (+2 Attack), Growl (-1 Attack) with proper stacking

### Advanced Battle Features
- **Team Conditions**: Battlefield effects lasting multiple turns
  - **Reflect**: 50% physical damage reduction for 5 turns
  - **Light Screen**: 50% special damage reduction for 5 turns  
  - **Mist**: Prevents stat stage reductions for 5 turns
  - **Turn Tracking**: Automatic expiration and renewal handling
- **Active Conditions**: Pokemon-specific multi-turn effects
  - **Leech Seed**: 1/8 max HP drain per turn with HP transfer to opponent
  - **Binding Moves**: Wrap, Bind, Clamp with 2-5 turn duration and damage
  - **Charging States**: Solar Beam, Dig, Fly with two-turn execution cycles
- **Semi-Invulnerable States**: Pokemon temporarily untargetable during certain moves
  - **Dig Underground**: Immune to most attacks except Earthquake, Fissure
  - **Fly Airborne**: Immune to most attacks except Thunder, Hurricane, Sky Attack
  - **Custom Teleported**: Unique state for moves like Teleport with strategic implications

### Complex Move Categories & Effects
- **Physical Moves**: Attack stat-based damage with potential contact effects
  - Recoil moves: Take Down (25% recoil), Double-Edge (33% recoil)
  - High critical hit: Slash (high crit rate), Karate Chop (fighting-type high crit)
  - Multi-hit potential: Fury Swipes (2-5 hits), Pin Missile (2-5 hits probabilistic)
- **Special Moves**: Special Attack stat-based damage with elemental effects
  - Charging moves: Solar Beam (charge + release cycle)
  - Weather effects: Thunder (100% accuracy in rain, 50% in sun)
  - Status chances: Fire moves (10% burn), Ice moves (10% freeze)
- **Status Moves**: Non-damaging effects with strategic battlefield control
  - Stat modifications: Swords Dance (+2 Attack), Amnesia (+2 Special Defense)
  - Status infliction: Sleep Powder (sleep), Thunder Wave (paralysis)
  - Healing moves: Recover (50% HP), Rest (full HP + sleep)
  - Protection moves: Protect (prevents all damage for one turn)
- **Special Mechanics**: Unique move behaviors requiring custom implementation
  - **Bide**: Stores damage for 2-3 turns, then releases 2x accumulated damage
  - **Counter**: Returns 2x physical damage received during the same turn
  - **Transform**: Copies opponent's type, stats, and moveset permanently
  - **Metronome**: Randomly selects and executes any move in the game
  - **Explosion/Self-Destruct**: Maximum damage output with user fainting

### Priority System & Turn Order
- **Move Priority Levels**: -7 to +5 priority range with authentic move assignments
  - +5: No moves in Gen 1 (reserved for future expansion)
  - +1: Quick Attack, Agility-boosted moves
  - 0: Most standard moves (Tackle, Thunderbolt, etc.)
  - -7: Lowest priority moves (none in current implementation)
- **Speed Tiebreakers**: When priority is equal, higher speed Pokemon moves first
- **Switch Priority**: Pokemon switches always execute before any moves
- **Action Queue Ordering**: Proper sorting of all battle actions before execution

### Multi-Hit Move System
- **Hit Count Determination**: Probabilistic system for moves like Fury Swipes
  - 2 hits: 37.5% chance (3/8 probability)
  - 3 hits: 37.5% chance (3/8 probability) 
  - 4 hits: 12.5% chance (1/8 probability)
  - 5 hits: 12.5% chance (1/8 probability)
- **Damage Per Hit**: Each hit calculated independently with full damage formula
- **Status Effect Chances**: Each hit has independent chance for secondary effects
- **Early Termination**: Fainting stops multi-hit sequence immediately

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
- **Command-Based Architecture**: Elegant separation of calculation and execution for maintainable code

## Development & Testing

- **211+ Comprehensive Tests**: Full coverage including command-based architecture and team system validation
- **Pure Function Testing**: Calculator functions tested independently of state mutation
- **Command Execution Testing**: Atomic command testing with automatic event validation
- **Integration Test Examples**: Complete battle scenarios demonstrating system capabilities
- **Team System Testing**: RON-based team loading, template processing, and compatibility validation
- **RSTest Framework**: Parametric testing with descriptive case names for confusion damage, stat modifications
- **Special Damage Move Testing**: Comprehensive coverage of SuperFang, Dragon Rage, Sonic Boom, level-based damage
- **Deterministic RNG**: Predictable random outcomes for reliable testing
- **Modular Architecture**: Perfect separation between calculation (`calculators.rs`) and execution (`engine.rs`)
- **Type-Safe Conditions**: HashMap-based condition system with proper `Hash`/`Eq` implementations

## Technical Highlights

- **Memory Safety**: Full Rust ownership model with zero unsafe code
- **Command-Execution Pattern**: Pure functional calculations with centralized state mutation
- **Automatic Event Generation**: Commands self-emit events, eliminating manual event management
- **Status Damage Consistency**: Pokemon's own calculation logic used throughout battle system
- **Concurrency Ready**: Thread-safe global data stores with proper synchronization
- **Extensible Design**: Easy addition of new moves, species, and effects through pure command generation
- **Performance Focused**: Minimal allocations during battle resolution, efficient data structures
- **Type-Safe**: Compile-time prevention of invalid Pokemon/move combinations
- **Deterministic Testing**: Reproducible battle outcomes for comprehensive test coverage

## System Evolution & Architecture Migration

### RON-Based Team System Migration (Recent)
The system recently underwent a significant architectural improvement, migrating from hardcoded team definitions to a flexible, data-driven approach:

**Previous System**: Hardcoded teams in `prefab_teams.rs` with fixed move assignments
**Current System**: RON-based team templates with build-time compilation and flexible move specification

**Key Improvements**:
- **Data-Driven Teams**: Team definitions moved to human-readable RON files in `data/teams/`
- **Flexible Move Assignment**: Teams can use curated movesets or auto-generated learnset moves
- **Extensibility**: New teams easily added without code changes
- **Compile-Time Optimization**: Team data embedded in binary via postcard serialization
- **Backward Compatibility**: All existing team functions preserved (`get_venusaur_team()`, etc.)

**Migration Details**:
- Created `src/teams.rs` module replacing `src/prefab_teams.rs`
- Updated `build.rs` with team data generation and serialization
- Preserved exact move combinations from original hardcoded teams
- Updated `mcp_interface.rs` and `main.rs` to use new team system
- All 211 tests passing with complete functionality preservation

This architectural evolution maintains the system's performance characteristics while significantly improving maintainability and extensibility for team management.

---

## Code Style & Development Patterns

### Architectural Philosophy
The codebase follows consistent architectural principles that prioritize maintainability, type safety, and testability:

- **Pure Function Design**: Battle logic functions are pure, returning command lists without side effects
- **Centralized State Mutation**: Only designated command executors modify battle state, ensuring consistency
- **Event-Driven Architecture**: All actions generate events through a centralized `EventBus` for comprehensive logging
- **Builder Pattern Usage**: Test utilities use fluent builder APIs (`TestPokemonBuilder`) for clear, composable test setup

### Code Style Conventions

**Documentation Standards**:
- Comprehensive `///` doc comments on public APIs with usage examples
- Inline explanations for complex battle mechanics and calculations
- Clear module-level documentation explaining architectural decisions

**Error Handling Patterns**:
- Custom error enums (`SpeciesDataError`, `UseMoveError`) with descriptive variants
- Consistent `Result<T, E>` return patterns throughout the codebase
- Graceful degradation for data loading failures with fallback behaviors

**Type Safety & Validation**:
- Heavy use of enums for domain concepts (`Species`, `Move`, `StatusCondition`)
- Compile-time validation of Pokemon/move/team references through schema types
- `Hash` and `Eq` trait implementations for type-safe HashMap keys

**Testing Framework**:
- `rstest` crate for parametric testing with `#[case]` attributes
- Descriptive test case names explaining expected behavior
- Comprehensive event validation through pattern matching
- Deterministic RNG with `TurnRng` oracle pattern for reproducible outcomes

**Formatting & Display**:
- Consistent `fmt::Display` implementations with aligned tabular output
- `LABEL_WIDTH` constants for proper text alignment in battle logs
- Human-readable event formatting with context-aware messaging

**Data Management**:
- RON (Rusty Object Notation) for human-readable configuration files
- Compile-time data embedding via `build.rs` and postcard serialization
- Zero-runtime-cost data access through static function interfaces
- Modular schema definitions in separate `schema` crate for build script compatibility

### Development Workflow Patterns

**Command-Execution Separation**:
- Calculation functions (`calculators.rs`) return command lists without mutation
- Execution functions (`commands.rs`) apply state changes and emit events
- Clear separation between "what to do" (commands) and "doing it" (execution)

**Testing Organization**:
- Feature-based test file organization (`test_fainting.rs`, `test_special_moves.rs`)
- Common test utilities in `tests/common.rs` with builder patterns
- Both unit tests (individual methods) and integration tests (full battles) in same files
- Event bus integration for detailed battle flow debugging

This system provides a robust, well-tested foundation for Pokemon battle mechanics with authentic Gen 1 accuracy, elegant Command-Execution architecture, modern RON-based data management, and room for expansion into advanced features like abilities, held items, and additional generations.