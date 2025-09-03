# Battle Commands Design

## Overview

BattleCommands represent player intentions that are provided to the Battle FSM through the `submit_commands()` interface. They are converted to BattleActions during execution and serve as the primary input mechanism for all player decisions across different battle types.

## BattleCommand Enum Definition

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum BattleCommand {
    SwitchPokemon { team_index: usize },
    UseMove { team_index: usize, chosen_move: Move },
    UseBall { ball: PokeballType },
    Nothing,
    Forfeit,  // Tournament/Trainer battles - concede defeat
    Flee,     // Wild/Safari battles - escape from encounter
    AcceptEvolution { accept: bool },
    ChooseMoveToForget { move_index: usize },
}
```

## Command Types and Usage

### `SwitchPokemon { team_index: usize }`
**Purpose**: Switch to a different Pokemon from the player's team
**Usage**: 
- Standard battle switching for tactical advantage
- Required replacement when active Pokemon faints
- Available in all battle types except Safari (player only)
**Validation**:
- `team_index` must be valid (0-5)
- Target Pokemon must be alive and not already active

**Converts to**: `BattleAction::DoSwitch`

### `UseMove { team_index: usize, chosen_move: Move }`
**Purpose**: Execute a move with the active Pokemon
**Parameters**:
- `team_index`: Index of Pokemon using the move (typically active Pokemon)
- `chosen_move`: Specific move from Pokemon's moveset
**Usage**: Primary combat action in all battle types
**Validation**:
- Pokemon must know the specified move
- Move must exist in Pokemon's current moveset

**Converts to**: `BattleAction::DoMove`

**Note**: Execution-time effects (sleep, paralysis, PP depletion, binding) are handled during action execution, not command validation.

**Edge Case**: If Pokemon learns a new move after queueing this command, the move_index is preserved (reproducing original game behavior where cached move indices could change)

### `UseBall { ball: PokeballType }`
**Purpose**: Attempt to catch wild Pokemon
**Parameters**:
- `ball`: Type of Pokeball being used (Pokeball, Great Ball, Ultra Ball, Safari Ball, etc.)
**Usage**: Only available in Wild and Safari battle types
**Validation**:
- Must be Wild or Safari battle type
- Player must have the specified ball type in inventory

**Converts to**: `BattleAction::ThrowBall`

## `Nothing`
**Purpose**: Handle Bide, post-HyperBeam exhaustion, and disobedience (when strong traded pokemon do not obey the player)
**Usage**: Secondary combat action in all battle types
**Validation**: Can only be injected, cannot be chosen by player input.

**Converts to**: `BattleAction::DoNothing`

### `Forfeit`
**Purpose**: Concede defeat in competitive battles
**Parameters**: None
**Usage**: Available in Tournament and Trainer battle types
- **Tournament**: Player loses, opponent wins
- **Trainer**: Player loses, opponent wins (affects story progression)
**Validation**: 
- Must be Tournament or Trainer battle type
- Always valid within those battle types
- Cannot be used in Wild or Safari battles

**Converts to**: `BattleAction::DoForfeit` → `BattleAction::EndBattle` with opponent victory

### `Flee`
**Purpose**: Escape from wild encounters
**Parameters**: None
**Usage**: Available in Wild and Safari battle types
- **Wild**: Player escapes, no winner (battle ends without resolution)
- **Safari**: End Safari encounter, return to Safari Zone
**Validation**:
- Must be Wild or Safari battle type  
- Always valid within those battle types
- Cannot be used in Tournament or Trainer battles

**Converts to**: `BattleAction::DoFlee` → `BattleAction::EndBattle` with escape resolution

### `AcceptEvolution { accept: bool }`
**Purpose**: Choose whether to allow Pokemon evolution
**Parameters**:
- `accept`: true to evolve, false to cancel evolution
**Usage**: Response to `InputRequest::ForEvolution`
**Context**: 
- Triggered after leveling up when Pokemon meets evolution criteria
- Only available in Trainer and Wild battle types (experience-gaining battles)
**Validation**: Must match current evolution offer

**Converts to**: Evolution processing actions or cancellation

### `ChooseMoveToForget { move_index: usize }`
**Purpose**: Select which move to replace when Pokemon tries to learn a 5th move
**Parameters**:
- `move_index`: Index (0-3) of existing move to replace, or special value for "don't learn"
**Usage**: Response to `InputRequest::ForMoveToForget`
**Context**:
- Triggered when Pokemon levels up and tries to learn new move but moveset is full
- Available in Trainer and Wild battle types
**Validation**: Move index must be valid (0-3) or special "cancel" value

**Converts to**: Move learning/forgetting actions

## Battle Type Restrictions

### Tournament Battles
**Available Commands**: `SwitchPokemon`, `UseMove`, `Forfeit`
**Restrictions**: No catching, fleeing, evolution, or move learning

### Trainer Battles  
**Available Commands**: `SwitchPokemon`, `UseMove`, `Forfeit`, `AcceptEvolution`, `ChooseMoveToForget`
**Restrictions**: No catching (trainer Pokemon cannot be caught) or fleeing

### Wild Encounters
**Available Commands**: `SwitchPokemon`, `UseMove`, `UseBall`, `Flee`, `AcceptEvolution`, `ChooseMoveToForget`
**Special Behavior**: `UseBall` for catching, `Flee` to escape, experience gain enables evolution/move learning

### Safari Zone Battles
**Available Commands**: `UseBall`, `Flee`
**Restrictions**: No moves or switching for player (Safari mechanics only)
**Special Behavior**: Safari balls with special catch mechanics, `Flee` to exit Safari encounter

## Command Validation Philosophy

### Input Submission vs Execution Success

**Command Validation** (at submission time):
- Is this command type allowed for the current input request?
- Does the command reference valid game objects (Pokemon indices, moves, items)?
- Is this command permitted by the current battle type?

**Execution Success** (during action processing):
- Does the Pokemon have PP for this move?
- Is the Pokemon prevented from acting by status conditions?
- Are there battlefield conditions preventing this action?

### Examples of Validation vs Execution

```rust
// VALID COMMAND - player can submit this
BattleCommand::UseMove { team_index: 0, chosen_move: Move::Tackle }
// Even if Pokemon is asleep, paralyzed, or has 0 PP - these are execution concerns

// INVALID COMMAND - should be rejected at submission
BattleCommand::UseMove { team_index: 0, chosen_move: Move::Surf }
// If Pokemon doesn't know Surf - this is a submission validation error

// VALID COMMAND - battle type allows
BattleCommand::UseBall { ball: PokeballType::Pokeball }
// In Wild battle - even if player has 0 Pokeballs (inventory checked at execution)

// INVALID COMMAND - battle type restriction  
BattleCommand::UseBall { ball: PokeballType::Pokeball }
// In Tournament battle - command type not allowed regardless of inventory
```

## Input Request Context

Commands are only valid when responding to appropriate InputRequests:

```rust
match input_request {
    InputRequest::ForTurnActions { player_index } => {
        // Accept: SwitchPokemon, UseMove, UseBall (Wild/Safari only), Forfeit (Tournament/Trainer only), Flee (Wild/Safari only)
    }
    InputRequest::ForNextPokemon { player_index } => {
        // Accept: SwitchPokemon, Forfeit (Tournament/Trainer only), Flee (Wild/Safari only)
    }
    InputRequest::ForMoveToForget { player_index, team_index, new_move } => {
        // Accept: ChooseMoveToForget
    }
    InputRequest::ForEvolution { player_index, team_index, new_species } => {
        // Accept: AcceptEvolution
    }
    InputRequest::ForBattleComplete { resolution } => {
        // No commands accepted, battle is over
    }
}
```

## Command Legality Checking

```rust
impl BattleCommand {
    pub fn is_valid_for_request(
        &self, 
        player_index: usize, 
        battle: &Battle, 
        input_request: &InputRequest
    ) -> Result<(), CommandError> {
        // Check: Command type matches input request
        // Check: Referenced objects exist (Pokemon, moves, items)  
        // Check: Battle type allows this command
        // Do NOT check: Execution success conditions
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandError {
    WrongCommandForRequest,     // Command doesn't match input request type
    InvalidReference,           // Referenced Pokemon/move/item doesn't exist
    BattleTypeRestriction,      // Command not allowed in this battle type
    // Note: No PP, status, or other execution-time errors here
}
```

## Command to Action Conversion

### Conversion Process
1. **Validation**: Ensure command is submittable for current context
2. **Priority Determination**: Calculate action priority based on command type
3. **Action Creation**: Create corresponding BattleAction with necessary parameters
4. **Stack Management**: Push actions onto stack in correct priority order

### Priority Order (highest to lowest)
1. **Switches**: Always execute first (immediate replacement)
2. **Items**: Pokeball usage (in applicable battle types)  
3. **Moves**: Based on move priority from MoveData.priority field and Pokemon speed
4. **Forfeit/Flee**: Processed immediately as EndBattle

**Move Priority Integration**: The new scripting-based move system stores priority directly in `MoveData.priority` (i8 value). Command priority resolution uses this value along with Pokemon speed for turn order determination.

### Simultaneous Commands
When both players provide commands simultaneously:
- **Both Switch**: Execute simultaneously 
- **Both Move**: Order by move priority, then speed, then random tiebreaker
- **Mixed Actions**: Switches go first, then other actions by priority

## AI Command Generation

### AI Integration
```rust
impl BattleAI for ScoringAI {
    fn decide_command(
        &self, 
        player_index: usize, 
        battle: &Battle, 
        rng: &mut dyn BattleRng
    ) -> BattleCommand {
        // Generate appropriate command based on current input request
        match battle.get_input_request() {
            Some(InputRequest::ForTurnActions { .. }) => {
                // Choose between UseMove and SwitchPokemon
                // AI handles execution-time considerations (PP, status effects)
            }
            Some(InputRequest::ForNextPokemon { .. }) => {
                // Choose SwitchPokemon or Forfeit if no Pokemon available
            }
            // AI should not need to handle evolution/move learning in most cases
            _ => BattleCommand::Forfeit // Fallback
        }
    }
}
```

## Testing Patterns

### Command Validation Testing
```rust
#[rstest]
#[case(BattleType::Tournament, BattleCommand::UseBall { ball: PokeballType::Pokeball }, false)]
#[case(BattleType::Wild, BattleCommand::UseBall { ball: PokeballType::Pokeball }, true)]
fn test_command_validity_by_battle_type(
    #[case] battle_type: BattleType,
    #[case] command: BattleCommand,
    #[case] should_be_valid: bool
) {
    let battle = create_test_battle(battle_type);
    let input_request = InputRequest::ForTurnActions { player_index: 0 };
    
    let result = command.is_valid_for_request(0, &battle, &input_request);
    assert_eq!(result.is_ok(), should_be_valid);
}
```

### Execution vs Validation Testing
```rust
#[test]
fn test_sleeping_pokemon_can_receive_move_command() {
    let mut battle = create_test_battle();
    battle.players[0].active_pokemon_mut().status = Some(PokemonStatus::Sleep { turns_remaining: 2 });
    
    let command = BattleCommand::UseMove { team_index: 0, chosen_move: Move::Tackle };
    let input_request = InputRequest::ForTurnActions { player_index: 0 };
    
    // Command validation should pass - player can submit move command
    assert!(command.is_valid_for_request(0, &battle, &input_request).is_ok());
    
    // Execution handling (sleep check) happens during action execution
    // This test only covers command submission validation
}
```

## Design Principles

### Clear Separation of Concerns
- Commands represent player intent, not execution guarantees
- Validation only checks submission eligibility
- Execution success determined during action processing

### Type Safety
- Enum-based commands prevent invalid command construction
- Compile-time validation of command parameters
- Clear error types for submission-time validation

### Permissive Submission
- Allow players to submit any reasonable command
- Let execution system handle success/failure
- Provides better user experience (don't block valid attempts)

This command system provides a clean interface for all player input while properly separating submission validation from execution success, allowing the FSM to handle all complex battle mechanics during action processing.