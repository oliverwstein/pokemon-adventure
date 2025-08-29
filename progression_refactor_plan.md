# Progression System Refactoring Plan - Hybrid Architecture

## Overview
Refactor progression logic using a **hybrid approach** that separates core progression logic from battle integration:
- **`src/progression/`** = Pure calculation logic (reusable across contexts)  
- **`src/battle/progression/`** = Battle-specific integration layer

This enables progression to work both in battles and future contexts (day care, rare candies, trading).

## 1. Core Progression System (`src/progression/`)

### `src/progression/mod.rs`
```rust
pub mod rewards;
pub mod experience; 
pub mod evolution;
pub mod moves;
pub mod participation;

pub use rewards::*;
pub use experience::*;
pub use evolution::*;
pub use moves::*;
pub use participation::*;

// Re-export for backward compatibility
pub use rewards::RewardCalculator;
```

### `src/progression/rewards.rs`
- **Move** `RewardCalculator` from current `src/progression.rs`
- **Move** `EvYield` struct from current `src/progression.rs`
- **Keep** base experience and EV yield calculation logic
- **Pure functions** - no battle state dependencies

### `src/progression/experience.rs`
- **Move** experience group logic from schema or current locations
- **Add** level-up calculation functions
- **Add** experience requirement calculations
- **Pure calculation logic**

### `src/progression/evolution.rs`
- **Move** evolution logic from current `src/progression.rs`
- **Add** evolution condition checking
- **Pure evolution rule logic**

### `src/progression/moves.rs`
- **Move** move learning logic from current `src/progression.rs`
- **Add** learnset management functions
- **Pure move learning calculations**

### `src/progression/participation.rs`
- **Move** `BattleParticipationTracker` from current `src/progression.rs`
- **Keep** participation tracking logic

## 2. Battle Integration Layer (`src/battle/progression/`)

### `src/battle/progression/mod.rs`
```rust
pub mod commands;
pub mod calculation;
pub mod validation;

pub use commands::*;
pub use calculation::*;
pub use validation::*;
```

### `src/battle/progression/calculation.rs`
- **Add** `calculate_progression_commands()` function (similar to `calculate_catch_commands()`)
- **Integrates** core progression logic with battle state
- **Determines when and how** to apply progression during battles
- **Uses** `src/progression/` functions to determine what should happen

### `src/battle/progression/commands.rs`
- **Move** progression command execution logic from `src/battle/commands.rs` (currently TODOs)
- **Implement** all 6 progression command handlers:
  - `execute_award_experience()`
  - `execute_level_up_pokemon()`
  - `execute_learn_move()`
  - `execute_evolve_pokemon()`
  - `execute_distribute_effort_values()`
  - `execute_update_battle_participation()`

### `src/battle/progression/validation.rs`
- **Add** battle-specific validation functions:
  - `can_award_experience_in_battle()`
  - `can_level_up_in_battle()`
  - `can_learn_move_in_battle()`
  - `can_evolve_in_battle()`
- **Battle context validation** (team slots, battle state, etc.)

## 3. Update Existing Files

### `src/progression.rs` (Refactor to module structure)
- **Convert** to directory-based module with `src/progression/mod.rs`
- **Split** current content across specialized files (rewards, experience, etc.)
- **Maintain** backward compatibility through re-exports
- **Keep** as public interface for all progression functionality

### `src/battle/mod.rs`
- **Add** `pub mod progression;` after catch module

### `src/battle/commands.rs`
- **Keep** progression command enum variants (lines 161-186)
- **Keep** progression command event emission (lines 434-531)
- **Remove** progression command execution (lines 890-926, currently TODOs)
- **Update** `HandlePokemonFainted` to call `calculate_progression_commands()`
- **Import** `use crate::battle::progression::calculate_progression_commands;`

### `src/battle/engine.rs`
- **Import** `use crate::battle::progression::calculate_progression_commands;`

## 4. Implementation Steps

### Step 1: Refactor core progression system
1. **Convert** `src/progression.rs` to `src/progression/` directory
2. **Create** specialized modules (rewards, experience, evolution, moves, participation)
3. **Move** existing logic to appropriate modules
4. **Set up** re-exports in `src/progression/mod.rs` for backward compatibility

### Step 2: Create battle integration layer
1. **Create** `src/battle/progression/` directory
2. **Create** battle-specific integration modules (calculation, commands, validation)
3. **Implement** `calculate_progression_commands()` function

### Step 3: Implement command execution
1. **Create** `src/battle/progression/commands.rs` with command execution functions
2. **Implement** all 6 progression command handlers
3. **Follow** existing patterns from catch commands

### Step 4: Create validation layer
1. **Create** `src/battle/progression/validation.rs` 
2. **Add** battle-specific validation functions
3. **Add** error types for progression failures

### Step 5: Integrate with battle system
1. **Update** `HandlePokemonFainted` to call progression calculation
2. **Update** imports in engine.rs and commands.rs
3. **Remove** TODO implementations from commands.rs

### Step 6: Update tests and ensure compatibility
1. **Move** progression tests to appropriate files
2. **Add** integration tests for new progression commands
3. **Ensure** all existing tests still pass
4. **Verify** backward compatibility

## 4. Key Integration Points

### HandlePokemonFainted Flow:
```rust
BattleCommand::HandlePokemonFainted { target } => {
    let mut commands = vec![];
    commands.push(BattleCommand::ClearPlayerState { target: *target });
    
    // NEW: Calculate progression rewards
    let progression_commands = calculate_progression_commands(
        target, battle_state, &participation_tracker
    );
    commands.extend(progression_commands);
    
    return Ok(commands);
}
```

### Benefits:
- **Cleaner separation** of concerns
- **Shorter** `commands.rs` file
- **Modular** progression system
- **Easier testing** of progression logic
- **Consistent** with existing catch pattern
- **Maintainable** codebase structure

### Architecture Benefits:
- **Separation of concerns**: Pure calculation logic separate from battle integration
- **Reusability**: Core progression can be used outside battles (day care, trading, etc.)
- **Maintainability**: Clear boundaries between "what" and "when" logic
- **Future-proof**: Easy to extend for new progression contexts
- **Consistent patterns**: Battle integration follows existing catch pattern
- **Testability**: Pure functions easier to test independently

## Lessons Learned from Implementation

### Key Insights:

1. **Pokemon Data Structure Complexities**:
   - Field names: `curr_exp` not `experience`, `evs` not `effort_values`
   - Move storage: `[Option<MoveInstance>; 4]` not `Vec<Move>`  
   - Stat calculation: No `recalculate_stats()` method - would need `calculate_stats()` + species data
   - Missing methods for progression (level-up stat updates, proper EV application)

2. **Battle Command Enum Mismatches**:
   - `LevelUpPokemon` doesn't have `new_level` field (just triggers level up by 1)
   - `LearnMove` uses `move_` field not `new_move`
   - Commands are more imperative ("do this") vs declarative ("set to this value")

3. **Integration Points**:
   - `BattleParticipationTracker` needs proper storage in `BattleState`
   - Experience → level calculation requires accessing `ExperienceGroup` from species data
   - Stat recalculation needs species base stats + current level/IVs/EVs

4. **Architecture Pattern**:
   - The catch system pattern works well: `calculate_*_commands()` → command list → execution
   - Command execution should be simple state mutations, complex logic in calculation layer
   - Validation layer catches edge cases before calculation

### Remaining Implementation Gaps:

1. **Pokemon Methods Needed**:
   ```rust
   impl PokemonInst {
       pub fn recalculate_stats_with_species(&mut self, species_data: &PokemonSpecies);
       pub fn can_level_up(&self, species_data: &PokemonSpecies) -> Option<u8>;
       pub fn apply_level_up(&mut self, new_level: u8, species_data: &PokemonSpecies);
   }
   ```

2. **BattleState Integration**:
   - Add `participation_tracker: BattleParticipationTracker` field
   - Update tracker during battle turns
   - Pass tracker to progression calculations

3. **Command Refinement**:
   - `LevelUpPokemon` needs level parameter or auto-calculation logic
   - Better move replacement strategies (player choice vs automatic)
   - Evolution stat recalculation integration

4. **Error Handling**:
   - Species data access failures during progression
   - Invalid Pokemon states (fainted, missing)
   - EV/level limits and constraints

## 5. Files to Create/Modify:

### Core Progression System:
- Convert: `src/progression.rs` → `src/progression/` directory
- Create: `src/progression/mod.rs`
- Create: `src/progression/rewards.rs`
- Create: `src/progression/experience.rs`
- Create: `src/progression/evolution.rs`
- Create: `src/progression/moves.rs`
- Create: `src/progression/participation.rs`

### Battle Integration Layer:
- Create: `src/battle/progression/mod.rs`
- Create: `src/battle/progression/calculation.rs`
- Create: `src/battle/progression/commands.rs`
- Create: `src/battle/progression/validation.rs`

### Integration Updates:
- Modify: `src/battle/mod.rs`
- Modify: `src/battle/commands.rs`
- Modify: `src/battle/engine.rs`

### Tests:
- Update: All related tests for new structure
- Add: Integration tests for battle progression