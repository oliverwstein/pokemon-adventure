# Dual Battle Mode System Design

## Overview

This document outlines the design and implementation of two distinct battle modes for the Pokemon Adventure Battle System:

1. **Competitive Mode** - Fast, deterministic battles with no progression (current system)
2. **Story Mode** - RPG-style battles with real-time experience gain, leveling, move learning, and evolution

## Core Architecture Principles

### Event-Driven Progression
- EXP is awarded **immediately** when enemy Pokemon faint (authentic Pokemon mechanics)
- Level-ups, move learning, and evolution happen **during** battle, not after
- Uses pub/sub pattern with existing `BattleEvent` system
- Battle can pause for user progression decisions and resume cleanly

### Battle Engine Purity Preserved
- Core battle mechanics remain completely unchanged
- All existing tests continue to pass
- Performance characteristics maintained
- Battle calculations stay pure and deterministic

### Configuration-Driven Modes
- Battle behavior controlled by `BattleModeConfig`, not hardcoded logic
- Easy to add new modes (Tournament, Training, etc.) in the future
- Clean separation between battle logic and progression logic

## System Components

### 1. Battle Mode Configuration

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BattleMode {
    Competitive,  // Current fast mode - no progression
    Story,        // New RPG mode - with EXP, leveling, evolution
}

pub struct BattleModeConfig {
    pub mode: BattleMode,
    pub award_experience: bool,
    pub enable_level_ups: bool,
    pub enable_move_learning: bool,
    pub enable_evolution: bool,
}
```

### 2. Event Subscription System

```rust
pub trait BattleEventSubscriber {
    fn on_battle_event(
        &mut self, 
        event: &BattleEvent, 
        battle_state: &mut BattleState
    ) -> ProgressionInterrupt;
}

#[derive(Debug)]
pub enum ProgressionInterrupt {
    Continue,                    // No interruption needed
    PauseForLevelUp { /* ... */ },
    PauseForMoveLearn { /* ... */ },
    PauseForEvolution { /* ... */ },
}
```

### 3. Experience & Progression Systems

```rust
pub struct ExperienceCalculator;
impl ExperienceCalculator {
    pub fn calculate_faint_exp(&self, fainted_pokemon: &PokemonInst) -> u32;
    // Authentic Pokemon EXP formula: (base_exp * level) / 7
}

pub struct ProgressionSystem;
impl ProgressionSystem {
    pub fn apply_level_up(&self, pokemon: &mut PokemonInst, new_level: u8) -> LevelUpResult;
    pub fn check_move_learning(&self, species: Species, new_level: u8) -> Option<Move>;
    pub fn check_evolution(&self, pokemon: &PokemonInst) -> Option<Species>;
}
```

### 4. Story Mode Event Subscriber

The core implementation that makes story mode work:

```rust
pub struct StoryModeProgressionSubscriber {
    experience_calculator: ExperienceCalculator,
    progression_system: ProgressionSystem,
}

impl BattleEventSubscriber for StoryModeProgressionSubscriber {
    fn on_battle_event(&mut self, event: &BattleEvent, battle_state: &mut BattleState) -> ProgressionInterrupt {
        match event {
            BattleEvent::PokemonFainted { player_index, pokemon_index, .. } => {
                // 1. Award EXP to opponent's active Pokemon
                // 2. Check for level-up → update stats immediately
                // 3. Check for move learning → return interrupt for user decision
                // 4. Check for evolution → return interrupt for user decision
                self.handle_pokemon_fainted(*player_index, battle_state)
            }
            _ => ProgressionInterrupt::Continue
        }
    }
}
```

### 5. Enhanced BattleState

Minimal changes to support event subscription:

```rust
pub struct BattleState {
    // ... existing fields unchanged ...
    pub battle_mode: BattleMode,
    pub event_subscribers: Vec<Box<dyn BattleEventSubscriber>>,
    pub pending_progression: Option<ProgressionInterrupt>,
}
```

## Implementation Flow

### Real-Time Progression Example

```
1. Player's Pikachu uses Tackle
2. Opponent's Caterpie takes damage and faints
3. BattleEvent::PokemonFainted emitted
4. StoryModeProgressionSubscriber receives event
5. Awards EXP: 53 * 7 / 7 = 53 EXP to Pikachu
6. Pikachu levels up: 12 → 13!
7. Stats recalculated immediately (HP, Attack, etc.)
8. Check: Does Pikachu learn Thunder Wave at level 13? Yes!
9. Return ProgressionInterrupt::PauseForMoveLearn
10. Battle pauses, UI asks: "Replace which move?"
11. Player chooses to replace Growl
12. Thunder Wave learned, battle resumes
13. Next turn starts with newly leveled Pikachu
```

### Battle Engine Integration

```rust
// In battle/engine.rs - minimal change to notify subscribers
pub fn resolve_turn(battle_state: &mut BattleState, rng: TurnRng) -> EventBus {
    let event_bus = internal_resolve_turn(battle_state, rng); // Existing logic unchanged
    
    // NEW: Notify subscribers of events (only in story mode)
    if battle_state.battle_mode == BattleMode::Story {
        for event in event_bus.events() {
            battle_state.notify_subscribers(event);
            if battle_state.pending_progression.is_some() {
                break; // Pause on first progression interrupt
            }
        }
    }
    
    event_bus
}
```

## Key Benefits

### ✅ Authentic Pokemon Experience
- EXP awarded exactly when Pokemon faint (not at battle end)
- Mid-battle level-ups affect remaining battle turns
- Move learning happens at level-up moment
- Evolution can change Pokemon species during battle

### ✅ Zero Battle Engine Impact
- All existing competitive battles work unchanged
- Pure battle calculations preserved
- All 211+ tests continue passing
- Performance characteristics maintained

### ✅ Clean Architecture
- Story progression completely separate from battle logic
- Easy to extend with new progression features
- Configuration-driven behavior
- Event-driven design with loose coupling

### ✅ User Experience
- Battles can pause naturally for progression decisions
- Clear interrupts for move learning and evolution choices
- Battle state remains consistent throughout
- Smooth integration with MCP interface

## Implementation Phases

1. **Foundation**: Battle mode config, event subscription architecture
2. **Core Systems**: Experience calculation, level-up mechanics  
3. **Progression**: Move learning, evolution systems
4. **Integration**: Battle flow handling, user decision processing
5. **Polish**: Story mode teams, MCP interface, comprehensive testing

## File Organization

```
src/
├── battle/
│   ├── progression/           # NEW - story mode systems
│   │   ├── mod.rs
│   │   ├── experience.rs      # ExperienceCalculator
│   │   ├── progression.rs     # ProgressionSystem
│   │   ├── subscriber.rs      # StoryModeProgressionSubscriber
│   │   └── evolution.rs       # EvolutionSystem
│   ├── state.rs              # Enhanced with event subscription
│   └── engine.rs             # Minimal change for event notification
└── mcp_interface.rs          # Story mode battle functions
```

## Testing Strategy

- **Existing Tests**: All current tests must pass (competitive mode unchanged)
- **Story Mode Tests**: New test suite for progression mechanics
- **Integration Tests**: End-to-end story battles with real progression
- **Event Tests**: Event subscription and interrupt handling
- **Edge Cases**: Level-up during multi-hit moves, evolution cancellation, etc.

---

This design maintains the elegance and performance of the current system while adding rich RPG progression mechanics. The clear separation ensures competitive battles remain fast and deterministic, while story battles provide the authentic Pokemon experience players expect.