# Special Move Cases and PP/Status Handling

## Overview

This document catalogues all special cases for move execution that deviate from normal PP usage, status checking, or player choice patterns. These cases inform the design of the DoMove action and its parameters.

## Move Cases Summary Table

| Move Type | Turn | PP Usage | Status Check | Player Choice | Prevention Effects | Implementation |
|-----------|------|----------|--------------|---------------|-------------------|----------------|
| **Normal Move** | Single | Deduct from slot | Yes | Free choice | Standard prevention | `DoMove{pp_source: Some(idx), status_check: true}` |
| **Struggle** | Single | None | Yes | Choice vs switch | Standard prevention | `DoMove{pp_source: None, status_check: true, move_data: struggle}` |
| **Two-Turn (Charge)** | 1st | None | Yes | Free choice | Standard prevention | `DoMove{pp_source: None, status_check: true, move_data: charge}` |
| **Two-Turn (Release)** | 2nd | Deduct from slot | Yes | Free choice | Removes charge condition if prevented | `DoMove{pp_source: Some(idx), status_check: true, move_data: attack}` |
| **Lock-in (Initial)** | 1st | Deduct from slot | Yes | Free choice | Standard prevention | `DoMove{pp_source: Some(idx), status_check: true}` |
| **Lock-in (Continue)** | 2-N | None | Yes | Forced action | Removes lock-in if prevented | `DoMove{pp_source: None, status_check: true, move_data: same}` |
| **Lock-in (Final)** | Last | None | Yes | Forced action | Removes lock-in if prevented | `DoMove{pp_source: None, status_check: true, move_data: plus_confusion}` |
| **Bide (Initial)** | 1st | Deduct from slot | Yes | Free choice | Standard prevention | `DoMove{pp_source: Some(idx), status_check: true}` |
| **Bide (Accumulate)** | 2-N | None | Yes | Forced nothing | Continues but no damage accumulated | `DoNothing` with accumulation |
| **Bide (Release)** | Auto | None | Yes | Forced action | Loses damage + condition if prevented | `DoMove{pp_source: None, status_check: true, move_data: release}` |
| **Hyper Beam** | 1st | Deduct from slot | Yes | Free choice | Standard prevention | `DoMove{pp_source: Some(idx), status_check: true}` |
| **Hyper Beam Recharge** | 2nd | None | Yes | Forced nothing | Must recharge even if prevented | `DoNothing` (forced) |
| **Copy Source (Metronome)** | 1st | Deduct from source | Yes | Free choice | Standard prevention | `DoMove{pp_source: Some(source_idx), status_check: true}` |
| **Copy Execute (Metronome)** | 2nd | None | No | Automatic | No additional prevention | `DoMove{pp_source: None, status_check: false, move_data: copied}` |
| **Mimic/Transform Initial** | 1st | Deduct from slot | Yes | Free choice | Standard prevention | `DoMove{pp_source: Some(idx), status_check: true}` |
| **Mimic/Transform Usage** | Later | Deduct from temp slot | Yes | Free choice | Standard prevention | `DoMove{pp_source: Some(temp_idx), status_check: true}` |
| **Disabled Move** | N/A | N/A | N/A | Unavailable | Blocked at input validation | Command rejection |

## Detailed Case Descriptions

### Normal Move Usage
**Examples**: Tackle, Thunderbolt, Surf
- Standard case: deduct PP, check status, allow prevention
- Most moves follow this pattern

### Struggle
**Trigger**: No moves have PP, player chooses attack over switch
- Uses built-in move data (Normal, 50 power, 25% recoil)
- No PP cost since not from moveset
- Still subject to status prevention

### Two-Turn Moves
**Examples**: Solar Beam, Sky Attack, Razor Wind

**Charge Turn**: 
- No PP used (charging phase)
- Applies charging condition
- Can be prevented normally

**Release Turn**:
- PP deducted on execution
- If prevented, charging condition is removed
- Executes the actual attack

### Lock-in Moves  
**Examples**: Thrash, Petal Dance, Outrage

**Initial Turn**:
- PP deducted once for entire sequence
- Applies lock-in condition (2-3 turns)
- Normal move execution

**Continuation Turns**:
- No PP cost
- Forced action (player cannot choose different move)
- If prevented, lock-in condition removed (no confusion penalty)

**Final Turn**:
- Natural completion applies confusion after move execution
- Still subject to status prevention

### Accumulation Moves
**Examples**: Bide

**Initial Turn**:
- PP deducted once
- Begins damage accumulation
- Applies Bide condition

**Accumulation Turns**:
- Pokemon does nothing but accumulates damage
- Status prevention stops accumulation for that turn
- Action is forced (DoNothing)

**Release Turn**:
- Automatic when accumulation period ends
- Deals 2x accumulated damage
- If prevented, all accumulated damage is lost

### Recharge Moves
**Examples**: Hyper Beam, Blast Burn

**Attack Turn**:
- Normal PP usage and status checks
- If target is KO'd, applies recharge condition

**Recharge Turn**:
- Forced inaction
- Status prevention still forces recharge (no escape)
- Player cannot choose actions

### Copy Moves
**Examples**: Metronome, Mirror Move

**Source Execution**:
- Uses PP from Metronome/Mirror Move
- Subject to normal status checks
- Determines what move to copy

**Copied Execution**:
- No PP cost (not from user's moveset)  
- No additional status checks
- Executes copied move's behavior

### Temporary Moveset
**Examples**: Mimic, Transform

**Initial Usage**:
- Uses PP from Mimic/Transform
- Copies move(s) into temporary slots with 5 PP each
- Transform copies entire moveset

**Using Copied Moves**:
- Uses PP from temporary slots
- Normal status checks apply
- Functions as regular moves

### Disabled Moves
**Examples**: Any move affected by Disable
- Move becomes completely unavailable
- Input validation should prevent selection
- No execution occurs at all

## Design Implications

### DoMove Action Parameters
```rust
DoMove {
    player_index: usize,
    team_index: usize, 
    move_data: MoveData,
    pp_source: Option<usize>,    // Which slot to deduct PP from
    status_check: bool,          // Whether to check status
}
```

### Key Design Rules
1. **PP deducted once per move sequence** (first turn only for multi-turn moves)
2. **Status always checked** except for copied move execution
3. **Prevention can interrupt sequences** and removes associated conditions  
4. **Forced actions still allow switching** (player choice at command level)
5. **Temporary movesets create new PP pools** independent of original moveset