# Battle Actions Design

## Overview

BattleActions represent atomic operations that mutate battle state during FSM execution. They are the only mechanism through which battle state changes occur, providing deterministic execution and comprehensive event generation. All battle logic flows through action execution on the action stack.

## Action Categories

BattleActions fall into several categories based on their role in battle flow:

### Input Request Actions
Actions that can trigger `AwaitingInput` state when input is needed:
- `RequestBattleCommands`
- `RequestNextPokemon { p1: bool, p2: bool }`  
- `OfferMove { player_index: usize, team_index: usize, new_move: Move }`
- `OfferEvolution { player_index: usize, team_index: usize, species: Species }`
- `EndBattle { outcome: BattleResolution }`

### Command Execution Actions  
Actions generated from BattleCommands:
- `DoSwitch { player_index: usize, team_index: usize }`
- `DoMove { player_index: usize, team_index: usize, move_index: usize }`
- `DoForfeit { player_index: usize }`
- `DoFlee { player_index: usize }`
- `ThrowBall { ball: PokeballType }`

### Battle Flow Actions
Actions that manage turn progression:
- `EndTurn`

### Direct Effect Actions
Actions that apply immediate state changes:
- `Damage { player_index: usize, team_index: usize, amount: u16 }`
- `Heal { player_index: usize, team_index: usize, amount: u16 }`
- `Knockout { player_index: usize, team_index: usize }`
- `ModifyStatStage { player_index: usize, target_team_index: usize, stat: Stat, delta: i8 }`
- `ResetStatChanges { player_index: usize, target_team_index: usize }`
- `ApplyStatus { player_index: usize, target_team_index: usize, status: StatusCondition }`
- `RemoveStatus { player_index: usize, target_team_index: usize }`
- `ApplyCondition { player_index: usize, target_team_index: usize, condition: PokemonCondition }`
- `RemoveCondition { player_index: usize, target_team_index: usize, condition: PokemonCondition }`
- `ApplyTeamCondition { player_index: usize, condition: TeamCondition }`

### Move Effect Actions
Actions for specific move mechanics:
- `StrikeAction { player_index: usize, team_index: usize, target_team_index: usize, move_used: Move }`
- `PassiveAction { player_index: usize, team_index: usize, move_used: Move }`
- `Miss { player_index: usize, team_index: usize, move_used: Move }`

### Action Prevention Actions
Actions that handle conditions preventing Pokemon from acting:
- `StatusPreventedAction { player_index: usize, team_index: usize, status: PokemonStatus }`
- `ConfusionSelfDamage { player_index: usize, team_index: usize }`
- `VolatilePreventedAction { player_index: usize, team_index: usize, condition: PokemonCondition }`

## BattleAction Enum Definition

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum BattleAction {
    // Actions that can trigger Awaiting Input
    RequestBattleCommands,
    RequestNextPokemon { p1: bool, p2: bool },
    OfferMove { player_index: usize, team_index: usize, new_move: Move },
    OfferEvolution { player_index: usize, team_index: usize, species: Species },

    // Actions generated from BattleCommands
    DoSwitch { player_index: usize, team_index: usize },
    DoMove { player_index: usize, team_index: usize, move_index: usize },
    DoForfeit { player_index: usize },
    DoFlee { player_index: usize },
    ThrowBall { ball: PokeballType },

    // Battle flow control
    EndTurn,

    // Direct state modifications
    Damage { player_index: usize, team_index: usize, amount: u16 },
    Heal { player_index: usize, team_index: usize, amount: u16 },
    Knockout { player_index: usize, team_index: usize },
    
    ModifyStatStage { 
        player_index: usize, 
        target_team_index: usize, 
        stat: Stat, 
        delta: i8 
    },
    ResetStatChanges { player_index: usize, target_team_index: usize },

    ApplyStatus { 
        player_index: usize, 
        target_team_index: usize, 
        status: PokemonStatus 
    },
    RemoveStatus { 
        player_index: usize, 
        target_team_index: usize, 
        status_type: PokemonStatus 
    },

    ApplyCondition { 
        player_index: usize, 
        target_team_index: usize, 
        condition: PokemonCondition 
    },
    RemoveCondition { 
        player_index: usize, 
        target_team_index: usize, 
        condition_key: ConditionKey 
    },
    RemoveAllConditions { player_index: usize, target_team_index: usize },
    
    ApplyTeamCondition { player_index: usize, condition: TeamCondition },

    // Move execution actions
    StrikeAction { 
        player_index: usize, 
        team_index: usize, 
        target_team_index: usize, 
        move_used: Move 
    },
    PassiveAction { 
        player_index: usize, 
        team_index: usize, 
        move_used: Move 
    },
    Miss { 
        player_index: usize, 
        team_index: usize, 
        move_used: Move 
    },

    // Action prevention actions
    StatusPreventedAction { 
        player_index: usize, 
        team_index: usize, 
        status: PokemonStatus 
    },
    ConfusionSelfDamage { 
        player_index: usize, 
        team_index: usize 
    },
    VolatilePreventedAction { 
        player_index: usize, 
        team_index: usize, 
        condition: PokemonCondition 
    },

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
- Convert commands to execution actions: `DoSwitch`, `DoMove`, `DoForfeit`, `DoFlee`, `ThrowBall`
- Push `EndTurn` action first (executes last due to LIFO)
- Push actions in REVERSE priority order (switches first, then by move priority/speed)
- Clear `battle_commands` array

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

**Switch Prevention Logic** (internal calculations based on legacy engine.rs):
- **Trapped Condition**: If active Pokemon has `Trapped` condition and is not fainted, switch fails
- **Target Fainted**: If target Pokemon is fainted, switch fails

**State Mutations** (if switch succeeds):
- Update `battle.players[player_index].active_pokemon_index = team_index`
- Call `battle.players[player_index].clear_active_pokemon_state()` to reset temporary modifiers

**Stack Operations**:
- Presently, none.

**Event Emission**:
- `ActionFailed` events for prevented switches (trapped, target fainted)
- `PokemonSwitched` events for successful switches

**Return Value**: Always `GameState::Advancing`

#### `DoMove { player_index, team_index, move_index }`
**Purpose**: Execute move usage and handle action prevention
**Execution Logic**:

**State Mutations**:
- Decrement PP for the specified move by 1
- Update pokemon's last_move.

**Action Prevention Logic** (internal calculations using helper functions based on `calculate_action_prevention`):
- **Sleep**: If turns_remaining > 0, Pokemon fails to act and sleep counter decrements
- **Freeze**: 25% chance to thaw out, otherwise fails to act  
- **Flinch**: Check for flinched condition, prevents action
- **Exhaustion**: Check for exhausted condition, prevents action
- **Paralysis**: 25% chance to be fully paralyzed, prevents action
- **Confusion**: 
  - If turns_remaining == 0, confusion expires and action proceeds
  - If turns_remaining > 0, 50% chance to hit self instead of using move
- **Move Disable**: Check if specific move is disabled
- **Status Counter Updates**: Automatically update status/condition turn counters

**Stack Operations** (push EITHER normal actions OR failure actions):
- **If action prevented**: Push appropriate failure action instead:
  - Sleep: Push sleep failure action (Pokemon remains asleep)
  - Paralysis: Push paralysis failure action  
  - Confusion + self-hit: Push confusion self-damage action
  - Frozen: Push frozen failure action
  - etc

- **If action succeeds**: Push normal move execution actions:
  - Move's data determines which actions to push based on effects:
    - Damaging effects: Push `StrikeAction { player_index, team_index, target_team_index, move_used }`
    - Multi-hit moves: Push multiple `StrikeAction`s 
    - Passive effects: Push `PassiveAction { player_index, team_index, move_used }`
    - Complex moves (e.g., Explosion): Push both `StrikeAction` (damage) and `PassiveAction` (self-destruction)
  - Exact action generation logic will be determined during implementation based on move effect analysis

**Event Emission**:
- `MoveUsed { pokemon: species, move: move_used }`

**Return Value**: Always `GameState::Advancing` (actual move effects handled by StrikeAction/PassiveAction)

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

#### `StrikeAction { player_index, team_index, target_team_index, move_used }`
**Purpose**: Execute offensive move against target
**Execution Logic**:
1. Calculate damage using battle damage formula
2. Apply type effectiveness, STAB, critical hits
3. Apply secondary effects (status, stat changes)
4. Push actions for (recoil, drain, etc.)
5. Push `Damage` action for calculated amount
Note: Must decide whether to check for hit/miss here or in DoMove. 
PassiveActions can't miss, whereas StrikeActions can, so it seems reasonable to check here.

#### `PassiveAction { player_index, team_index, move_used }`
**Purpose**: Execute non-damaging move effects
**Execution Logic**:
1. Apply move effects to user or field
2. Handle stat modifications, status healing, field effects
3. Process move-specific passive effects
4. Generate appropriate effect actions
Note: PassiveAction is move-specific, not effect-specific, and most effects are going to be actions, so this should perhaps be a matter of calling the appropriate actions?

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
**Execution Logic**:
1. **Sleep**: Pokemon fails to act, decrement turns_remaining, emit sleep prevention event
2. **Freeze**: 25% chance to thaw (remove status), otherwise prevent action and emit freeze event
3. **Paralysis**: 25% chance to be fully paralyzed, prevent action and emit paralysis event
4. **Other Status**: Handle status-specific prevention logic

#### `ConfusionSelfDamage { player_index, team_index }`
**Purpose**: Handle confusion causing Pokemon to damage itself
**Execution Logic**:
1. Calculate confusion self-damage (typically 40 base power typeless move)
2. Generate `Damage` action for calculated self-damage amount
3. Emit confusion self-hit event


#### `VolatilePreventedAction { player_index, team_index, condition: PokemonCondition }`
**Purpose**: Handle action prevention due to volatile conditions
**Execution Logic**:
1. **Flinched**: Pokemon cannot act this turn, emit flinch prevention event
2. **Exhaustion**: Pokemon must recharge after Hyper Beam, emit exhaustion event
3. **Other Volatile**: Handle condition-specific prevention logic
4. Remove condition (exhaustion and flinch are both single-turn conditions)

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