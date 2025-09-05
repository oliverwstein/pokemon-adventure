# Battle Struct Design

## Overview

The Battle struct is the core finite state machine that manages all battle state and execution. It provides a minimal three-method interface for external systems while handling complex battle logic internally through an action-based execution model.

## Battle Struct Definition

```rust
pub struct Battle {
    pub battle_type: BattleType, // Tournament, Trainer, Wild, Safari
    pub players: [Player; 2],    // Persistent player data only
    pub battle_state: BattleState, // All temporary battle state
    pub battle_commands: [Option<BattleCommand>; 2],
    pub action_stack: Vec<BattleAction>, // LIFO Stack
    pub turn: u8,
}
```

### Field Descriptions

#### `battle_type: BattleType`
Determines which actions are available and how certain mechanics behave:
- **Tournament**: Standard competitive rules, win/loss/draw only
- **Trainer**: Experience gain, evolution, move learning
- **Wild**: Catching mechanics, running away, experience gain  
- **Safari**: Special catching rules, limited turns/balls

#### `players: [Player; 2]`
Contains persistent player data that survives battle context:
- Player identity and metadata (ID, name, type)
- Pokemon teams (up to 6 Pokemon each with persistent state)
- Accumulated ante/prize money
- No battle-specific state (active Pokemon, conditions, etc.)

#### `battle_state: BattleState`
Contains all temporary battle-specific state for both players:
- Active Pokemon indices for each player
- Volatile conditions (Confused, Trapped, Rampaging, Disabled, Biding)
- Battle flags (Exhausted, Underground, Flinched, Seeded, etc.)
- Special transformation flags (Converted, Transformed, Substituted, Countering)
- Stat stage modifications (±6 for each stat per player)
- Team conditions (Reflect, Light Screen, Mist per player)
- Temporary movesets for Transform/Mimic effects
- Last moves used by each player

#### `battle_commands: [Option<BattleCommand>; 2]`
Temporary storage for player commands awaiting execution:
- Commands provided via `submit_commands()` method
- Consumed and cleared when converted to BattleActions
- Used to coordinate multi-player input requirements

#### `action_stack: Vec<BattleAction>`
LIFO stack containing all pending battle actions:
- All battle state mutations occur through action execution
- Complex moves can inject additional actions mid-execution  
- Stack-based execution enables proper priority handling
- Empty stack indicates battle completion

#### `turn: u8`
Current turn counter for battle tracking and time-based effects.

## Core Interface Methods

### `new() -> Battle`
```rust
impl Battle {
    pub fn new(battle_type: BattleType, players: [Player; 2]) -> Self {
        let mut battle = Battle {
            battle_type,
            players,
            battle_state: BattleState::new(),
            battle_commands: [None, None],
            action_stack: Vec::new(),
            turn: 1,
        };
        
        // Initialize with first action
        battle.action_stack.push(BattleAction::RequestBattleCommands);
        battle
    }
}

impl BattleState {
    pub fn new() -> Self {
        Self {
            active_pokemon_indices: [0, 0],
            team_conditions: [TeamConditionSet::new(), TeamConditionSet::new()],
            stat_stages: [StatStageSet::default(), StatStageSet::default()],
            last_moves: [None, None],
            active_conditions: [PokemonConditionSet::new(), PokemonConditionSet::new()],
            simple_flags: [PokemonFlagSet::new(), PokemonFlagSet::new()],
            special_flags: [SpecialFlagSet::new(), SpecialFlagSet::new()],
            temporary_moves: [TempMoveSet::new(), TempMoveSet::new()],
            scattered_coins: 0,
        }
    }
}
```

**Purpose**: Create new battle instance and initialize with starting action
**Initialization**: Sets up empty command slots and pushes initial `RequestBattleCommands`

### `advance() -> GameState`
```rust
pub fn advance(&mut self, events: &mut EventBus, rng: &mut dyn BattleRng) -> GameState {
    // Pop an action. If the stack is empty, this shouldn't happen - generate emergency EndBattle
    if let Some(action) = self.action_stack.pop() {
        // Execute the action, passing a mutable reference to the entire battle
        // so that the action can modify it.
        match action.execute(self, events, rng) {
            Ok(next_state) => next_state,
            Err(_) => GameState::AwaitingInput, // Or some other error state
        }
    } else {
        // Stack should never be empty - this is an error condition
        // Generate emergency EndBattle and set to AwaitingInput
        self.action_stack.push(BattleAction::EndBattle { outcome: BattleResolution::Draw });
        GameState::AwaitingInput
    }
}
```

**Purpose**: Execute one action from the stack and return the resulting game state
**Execution**: Pops action from stack, executes it, returns `Advancing` or `AwaitingInput`
**Error Handling**: Generates emergency `EndBattle` if stack is unexpectedly empty

### `submit_commands() -> Result<(), BattleError>`
```rust
pub fn submit_commands(&mut self, commands: [Option<BattleCommand>; 2]) -> Result<(), BattleError> {
    // Validate and update battle_commands array
    // This method provides commands to satisfy InputRequest requirements
    self.battle_commands = commands;
    Ok(())
}
```

**Purpose**: Provide commands to satisfy input requests when `GameState::AwaitingInput`
**Validation**: Should validate command legality based on current battle state
**Usage**: Called by external systems (BattleRunner) to provide player input

### `get_input_request() -> Option<InputRequest>`
```rust
pub fn get_input_request(&self) -> Option<InputRequest> {
    // The action that paused the engine is the last one pushed onto the stack.
    let waiting_action = self.action_stack.last()?; // Return None if the stack is empty.

    match waiting_action {
        BattleAction::RequestBattleCommands => {
            // Find which human player, if any, still needs to provide a command.
            for i in 0..2 {
                if self.players[i].player_type == PlayerType::Human && self.battle_commands[i].is_none() {
                    return Some(InputRequest::ForTurnActions { player_index: i });
                }
            }
            None // Should not happen if state is AwaitingInput, but good to be safe.
        }

        BattleAction::RequestNextPokemon { p1, p2 } => {
            // Check the specific players flagged as needing replacements
            if *p1 && self.players[0].player_type == PlayerType::Human && self.battle_commands[0].is_none() {
                return Some(InputRequest::ForNextPokemon { player_index: 0 });
            }
            if *p2 && self.players[1].player_type == PlayerType::Human && self.battle_commands[1].is_none() {
                return Some(InputRequest::ForNextPokemon { player_index: 1 });
            }
            None // Neither flagged player is a human who needs to act right now.
        }

        BattleAction::OfferMove { player_index, team_index, new_move } => {
            // Check if the specified player is a human needing to act.
            if self.players[*player_index].player_type == PlayerType::Human && self.battle_commands[*player_index].is_none() {
                return Some(InputRequest::ForMoveToForget {
                    player_index: *player_index,
                    team_index: *team_index,
                    new_move: *new_move,
                });
            }
            None
        }

        BattleAction::OfferEvolution { player_index, team_index, species } => {
            // Check if the specified player is a human needing to act.
            if self.players[*player_index].player_type == PlayerType::Human && self.battle_commands[*player_index].is_none() {
                return Some(InputRequest::ForEvolution {
                    player_index: *player_index,
                    team_index: *team_index,
                    new_species: *species,
                });
            }
            None
        }

        BattleAction::EndBattle { outcome } => {
            // Battle is complete - provide the resolution to external systems
            Some(InputRequest::ForBattleComplete { 
                resolution: *outcome 
            })
        }

        // For any other action, the engine should not be waiting for input.
        _ => None,
    }
}
```

**Purpose**: Query what input is needed when `GameState::AwaitingInput`
**Logic**: Examines top of action stack to determine appropriate input request
**Return**: Specific `InputRequest` variant or `None` if no input needed

## GameState Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameState {
    Advancing,     // FSM can continue execution
    AwaitingInput, // FSM paused, needs external input
}
```

**Binary State Model**: Simple two-state system for FSM control
**Advancing**: Battle can continue executing actions from stack
**AwaitingInput**: Battle paused, external systems must provide input

## Action Execution Model

### Action Stack Management
- **LIFO Structure**: Actions execute in last-in-first-out order
- **Dynamic Injection**: Executing actions can add new actions to stack
- **Priority Handling**: Higher priority actions pushed later (execute first)
- **Complex Sequences**: Multi-turn moves, combo effects managed through action injection

### Command to Action Conversion
When both players have provided commands:
1. **Validation**: Ensure commands are legal for current battle state
2. **Conversion**: Transform `BattleCommand`s into corresponding `BattleAction`s
3. **Priority Resolution**: Determine execution order based on move priority and speed
4. **Stack Population**: Push actions onto stack in reverse priority order
5. **Cleanup**: Clear command slots for next input cycle

## Read-Only Access Methods

The Battle struct should provide various read-only accessor methods for external systems:

```rust
impl Battle {
    pub fn get_active_pokemon(&self, player_index: u8) -> Option<&Pokemon> { /* ... */ }
    pub fn get_battle_type(&self) -> BattleType { /* ... */ }
    pub fn get_turn_number(&self) -> u8 { /* ... */ }
    pub fn get_player(&self, player_index: u8) -> &Player { /* ... */ }
    pub fn is_battle_over(&self) -> bool { /* ... */ }
    // Additional getters as needed for UI/logging/debugging
}
```

## Battle Lifecycle

### 1. Initialization
- Create Battle with `new(battle_type, players)`
- Initial `RequestBattleCommands` action pushed to stack
- Ready to begin FSM execution

### 2. Execution Loop
- External systems call `advance()` repeatedly
- Actions execute and modify battle state
- Continue until `GameState::AwaitingInput` returned

### 3. Input Handling
- External systems call `get_input_request()` to determine needed input
- Appropriate input provided via `submit_commands()`
- Return to execution loop

### 4. Battle Completion
- `EndBattle` action executes, battle finished
- `get_input_request()` returns `ForBattleComplete` with resolution
- External systems can safely dispose of Battle instance

## Design Principles

### Pure FSM Interface
- Only three methods mutate battle state: `new()`, `advance()`, `submit_commands()`
- All other methods are read-only accessors
- Complete battle state encapsulated within struct

### Action-Based Execution
- All state mutations occur through `BattleAction` execution
- Centralized execution model ensures consistency
- Easy to add new battle mechanics through new action types

### External Integration
- Battle struct has no knowledge of AI, UI, or external systems
- Clean interface for any external system to drive battles
- Events provide comprehensive logging without coupling

This design provides a robust, testable battle engine that can support all battle types while maintaining clean separation of concerns and enabling comprehensive testing through deterministic action sequences.