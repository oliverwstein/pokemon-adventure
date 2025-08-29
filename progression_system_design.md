# Progression System Design Specification (Revised)

## Overview
This document specifies the complete interface and implementation design for integrating Pokemon progression (experience, leveling, evolution, move learning) into the battle system.

## System Dependencies

### Existing Data Structures

#### `PokemonInst` (src/pokemon.rs)
```rust
pub struct PokemonInst {
    pub name: String,
    pub species: Species,
    pub level: u8,
    pub curr_exp: u32,           // ← Use this field for total experience
    curr_hp: u16,                // Private field
    pub ivs: [u8; 6],            // HP, ATK, DEF, SP.ATK, SP.DEF, SPD
    pub evs: [u8; 6],            // ← Use this field for Effort Values
    pub stats: CurrentStats,
    pub moves: [Option<MoveInstance>; 4],  // ← Fixed array, not Vec<Move>
    pub status: Option<StatusCondition>,
}

// Required methods that must exist or be added:
impl PokemonInst {
    fn calculate_stats(base_stats: &BaseStats, level: u8, ivs: &[u8; 6], evs: &[u8; 6]) -> CurrentStats;

    /// The primary public method for applying a single level-up.
    /// Increments the level and triggers an internal stat recalculation.
    pub fn apply_level_up(&mut self);

    /// A simple, atomic mutator for adding experience points.
    /// This only updates `curr_exp` and does not trigger level-ups directly.
    pub fn add_experience(&mut self, amount: u32);
    
    /// A private helper for recalculating stats, called internally by apply_level_up,
    /// apply_evolution, etc., to ensure state consistency.
    fn recalculate_stats(&mut self); 
}
```

#### `BattleCommand` Enum (src/battle/commands.rs)
```rust
pub enum BattleCommand {
    // Existing progression commands - use these exact variants and field names:
    AwardExperience {
        recipients: Vec<(PlayerTarget, usize, u32)>, // (player, pokemon_index, exp_amount)
    },
    LevelUpPokemon {
        target: PlayerTarget,
        pokemon_index: usize,
        // Note: No new_level field. This command triggers a single level increase.
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
}
```


## Function Specifications

### Core Progression Calculation
```rust
// src/battle/progression/calculation.rs
pub fn calculate_progression_commands(
    fainted_target: PlayerTarget,      // Which player's Pokemon fainted
    fainted_species: Species,          // Species of fainted Pokemon
    battle_state: &BattleState,        // Complete battle state for context
) -> Vec<BattleCommand> {
    // Implementation logic:
    // 1. Use RewardCalculator to get base experience and EV yield for the fainted Pokémon.
    // 2. Use battle_state.participation_tracker to get a Vec<usize> of participants.
    // 3. For each valid participant (not fainted, not max level):
    //    a. Generate an AwardExperience command with the calculated share of XP.
    //    b. Generate a DistributeEffortValues command with the full EV yield.
    //    c. Calculate the Pokémon's new total experience.
    //    d. Use species_data.experience_group to calculate the new level from the new total experience.
    //    e. For each level gained (from old_level to new_level), generate a LevelUpPokemon command.
    //       - After each hypothetical level up, check for new moves to learn or evolutions to trigger and generate those commands as well.
    // 4. Return the complete Vec<BattleCommand>.

    // Alternatively, we can revise add_experience to return a vector of the levels as u8s, 
    // then execute_award_experience can return the LevelUp BattleCommands as appropriate.
    // Then, execute_level_up_pokemon can similarly return the LearnMove and EvolvePokemon commands as appropriate.
    // This approach would decentralize the calculations from calculate_progression_commands. 
}
```

### Command Execution Functions
```rust
// src/battle/progression/commands.rs

pub fn execute_award_experience(
    recipients: &[(PlayerTarget, usize, u32)],
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    // Implementation: For each recipient, get a mutable reference to the Pokémon
    // and call pokemon.add_experience(exp_amount). Returns an empty Vec.
}

pub fn execute_level_up_pokemon(
    target: PlayerTarget,
    pokemon_index: usize,
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    // Implementation:
    // 1. Get a mutable reference to the Pokémon.
    // 2. Call pokemon.apply_level_up(). Returns an empty Vec.
}

pub fn execute_learn_move(
    target: PlayerTarget,
    pokemon_index: usize,
    move_: Move,
    replace_index: Option<usize>,
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    // Implementation:
    // 1. Create a new MoveInstance from `move_`.
    // 2. If replace_index is Some(i), replace the move at pokemon.moves[i].
    // 3. If replace_index is None, find the first empty (None) slot and place the move there.
    // 4. (Optional) If no empty slot, default to replacing the last move, pokemon.moves[3].
}

pub fn execute_evolve_pokemon(
    target: PlayerTarget,
    pokemon_index: usize,
    new_species: Species,
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    // Implementation:
    // 1. Get a mutable reference to the Pokémon.
    // 2. Set pokemon.species = new_species.
    // 3. (Optional) Create a pokemon.apply_evolution() method that handles the species
    //    change and internally calls the private `recalculate_stats()`.
}

pub fn execute_distribute_effort_values(
    target: PlayerTarget,
    pokemon_index: usize,
    stats: [u8; 6],
    state: &mut BattleState,
) -> Result<Vec<BattleCommand>, ExecutionError> {
    // Implementation:
    // 1. Get a mutable reference to the Pokémon.
    // 2. Add the `stats` array to `pokemon.evs`, ensuring no single EV exceeds 255 and the total does not exceed 510.
    // 3. Trigger a stat recalculation via the private `recalculate_stats()` method.
}
```

## Integration Points

### `HandlePokemonFainted` Integration
```rust
// In src/battle/commands.rs, inside the execute_state_change() function:
BattleCommand::HandlePokemonFainted { target } => {
    let mut commands = vec![];
    commands.push(BattleCommand::ClearPlayerState { target: *target });
    
    let player_index = target.to_index();
    if let Some(fainted_pokemon) = state.players[player_index].active_pokemon() {
        let fainted_species = fainted_pokemon.species;
        
        let progression_commands = crate::battle::progression::calculate_progression_commands(
            *target,
            fainted_species,
            state, // Pass the whole state, which contains the tracker
        );
        commands.extend(progression_commands);
    }
    
    return Ok(commands);
}
```

### `SwitchPokemon` Command Integration
```rust
// In src/battle/commands.rs, inside the execute_state_change() function:
BattleCommand::SwitchPokemon { target, new_pokemon_index } => {
    // ... validation and state mutation to update player.active_pokemon_index ...
    
    // After the switch, determine the new p0 and p1 active indices from the BattleState.
    let p0_active_index = state.players[0].active_pokemon_index;
    let p1_active_index = state.players[1].active_pokemon_index;

    // Record the new matchup. This is the sole integration point for tracking switches.
    state.participation_tracker.record_participation(p0_active_index, p1_active_index);
    
    return Ok(vec![]);
}
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

### `ExperienceGroup` Methods (schema crate)
The following methods on the `ExperienceGroup` enum are essential and are confirmed to exist in the `schema` crate.```rust
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
    InvalidMoveIndex,    // Ensure this variant exists
}
```

## Call Flow Summary

```
1. Pokemon faints → `HandlePokemonFainted` command is generated.
2. The `HandlePokemonFainted` handler calls `calculate_progression_commands()`.
3. `calculate_progression_commands()` acts as the "brain," using `RewardCalculator` and the `participation_tracker` to determine all necessary rewards.
4. It returns a complete `Vec<BattleCommand>` containing `[AwardExperience, DistributeEffortValues, LevelUpPokemon, LearnMove, EvolvePokemon, ...]`.
5. The engine executes each of these commands in sequence.
6. Each execution function performs a simple, atomic state mutation on the relevant `PokemonInst` and recalculates stats where necessary.
7. The battle continues with the updated state of the participating Pokémon.
```