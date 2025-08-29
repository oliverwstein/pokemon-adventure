# Progression System Design Specification

## Overview
This document specifies the complete interface and implementation design for integrating Pokemon progression (experience, leveling, evolution, move learning) into the battle system.

## System Dependencies

### Existing Data Structures

#### PokemonInst (src/pokemon.rs)
```rust
pub struct PokemonInst {
    pub name: String,
    pub species: Species,
    pub level: u8,
    pub curr_exp: u32,           // ← Use this field (not "experience")
    curr_hp: u16,                // Private field
    pub ivs: [u8; 6],            // HP, ATK, DEF, SP.ATK, SP.DEF, SPD
    pub evs: [u8; 6],            // ← Use this field (not "effort_values") 
    pub stats: CurrentStats,
    pub moves: [Option<MoveInstance>; 4],  // ← Fixed array, not Vec<Move>
    pub status: Option<StatusCondition>,
}

// Required methods that must exist or be added:
impl PokemonInst {
    // Existing method for stat calculation
    fn calculate_stats(base_stats: &BaseStats, level: u8, ivs: &[u8; 6], evs: &[u8; 6]) -> CurrentStats;
    
    // Methods that need to be added:
    pub fn can_level_up_with_exp(&self, species_data: &PokemonSpecies, new_exp: u32) -> Option<u8>;
    pub fn apply_level_up(&mut self, species_data: &PokemonSpecies);
    pub fn recalculate_stats_from_species(&mut self, species_data: &PokemonSpecies);
}
```

#### BattleCommand Enum (src/battle/commands.rs)
```rust
pub enum BattleCommand {
    // Existing progression commands - use these exact field names:
    AwardExperience {
        recipients: Vec<(PlayerTarget, usize, u32)>, // (player, pokemon_index, exp_amount)
    },
    LevelUpPokemon {
        target: PlayerTarget,
        pokemon_index: usize,
        // Note: No new_level field - level is determined by experience
    },
    LearnMove {
        target: PlayerTarget,
        pokemon_index: usize,
        move_: Move,                    // ← Use "move_" not "new_move"
        replace_index: Option<usize>,
    },
    EvolvePokemon {
        target: PlayerTarget,
        pokemon_index: usize,
        new_species: Species,
    },
    DistributeEffortValues {
        target: PlayerTarget,
        pokemon_index: usize,
        stats: [u8; 6],              // HP, Atk, Def, SpA, SpD, Spe
    },
    UpdateBattleParticipation {
        active_p0: usize,
        active_p1: usize,
    },
}
```

#### BattleState Modification (src/battle/state.rs)
```rust
pub struct BattleState {
    // ... existing fields
    
    // Add this field for progression tracking:
    pub participation_tracker: BattleParticipationTracker,
}

// Constructor update needed:
impl BattleState {
    pub fn new(battle_id: String, player1: BattlePlayer, player2: BattlePlayer) -> Self {
        BattleState {
            // ... existing field initialization
            participation_tracker: BattleParticipationTracker::new(),
        }
    }
}
```

## Function Specifications

### Core Progression Calculation
```rust
// src/battle/progression/calculation.rs
pub fn calculate_progression_commands(
    fainted_target: PlayerTarget,      // Which player's Pokemon fainted
    fainted_species: Species,          // Species of fainted Pokemon
    battle_state: &BattleState,        // Complete battle state for participant lookup
    participation_tracker: &BattleParticipationTracker, // Who participated against this Pokemon
) -> Vec<BattleCommand> {
    // Implementation logic:
    // 1. Use RewardCalculator::calculate_base_exp(fainted_species) -> Result<u32, _>
    // 2. Use RewardCalculator::calculate_ev_yield(fainted_species) -> Result<EvYield, _>
    // 3. Use participation_tracker.get_participants_against(player_index, pokemon_index) -> Vec<usize>
    // 4. For each participant:
    //    a. Generate AwardExperience command
    //    b. Generate DistributeEffortValues command  
    //    c. Check if new experience triggers level up using species_data.experience_group.can_level_up()
    //    d. If level up: generate LevelUpPokemon + check moves/evolution at new level
    // 5. Return Vec<BattleCommand>
}
```

### Command Execution Functions
```rust
// src/battle/progression/commands.rs

pub fn execute_award_experience(
    recipients: &[(PlayerTarget, usize, u32)],
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    // Implementation: pokemon.curr_exp += exp_amount for each recipient
}

pub fn execute_level_up_pokemon(
    target: PlayerTarget,
    pokemon_index: usize,
    _unused_level: u8,              // Parameter exists but level calculated from experience
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    // Implementation:
    // 1. Get species data: crate::get_species_data(pokemon.species)?
    // 2. Calculate new level: species_data.experience_group.calculate_level_from_exp(pokemon.curr_exp)
    // 3. Set pokemon.level = new_level
    // 4. Call pokemon.recalculate_stats_from_species(&species_data)
}

pub fn execute_learn_move(
    target: PlayerTarget,
    pokemon_index: usize,
    move_: Move,                    // Note: parameter name is "move_"
    replace_index: Option<usize>,
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    // Implementation:
    // 1. Create MoveInstance::new(move_)
    // 2. If replace_index.is_some(): pokemon.moves[index] = Some(move_instance)
    // 3. Else: find first None slot in pokemon.moves[0..4], or replace pokemon.moves[3]
}

pub fn execute_evolve_pokemon(
    target: PlayerTarget,
    pokemon_index: usize,
    new_species: Species,
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    // Implementation:
    // 1. pokemon.species = new_species
    // 2. Get new species data: crate::get_species_data(new_species)?
    // 3. Call pokemon.recalculate_stats_from_species(&species_data)
}

pub fn execute_distribute_effort_values(
    target: PlayerTarget,
    pokemon_index: usize,
    stats: [u8; 6],
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    // Implementation:
    // 1. For i in 0..6: pokemon.evs[i] = pokemon.evs[i].saturating_add(stats[i]).min(255)
    // 2. Enforce 510 total EV limit: if pokemon.evs.iter().sum() > 510, cap appropriately
    // 3. Get species data and call pokemon.recalculate_stats_from_species(&species_data)
}

pub fn execute_update_battle_participation(
    active_p0: usize,
    active_p1: usize,
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    // Implementation: state.participation_tracker.record_participation(active_p0, active_p1)
}
```

## Integration Points

### HandlePokemonFainted Integration
```rust
// In src/battle/commands.rs, execute_state_change() function:
BattleCommand::HandlePokemonFainted { target } => {
    let mut commands = vec![];
    commands.push(BattleCommand::ClearPlayerState { target: *target });
    
    // Get fainted Pokemon info
    let player_index = target.to_index();
    if let Some(fainted_pokemon) = state.players[player_index].active_pokemon() {
        let fainted_species = fainted_pokemon.species;
        
        // Calculate and add progression rewards
        let progression_commands = crate::battle::progression::calculate_progression_commands(
            *target,
            fainted_species,
            state,
            &state.participation_tracker,  // Use the tracker from BattleState
        );
        commands.extend(progression_commands);
    }
    
    return Ok(commands);
}
```

### Battle Engine Integration
```rust
// In src/battle/engine.rs, resolve_turn() or execute_battle_action():
// Add participation tracking during each turn:

fn update_participation_tracking(battle_state: &mut BattleState) {
    let p0_active = battle_state.players[0].active_pokemon_index;
    let p1_active = battle_state.players[1].active_pokemon_index;
    battle_state.participation_tracker.record_participation(p0_active, p1_active);
}

// Call this function during turn resolution when both Pokemon are active
```

## Data Dependencies

### Required Species Data Access
```rust
// Pattern used throughout progression system:
let species_data = crate::get_species_data(pokemon.species)?;

// Key species data fields used:
species_data.base_stats         // For stat recalculation  
species_data.experience_group   // For level calculations
species_data.evolution_data     // For evolution checks
species_data.learnset.level_up  // For move learning
```

### Experience Group Methods (schema crate)
```rust
// Methods that must exist on ExperienceGroup enum:
impl ExperienceGroup {
    pub fn can_level_up(&self, current_level: u8, current_exp: u32) -> bool;
    pub fn calculate_level_from_exp(&self, exp: u32) -> u8;
    pub fn exp_for_level(&self, level: u8) -> u32;
}
```

### Error Handling
```rust
// src/battle/progression/validation.rs
#[derive(Debug, Clone, PartialEq)]
pub enum ProgressionError {
    NoPokemon { player_index: usize, pokemon_index: usize },
    PokemonFainted { player_index: usize, pokemon: Species },
    MaxLevel { pokemon: Species, level: u8 },
    InvalidIndices { player_index: usize, pokemon_index: usize },
}

// Command execution error (existing)
pub enum ExecutionError {
    NoPokemon,
    InvalidPokemonIndex,
    InvalidMoveIndex,    // Add this variant if missing
}
```

## Call Flow Summary

```
1. Pokemon faints → HandlePokemonFainted command
2. HandlePokemonFainted → calculate_progression_commands()  
3. calculate_progression_commands() → RewardCalculator + participation lookup
4. Returns Vec<BattleCommand> with progression commands
5. Commands executed via execute_award_experience(), execute_level_up_pokemon(), etc.
6. Each execution function performs simple state mutations + stat recalculation
7. Battle continues with updated Pokemon state
```

This design ensures progression rewards are automatically applied when Pokemon faint, following the existing command-execution pattern while maintaining separation between calculation logic and state mutation.