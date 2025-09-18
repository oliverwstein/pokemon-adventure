# Battle Actions Design

## Overview

BattleActions represent atomic operations that mutate battle state during FSM execution. They are the only mechanism through which battle state changes occur, providing deterministic execution and comprehensive event generation. All battle logic flows through action execution on the action stack.

## Action Categories

BattleActions fall into several categories based on their role in battle flow:

### Input Request Actions
Actions that can trigger `AwaitingInput` state when input is needed:
- `RequestBattleCommands`
- `RequestNextPokemon { p1: bool, p2: bool }`  
- `OfferMove { player_index: u8, team_index: u8, new_move: Move }`
- `OfferEvolution { player_index: u8, team_index: u8, species: Species }`
- `EndBattle { outcome: BattleResolution }`

### Command Execution Actions  
Actions generated from BattleCommands:
- `DoSwitch { player_index: u8, team_index: u8 }`
- `ChooseMove { player_index: u8, team_index: u8, move_index: u8 }`
- `DoMove { player_index: u8, team_index: u8, move_data: MoveData }`
- `DoForfeit { player_index: u8 }`
- `DoFlee { player_index: u8 }`
- `ThrowBall { ball: PokeballType }`

### Battle Flow Actions
Actions that manage turn progression:
- `EndTurn`
- `HandleKnockout`


## BattleAction Enum Definition

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum BattleAction {
    // Actions that can trigger Awaiting Input
    RequestBattleCommands,
    RequestNextPokemon { p1: bool, p2: bool },
    OfferMove { player_index: u8, team_index: u8, new_move: Move },
    OfferEvolution { player_index: u8, team_index: u8, species: Species },

    // Actions generated from BattleCommands
    DoSwitch { player_index: u8, team_index: u8 },
    ChooseMove { player_index: u8, team_index: u8, move_index: u8 },
    DoMove { player_index: u8, team_index: u8, move_data: MoveData },
    DoForfeit { player_index: u8 },
    DoFlee { player_index: u8 },
    ThrowBall { ball: PokeballType },

    // Battle flow control
    EndTurn,

    HandleKnockout { player_index: u8, team_index: u8 },

    // Final battle resolution
    EndBattle { outcome: BattleResolution },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BattleResolution {
    Player1Wins,
    Player2Wins,
    Draw, // Includes wild Pokemon escapes (flee) and other no-winner scenarios
    }
```

## Action Execution Model

### Execution Interface
```rust
impl BattleAction {
    pub fn execute(
        &self, 
        battle: &mut Battle, 
        events: &mut EventBus,
        rng: &mut dyn BattleRng
    ) -> Result<GameState, BattleError> {
        match self {
            // Each action variant has specific execution logic
            // Returns Advancing or AwaitingInput based on results
        }
    }
}
```

### State Mutation Principle
- **ONLY** action execution mutates battle state
- All mutations flow through centralized execution logic
- No direct field access outside of action execution
- Ensures consistency and proper event generation

### Event Generation
Each action emits appropriate events directly during execution:
```rust
impl BattleAction {
    pub fn execute(
        &self, 
        battle: &mut Battle, 
        events: &mut EventBus,
        rng: &mut dyn BattleRng
    ) -> Result<GameState, BattleError> {
        match self {
            BattleAction::DoMove { player_index, team_index, move_data } => {
                // Execute complete move atomically
                self.execute_complete_move(battle, events, rng);
                
                // Only push HandleKnockout if Pokemon fainted
                if self.check_for_knockouts(battle) {
                    battle.action_stack.push(BattleAction::HandleKnockout { 
                        player_index, team_index 
                    });
                }
                
                Ok(GameState::Advancing)
            }
            // ... other actions handle their specific responsibilities
        }
    }
}
```

## Detailed Action Descriptions

### Input Request Actions

#### `RequestBattleCommands`
**Purpose**: Collect primary actions from both players for turn execution
**Execution Logic**:

**Forced Action Check**:
- Check for forced moves (charge attacks, binding effects) and generate commands automatically
- Populate `battle_commands` slots for players with forced actions

**Input Validation**:
- Check if both players have provided commands in `battle_commands` array
- If any command slot is empty, push `RequestBattleCommands` back onto stack and return `AwaitingInput`

**State Mutations**:
- No direct state mutations (handled by generated command execution actions)

**Stack Operations**:
- Convert commands to execution actions:
  - `UseMove` → Push `ChooseMove` (handles prevention check + PP consumption)
  - `Continue` → Push `DoMove` (checks prevention if not already checked this turn)  
  - `SwitchPokemon` → Check trapped condition → Push `DoSwitch` OR emit prevention event
  - `DoForfeit`, `DoFlee`, `ThrowBall` → Direct conversion (no prevention)
- Push `EndTurn` action first (executes last due to LIFO)
- Push actions in REVERSE priority order (switches first, then by move priority/speed)
- Clear `battle_commands` array

**Action Prevention Tracking**:
- **BattleState Prevention Status**: `prevention_checked: [bool; 2]` tracks per-player prevention checks
- **Turn Initialization**: Reset `prevention_checked = [false, false]` at turn start
- **ChooseMove**: Always runs prevention check, sets `prevention_checked[player] = true`
- **DoMove**: Only runs prevention check if `!prevention_checked[player]`
- **Prevention Logic** (when checked):
  - Sleep, Freeze, Paralysis: Status-based prevention with turn decrements
  - Flinch, Disabled: Condition-based prevention with condition removal
  - Confusion: 50% chance for self-damage instead of move execution
- **Switch Prevention** (checked before pushing DoSwitch):
  - Trapped: Emit prevention event instead of pushing DoSwitch

**Event Emission**:
- `TurnStart` events when awaiting input
- No direct action events (execution actions handle their own events)

**Return Value**: `AwaitingInput` if any command missing, `Advancing` otherwise

#### `RequestNextPokemon { p1: bool, p2: bool }`
**Purpose**: Handle Pokemon replacement after fainting
**Execution Logic**:

**Team Validation**:
- Check if flagged players have conscious Pokemon available
- If flagged player has no conscious Pokemon: Push `EndBattle { outcome: opponent_wins }`

**Input Validation**:
- Check if required command slots have commands in `battle_commands` array
- If any required command slot is empty, push `RequestNextPokemon` back onto stack and return `AwaitingInput`

**State Mutations**:
- No direct state mutations (handled by generated DoSwitch actions)

**Stack Operations**:
- Convert replacement commands to `DoSwitch { player_index, team_index }` actions
- Push DoSwitch actions for all flagged players with valid commands
- Clear relevant `battle_commands` slots

**Event Emission**:
- `ReplacementRequired` events when awaiting input
- No direct events (DoSwitch actions handle their own events)

**Return Value**: `AwaitingInput` if commands needed, `Advancing` otherwise

#### `OfferMove { player_index, team_index, new_move }`
**Purpose**: Handle move learning when Pokemon's moveset is full
**Execution Logic**:

**Moveset Check**:
- If moveset is not full (< 4 moves): Add move directly to moveset, push no additional actions

**Input Validation** (if moveset is full):
- Check if `battle_commands[player_index]` has command
- If command slot is empty, push `OfferMove` back onto stack and return `AwaitingInput`

**State Mutations** (if moveset is full and command provided):
- **If move replacement chosen**: Replace specified move with new_move in Pokemon's moveset
- **If learning declined**: No changes to moveset

**Stack Operations**:
- Push no additional actions (move learning is atomic)
- Clear `battle_commands[player_index]` slot

**Event Emission**:
- `MoveLearnOffer` event when awaiting input
- `MoveLearned` or `MoveLearningDeclined` events based on command

**Return Value**: `AwaitingInput` if moveset full and command slot empty, `Advancing` otherwise

#### `OfferEvolution { player_index, team_index, species }`
**Purpose**: Handle Pokemon evolution choice
**Execution Logic**:

**Input Validation**:
- Check if `battle_commands[player_index]` has command
- If command slot is empty, push `OfferEvolution` back onto stack and return `AwaitingInput`

**State Mutations** (if evolution accepted):
- Update Pokemon species to new evolved form
- Recalculate Pokemon stats based on new species
- Maintain current HP percentage relative to new max HP
- Preserve current status conditions and stat stages

**Stack Operations**:
- **If evolution rejected**: Push no additional actions
- **If evolution accepted**: Check if evolved Pokemon learns new move:
  - Push `OfferMove { player_index, team_index, new_move }` (OfferMove handles moveset full/not full cases)
- Clear `battle_commands[player_index]` slot

**Event Emission**:
- `EvolutionAccepted` or `EvolutionRejected` events
- `MoveLearnOffer` event if new move triggers OfferMove

**Return Value**: `AwaitingInput` if command slot empty, `Advancing` otherwise

### Command Execution Actions

#### `DoSwitch { player_index, team_index }`
**Purpose**: Execute Pokemon switching
**Execution Logic**:

**Target Validation**:
- Check if target Pokemon is fainted, fail if target is fainted

**State Mutations** (if switch succeeds):
- Update `battle.players[player_index].active_pokemon_index = team_index`
- Call `battle.players[player_index].clear_active_pokemon_state()` to reset temporary modifiers

**Stack Operations**:
- Push no additional actions (switching is atomic)

**Event Emission**:
- `ActionFailed` events for invalid switches (target fainted)
- `PokemonSwitched` events for successful switches

**Return Value**: Always `GameState::Advancing`

**Note**: Trapped condition prevention is handled by the command-to-action conversion layer. SwitchPokemon commands only generate DoSwitch actions if trapped checks pass.

#### `ChooseMove { player_index, team_index, move_index }`
**Purpose**: Handle action prevention, move selection, and PP consumption
**Execution Logic**:

**Action Prevention Check**:
- Run prevention check for sleep, paralysis, freeze, flinch, confusion, etc.
- Set `battle_state.prevention_checked[player_index] = true`
- If prevented: emit prevention event and return without pushing DoMove

**PP Validation** (if not prevented):
- Check if move has PP > 0, fail if no PP available
- Deduct 1 PP from move slot

**State Mutations**:
- Set prevention check flag to true
- Decrement PP for the specified move by 1 (if not prevented)
- Update pokemon's last_move (if not prevented)

**Stack Operations**:
- If not prevented: Push `DoMove { player_index, team_index, move_data: get_move_data(move) }`
- If prevented: Push no additional actions

**Event Emission**:
- `ActionPrevented` events for status/condition prevention
- `MoveUsed { pokemon: species, move: move_used }` (if not prevented)

**Return Value**: Always `GameState::Advancing`

#### `DoMove { player_index, team_index, move_data }`
**Purpose**: Execute complete move atomically, handling all effects internally
**Execution Logic**:

**Conditional Prevention Check**:
- If `!battle_state.prevention_checked[player_index]`: run prevention check, set flag to true
- If prevented: emit prevention event and return without move execution
- If already checked this turn (from ChooseMove): skip prevention check

**Complete Move Processing** (if not prevented):
- Set `battle_state.last_moves[player_index]` for context
- Execute entire move sequence atomically within single action
- Handle accuracy checks, damage calculation, and all secondary effects
- Apply recoil, drain, status effects, stat changes, and special mechanics
- Process multi-hit moves, two-turn moves, and complex interactions

**State Mutations**:
- Prevention check flag (if not already set)
- All move-related state changes applied directly during execution
- Pokemon HP, status conditions, stat stages, team conditions
- Battle flags and volatile conditions updated as needed

**Stack Operations**:
- Only pushes `HandleKnockout` actions if Pokemon fainted during move execution
- No granular effect actions generated (all handled internally)

**Event Emission**:
- `ActionPrevented` events (if prevention check runs and prevents)
- `MoveUsed`, `MoveHit`/`MoveMissed`, `DamageTaken`, `StatusApplied` events
- All battle events emitted directly during atomic execution

**Return Value**: Always `GameState::Advancing`

#### `DoForfeit { player_index }`
**Purpose**: Handle battle forfeit (competitive contexts)
**Execution Logic**:

**State Mutations**:
- No direct state mutations

**Stack Operations**:
- Push `EndBattle { outcome: BattleResolution::Forfeit { player: player_index } }`

**Event Emission**:
- `BattleForfeited` events

**Return Value**: Always `GameState::Advancing`

#### `DoFlee { player_index }`
**Purpose**: Handle escape from wild encounters
**Execution Logic**:

**Flee Success Check**:
- Calculate flee success (Always fails if trapped, otherwise there is a chance to flee based on relative level)

**State Mutations**:
- No direct state mutations

**Stack Operations**:
- **If flee succeeds**: Push `EndBattle { outcome: BattleResolution::Draw }`
- **If flee fails**: Push no additional actions (battle continues)

**Event Emission**:
- `FleeAttempt` events with success/failure result

**Return Value**: Always `GameState::Advancing`

#### `ThrowBall { ball }`
**Purpose**: Execute Pokeball usage for catching
**Execution Logic**:

**Catch Probability Check**:
- Calculate catch probability based on ball type, target Pokemon species, HP, and status
- Use RNG to determine catch success

**State Mutations**:
- Decrement ball count from player's inventory

**Stack Operations**:
- **If catch succeeds**: Push `CatchPokemon { player_index, target_species }` action
- **If catch fails**: Push no additional actions (battle continues)

**Event Emission**:
- `BallThrown` events with ball type
- `CatchAttempt` events with success/failure result

**Return Value**: Always `GameState::Advancing`


## Action Stack Management

### Stack Execution Order
- **LIFO Execution**: Last action pushed executes first
- **Priority Handling**: Higher priority actions pushed later
- **Dynamic Injection**: Executing actions can add new actions
- **Complex Sequences**: Multi-turn moves managed through action chaining

### Action Priority Guidelines
1. **Battle Flow**: `EndBattle`, `EndTurn` (highest priority)
2. **Input Requests**: `RequestBattleCommands`, `RequestNextPokemon` 
3. **Command Execution**: `DoSwitch`, `DoMove`, `DoForfeit`
4. **Battle Management**: `HandleKnockout`

### Stack State Management
- **Empty Stack**: Indicates battle completion (should not occur)
- **Single Action**: Normal execution state
- **Multiple Actions**: Complex sequences in progress
- **Action Chains**: Related actions executed in sequence

## Testing Strategy

### Action Unit Testing
```rust
#[test]
fn test_domove_action_execution() {
    let mut battle = create_test_battle();
    let mut events = EventBus::new();
    let mut rng = MockBattleRng::new();
    
    let move_data = get_move_data(Move::Tackle).unwrap();
    let move_action = BattleAction::DoMove {
        player_index: 0,
        team_index: 0,
        move_data
    };
    
    let initial_hp = battle.players[1].team[0].current_hp;
    let result = move_action.execute(&mut battle, &mut events, &mut rng);
    
    assert_eq!(result, Ok(GameState::Advancing));
    assert!(battle.players[1].team[0].current_hp < initial_hp); // Damage was dealt
    
    // Verify move events were emitted
    assert!(events.events().iter().any(|e| matches!(e, BattleEvent::MoveUsed { .. })));
}
```

### Action Flow Testing
```rust
#[test]
fn test_knockout_handling() {
    let mut battle = create_test_battle();
    let mut events = EventBus::new();
    let mut rng = MockBattleRng::new();
    
    // Set target Pokemon to low HP
    battle.players[1].team[0].set_hp(1);
    
    let move_data = get_move_data(Move::Tackle).unwrap();
    let move_action = BattleAction::DoMove {
        player_index: 0,
        team_index: 0,
        move_data
    };
    
    // Execute move action
    battle.action_stack.push(move_action);
    battle.advance(&mut events, &mut rng);
    
    // Verify HandleKnockout was pushed to stack if Pokemon fainted
    // Check that replacement request was generated if needed
}
```

### Coarse Action Testing
```rust
#[test]
fn test_complete_move_execution() {
    let mut battle = create_test_battle();
    
    // Test that DoMove handles complete move execution internally
    let move_data = get_move_data(Move::Absorb).unwrap(); // Drain move
    let initial_attacker_hp = battle.players[0].team[0].current_hp;
    let initial_target_hp = battle.players[1].team[0].current_hp;
    
    let move_action = BattleAction::DoMove {
        player_index: 0,
        team_index: 0,
        move_data
    };
    
    move_action.execute(&mut battle, &mut events, &mut rng);
    
    // Verify complete move effects applied atomically
    assert!(battle.players[1].team[0].current_hp < initial_target_hp); // Damage dealt
    assert!(battle.players[0].team[0].current_hp > initial_attacker_hp); // Health drained
}
```

## Design Principles

### Single Responsibility
- Each action performs one atomic operation
- Complex operations composed of multiple simple actions
- Clear separation between different types of state changes

### Integrated Event Generation
- Actions emit events directly during state mutation in `execute()`
- No separate event generation step required
- State changes and events always stay in sync
- Comprehensive logging for replay and debugging

### Deterministic Execution
- Given same action and state, execution is predictable
- RNG used only where explicitly required
- State mutations follow consistent patterns

### Composable Operations
- Actions can generate additional actions during execution
- Complex move effects built from simple action primitives
- Flexible system for implementing new mechanics

This action system provides a robust foundation for all battle mechanics while maintaining clear separation of concerns and enabling comprehensive testing of battle behavior.