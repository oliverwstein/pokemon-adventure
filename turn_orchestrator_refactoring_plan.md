# Turn Orchestrator Refactoring Plan

## Overview

This document outlines a comprehensive refactoring plan to transform the monolithic `turn_orchestrator.rs` module from a direct state mutation architecture to a command-based system that separates intent from execution. This refactoring addresses architectural concerns around testability, concurrency readiness, and separation of concerns.

## Current Architecture Problems

### 1. Extensive Direct Mutation
- **~150 mutation points** identified across the turn orchestrator
- Direct modifications to `BattleState`, `BattlePlayer`, and `PokemonInst` throughout execution
- State changes scattered across multiple functions without central control

### 2. Monolithic Functions
- `execute_attack_hit()`: 600+ lines handling damage, effects, conditions, and events
- `apply_move_effects()`: Complex branching logic with embedded state mutations
- `execute_end_turn_phase()`: Status processing interleaved with state updates

### 3. Scattered Responsibilities
State mutations occur in multiple locations:
- Turn resolution logic
- Move effect application
- Status condition processing  
- End-of-turn cleanup
- Battle state transitions

### 4. Testing and Concurrency Challenges
- Functions require mutable state making unit testing complex
- Direct mutation prevents parallel execution of battle calculations
- Difficult to isolate and test individual battle mechanics

## Proposed Command/Effect Architecture

### Core Principles
1. **Separation of Intent and Execution**: Pure calculation functions generate commands, single executor applies them
2. **Immutable Calculations**: All battle logic becomes pure functions operating on read-only state
3. **Atomic Result Commands**: Commands represent final, validated state changes, not procedural steps
4. **Simple Executor**: CommandExecutor only applies state mutations and emits events, no interpretation logic
5. **Deterministic RNG Integration**: TurnRng oracle pattern maintained for reproducible calculations while preserving pure function benefits

### Command System Design

#### Command Types
```rust
#[derive(Debug, Clone)]
pub enum BattleCommand {
    // Direct state changes
    SetGameState(GameState),
    IncrementTurnNumber,
    ClearActionQueue,
    
    // Pokemon modifications
    DealDamage { target: PlayerTarget, amount: u16 },
    HealPokemon { target: PlayerTarget, amount: u16 },
    SetPokemonStatus { target: PlayerTarget, status: Option<StatusCondition> },
    FaintPokemon { target: PlayerTarget },
    RestorePP { target: PlayerTarget, move_slot: usize, amount: u8 },
    
    // Player state changes
    ChangeStatStage { target: PlayerTarget, stat: StatType, delta: i8 },
    AddCondition { target: PlayerTarget, condition: PokemonCondition },
    RemoveCondition { target: PlayerTarget, condition_type: PokemonConditionType },
    AddTeamCondition { target: PlayerTarget, condition: TeamCondition, turns: u8 },
    RemoveTeamCondition { target: PlayerTarget, condition: TeamCondition },
    SetLastMove { target: PlayerTarget, move_used: Move },
    SwitchPokemon { target: PlayerTarget, new_pokemon_index: usize },
    
    // Battle flow
    EmitEvent(BattleEvent),
    PushAction(BattleAction),
    
    // Note: Only atomic, result-based commands that represent final state changes
    // No procedural commands like "CheckMoveHit" - all calculations happen in pure functions
}

#[derive(Debug, Clone, Copy)]
pub enum PlayerTarget { 
    Player1, 
    Player2,
}

impl PlayerTarget {
    pub fn to_index(self) -> usize {
        match self {
            PlayerTarget::Player1 => 0,
            PlayerTarget::Player2 => 1,
        }
    }
    
    pub fn opponent(self) -> PlayerTarget {
        match self {
            PlayerTarget::Player1 => PlayerTarget::Player2,
            PlayerTarget::Player2 => PlayerTarget::Player1,
        }
    }
}
```

#### Command Executor
```rust
pub struct CommandExecutor<'a> {
    state: &'a mut BattleState,
    event_bus: &'a mut EventBus,
    action_stack: &'a mut ActionStack,
}

impl<'a> CommandExecutor<'a> {
    pub fn new(
        state: &'a mut BattleState, 
        event_bus: &'a mut EventBus,
        action_stack: &'a mut ActionStack,
    ) -> Self {
        Self { state, event_bus, action_stack }
    }
    
    pub fn execute_commands(&mut self, commands: Vec<BattleCommand>) -> Result<(), ExecutionError> {
        for command in commands {
            self.execute_command(command)?;
        }
        Ok(())
    }
    
    fn execute_command(&mut self, command: BattleCommand) -> Result<(), ExecutionError> {
        match command {
            BattleCommand::DealDamage { target, amount } => {
                self.execute_deal_damage(target, amount)
            }
            BattleCommand::EmitEvent(event) => {
                self.event_bus.push(event);
                Ok(())
            }
            BattleCommand::PushAction(action) => {
                self.action_stack.push_back(action);
                Ok(())
            }
            // ... handle all other atomic command types
        }
    }
}

#[derive(Debug)]
pub enum ExecutionError {
    NoPokemon,
    InvalidPlayerIndex,
    InvalidPokemonIndex,
    InvalidMove,
    StateValidationError(String),
}
```

### Main Turn Resolution Loop

The new `resolve_turn` function maintains the existing action-based flow while using the command pattern:

```rust
pub fn resolve_turn(battle_state: &mut BattleState, mut rng: TurnRng) -> EventBus {
    let mut event_bus = EventBus::new();
    let mut action_stack = ActionStack::new();
    
    // Phase 1: Setup turn and populate action stack
    let setup_commands = calculate_turn_setup_commands(battle_state);
    {
        let mut executor = CommandExecutor::new(battle_state, &mut event_bus, &mut action_stack);
        executor.execute_commands(setup_commands).unwrap();
    }
    
    // Phase 2: Process each action sequentially
    while let Some(action) = action_stack.pop_front() {
        // Read current state, calculate all effects, generate atomic commands
        let action_commands = match action {
            BattleAction::AttackHit { attacker_index, defender_index, move_used, .. } => {
                calculate_attack_outcome(battle_state, attacker_index, defender_index, move_used, &mut rng)
            }
            BattleAction::Switch { player_index, target_pokemon_index } => {
                calculate_switch_outcome(battle_state, player_index, target_pokemon_index)
            }
            BattleAction::Forfeit { player_index } => {
                calculate_forfeit_outcome(battle_state, player_index)
            }
            // ... other action types
        };
        
        // Apply all calculated changes atomically
        {
            let mut executor = CommandExecutor::new(battle_state, &mut event_bus, &mut action_stack);
            executor.execute_commands(action_commands).unwrap();
        }
    }
    
    // Phase 3: End of turn processing
    let cleanup_commands = calculate_end_of_turn_commands(battle_state);
    {
        let mut executor = CommandExecutor::new(battle_state, &mut event_bus, &mut action_stack);
        executor.execute_commands(cleanup_commands).unwrap();
    }
    
    event_bus
}
```

### Pure Calculation Functions

Transform existing mutation functions into pure calculators:

#### Attack Resolution
```rust
// Current: execute_attack_hit(&mut BattleState, ...) 
// New: calculate_attack_outcome(&BattleState, ...) -> Vec<BattleCommand>

pub fn calculate_attack_outcome(
    state: &BattleState,
    attacker_index: usize,
    defender_index: usize, 
    move_used: Move,
    rng: &mut TurnRng
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    
    let attacker_target = PlayerTarget::from_index(attacker_index);
    let defender_target = PlayerTarget::from_index(defender_index);
    
    // Pure calculations - no mutations
    let attacker = &state.players[attacker_index];
    let defender = &state.players[defender_index];
    let attacker_pokemon = attacker.active_pokemon().unwrap();
    let defender_pokemon = defender.active_pokemon().unwrap();
    
    // Check if move hits
    let hit_result = move_hits(
        attacker_pokemon, defender_pokemon, 
        attacker, defender, move_used, rng
    );
    
    if !hit_result {
        commands.push(BattleCommand::EmitEvent(BattleEvent::MoveMissed {
            attacker: attacker_pokemon.species,
            defender: defender_pokemon.species,
            move_used,
        }));
        return commands;
    }
    
    // Move hits - emit hit event
    commands.push(BattleCommand::EmitEvent(BattleEvent::MoveHit {
        attacker: attacker_pokemon.species,
        defender: defender_pokemon.species,
        move_used,
    }));
    
    // Check for critical hit
    let is_critical = move_is_critical_hit(attacker_pokemon, attacker, move_used, rng);
    if is_critical {
        commands.push(BattleCommand::EmitEvent(BattleEvent::CriticalHit {
            attacker: attacker_pokemon.species,
            defender: defender_pokemon.species,
            move_used,
        }));
    }
    
    // Calculate damage
    let damage = calculate_attack_damage(
        attacker_pokemon, defender_pokemon,
        attacker, defender, move_used, is_critical, rng
    );
    
    if damage > 0 {
        commands.push(BattleCommand::DealDamage { 
            target: defender_target, 
            amount: damage 
        });
    }
    
    // Generate move effect commands
    commands.extend(calculate_move_effect_commands(
        state, attacker_index, defender_index, move_used, damage, rng
    ));
    
    // Check for fainting after damage
    if defender_pokemon.current_hp().saturating_sub(damage) == 0 {
        commands.push(BattleCommand::FaintPokemon { target: defender_target });
        commands.push(BattleCommand::EmitEvent(BattleEvent::PokemonFainted {
            player_index: defender_index,
            pokemon: defender_pokemon.species,
        }));
    }
    
    commands
}
```

#### Move Effects
```rust
pub fn calculate_move_effect_commands(
    state: &BattleState,
    attacker_index: usize,
    defender_index: usize,
    move_used: Move,
    damage_dealt: u16,
    rng: &mut TurnRng
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    let move_data = get_move_data(move_used).expect("Move data should exist");
    
    let attacker_target = PlayerTarget::from_index(attacker_index);
    let defender_target = PlayerTarget::from_index(defender_index);
    
    for effect in &move_data.effects {
        match effect {
            MoveEffect::Heal(percentage) => {
                let heal_amount = calculate_heal_amount(damage_dealt, *percentage);
                commands.push(BattleCommand::HealPokemon {
                    target: attacker_target,
                    amount: heal_amount,
                });
            }
            MoveEffect::Recoil(percentage) => {
                let recoil_damage = calculate_recoil_damage(damage_dealt, *percentage);
                commands.push(BattleCommand::DealDamage {
                    target: attacker_target,
                    amount: recoil_damage,
                });
            }
            MoveEffect::Poison(chance) => {
                if rng.next_outcome() <= *chance {
                    commands.push(BattleCommand::SetPokemonStatus {
                        target: defender_target,
                        status: Some(StatusCondition::Poison),
                    });
                }
            }
            // Handle all other move effects...
        }
    }
    
    commands
}
```

#### End of Turn Processing
```rust
pub fn calculate_end_of_turn_commands(
    state: &BattleState,
    player_index: usize
) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    let player_target = PlayerTarget::from_index(player_index);
    let player = &state.players[player_index];
    
    // Process status conditions
    if let Some(pokemon) = player.active_pokemon() {
        if let Some(status) = &pokemon.status {
            commands.extend(calculate_status_damage_commands(player_target, status));
        }
    }
    
    // Process active conditions (Leech Seed, binding moves, etc.)
    for condition in player.active_pokemon_conditions.values() {
        commands.extend(calculate_condition_commands(state, player_target, condition));
    }
    
    // Process team conditions (Reflect, Light Screen expiration)
    for (condition, turns_remaining) in &player.team_conditions {
        if *turns_remaining <= 1 {
            commands.push(BattleCommand::RemoveTeamCondition {
                target: player_target,
                condition: condition.clone(),
            });
        }
    }
    
    commands
}
```

## Implementation Strategy: The "Hollowing Out" Method

This incremental approach keeps existing function signatures intact while progressively replacing their internal logic with the command-based pattern. This leverages the existing integration test suite at every step.

### Step 0: Build the Scaffolding (Non-Breaking Foundation)

1. **Create the Command System**
   ```bash
   # Create new module without touching existing code
   touch src/battle/commands.rs
   ```
   - Implement `BattleCommand` enum and private `CommandExecutor`
   - Create helper function `execute_commands_locally()` for bridging

2. **Unit Test the Executor**
   ```rust
   // tests/test_command_executor.rs
   #[test]
   fn test_deal_damage_command() {
       let mut state = create_test_battle_state();
       let mut bus = EventBus::new();
       
       execute_commands_locally(vec![
           BattleCommand::DealDamage { target: 0, amount: 20 }
       ], &mut state, &mut bus);
       
       assert_eq!(state.players[0].active_pokemon().unwrap().current_hp(), 80);
   }
   ```

**Result**: New code and tests added, existing system 100% functional.

### Step 1: Create the First "Bubble" of Purity

1. **Implement Minimal Calculator**
   ```rust
   // src/battle/calculators.rs
   pub fn calculate_attack_outcome(
       state: &BattleState,
       attacker_index: usize,
       defender_index: usize,
       move_used: Move,
       rng: &mut TurnRng
   ) -> Vec<BattleCommand> {
       // Start with ONLY hit/miss logic
       let hit_result = move_hits(/* ... */);
       
       if hit_result {
           vec![BattleCommand::EmitEvent(BattleEvent::MoveHit { /* ... */ })]
       } else {
           vec![BattleCommand::EmitEvent(BattleEvent::MoveMissed { /* ... */ })]
       }
   }
   ```

2. **Unit Test Only This Function**
   ```rust
   // tests/test_calculators.rs
   #[test]
   fn test_calculate_attack_outcome_hit() {
       let state = create_test_state();
       let mut rng = TurnRng::new_for_test(vec![1]); // Forces hit
       
       let commands = calculate_attack_outcome(&state, 0, 1, Move::Tackle, &mut rng);
       
       assert_eq!(commands.len(), 1);
       assert!(matches!(commands[0], BattleCommand::EmitEvent(BattleEvent::MoveHit { .. })));
   }
   ```

### Step 2: The Bridge - Connect New World to Old

Modify existing `execute_attack_hit()` to use the calculator:

```rust
// src/battle/turn_orchestrator.rs
pub fn execute_attack_hit(/* existing signature */) {
    // ... existing setup code remains ...

    // === THE BRIDGE ===
    // 1. Call the new pure calculator for hit/miss
    let hit_miss_commands = calculate_attack_outcome(battle_state, attacker_index, defender_index, move_used, rng);
    
    // 2. Execute immediately using local bridge function
    execute_commands_locally(hit_miss_commands, battle_state, bus, action_stack);
    
    // 3. DELETE the old hit/miss logic that was here
    // === END BRIDGE ===
    
    // ... rest of old damage/effects code remains for now ...
}

// Temporary bridge function
fn execute_commands_locally(
    commands: Vec<BattleCommand>, 
    state: &mut BattleState, 
    bus: &mut EventBus,
    action_stack: &mut ActionStack
) {
    for command in commands {
        match command {
            BattleCommand::EmitEvent(event) => bus.push(event),
            BattleCommand::DealDamage { target, amount } => {
                // Direct mutation, just like old code
                if let Some(pokemon) = state.players[target].active_pokemon_mut() {
                    pokemon.take_damage(amount);
                }
            }
            // Add other handlers as needed
        }
    }
}
```

**Result**: Run existing test suite - should still pass! Small piece of logic now pure.

**Key Point**: The `execute_commands_locally` bridge function is the linchpin of this strategy. Make its initial implementation robust enough to handle the first few commands (EmitEvent, DealDamage, SetPokemonStatus) with comprehensive unit tests.

### Step 3: The Incremental Loop - Grow Pure, Shrink Impure

Repeat this loop until `execute_attack_hit()` is hollowed out:

1. **Pick Next Logic Piece** (e.g., damage calculation)
2. **Add to Calculator**
   ```rust
   pub fn calculate_attack_outcome(/* ... */) -> Vec<BattleCommand> {
       let mut commands = Vec::new();
       
       // Hit/miss logic (already done)
       let hit_result = move_hits(/* ... */);
       if !hit_result {
           return vec![BattleCommand::EmitEvent(BattleEvent::MoveMissed { /* */ })];
       }
       
       commands.push(BattleCommand::EmitEvent(BattleEvent::MoveHit { /* */ }));
       
       // NEW: Add damage calculation
       let damage = calculate_attack_damage(/* ... */);
       if damage > 0 {
           commands.push(BattleCommand::DealDamage { target: defender_index, amount: damage });
       }
       
       commands
   }
   ```

3. **Add Handler to Bridge**
   ```rust
   fn execute_commands_locally(/* ... */) {
       for command in commands {
           match command {
               BattleCommand::EmitEvent(event) => bus.push(event),
               BattleCommand::DealDamage { target, amount } => {
                   // Handle damage application
               }
               // Add more as calculator grows
           }
       }
   }
   ```

4. **Delete Old Code** from `execute_attack_hit()`
5. **Run Tests** - should still pass

**Continue the loop for:**
- Critical hit calculation → `BattleCommand::EmitEvent(CriticalHit)`
- Status effects → `BattleCommand::SetPokemonStatus`
- Stat changes → `BattleCommand::ChangeStatStage`
- Move effects → Various commands
- Fainting → `BattleCommand::FaintPokemon`

**Note**: This phase requires careful attention to read-after-write simulation. When moving recoil logic, the calculator must track that initial damage was dealt to calculate recoil amount, then simulate the attacker's HP reduction to determine if recoil causes fainting. This internal state tracking is unavoidable but becomes manageable when done incrementally with tests at each step.

### Step 4: The Final Switch

When `execute_attack_hit()` is just setup + bridge + cleanup:

1. **Rewrite `resolve_turn()`** to use the new pattern fully
2. **Create public `CommandExecutor`** and remove bridge functions
3. **Delete hollowed-out functions**
4. **Run final test suite**

### Benefits of This Approach

- **Always Compiling**: Never have broken code for extended periods
- **Always Passing Tests**: Existing integration tests validate each step
- **Small Victories**: Each step is a verifiable improvement
- **Low Risk**: Easy to revert any single step if needed
- **Incremental Understanding**: Learn the system progressively rather than all at once

This transforms a potentially overwhelming refactor into a series of small, confident steps where each iteration leaves you with a working, slightly improved system.

## Benefits and Outcomes

### Immediate Benefits
- **Improved Testability**: Pure functions easier to unit test with predictable inputs/outputs
- **Better Debugging**: Command stream provides clear audit trail of all state changes
- **Cleaner Separation**: Calculation logic cleanly separated from state mutation

### Long-term Benefits
- **Concurrency Ready**: Read-only calculations can run in parallel threads
- **Extensibility**: New moves and effects just generate appropriate commands
- **Maintainability**: Single executor handles all mutations with consistent validation

### Architectural Improvements
- **Single Responsibility**: Each function has one clear purpose
- **Immutable Calculations**: Battle logic becomes pure and predictable
- **Command Pattern**: Explicit representation of all state changes

## Implementation Best Practices

### Design Decisions
1. **PlayerTarget vs usize**: **Recommendation: Stick with PlayerTarget.** The compile-time safety it provides within pure calculator functions is invaluable - you can't accidentally create a command for `player_index: 2`. Let the CommandExecutor be the only place that translates PlayerTarget to usize, enforcing a strong boundary between the safe logical world of calculations and the "real" world of array indices.

2. **Calculator Function Organization**: As `calculate_move_effect_commands` grows, break it into focused helper functions:
   ```rust
   pub fn calculate_move_effect_commands(...) -> Vec<BattleCommand> {
       let mut commands = Vec::new();
       
       for effect in &move_data.effects {
           match effect {
               MoveEffect::Poison(_) | MoveEffect::Burn(_) | MoveEffect::Paralyze(_) => {
                   commands.extend(calculate_status_effect_commands(effect, ...));
               }
               MoveEffect::AttackUp(_) | MoveEffect::DefenseDown(_) => {
                   commands.extend(calculate_stat_change_commands(effect, ...));
               }
               MoveEffect::Heal(_) | MoveEffect::Recoil(_) => {
                   commands.extend(calculate_damage_effect_commands(effect, ...));
               }
               // ... other effect categories
           }
       }
       
       commands
   }
   ```

3. **Calculation Function Granularity**: Keep individual calculator functions focused on single responsibilities (attack outcome, switch outcome, status processing) while using helper functions for effect categories.

### Critical Implementation Details

#### RemoveCondition Command Design
The `RemoveCondition` command needs a condition type without data payload:

```rust
// In commands.rs or player.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PokemonConditionType {
    Flinched,
    Confused,
    Seeded,
    Transformed,
    Charging,
    // ... all other variants without their data
}

impl PokemonCondition {
    pub fn get_type(&self) -> PokemonConditionType {
        match self {
            PokemonCondition::Flinched => PokemonConditionType::Flinched,
            PokemonCondition::Confused { .. } => PokemonConditionType::Confused,
            PokemonCondition::Seeded { .. } => PokemonConditionType::Seeded,
            // ... etc.
        }
    }
}
```

#### Read-After-Write Simulation
During the incremental loop, calculators must simulate state changes internally:

```rust
// Example: Recoil damage calculation
pub fn calculate_attack_outcome(/* ... */) -> Vec<BattleCommand> {
    let mut commands = Vec::new();
    
    // Calculate initial damage
    let damage = calculate_attack_damage(/* ... */);
    commands.push(BattleCommand::DealDamage { target: defender, amount: damage });
    
    // Calculate recoil (needs to know damage dealt)
    let recoil = (damage as f64 * 0.25) as u16;
    commands.push(BattleCommand::DealDamage { target: attacker, amount: recoil });
    
    // Check if recoil causes fainting (needs to simulate HP reduction)
    let attacker_pokemon = state.players[attacker_index].active_pokemon().unwrap();
    if attacker_pokemon.current_hp().saturating_sub(recoil) == 0 {
        commands.push(BattleCommand::FaintPokemon { target: attacker });
    }
    
    commands
}
```

This internal simulation is unavoidable but testable when done incrementally.

#### Litmus Test: Calculator vs Executor Responsibility

**Calculator (calculate_*)**: Answers "What should happen?"
- Contains all game rules, if/else branches, RNG consumption, formulas
- Output is declarative list of results
- Pure functions with no side effects

**Executor (execute_command)**: Answers "How do I apply this change?"
- Contains no game logic
- Simple, dumb machine that applies state mutations
- Single action per command (e.g., `state.players[i].take_damage()`)

If you're tempted to add game logic to the executor, it belongs in a calculator instead.

## Risk Mitigation

### Potential Risks
1. **Performance overhead** from command generation and execution
2. **Increased complexity** during transition period
3. **Regression risks** from major architectural changes

### Mitigation Strategies
1. **Incremental migration** with continuous testing
2. **Performance benchmarking** at each phase
3. **Comprehensive test coverage** to catch regressions
4. **Backward compatibility** during transition

## Success Metrics

- **Code Quality**: Reduced cyclomatic complexity, improved separation of concerns
- **Test Coverage**: Increased unit test coverage, faster test execution
- **Performance**: No significant performance degradation
- **Maintainability**: Easier addition of new moves and effects

This refactoring transforms the turn orchestrator from a monolithic state mutator into a clean pipeline: **Intent → Commands → Execution**, providing a solid foundation for future enhancements and concurrent battle processing.