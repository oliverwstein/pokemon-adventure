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
            BattleAction::Damage { player_index, team_index, amount } => {
                // Apply state change
                battle.players[*player_index].team[*team_index].current_hp -= *amount;
                
                // Emit event immediately
                events.emit(BattleEvent::DamageTaken {
                    pokemon: battle.players[*player_index].team[*team_index].species,
                    amount: *amount,
                });
                
                Ok(GameState::Advancing)
            }
            // ... other actions handle state + events together
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
- Convert commands to execution actions with prevention checks:
  - `UseMove` → Check action prevention → Push `ChooseMove` OR prevention action
  - `Continue` → Check action prevention → Push `DoMove` OR prevention action  
  - `SwitchPokemon` → Check trapped condition → Push `DoSwitch` OR `ConditionPreventedAction { condition: Trapped }`
  - `DoForfeit`, `DoFlee`, `ThrowBall` → Direct conversion (no prevention)
- Push `EndTurn` action first (executes last due to LIFO)
- Push actions in REVERSE priority order (switches first, then by move priority/speed)
- Clear `battle_commands` array

**Action Prevention Logic**:
- **Move Action Prevention** (applied before pushing ChooseMove/DoMove):
  - Sleep: Push `StatusPreventedAction { status: Sleep }`
  - Freeze: 25% thaw chance, otherwise push `StatusPreventedAction { status: Freeze }`
  - Paralysis: 25% prevent chance, push `StatusPreventedAction { status: Paralysis }`
  - Flinch: Push `ConditionPreventedAction { condition: Flinch }`
  - Move Disable: Push `ConditionPreventedAction { condition: Disabled }`
  - Confusion: 50% chance push `ConfusionSelfDamage`, otherwise continue normally
- **Switch Action Prevention** (applied before pushing DoSwitch):
  - Trapped: Push `ConditionPreventedAction { condition: Trapped }`
- **Special Cases**:
  - Exhaustion: Handled by `Continue { action: Recharge }` generating `DoNothing` (not prevention)

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
**Purpose**: Handle move selection and PP consumption
**Execution Logic**:

**PP Validation**:
- Check if move has PP > 0, fail if no PP available
- Deduct 1 PP from move slot

**State Mutations**:
- Decrement PP for the specified move by 1
- Update pokemon's last_move

**Stack Operations**:
- Push `DoMove { player_index, team_index, move_data: get_move_data(move) }`

**Event Emission**:
- `MoveUsed { pokemon: species, move: move_used }`

**Return Value**: Always `GameState::Advancing`

**Note**: Action prevention (sleep, paralysis, etc.) is handled by the command-to-action conversion layer. UseMove commands only generate ChooseMove actions if prevention checks pass.

#### `DoMove { player_index, team_index, move_data }`
**Purpose**: Execute move script and generate appropriate move effect actions
**Execution Logic**:

**Script Processing**:
- Process `move_data.script` instructions in sequence
- Convert each Instruction to appropriate BattleActions

**State Mutations**:
- No direct state mutations (handled by generated actions)

**Stack Operations** (process each instruction in reverse order for LIFO execution):
- **Strike Instruction**: Push `StrikeAction { player_index, team_index, target_team_index, strike_data }`
- **Passive Instruction**: Push `PassiveAction { player_index, team_index, passive_effect }`
- **MultiHit Instruction**: Generate hit count, push multiple `StrikeAction`s
- **Prepare Instruction**: Check condition state, push appropriate action based on preparation status

**Event Emission**:
- No direct events (generated actions handle their own events)

**Return Value**: Always `GameState::Advancing`

**Note**: Action prevention (sleep, paralysis, etc.) is handled by the command-to-action conversion layer. Continue commands only generate DoMove actions if prevention checks pass.

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

### Direct Effect Actions

#### `Damage { player_index, team_index, amount }`
**Purpose**: Apply damage to specific Pokemon
**Execution Logic**:

**State Mutations**:
- Apply saturating subtraction: `current_hp = current_hp.saturating_sub(amount)`
- Clamp HP to 0 minimum (no negative HP)

**Stack Operations**:
- **If HP reaches 0**: Push `Knockout { player_index, team_index }` action
- **If HP remains > 0**: Push no additional actions

**Event Emission**:
- `DamageTaken` events with actual amount dealt and remaining HP

**Return Value**: Always `GameState::Advancing`

**Note**: On-damage effects (recoil, drain) are handled by the action that generated this Damage action, not by Damage itself. 

#### `Knockout { player_index, team_index }`
**Purpose**: Handle Pokemon fainting
**Execution Logic**:

**State Mutations**:
- Set Pokemon HP to 0 and mark as fainted
- Call `battle.players[player_index].clear_active_pokemon_state()` to clear temporary conditions and stat stages

**Stack Operations**:
- Check if team has remaining conscious Pokemon:
  - **If no conscious Pokemon remaining**: Push `EndBattle { outcome: opponent_wins }`
  - **If conscious Pokemon available**: Push `RequestNextPokemon` with appropriate player flags (only the player who lost a Pokemon needs replacement)

**Event Emission**:
- `PokemonFainted` events with species and player information

**Return Value**: Always `GameState::Advancing`

#### `ModifyStatStage { player_index, target_team_index, stat, delta }`
**Purpose**: Modify Pokemon's battle stat stages
**Execution Logic**:

**State Mutations**:
- Apply stat stage modification within ±6 limits: `new_stage = (current_stage + delta).clamp(-6, 6)`
- Calculate actual change applied (may be capped by limits)
- Update Pokemon's stat stage tracking for specified stat

**Stack Operations**:
- Push no additional actions (stat changes are atomic)

**Event Emission**:
- `StatStageChanged` events with stat, actual delta applied, and new stage value
- `StatChangeBlocked` events if change was prevented by limits

**Return Value**: Always `GameState::Advancing`

### Move Effect Actions

#### `StrikeAction { player_index, team_index, target_team_index, strike_data }`
**Purpose**: Execute offensive strike with accuracy check and damage calculation
**Generated From**: `Instruction::Strike` in move scripts
**Execution Logic**:

**Accuracy Check**:
- Calculate accuracy based on `strike_data.accuracy` and stat modifiers
- Account for `SureHit` effect if present in strike_data.effects

**Damage Calculation** (if accuracy succeeds):
- Calculate damage using battle damage formula with `strike_data.power`
- Apply type effectiveness using `strike_data.move_type`
- Apply STAB, critical hits, and damage category modifiers
- Process `DamageCategory::Other` for custom damage effects (FixedDamage, PercentHpDamage)

**Secondary Effects Processing**:
- Process all effects in `strike_data.effects` list
- Generate appropriate Direct Effect Actions (ApplyStatus, StatChange, etc.)
- Handle special effects (Drain, Recoil, CritRatio)

**Stack Operations**:
- **On Hit**: Push `Damage` action + secondary effect actions
- **On Miss**: Push `Miss` action for miss-specific effects

**Event Emission**:
- `MoveHit` or `MoveMissed` events

**Return Value**: Always `GameState::Advancing`

#### `PassiveAction { player_index, team_index, passive_effect }`
**Purpose**: Execute guaranteed non-damaging effect
**Generated From**: `Instruction::Passive` in move scripts
**Execution Logic**:

**Effect Processing**:
- Process the single `passive_effect` specified
- No accuracy check (guaranteed to happen)

**Stack Operations** (based on effect type):
- **StatChange**: Push `ModifyStatStage` action
- **Heal**: Push `Heal` action
- **Status effects**: Push `ApplyStatus`, `RemoveStatus` actions
- **Team effects**: Push `ApplyTeamCondition` action
- **Special effects**: Handle Transform, Metronome, etc. with appropriate actions

**Event Emission**:
- Effect-specific events (handled by generated actions)

**Return Value**: Always `GameState::Advancing`

#### `Miss { player_index, team_index, move_used }`
**Purpose**: Handle move accuracy failure
**Execution Logic**:
1. Emit miss event for move
2. Apply any miss-specific effect (Just Reckless at the moment)
3. Continue battle flow
Note: As noted, I think this should be raised by `StrikeAction`, rather than `DoMove`.

### Action Prevention Actions
Actions that handle various conditions preventing Pokemon from acting:

#### `StatusPreventedAction { player_index, team_index, status: PokemonStatus }`
**Purpose**: Handle action prevention due to major status conditions
**Generated From**: Command-to-action conversion layer when major status prevents action
**Execution Logic**:
1. **Sleep**: Pokemon fails to act, decrement turns_remaining, emit sleep prevention event
2. **Freeze**: Pokemon fails to act, emit freeze prevention event (thaw chance already checked)
3. **Paralysis**: Pokemon fails to act, emit paralysis prevention event (prevent chance already checked)
4. **Other Status**: Handle status-specific prevention logic

#### `ConditionPreventedAction { player_index, team_index, condition: PokemonCondition }`
**Purpose**: Handle action prevention due to volatile conditions
**Generated From**: Command-to-action conversion layer when volatile condition prevents action
**Execution Logic**:
1. **Flinch**: Pokemon cannot act this turn, emit flinch prevention event
2. **Disabled**: Pokemon cannot use disabled move, emit disable prevention event
3. **Trapped**: Pokemon cannot switch out, emit trap prevention event
4. **Other Conditions**: Handle condition-specific prevention logic
5. Remove single-turn conditions (flinch is removed after preventing action)

#### `ConfusionSelfDamage { player_index, team_index }`
**Purpose**: Handle confusion causing Pokemon to damage itself instead of using move
**Generated From**: Command-to-action conversion layer when confusion self-hit occurs (50% chance)
**Execution Logic**:
1. Calculate confusion self-damage (typically 40 base power typeless move)
2. Generate `Damage` action for calculated self-damage amount
3. Emit confusion self-hit event
4. Decrement confusion turns_remaining

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
4. **Direct Effects**: `Damage`, `Heal`, `Knockout`
5. **Secondary Effects**: Status/condition applications

### Stack State Management
- **Empty Stack**: Indicates battle completion (should not occur)
- **Single Action**: Normal execution state
- **Multiple Actions**: Complex sequences in progress
- **Action Chains**: Related actions executed in sequence

## Testing Strategy

### Action Unit Testing
```rust
#[test]
fn test_damage_action_execution() {
    let mut battle = create_test_battle();
    let mut events = EventBus::new();
    let mut rng = MockBattleRng::new();
    
    let damage_action = BattleAction::Damage {
        player_index: 0,
        team_index: 0,
        amount: 25
    };
    
    let initial_hp = battle.players[0].team[0].current_hp;
    let result = damage_action.execute(&mut battle, &mut events, &mut rng);
    
    assert_eq!(result, Ok(GameState::Advancing));
    assert_eq!(battle.players[0].team[0].current_hp, initial_hp - 25);
    
    // Verify damage event was emitted
    assert!(events.events().iter().any(|e| matches!(e, BattleEvent::DamageTaken { amount: 25, .. })));
}
```

### Action Sequence Testing
```rust
#[test]
fn test_move_execution_sequence() {
    let mut battle = create_test_battle();
    let move_action = BattleAction::DoMove {
        player_index: 0,
        team_index: 0,
        move_index: 0 // Tackle
    };
    
    // Execute move action
    battle.action_stack.push(move_action);
    battle.advance(&mut events, &mut rng);
    
    // Verify sequence: DoMove -> StrikeAction -> Damage
    // Check that appropriate actions were generated and executed
}
```

### State Consistency Testing
```rust
#[test]
fn test_action_state_consistency() {
    let mut battle = create_test_battle();
    
    // Apply series of actions
    let actions = vec![
        BattleAction::Damage { player_index: 0, team_index: 0, amount: 50 },
        BattleAction::ModifyStatStage { player_index: 1, target_team_index: 0, stat: Stat::Attack, delta: 2 },
        BattleAction::ApplyStatus { player_index: 0, target_team_index: 0, status: StatusCondition::Burn }
    ];
    
    for action in actions {
        action.execute(&mut battle, &mut events, &mut rng);
    }
    
    // Verify all state changes were applied correctly
    // Verify events were generated for each change
    // Verify no invalid state exists
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