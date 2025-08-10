# Pokemon Adventure Battle System TODO

## High Priority - Core Battle System Features

### End-of-Turn Effects System
- [x] Implement status damage (poison, burn) during end-of-turn phase
- [x] Add status time timers (sleep has a duration set when it begins, poison worsens each turn)
- [ ] Handle trapped Pokemon effects (bind, wrap, fire spin)
    - The data structures are in place: PokemonCondition::Trapped exists in player.rs, and its timer is correctly decremented in tick_active_conditions. However, the core logic to apply damage during the end-of-turn phase is missing from execute_end_turn_phase and it does not prevent switching as it ought (TODO).
- [ ] Implement leech seed draining
    - The PokemonCondition::Seeded enum exists, but there is no logic in execute_end_turn_phase to handle the HP draining from the target and healing the user.

### Move Effects Implementation
#### (This relates to moves causing effects, not implementing the effects themselves)

#### Basic Status Effects:
- [ ] **Flinch**: Chance to prevent opponent from acting later in the turn
- [ ] **Burn**: Chance to inflict burn status (halves physical attack, deals damage)
- [ ] **Freeze**: Chance to inflict freeze status (prevents action until thaw)
- [ ] **Paralyze**: Chance to inflict paralysis (quarters speed, chance to skip turn)
- [ ] **Poison**: Chance to inflict poison status (deals damage each turn)
- [ ] **Sedate**: Chance to inflict sleep status (prevents action for set turns)
- [ ] **Confuse**: Chance to inflict confusion (may attack self instead)

#### Stat Modifications
- [ ] **StatChange**: Modify target's stat stages (attack, defense, speed, etc.)
- [ ] **RaiseAllStats**: Boost all user's stats simultaneously

#### Damage Modifiers
- [ ] **Recoil**: User takes percentage of damage dealt as recoil
- [ ] **Drain**: User heals percentage of damage dealt
- [ ] **Crit**: Increased critical hit ratio for this move
- [ ] **IgnoreDef**: Chance to bypass defender's defense stat
- [x] **SuperFang**: Chance to deal damage equal to half target's current HP (already implemented)
- [x] **SetDamage**: Deal fixed amount of damage regardless of stats (already implemented)
- [x] **LevelDamage**: Deal damage equal to user's level (already implemented)

#### Multi-Hit Mechanics
- [x] **MultiHit**: Multiple hits with continuation chance (already implemented)

#### Status and Conditions
- [ ] **Trap**: Chance to trap opponent (prevents switching, deals damage)
- [ ] **Exhaust**: Chance to force opponent to skip next turn
- [ ] **Flinch**: Prevents the opponent from making a move later in the turn
- [x] **Priority**: Modify move's priority in turn order (already implemented)
- [ ] **SureHit**: Move cannot miss regardless of accuracy/evasion
- [ ] **ChargeUp**: Move requires charging turn before execution
- [ ] **InAir**: User becomes semi-invulnerable in air (fly, bounce)
- [ ] **Underground**: User becomes semi-invulnerable underground (dig)
- [ ] **Teleport**: User can't be hit by enemy attacks later in the turn.

#### Special Mechanics
- [x] **OHKO**: One-hit KO if level difference allows (already implemented)
- [ ] **Explode**: User faints after dealing damage
- [ ] **Reckless**: Recoil damage if move misses
- [ ] **Transform**: Copy target's appearance, stats, and moveset
- [ ] **Conversion**: Change user's type to match last move used
- [ ] **Disable**: Disable target's last used move for several turns
- [ ] **Counter**: Return double the physical damage received this turn
- [ ] **MirrorMove**: Use the same move the opponent just used
- [ ] **Metronome**: Randomly select and execute another move
- [ ] **Substitute**: Create substitute with 25% of user's HP
- [ ] **Rest**: User sleeps for set turns but fully heals HP and status
- [ ] **Bide**: Store damage for set turns, then release double
- [ ] **Rage**: Enter rage mode with attack boosts when hit
- [ ] **Rampage**: Multi-turn uncontrollable attack with confusion/exhaustion
- [ ] **Seed**: Leech seed effect - drain HP each turn to user
- [ ] **Nightmare**: A move with this effect can only affect sleeping pokemon (see: Dream Eater)

#### Field Effects
- [ ] **Haze**: Remove all stat stage changes from all Pokemon
- [ ] **Reflect**: Reduce physical or special damage for team
- [ ] **Mist**: Prevent stat reductions for team

#### Utility Effects
- [ ] **Heal**: Restore percentage of user's max HP
- [ ] **CureStatus**: Remove specific status condition from target
- [ ] **Ante**: Gain money after battle (Pay Day effect)

### Active Pokemon Conditions System
- [x] **Flinched**: Prevents Pokemon from taking action for one turn
- [x] **Confused**: Chance to hurt self instead of using move, with turn counter
- [ ] **Seeded**: Takes damage each turn that heals the opponent (leech seed)
- [ ] **Underground**: Invulnerable to most moves while underground (dig mechanics)
- [ ] **InAir**: Invulnerable to most moves while in air (fly mechanics)
- [ ] **Teleported**: Prevents user from being hit, but does not prevent status moves (custom feature)
- [ ] **Enraged**: Attack boost accumulation for rage mechanics
- [x] **Exhausted**: Skip next turn (after hyper beam, etc.)
- [ ] **Trapped**: Binding moves prevent switching and deal damage each turn
- [ ] **Charging**: Two-turn moves (solar beam, skull bash) preparation phase
- [ ] **Rampaging**: Multi-turn uncontrollable attack (thrash, petal dance)
- [ ] **Transformed**: Copy target's stats, appearance, and moveset
- [ ] **Converted**: Change user's type to match last move used
- [ ] **Disabled**: Prevent use of specific move for several turns
- [ ] **Substitute**: Create decoy that absorbs damage until destroyed
- [ ] **Biding**: Store damage taken for 2-3 turns, then release double damage
- [ ] **Countering**: Return double the physical damage received this turn

### Status Move Categories
- [ ] Implement pure status moves that don't deal damage
- [ ] Add field effect moves (reflect, light screen, mist, haze)
- [ ] Implement utility moves (roar, whirlwind for forced switching)
- [ ] Add accuracy/evasion modifying moves

## Medium Priority - Advanced Battle Features

### Special Move Mechanics
- [x] OHKO moves (guillotine, horn drill, fissure)
- [ ] Counter move (return double physical damage)
- [ ] Transform move (copy target's stats and moveset)
- [ ] Metronome (random move selection)
- [ ] Mirror move (copy opponent's last move)
- [ ] Disable (prevent use of last move used)

### Field Conditions
- [ ] Team-wide effects (reflect, light screen, mist)

### Enhanced Status System
- [x] Freeze with thaw chances
- [x] Poison damage scaling over time

## Low Priority - Polish and Completeness

### Move Data Expansion
- [ ] Implement all MoveEffect variants from move_data.rs
- [ ] Add proper move descriptions and flavor text

### AI Improvements
- [ ] Smarter move selection beyond "first available"
- [ ] Type effectiveness consideration in AI
- [ ] Switch decision logic for AI

### Testing and Validation
- [ ] Comprehensive test suite for all move effects
- [ ] Integration tests for complex battle scenarios  
- [ ] Performance testing for large battle sequences

### Code Quality
- [ ] Refactor large functions into smaller components
- [ ] Add comprehensive documentation
- [ ] Implement proper error handling for edge cases

## Implementation Strategy

1. **Start with end-of-turn effects** - This foundational system affects multiple existing features
2. **Implement basic move effects** - Focus on the most common effects first (status infliction, stat changes)
3. **Add active conditions system** - This enables more complex move interactions
4. **Expand move data** - Create RON files and implement remaining move effects
5. **Polish and test** - Add comprehensive testing and improve code quality

The TODO list prioritizes core battle functionality that affects multiple systems, then moves to specific move implementations, and finally addresses polish and completeness features.

## Current State Summary

### ✅ Implemented Features
- Basic Pokemon stats and type system
- Critical hit calculation with Focus Energy support
- Move accuracy and evasion mechanics
- Status effects: Paralysis (speed reduction), Burn (attack reduction), Fainting
- Multi-hit attack mechanics with probabilistic continuation
- Type effectiveness calculations with immunity checks
- PP usage and validation system
- Forced Pokemon switching after fainting (end-of-turn)
- Action priority system (switch > moves by priority > moves by speed)
- Basic damage calculation with STAB, critical hits, and random variance

### 🚧 Partial Implementation
- End-of-turn phase exists but only handles some elements
- Move effects system exists but only implements MultiHit
- Status system has basic conditions but not all
- Active pokemon condition processing

### ❌ Missing Core Features
- Most move effects (99% of MoveEffect variants unimplemented)
- Status move category handling
- Advanced battle mechanics (transform, counter, etc.)