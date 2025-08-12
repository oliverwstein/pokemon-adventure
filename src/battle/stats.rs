use crate::move_data::{MoveCategory, get_move_data};
use crate::moves::Move;
use crate::player::{BattlePlayer, StatType};
use crate::pokemon::{PokemonInst, PokemonType};

/// Calculate effective attack stat including stat stages, conditions, and other modifiers
pub fn effective_attack(pokemon: &PokemonInst, player: &BattlePlayer, move_: Move) -> u16 {
    let move_data = get_move_data(move_).expect("Move data should exist");

    // Check if transformed - use target Pokemon's base stats
    let base_attack = if let Some(transform_condition) = player
        .active_pokemon_conditions
        .values()
        .find_map(|condition| match condition {
            crate::player::PokemonCondition::Transformed { target } => Some(target),
            _ => None,
        }) {
        match move_data.category {
            MoveCategory::Physical => transform_condition.stats.attack,
            MoveCategory::Special => transform_condition.stats.sp_attack,
            MoveCategory::Status => return 0,
            MoveCategory::Other => return 0,
        }
    } else {
        match move_data.category {
            MoveCategory::Physical => pokemon.stats.attack,
            MoveCategory::Special => pokemon.stats.sp_attack,
            MoveCategory::Status => return 0, // Status moves don't use attack stats
            MoveCategory::Other => return 0, // Set damage, OHKO, status effects targeting enemy don't use attack stats
        }
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

    // Apply burn status (halves physical attack only)
    if matches!(move_data.category, MoveCategory::Physical) {
        if let Some(status) = &pokemon.status {
            if matches!(status, crate::pokemon::StatusCondition::Burn) {
                multiplied_attack /= 2;
            }
        }
    }

    // TODO: Apply move-specific modifiers based on move_data
    // Examples: Foul Play uses target's attack instead, Psyshock uses special attack vs physical defense

    // TODO: Apply other modifiers (items, abilities, etc.)

    multiplied_attack
}

/// Calculate effective defense stat including stat stages, conditions, and other modifiers
pub fn effective_defense(pokemon: &PokemonInst, player: &BattlePlayer, move_: Move) -> u16 {
    let move_data = get_move_data(move_).expect("Move data should exist");

    // Check if transformed - use target Pokemon's base stats
    let base_defense = if let Some(transform_condition) = player
        .active_pokemon_conditions
        .values()
        .find_map(|condition| match condition {
            crate::player::PokemonCondition::Transformed { target } => Some(target),
            _ => None,
        }) {
        match move_data.category {
            MoveCategory::Physical => transform_condition.stats.defense,
            MoveCategory::Special => transform_condition.stats.sp_defense,
            MoveCategory::Status => return 0,
            MoveCategory::Other => return 0,
        }
    } else {
        match move_data.category {
            MoveCategory::Physical => pokemon.stats.defense,
            MoveCategory::Special => pokemon.stats.sp_defense,
            MoveCategory::Status => return 0, // Status moves don't target defense
            MoveCategory::Other => return 0, // Set damage, OHKO, status effects targeting enemy don't use defense stats
        }
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
    for effect in &move_data.effects {
        if let crate::move_data::MoveEffect::IgnoreDef(percentage) = effect {
            // The percentage is how much of the defense to ignore.
            // For example, IgnoreDef(50) means 50% is ignored, so the final defense is multiplied by 0.5.
            let remaining_defense_factor = 1.0 - (*percentage as f64 / 100.0);

            // Apply the reduction to the calculated defense.
            multiplied_defense =
                ((multiplied_defense as f64) * remaining_defense_factor).round() as u16;

            // A move should only have one IgnoreDef effect, so we can stop looking.
            break;
        }
    }

    // Apply team condition modifiers (Reflect/Light Screen)
    match move_data.category {
        MoveCategory::Physical => {
            // Reflect reduces damage from physical moves by 50%
            if player.has_team_condition(&crate::player::TeamCondition::Reflect) {
                multiplied_defense = (multiplied_defense as f64 * 2.0).round() as u16;
            }
        }
        MoveCategory::Special => {
            // Light Screen reduces damage from special moves by 50%
            if player.has_team_condition(&crate::player::TeamCondition::LightScreen) {
                multiplied_defense = (multiplied_defense as f64 * 2.0).round() as u16;
            }
        }
        _ => {} // Status and Other moves don't use defense stats
    }

    // TODO: Apply move-specific modifiers based on move_data
    // Examples: Psyshock/Psystrike use special attack vs physical defense

    // TODO: Apply other modifiers (items, abilities, etc.)

    multiplied_defense
}

/// Calculate effective speed including stat stages, paralysis, and other modifiers
pub fn effective_speed(pokemon: &PokemonInst, player: &BattlePlayer) -> u16 {
    // Check if transformed - use target Pokemon's base speed
    let base_speed = if let Some(transform_condition) = player
        .active_pokemon_conditions
        .values()
        .find_map(|condition| match condition {
            crate::player::PokemonCondition::Transformed { target } => Some(target),
            _ => None,
        }) {
        transform_condition.stats.speed
    } else {
        pokemon.stats.speed
    };

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

/// Calculate if a move is a critical hit based on critical hit ratio and focus energy
/// Returns true if the move is a critical hit
pub fn move_is_critical_hit(
    attacker: &PokemonInst,
    attacker_player: &BattlePlayer,
    move_: Move,
    rng: &mut crate::battle::state::TurnRng,
) -> bool {
    let move_data = get_move_data(move_).expect("Move data should exist");

    // Status moves cannot be critical hits (with very rare exceptions)
    if matches!(move_data.category, MoveCategory::Status) {
        return false;
    }

    // Base critical hit ratio - starts at 1 (1/24 chance in Gen 1)
    let mut crit_ratio = 1u8;

    // Check for moves with increased critical hit ratio
    for effect in &move_data.effects {
        if let crate::move_data::MoveEffect::Crit(ratio_boost) = effect {
            crit_ratio = crit_ratio.saturating_add(*ratio_boost);
        }
    }

    // Check for Focus Energy stat stage (increases crit ratio)
    let focus_stage = attacker_player.get_stat_stage(StatType::Focus);
    if focus_stage > 0 {
        crit_ratio = crit_ratio.saturating_add(focus_stage as u8);
    }

    // Calculate critical hit threshold based on ratio
    // Gen 1 formula: (base_speed / 2) * crit_ratio / 256
    // For simplicity, using fixed thresholds based on crit ratio
    let crit_threshold = match crit_ratio {
        1 => 4,  // ~1/24 chance (base rate)
        2 => 12, // ~1/8 chance (high crit moves like Slash)
        3 => 25, // ~1/4 chance
        4 => 33, // ~1/3 chance
        5 => 50, // ~1/2 chance
        6 => 75, // ~3/4 chance
        _ => 90, // Nearly guaranteed (7+ ratio)
    };

    // Roll for critical hit
    let roll = rng.next_outcome();
    roll <= crit_threshold
}

/// Calculate if a move hits based on accuracy, evasion, and move accuracy
/// Returns true if the move hits, false if it misses
pub fn move_hits(
    attacker: &PokemonInst,
    defender: &PokemonInst,
    attacker_player: &BattlePlayer,
    defender_player: &BattlePlayer,
    move_: Move,
    rng: &mut crate::battle::state::TurnRng,
) -> bool {
    let move_data = get_move_data(move_).expect("Move data should exist");

    // If move has no accuracy value, it never misses (like Swift)
    let Some(base_accuracy) = move_data.accuracy else {
        return true;
    };

    // If defender is Teleported, moves with accuracy always miss
    if defender_player.has_condition(&crate::player::PokemonCondition::Teleported) {
        return false;
    }

    // Calculate adjusted stages: attacker's accuracy - defender's evasion
    let accuracy_stage = attacker_player.get_stat_stage(StatType::Accuracy);
    let evasion_stage = defender_player.get_stat_stage(StatType::Evasion);
    let adjusted_stage = (accuracy_stage - evasion_stage).clamp(-6, 6);

    // Calculate stage multiplier
    let stage_multiplier = apply_accuracy_stage_multiplier(adjusted_stage);

    // Calculate final accuracy threshold
    let modified_accuracy = (base_accuracy as f64 * stage_multiplier).round() as u8;
    let clamped_accuracy = modified_accuracy.clamp(1, 100);

    // Roll for hit/miss
    let roll = rng.next_outcome();
    roll <= clamped_accuracy
}

/// Apply accuracy/evasion stage multipliers according to Pokemon formula
/// Uses different multipliers than regular stats
/// Stages range from -6 to +6
fn apply_accuracy_stage_multiplier(stage: i8) -> f64 {
    match stage {
        -6 => 3.0 / 9.0, // 33%
        -5 => 3.0 / 8.0, // 37.5%
        -4 => 3.0 / 7.0, // 43%
        -3 => 3.0 / 6.0, // 50%
        -2 => 3.0 / 5.0, // 60%
        -1 => 3.0 / 4.0, // 75%
        0 => 3.0 / 3.0,  // 100%
        1 => 4.0 / 3.0,  // 133%
        2 => 5.0 / 3.0,  // 167%
        3 => 6.0 / 3.0,  // 200%
        4 => 7.0 / 3.0,  // 233%
        5 => 8.0 / 3.0,  // 267%
        6 => 9.0 / 3.0,  // 300%
        _ => 1.0,        // Should never happen due to clamp, but safety fallback
    }
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

pub fn get_type_effectiveness(attack_type: PokemonType, defense_types: &[PokemonType]) -> f64 {
    defense_types
        .iter()
        .map(|&def_type| PokemonType::type_effectiveness(attack_type, def_type) as f64)
        .product()
}

/// Formula: ((((2 * Level / 5 + 2) * Power * STAB * A / D) / 50 + 2) * CRIT * TYPE_ADV * RAND * MODIFIERS)
pub fn calculate_attack_damage(
    attacker: &PokemonInst,
    defender: &PokemonInst,
    attacker_player: &BattlePlayer,
    defender_player: &BattlePlayer,
    move_used: Move,
    is_critical: bool,
    rng: &mut crate::battle::state::TurnRng,
) -> u16 {
    let move_data =
        get_move_data(move_used).expect("Move data should exist for damage calculation");

    // 1. Get Power from move data. If no power, no damage.
    let Some(power) = move_data.power else {
        return 0;
    };
    if power == 0 {
        return 0;
    }

    // 2. Determine effective Attack and Defense stats.
    // These functions already account for stat stages, burn, etc.
    let attack = effective_attack(attacker, attacker_player, move_used);
    let defense = effective_defense(defender, defender_player, move_used);

    // Assume a fixed level for all battle calculations, a common standard for competitive play.
    let level: u16 = 50;

    // 3. Calculate STAB (Same-Type Attack Bonus)
    let stab_multiplier = {
        let attacker_types = attacker.get_current_types(attacker_player);
        if attacker_types.contains(&move_data.move_type) {
            1.5
        } else {
            1.0
        }
    };

    // 4. Calculate the core part of the formula using integer arithmetic first.
    let term1 = (2 * level / 5) + 2;
    // We cast to f64 to incorporate the STAB multiplier before the main division.
    let base_damage_part =
        (term1 as f64) * (power as f64) * (stab_multiplier) * (attack as f64) / (defense as f64);
    let base_damage = (base_damage_part / 50.0) + 2.0;

    // 5. Gather all final multipliers.
    // Critical Hit: 2x multiplier
    let crit_multiplier = if is_critical { 2.0 } else { 1.0 };

    // Use the centralized type getter that handles Transform and Conversion
    let defender_types = defender.get_current_types(defender_player);
    let type_adv_multiplier = get_type_effectiveness(move_data.move_type, &defender_types);
    // Random Variance: A random multiplier between 0.85 and 1.00
    let random_multiplier = (85.0 + (rng.next_outcome() % 16) as f64) / 100.0;

    // Other modifiers (e.g., from items, abilities). Placeholder for now.
    let other_modifiers = 1.0;

    // 6. Apply all multipliers to the base damage.
    let final_damage_float =
        base_damage * crit_multiplier * type_adv_multiplier * random_multiplier * other_modifiers;

    // 7. Convert to integer and ensure damage is at least 1.
    let final_damage = final_damage_float.round() as u16;

    final_damage.max(1)
}

pub fn calculate_special_attack_damage(
    move_used: Move,
    attacker: &PokemonInst,
    defender: &PokemonInst,
) -> Option<u16> {
    let move_data =
        get_move_data(move_used).expect("Move data must exist for special damage calculation");

    // For now, we assume a fixed level for all battle calculations, consistent with the standard formula.
    // TODO: When/if PokemonInst gets a `level` field, this should be changed to `attacker.level`.
    let attacker_level: u16 = 50;
    let defender_level: u16 = 50;

    for effect in &move_data.effects {
        match effect {
            crate::move_data::MoveEffect::OHKO => {
                // OHKO moves fail if the attacker's level is less than the defender's.
                // Otherwise, they deal damage equal to the target's current HP.
                if attacker_level < defender_level {
                    return Some(0); // The move fails
                } else {
                    return Some(defender.current_hp());
                }
            }
            crate::move_data::MoveEffect::SuperFang(_) => {
                // Super Fang deals damage equal to half of the opponent's current HP.
                return Some((defender.current_hp() / 2).max(1));
            }
            crate::move_data::MoveEffect::LevelDamage => {
                // Deals damage equal to the user's level.
                return Some(attacker_level);
            }
            crate::move_data::MoveEffect::SetDamage(fixed_damage) => {
                // Deals a fixed amount of damage.
                return Some(*fixed_damage);
            }
            _ => {} // Ignore other effects, continue searching.
        }
    }

    // If the loop completes without finding a special damage effect, return None.
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::species::Species;
    use std::collections::HashMap;

    #[test]
    fn test_stat_stage_multipliers() {
        // Test various stat stage multipliers
        assert_eq!(apply_stat_stage_multiplier(100, 0), 100); // No change
        assert_eq!(apply_stat_stage_multiplier(100, 1), 150); // +1 stage: 1.5x
        assert_eq!(apply_stat_stage_multiplier(100, 2), 200); // +2 stage: 2.0x
        assert_eq!(apply_stat_stage_multiplier(100, -1), 67); // -1 stage: 2/3x
        assert_eq!(apply_stat_stage_multiplier(100, -2), 50); // -2 stage: 1/2x
        assert_eq!(apply_stat_stage_multiplier(100, 6), 400); // +6 stage: 4.0x
        assert_eq!(apply_stat_stage_multiplier(100, -6), 25); // -6 stage: 1/4x
    }

    #[test]
    fn test_accuracy_stage_multipliers() {
        // Test accuracy/evasion stage multipliers
        assert!((apply_accuracy_stage_multiplier(0) - 1.0).abs() < 0.001); // No change: 100%
        assert!((apply_accuracy_stage_multiplier(1) - 4.0 / 3.0).abs() < 0.001); // +1: 133%
        assert!((apply_accuracy_stage_multiplier(-1) - 3.0 / 4.0).abs() < 0.001); // -1: 75%
        assert!((apply_accuracy_stage_multiplier(6) - 3.0).abs() < 0.001); // +6: 300%
        assert!((apply_accuracy_stage_multiplier(-6) - 1.0 / 3.0).abs() < 0.001); // -6: 33%
    }

    #[test]
    fn test_effective_speed_paralysis() {
        let mut pokemon = crate::pokemon::PokemonInst::new_for_test(
            Species::Pikachu,
            0,
            0,
            100,
            [15; 6],
            [0; 6],
            [100, 80, 80, 80, 80, 100], // Speed = 100
            [const { None }; 4],
            Some(crate::pokemon::StatusCondition::Paralysis),
        );

        let player = crate::player::BattlePlayer {
            player_id: "test".to_string(),
            player_name: "Test".to_string(),
            team: [const { None }; 6],
            active_pokemon_index: 0,
            stat_stages: HashMap::new(),
            team_conditions: HashMap::new(),
            active_pokemon_conditions: HashMap::new(),
            last_move: None,
            ante: 200,
        };

        // Paralysis should quarter speed: 100 / 4 = 25
        assert_eq!(effective_speed(&pokemon, &player), 25);

        // Test without paralysis
        pokemon.status = None;
        assert_eq!(effective_speed(&pokemon, &player), 100);
    }

    #[test]
    fn test_effective_attack_burn() {
        // Initialize move data (required for get_move_data to work)
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");
        let mut pokemon = crate::pokemon::PokemonInst::new_for_test(
            Species::Charmander,
            0,
            0,
            100, // Set current HP directly
            [15; 6],
            [0; 6],
            [100, 80, 80, 80, 80, 100], // Attack = 80
            [const { None }; 4],
            Some(crate::pokemon::StatusCondition::Burn),
        );

        let player = crate::player::BattlePlayer {
            player_id: "test".to_string(),
            player_name: "Test".to_string(),
            team: [const { None }; 6],
            active_pokemon_index: 0,
            stat_stages: HashMap::new(),
            team_conditions: HashMap::new(),
            active_pokemon_conditions: HashMap::new(),
            last_move: None,
            ante: 200,
        };

        // Burn should halve physical attack: 80 / 2 = 40
        assert_eq!(
            effective_attack(&pokemon, &player, crate::moves::Move::Tackle),
            40
        );

        // Burn should NOT affect special attacks
        assert_eq!(
            effective_attack(&pokemon, &player, crate::moves::Move::Ember),
            80
        );

        // Test without burn
        pokemon.status = None;
        assert_eq!(
            effective_attack(&pokemon, &player, crate::moves::Move::Tackle),
            80
        );
        assert_eq!(
            effective_attack(&pokemon, &player, crate::moves::Move::Ember),
            80
        );
    }

    #[test]
    fn test_critical_hit_calculation() {
        // Initialize move data (required for get_move_data to work)
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");

        let pokemon = crate::pokemon::PokemonInst::new_for_test(
            Species::Pikachu,
            10,
            0,
            100,
            [15; 6],
            [0; 6],
            [100, 80, 80, 80, 80, 100],
            [const { None }; 4],
            None,
        );

        let mut player = crate::player::BattlePlayer {
            player_id: "test".to_string(),
            player_name: "Test".to_string(),
            team: [const { None }; 6],
            active_pokemon_index: 0,
            stat_stages: HashMap::new(),
            team_conditions: HashMap::new(),
            active_pokemon_conditions: HashMap::new(),
            last_move: None,
            ante: 0,
        };

        // Test with deterministic RNG - low roll should not be critical hit
        let mut rng_low = crate::battle::state::TurnRng::new_for_test(vec![10, 10, 10]);
        let is_crit_low =
            move_is_critical_hit(&pokemon, &player, crate::moves::Move::Tackle, &mut rng_low);
        assert!(
            !is_crit_low,
            "Low roll (10) should not be critical hit with base rate (threshold 4)"
        );

        // Test with deterministic RNG - low roll should be critical hit
        let mut rng_high = crate::battle::state::TurnRng::new_for_test(vec![3, 3, 3]);
        let is_crit_high =
            move_is_critical_hit(&pokemon, &player, crate::moves::Move::Tackle, &mut rng_high);
        assert!(
            is_crit_high,
            "Low roll (3) should be critical hit with base rate (threshold 4)"
        );

        // Test with Focus Energy stat stage
        player.set_stat_stage(StatType::Focus, 2);
        let mut rng_focus = crate::battle::state::TurnRng::new_for_test(vec![20, 20, 20]);
        let is_crit_focus = move_is_critical_hit(
            &pokemon,
            &player,
            crate::moves::Move::Tackle,
            &mut rng_focus,
        );
        assert!(
            is_crit_focus,
            "Focus Energy should increase critical hit chance (ratio 3, threshold 25)"
        );

        // Test status moves cannot be critical hits
        let mut rng_status = crate::battle::state::TurnRng::new_for_test(vec![99, 99, 99]);
        let is_crit_status = move_is_critical_hit(
            &pokemon,
            &player,
            crate::moves::Move::Growl,
            &mut rng_status,
        );
        assert!(
            !is_crit_status,
            "Status moves should never be critical hits"
        );
    }

    #[test]
    fn test_combined_status_effects() {
        // Initialize move data (required for get_move_data to work)
        use std::path::Path;
        let data_path = Path::new("data");
        crate::move_data::initialize_move_data(data_path).expect("Failed to initialize move data");

        // Test Pokemon with burn status
        let mut burned_pokemon = crate::pokemon::PokemonInst::new_for_test(
            Species::Charmander,
            10,
            0,
            100, // Set current HP directly to max
            [15; 6],
            [0; 6],
            [100, 80, 60, 80, 60, 100], // Attack=80, Defense=60, Speed=100
            [const { None }; 4],
            Some(crate::pokemon::StatusCondition::Burn),
        );

        // Test Pokemon with paralysis
        let mut paralyzed_pokemon = crate::pokemon::PokemonInst::new_for_test(
            Species::Pikachu,
            10,
            0,
            100, // Set current HP directly to max
            [15; 6],
            [0; 6],
            [100, 80, 60, 80, 60, 100], // Attack=80, Defense=60, Speed=100
            [const { None }; 4],
            Some(crate::pokemon::StatusCondition::Paralysis),
        );

        let player = crate::player::BattlePlayer {
            player_id: "test".to_string(),
            player_name: "Test".to_string(),
            team: [const { None }; 6],
            active_pokemon_index: 0,
            stat_stages: HashMap::new(),
            team_conditions: HashMap::new(),
            active_pokemon_conditions: HashMap::new(),
            last_move: None,
            ante: 200,
        };

        // Test burn effects
        assert_eq!(
            effective_attack(&burned_pokemon, &player, crate::moves::Move::Tackle),
            40,
            "Burn should halve physical attack: 80/2=40"
        );
        assert_eq!(
            effective_attack(&burned_pokemon, &player, crate::moves::Move::Ember),
            80,
            "Burn should NOT affect special attack"
        );
        assert_eq!(
            effective_speed(&burned_pokemon, &player),
            100,
            "Burn should NOT affect speed"
        );

        // Test paralysis effects
        assert_eq!(
            effective_speed(&paralyzed_pokemon, &player),
            25,
            "Paralysis should quarter speed: 100/4=25"
        );
        assert_eq!(
            effective_attack(&paralyzed_pokemon, &player, crate::moves::Move::Tackle),
            80,
            "Paralysis should NOT affect attack"
        );
        assert_eq!(
            effective_attack(
                &paralyzed_pokemon,
                &player,
                crate::moves::Move::ThunderPunch
            ),
            80,
            "Paralysis should NOT affect special attack"
        );

        // Test healthy Pokemon (no status)
        burned_pokemon.status = None;
        paralyzed_pokemon.status = None;

        assert_eq!(
            effective_attack(&burned_pokemon, &player, crate::moves::Move::Tackle),
            80
        );
        assert_eq!(effective_speed(&paralyzed_pokemon, &player), 100);
    }
}
