# AI Design for Pokemon Battle System

## Overview

The Battle crate includes AI modules for NPC decision-making while maintaining separation from the Battle FSM. AI systems utilize shared battle calculation functions and the existing scoring-based decision logic.

## AI Architecture

### Separation from Battle FSM
- **Battle Struct**: Pure FSM that treats all players identically, no AI knowledge
- **AI Modules**: Part of Battle crate but external to Battle struct
- **External Integration**: BattleRunner calls AI modules to generate commands for NPCs
- **Shared Calculations**: AI and FSM use same battle calculation functions for consistency

### Current AI Implementation

Based on the existing `ScoringAI` system in `src/battle/ai.rs`:

#### Trainer AI (ScoringAI)
**Current Behavior**: Sophisticated move and switch scoring system
- **Move Scoring**: Damage calculation + utility scoring + accuracy weighting
  - Damage based on base power × type effectiveness × STAB × normalized attack power
  - Utility scoring for stat changes, status effects, flinching
  - Accuracy penalty for unreliable moves
  - Random factor (±5%) to break ties and prevent loops
- **Switch Scoring**: Basic positive score with random tiebreaker
- **Strategic Logic**: Compares best move vs best switch, chooses higher score

#### Wild Pokemon AI 
**Current Behavior**: Random move selection
- Selects randomly from available moves
- No strategic considerations
- No switching (wild Pokemon typically single-Pokemon encounters)

#### Safari Pokemon AI
**Current Behavior**: Simple binary choice
- Choose between doing nothing and running away
- No move usage in Safari encounters
- Probability-based decision making

### AI Interface

```rust
pub trait BattleAI {
    /// Generate a command for the specified player given current battle state
    fn decide_command(
        &self, 
        player_index: usize, 
        battle: &Battle, 
        rng: &mut dyn BattleRng
    ) -> BattleCommand;
}

/// Current sophisticated AI for trainer battles
pub struct ScoringAI;

/// Simple random AI for wild Pokemon
pub struct RandomAI;

/// Binary choice AI for Safari Pokemon (do nothing vs run)
pub struct SafariAI;
```

## Integration with Battle System

### Command Generation Flow
1. **BattleRunner** detects NPC needs command via `InputRequest`
2. **BattleRunner** selects appropriate AI:
   - `ScoringAI` for trainer battles
   - `RandomAI` for wild encounters  
   - `SafariAI` for Safari Zone encounters
3. **AI** receives current Battle state and RNG instance
4. **AI** returns optimal `BattleCommand` using existing decision logic
5. **BattleRunner** submits command to Battle FSM via `submit_commands()`

### Shared Battle Calculations
AI modules utilize the same calculation functions as the Battle FSM:
- Type effectiveness calculations
- Damage formulas and stat modifications
- Move data access and validation
- Legal action determination

## Battle Type Variations

### Tournament Battles
- Use `ScoringAI` for optimal competitive play
- Both players treated as strategic opponents

### Trainer Battles  
- Use `ScoringAI` for realistic trainer behavior
- Existing sophisticated scoring system provides good trainer-like decisions

### Wild Encounters
- Use `RandomAI` for unpredictable wild Pokemon behavior
- Simple random move selection from available moves

### Safari Zone Battles
- Use `SafariAI` for Safari Pokemon behavior
- Simple choice between doing nothing and running away
- No move usage or strategic considerations

## Testing Strategy

### Deterministic AI Testing
```rust
#[test]
fn test_scoring_ai_decisions() {
    let ai = ScoringAI::new();
    let mut rng = MockBattleRng::new().set_fixed(RngCategory::Percentage, 50);
    
    let battle = create_test_battle_scenario();
    let command = ai.decide_command(0, &battle, &mut rng);
    
    // Test that AI makes reasonable decisions given battle state
    assert!(matches!(command, BattleCommand::UseMove { .. } | BattleCommand::SwitchPokemon { .. }));
}
```

### Current AI Validation
- Preserve existing AI behavior and test coverage
- Ensure scoring calculations remain consistent
- Validate that move/switch decisions follow existing logic

## Future Extensions (Possible Improvements)

### AI Strategy Variations
- **Difficulty Levels**: Modify scoring weights or add suboptimal decision probability
- **Personality Types**: Aggressive (prefer high-damage moves), Defensive (prefer status/switches), Balanced
- **Type Specialists**: Gym leader AI that prefers moves of their specialty type

### Enhanced Wild AI
- **Species-Based Behavior**: Aggressive Pokemon favor attacking, timid Pokemon favor status moves
- **Environmental AI**: Different behavior in different locations (cave Pokemon, water Pokemon)

### Advanced Trainer AI
- **Team Composition Awareness**: Consider remaining team when making decisions
- **Battle Phase Adaptation**: Early game setup vs late game preservation
- **Opponent Modeling**: Learn opponent patterns over multiple battles

### Enhanced Safari AI
- **Species-Specific Flee Rates**: Different Pokemon species have different run probabilities
- **Turn-Based Probability**: Flee chance increases with turn count
- **Bait/Rock Response**: Enhanced behavior when bait/rock mechanics are used

## Performance Considerations

### Calculation Efficiency
- Reuse existing battle calculation functions to avoid duplication
- Cache frequently accessed data (type charts, move data) during AI decisions
- Minimize memory allocations during decision-making

### Decision Time
- Current `ScoringAI` is fast enough for real-time play
- Random AI and Safari AI have minimal computational overhead
- Future complex AIs should include time budgets if needed

## Implementation Notes

- **Preserve Current Logic**: Existing `ScoringAI` provides good trainer behavior
- **Simple Extensions**: New AI types can be added by implementing `BattleAI` trait  
- **Battle Type Mapping**: BattleRunner selects AI based on `BattleType` enum
- **Consistent Interface**: All AIs use same command generation pattern

This design maintains the current working AI system while providing a clean interface for the FSM architecture and allowing future AI enhancements without affecting battle logic.