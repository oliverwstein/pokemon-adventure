use crate::pokemon::PokemonInst;
use crate::player::{BattlePlayer, StatType};
use crate::move_data::{get_move_data, MoveCategory};
use crate::moves::Move;

/// Calculate effective attack stat including stat stages, conditions, and other modifiers
pub fn effective_attack(pokemon: &PokemonInst, player: &BattlePlayer, move_: Move) -> u16 {
    let move_data = get_move_data(&move_).expect("Move data should exist");
    
    let base_attack = match move_data.category {
        MoveCategory::Physical => pokemon.curr_stats[1], // Attack
        MoveCategory::Special => pokemon.curr_stats[3],  // Special Attack
        MoveCategory::Status => return 0, // Status moves don't use attack stats
        MoveCategory::Other => return 0,  // Set damage, OHKO, status effects targeting enemy don't use attack stats
    };
    
    // Apply stat stage modifiers
    let attack_stat = match move_data.category {
        MoveCategory::Physical => StatType::Attack,
        MoveCategory::Special => StatType::SpecialAttack,
        MoveCategory::Status => return 0,
        MoveCategory::Other => return 0,
    };
    
    let stage = player.get_stat_stage(attack_stat);
    let mut multiplied_attack = apply_stat_stage_multiplier(base_attack, stage);
    
    // TODO: Apply move-specific modifiers based on move_data
    // Examples: Foul Play uses target's attack instead, Psyshock uses special attack vs physical defense
    
    // TODO: Apply other modifiers (burn for physical attacks, items, abilities, etc.)
    
    multiplied_attack
}

/// Calculate effective defense stat including stat stages, conditions, and other modifiers
pub fn effective_defense(pokemon: &PokemonInst, player: &BattlePlayer, move_: Move) -> u16 {
    let move_data = get_move_data(&move_).expect("Move data should exist");
    
    let base_defense = match move_data.category {
        MoveCategory::Physical => pokemon.curr_stats[2], // Defense
        MoveCategory::Special => pokemon.curr_stats[4],  // Special Defense
        MoveCategory::Status => return 0, // Status moves don't target defense
        MoveCategory::Other => return 0,  // Set damage, OHKO, status effects targeting enemy don't use defense stats
    };
    
    // Apply stat stage modifiers
    let defense_stat = match move_data.category {
        MoveCategory::Physical => StatType::Defense,
        MoveCategory::Special => StatType::SpecialDefense,
        MoveCategory::Status => return 0,
        MoveCategory::Other => return 0,
    };
    
    let stage = player.get_stat_stage(defense_stat);
    let mut multiplied_defense = apply_stat_stage_multiplier(base_defense, stage);
    
    // TODO: Apply move-specific modifiers based on move_data
    // Examples: Psyshock/Psystrike use special attack vs physical defense
    
    // TODO: Apply other modifiers (items, abilities, etc.)
    
    multiplied_defense
}

/// Calculate effective speed including stat stages, paralysis, and other modifiers
pub fn effective_speed(pokemon: &PokemonInst, player: &BattlePlayer) -> u16 {
    let base_speed = pokemon.curr_stats[5]; // Speed
    
    // Apply stat stage modifiers
    let stage = player.get_stat_stage(StatType::Speed);
    let mut multiplied_speed = apply_stat_stage_multiplier(base_speed, stage);
    
    // Apply paralysis (quarter speed)
    if let Some(status) = &pokemon.status {
        if matches!(status, crate::pokemon::StatusCondition::Paralysis) {
            multiplied_speed /= 4;
        }
    }
    
    // TODO: Apply other modifiers (items, abilities, field effects, etc.)
    
    multiplied_speed
}

/// Apply stat stage multipliers according to Pokemon formula
/// Stages range from -6 to +6
/// Negative stages: (2 / (2 + |stage|))
/// Positive stages: ((2 + stage) / 2)
fn apply_stat_stage_multiplier(base_stat: u16, stage: i8) -> u16 {
    let clamped_stage = stage.clamp(-6, 6);
    
    if clamped_stage == 0 {
        return base_stat;
    }
    
    let multiplier = if clamped_stage < 0 {
        2.0 / (2.0 + (-clamped_stage) as f64)
    } else {
        (2.0 + clamped_stage as f64) / 2.0
    };
    
    ((base_stat as f64) * multiplier).round() as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::species::Species;
    use std::collections::HashMap;
    
    #[test]
    fn test_stat_stage_multipliers() {
        // Test various stat stage multipliers
        assert_eq!(apply_stat_stage_multiplier(100, 0), 100);   // No change
        assert_eq!(apply_stat_stage_multiplier(100, 1), 150);   // +1 stage: 1.5x
        assert_eq!(apply_stat_stage_multiplier(100, 2), 200);   // +2 stage: 2.0x
        assert_eq!(apply_stat_stage_multiplier(100, -1), 67);   // -1 stage: 2/3x
        assert_eq!(apply_stat_stage_multiplier(100, -2), 50);   // -2 stage: 1/2x
        assert_eq!(apply_stat_stage_multiplier(100, 6), 400);   // +6 stage: 4.0x
        assert_eq!(apply_stat_stage_multiplier(100, -6), 25);   // -6 stage: 1/4x
    }
    
    #[test] 
    fn test_effective_speed_paralysis() {
        let mut pokemon = crate::pokemon::PokemonInst {
            name: "Test".to_string(),
            species: Species::Pikachu,
            curr_exp: 0,
            ivs: [15; 6],
            evs: [0; 6],
            curr_stats: [100, 80, 80, 80, 80, 100], // Speed = 100
            moves: [None; 4],
            status: Some(crate::pokemon::StatusCondition::Paralysis),
        };
        
        let player = crate::player::BattlePlayer {
            player_id: "test".to_string(),
            player_name: "Test".to_string(),
            team: [None; 6],
            active_pokemon_index: 0,
            stat_stages: HashMap::new(),
            team_conditions: HashMap::new(),
            active_pokemon_conditions: HashMap::new(),
            last_move: None,
        };
        
        // Paralysis should quarter speed: 100 / 4 = 25
        assert_eq!(effective_speed(&pokemon, &player), 25);
        
        // Test without paralysis
        pokemon.status = None;
        assert_eq!(effective_speed(&pokemon, &player), 100);
    }
}