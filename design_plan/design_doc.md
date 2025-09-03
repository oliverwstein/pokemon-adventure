# Pokemon Adventure Battle Crate - Design Document

## Context and Scope

### Current System Limitations

The existing Pokemon Adventure battle system successfully handles tournament-style battles with predetermined teams and turn-based resolution. However, its architecture creates significant limitations when extending to story mode scenarios:

**Rigid Turn Structure**: The current `resolve_turn()` approach assumes exactly two player actions per turn, which breaks down when mid-turn events require additional input (Pokemon fainting, evolution prompts, move learning).

**Inflexible Input Handling**: The system lacks extensible hooks for different input types, making it difficult to add story-specific interactions like catching Pokemon or handling experience-driven events.

**Overlapping Conductor Systems**: Multiple "conductor" functions handle orchestration without clear separation of responsibilities, creating maintenance challenges and unclear data flow.

### Battle Type Requirements

The Battle FSM must handle four distinct battle scenarios through the `BattleType` enum, each with different rules and available actions:

**Tournament Battles**: 
- Two predetermined teams with fixed rosters
- Standard battle rules with switching and move usage
- Win/loss/draw resolution only

**Trainer Battles**:
- Experience gain and level progression
- Potential evolution after level-ups
- Move learning when leveling or evolution occurs
- Victory advances story progression

**Wild Encounters**:
- Catching mechanics with Pokeball usage
- Running away option for the player
- Experience gain for victory
- Potential to add caught Pokemon to team

**Safari Zone Battles**:
- Limited Pokeball supply and turn counts
- Special catching mechanics and bait/rock interactions
- Time-based or item-based battle termination

### Battle Crate Architecture

The Battle crate provides a complete battle subsystem with clear internal separation:

**Battle Struct (Pure FSM)**: A minimal state machine with exactly three mutating methods (`advance`, `submit_commands`, `get_input_request`) plus read-only accessors. The Battle struct treats all players identically and has no knowledge of AI decision-making.

**AI Modules (External to Battle Struct)**: Sophisticated decision-making systems (ScoringAI, WildAI, etc.) that utilize battle calculation functions to generate optimal commands. These modules are part of the Battle crate but are called by external systems, not by the Battle struct itself.

**Shared Battle Calculations**: Core battle logic (damage formulas, type effectiveness, stat modifications) used by both the Battle FSM and AI modules, ensuring consistency across all battle mechanics.

### Interface Design Requirements

The Battle crate must provide a clean interface that allows broader game systems to:

**Initialize Battles**: Create appropriate battle instances with correct teams, battle types, and initial conditions without understanding internal FSM mechanics.

**Handle Dynamic Input**: Respond to various input requests (moves, switches, catching, evolution decisions) through a unified interface that works identically for all battle types.

**Utilize AI Systems**: Access AI modules to generate commands for NPC players while maintaining the Battle struct's pure FSM design.

**Process Battle Outcomes**: Receive structured battle results with all relevant data (experience gained, Pokemon caught, items used) without needing to parse internal battle state.

**Maintain Separation**: Keep battle logic completely separate from story progression, UI implementation, and save/load systems while providing necessary data hooks.

### Randomness Architecture

The Battle crate requires randomness for battle calculations (damage variance, critical hits, accuracy rolls, status chances) but does not own or manage RNG state. External systems provide RNG instances to the Battle FSM, enabling deterministic testing through seeded or mocked RNG while maintaining battle calculation accuracy. This design preserves the existing pattern of external RNG control that supports comprehensive test coverage.

## Goals and Non-Goals

## Actual Design

### System Overview

### FSM State Management

### Action Stack Execution Model

### Command Validation System

### Data Structure Design

## Alternatives Considered

## Cross-Cutting Concerns

### Testing Strategy

### Performance Considerations

### Error Handling and Recovery