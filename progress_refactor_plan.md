## Design Plan: Refactoring the Battle Engine for State-Driven Command Processing

**Author:** Gemini
**Date:** August 29, 2025
**Version:** 1.0

### 1. Executive Summary

This document outlines a plan to refactor the core architecture of the Pokémon Adventure battle engine. The current system, which resolves an entire turn in a single function call, is rigid. It faces two primary challenges: an increasingly large `BattleCommand` enum that is hard to maintain, and no mechanism to gracefully pause the battle to ask for player input (e.g., "Which move to replace?").

The solution is to transition to a **state-driven, step-by-step command processing model**. This will be achieved by making a command stack a persistent part of the `BattleState` and using the `GameState` enum as the master conductor for the engine's flow.

This refactor will solve the immediate problems and create a more robust, scalable, and debuggable foundation. It also allows for greater code reuse by creating modular, self-contained components like a `ProgressionCommand` system that can be used outside of battles. We currently have the progression system semi-cordoned from the Battle system, but the commands for it are still called BattleCommands. 

### 2. Goals and Non-Goals

#### **Goals:**

1.  **Restructure `BattleCommand`:** Decompose the monolithic enum into smaller, namespaced sub-enums for improved clarity and maintainability.
2.  **Enable Implicit Pausing:** Implement a mechanism where the engine pauses itself by changing its `GameState` to a `WaitingFor...` variant, decoupling the core logic from the UI.
3.  **Decouple Execution:** Replace the monolithic `resolve_turn` function with a granular `advance_battle` function that processes one command at a time from a persistent stack, continuing until the GameState enters a waiting state.

#### **Non-Goals:**

1.  **Change Battle Mechanics:** This is primarily an architectural refactor. Existing damage formulas, move effects, and Pokémon data will not be altered. However, note that there currently is an ActionStack in the BattleState that has BattleActions. It will be stupid and dumb to have both BattleActions and BattleCommands. Right now, BattleActions mostly serves to enable multi-hit moves. Moving to the new architecture should allow us to do away with this once it is in place, but we don't want to make that change until we have the new architecture working and we can take advantage of it.
2.  **Implement a GUI:** The front-end will remain a command-line interface.
3.  **Add New Game Features (beyond the proof-of-concept):** The "learn move" choice will be the test case for the new architecture, but no other new features are in scope.

### 3. Core Architectural Changes

#### 3.1. The `BattleState` as the Single Source of Truth

**Problem:** The current engine's execution flow is managed by temporary stacks that are lost when the `resolve_turn` function returns. This makes pausing impossible.

**Solution:** We will add a command stack directly to the `BattleState` struct, which will be distinct from the ActionStack.

```rust
// In: src/battle/state.rs

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BattleState {
    // ... existing fields ...
    
    // NEW: A persistent stack for all commands to be executed.
    // We will use a Vec as a LIFO stack (last-in, first-out).
    pub command_stack: Vec<BattleCommand>, 
}
```

**Reasoning:** By making the command stack part of the `BattleState`, the execution flow itself becomes part of the game's persistent state. This is the key change that enables pausing, as the engine can stop and resume processing without losing track of what it needs to do next.

#### 3.2. The `GameState` Enum as the Engine's Conductor

**Problem:** The current `GameState` cannot represent the specific reason for a pause.

**Solution:** We will add new, highly specific variants to the `GameState` enum.

```rust
// In: src/battle/state.rs

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Copy)]
pub enum GameState {
    // ... existing states
    WaitingForMoveLearnChoice {
        player_index: usize,
        pokemon_index: usize,
        new_move: Move,
    },
    WaitingForEvolutionChoice {
        player_index: usize,
        pokemon_index: usize,
        new_species: Species,
    },
    // ...
}
```

**Reasoning:** The main game loop will become a state machine. If the state is `TurnInProgress`, it processes a command. If the state is `WaitingFor...`, it stops processing and waits for the UI to respond. The data stored in the enum variants gives the UI all the context it needs. **Changing the state *is* the pause signal.** No explicit `Pause` command is needed.

#### 3.3. Refactored `BattleCommand` and Reusable `ProgressionCommand`

**Problem:** The `BattleCommand` enum is a massive, flat list. Furthermore, progression logic is tightly coupled to the battle system.

**Solution:** We will group related commands into nested enums and create a standalone `ProgressionCommand` that can be reused.

```rust
// In: src/battle/commands.rs
#[derive(Debug, Clone)]
pub enum BattleCommand {
    State(StateCommand),
    Pokemon(PokemonCommand),
    Player(PlayerCommand),
    Progression(ProgressionCommand), // Wrapper
    Flow(FlowCommand),
}

// In a new file: src/progression/commands.rs (or similar)
#[derive(Debug, Clone)]
pub enum ProgressionCommand {
    AwardExperience { recipients: Vec<(/*...*/)> },
    LevelUpPokemon { /*...*/ },
    // ... etc.
}
```

**Reasoning:** This provides clear namespacing (`BattleCommand::Progression(...)`). More importantly, it establishes a "language" for the progression system that is independent of the battle engine, allowing other features like items (`Rare Candy`) to use the same reliable progression logic.

### 4. Step-by-Step Implementation Plan

Follow these steps in order. Use `cargo check` frequently; the compiler errors will be your guide.

#### **Phase 1: Foundational Type Changes**

*Objective: Update the core data structures. This will cause compile errors across the codebase, which will serve as a to-do list for the next phases.*

1.  **Modify `BattleState`:** In `src/battle/state.rs`, add `pub command_stack: Vec<BattleCommand>` to the `BattleState` struct and initialize it as empty in `BattleState::new`.
2.  **Expand `GameState`:** In `src/battle/state.rs`, add the new `WaitingForMoveLearnChoice` and `WaitingForEvolutionChoice` variants to the `GameState` enum.
3.  **Refactor `BattleCommand`:** In `src/battle/commands.rs`, restructure the `BattleCommand` enum into the nested structure. Create the new sub-enums (`StateCommand`, `PokemonCommand`, etc.) and move the existing variants into their appropriate categories. **Do not move progression-related commands yet.**

#### **Phase 2: Create the Modular Progression System**

*Objective: Decouple the progression logic from the battle system to make it reusable.*

1.  **Create `ProgressionCommand`:** In `src/battle/progression/mod.rs` (or a new sub-file), define the `ProgressionCommand` enum. Move all progression-related command definitions (e.g., `AwardExperience`, `LevelUpPokemon`) from `BattleCommand` into this new enum.
2.  **Create Progression Executor:** Create a new file `src/progression/executor.rs`. Move the corresponding execution logic (e.g., `execute_level_up_pokemon`) from `src/battle/commands.rs` into this new file.
3.  **Refactor Executor Signatures:** Modify the functions in `executor.rs` to remove their dependency on `BattleState`. They should operate on the most specific types possible, like `&mut PokemonInst`. They should return `Result<Vec<ProgressionCommand>, ...>`.
4.  **Update `BattleCommand`:** In `src/battle/commands.rs`, ensure the `BattleCommand::Progression` variant wraps the new `ProgressionCommand` enum.

#### **Phase 3: Adapt the Command Execution Flow**

*Objective: Fix the compile errors from the previous phases by adapting all code to the new structures.*

1.  **Update Command Instantiation:** Search the codebase for every place a `BattleCommand` is created and update it to use the new nested structure (e.g., `BattleCommand::Pokemon(PokemonCommand::DealDamage { ... })`).
2.  **Refactor `execute_command`:** In `src/battle/commands.rs`, rewrite `execute_command` to be `execute_command_and_requeue`. Its job is to take a command, execute it, and push any new commands onto the `battle_state.command_stack`.
    *   The main `match` will delegate to new helper functions like `execute_pokemon_command`.
    *   The `execute_progression_command` helper will be a **bridge**: it extracts the needed data from `BattleState`, calls the appropriate function in `progression::executor`, gets back `ProgressionCommand`s, wraps them in `BattleCommand::Progression`, and returns them.

#### **Phase 4: Rework the Main Engine Loop**

*Objective: Replace the monolithic `resolve_turn` function with the new state-driven loop.*

1.  **Define New Engine Functions:** In `src/battle/engine.rs`:
    *   `initialize_turn(battle_state: &mut BattleState)`: Takes queued actions, generates all commands for the turn, reverses them, and pushes them to `battle_state.command_stack`. Sets `GameState` to `TurnInProgress`.
    *   `advance_battle(battle_state: &mut BattleState, ...)`: Pops one command from `battle_state.command_stack` and calls `execute_command_and_requeue`.
    *   `finalize_turn(battle_state: &mut BattleState)`: Handles end-of-turn logic (win checks, etc.) and sets `GameState` to `WaitingForActions`.
2.  **Rewrite the Main Game Loop (`run_game_loop` in `src/main.rs`):**
    *   Delete the call to `resolve_turn`.
    *   Implement the new state machine logic:
        *   `match battle_state.game_state`:
            *   `GameState::WaitingForActions`: Collect input and call `initialize_turn`.
            *   `GameState::TurnInProgress`: If `command_stack` is empty, call `finalize_turn`. Otherwise, call `advance_battle`.
            *   `GameState::WaitingFor...`: Call the appropriate UI handler function.

#### **Phase 5: Implement the First Interactive Pause**

*Objective: Use the new architecture to implement the "learn move" player choice.*

1.  **Modify `execute_learn_move`:** In `src/progression/executor.rs`, update this function. If the moveset is full, it should not modify the Pokémon. Instead, it should `return Ok(vec![...])` containing a single `ProgressionCommand` that will be wrapped into a `BattleCommand` which sets the `GameState` to `WaitingForMoveLearnChoice`.
2.  **Implement the UI Handler (`handle_move_learn_choice` in `src/main.rs`):**
    *   This function is called by the main loop when the state is `WaitingForMoveLearnChoice`.
    *   It will display the prompt to the user and get their input.
    *   Based on the input, it will prepare a new `LearnMove` command with the `replace_index` filled in.
    *   It will **push** this command to `battle_state.command_stack`.
    *   Finally, it will **directly set `battle_state.game_state = GameState::TurnInProgress;`** to unpause the engine and hand control back.
