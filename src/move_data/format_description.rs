use crate::move_data::{MoveCategory, MoveData};

use super::{MoveEffect, StatType, Target};
use std::fmt;

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Target::User => write!(f, "user's"),
            Target::Target => write!(f, "target's"),
        }
    }
}

impl fmt::Display for StatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            StatType::Hp => "HP",
            StatType::Atk => "Attack",
            StatType::Def => "Defense",
            StatType::SpAtk => "Special Attack",
            StatType::SpDef => "Special Defense",
            StatType::Spe => "Speed",
            StatType::Acc => "Accuracy",
            StatType::Eva => "Evasion",
            StatType::Crit => "Critical Hit Ratio",
        };
        write!(f, "{}", name)
    }
}

impl fmt::Display for MoveCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveCategory::Physical => write!(f, "Physical"),
            MoveCategory::Special => write!(f, "Special"),
            MoveCategory::Other => write!(f, "Other"),
            MoveCategory::Status => write!(f, "Status"),
        }
    }
}

impl fmt::Display for MoveEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // --- STATUS AND VOLATILE CONDITIONS ---
            MoveEffect::Flinch(100) => write!(f, "Causes the target to flinch."),
            MoveEffect::Flinch(chance) => write!(f, "Has a {}% chance to make the target flinch.", chance),
            MoveEffect::Burn(100) => write!(f, "Burns the target."),
            MoveEffect::Burn(chance) => write!(f, "Has a {}% chance to burn the target.", chance),
            MoveEffect::Freeze(100) => write!(f, "Freezes the target."),
            MoveEffect::Freeze(chance) => write!(f, "Has a {}% chance to freeze the target.", chance),
            MoveEffect::Paralyze(100) => write!(f, "Paralyzes the target."),
            MoveEffect::Paralyze(chance) => write!(f, "Has a {}% chance to paralyze the target.", chance),
            MoveEffect::Poison(100) => write!(f, "Poisons the target."),
            MoveEffect::Poison(chance) => write!(f, "Has a {}% chance to poison the target.", chance),
            MoveEffect::Sedate(100) => write!(f, "Puts the target to sleep."),
            MoveEffect::Sedate(chance) => write!(f, "Has a {}% chance to put the target to sleep.", chance),
            MoveEffect::Confuse(100) => write!(f, "Confuses the target."),
            MoveEffect::Confuse(chance) => write!(f, "Has a {}% chance to confuse the target.", chance),
            MoveEffect::Trap(100) => write!(f, "Traps the target, preventing escape for several turns."),
            MoveEffect::Trap(chance) => write!(f, "Has a {}% chance to trap the target.", chance),
            MoveEffect::Seed(100) => write!(f, "Plants a seed on the target, draining HP each turn."),
            MoveEffect::Seed(chance) => write!(f, "Has a {}% chance to plant a seed on the target.", chance),
            MoveEffect::Disable(100) => write!(f, "Disables the last move the target used."),
            MoveEffect::Disable(chance) => write!(f, "Has a {}% chance to disable the target's last move.", chance),
            
            // --- STAT CHANGES ---
            MoveEffect::StatChange(target, stat, stages, chance) => {
                if *chance == 100 {
                    let action = if *stages > 0 { "Raises" } else { "Lowers" };
                    write!(f, "{} the {} {} by {} stage(s).", action, target, stat, stages.abs())
                } else {
                    let action = if *stages > 0 { "raise" } else { "lower" };
                    write!(f, "Has a {}% chance to {} the {} {} by {} stage(s).", chance, action, target, stat, stages.abs())
                }
            }
            MoveEffect::RaiseAllStats(chance) => write!(f, "Has a {}% chance to raise all of the user's primary stats.", chance),
            MoveEffect::Haze(_) => write!(f, "Eliminates all stat changes for all Pokémon on the field."),

            // --- DAMAGE MODIFIERS AND HP EFFECTS ---
            MoveEffect::Recoil(percent) => write!(f, "User receives recoil damage equal to {}% of the damage dealt.", percent),
            MoveEffect::Drain(percent) => match percent {
                100 => write!(f, "User recovers HP equal to the damage dealt."),
                50 => write!(f, "User recovers HP equal to half of the damage dealt."),
                _ => write!(f, "User recovers HP equal to {}% of the damage dealt.", percent),
            },
            MoveEffect::Crit(_) => write!(f, "Has an increased critical hit ratio."),
            MoveEffect::IgnoreDef(percent) => match percent {
                100 => write!(f, "Cuts through all the target's defenses."),
                _ => write!(f, "Cuts through {}% of the target's defenses", percent),
            },
            MoveEffect::SuperFang(_) => write!(f, "Cuts the target's current HP in half."),
            MoveEffect::SetDamage(amount) => write!(f, "Always deals {} damage.", amount),
            MoveEffect::LevelDamage => write!(f, "Deals damage equal to the user's level."),
            MoveEffect::Heal(percent) => match percent {
                100 => write!(f, "Fully restores the user's HP."),
                50 => write!(f, "Restores the user's HP by half of its maximum HP."),
                25 => write!(f, "Restores the user's HP by a quarter of its maximum HP."),
                _ => write!(f, "Restores the user's HP by {}% of its maximum HP.", percent),
            },
            MoveEffect::OHKO => write!(f, "A one-hit KO against weaker pokemon."),

            // --- MULTI-TURN AND EXECUTION FLOW ---
                        MoveEffect::MultiHit(guaranteed_hits, continuation_chance) => {
                match (*guaranteed_hits, *continuation_chance) {
                    // g > 1, c > 0
                    (g, c) if g > 1 && c > 0 => write!(f, "A series of {} attacks with a {}% chance of making more.", g, c),
                    // g > 1, c == 0
                    (g, 0) if g > 1 => write!(f, "A series of {} attacks.", g),
                    // g == 1, c > 0
                    (1, c) if c > 0 => write!(f, "Has a {}% chance of making more attacks.", c),
                    // Fallback for all other cases, primarily (1, 0).
                    _ => write!(f, "Hits once."),
                }
            }
            MoveEffect::Priority(p) if *p > 0 => write!(f, "Goes before other moves."),
            MoveEffect::Priority(p) if *p < 0 => write!(f, "Goes after other moves."),
            MoveEffect::Priority(_) => write!(f, "An ordinary attack."), // Should never happen.
            
            MoveEffect::ChargeUp => write!(f, "Charges on the first turn, then attacks on the second."),
            MoveEffect::InAir => write!(f, "User flies up on the first turn, then attacks on the second."),
            MoveEffect::Underground => write!(f, "User goes underground on the first turn, then attacks on the second."),
            MoveEffect::Exhaust(100) => write!(f, "User must recharge on the next turn."),
            MoveEffect::Exhaust(chance) => write!(f, "Has a {}% chance to exhaust the user for a turn after use", chance),
            MoveEffect::Rampage => write!(f, "User locks in to one attack for 2-3 turns, then becomes confused."),

            // --- SPECIAL AND UNIQUE MECHANICS ---
            MoveEffect::Explode => write!(f, "The user faints upon using this move."),
            MoveEffect::Reckless(_) => write!(f, "Hurts the user when it misses."),
            MoveEffect::Transform => write!(f, "The user transforms into a copy of the target."),
            MoveEffect::Conversion => write!(f, "Changes the user's type to match the type of its first move."),
            MoveEffect::Counter => write!(f, "Braces for impact, then returns twice the damage received this turn."),
            MoveEffect::MirrorMove => write!(f, "Copies and uses the last move the target used."),
            MoveEffect::Metronome => write!(f, "Performs a random move."),
            MoveEffect::Substitute => write!(f, "Creates a substitute with 25% of the user's max HP that blocks secondary effects."),
            MoveEffect::Rest(turns) => write!(f, "User sleeps for {} turns, fully restoring HP and status.", turns),
            MoveEffect::Bide(turns) => write!(f, "User absorbs damage for {} turns, then retaliates with double the power.", turns),
            MoveEffect::Rage(100) => write!(f, "Induces a trance that raises attack when hit until a different move is used."),
            MoveEffect::Rage(chance) => write!(f, "Has a {}% chance to induce a trance that raises attack when hit until a different move is used.", chance),
            MoveEffect::Teleport(100) => write!(f, "Moves in a flash to dodge enemy blows this turn."),
            MoveEffect::Teleport(chance) => write!(f, "Has a {}% chance to dodge enemy blows this turn.", chance),
            MoveEffect::Nightmare => write!(f, "Can only affect a sleeping foe."),

            // --- FIELD AND TEAM EFFECTS ---
            MoveEffect::SetTeamCondition(cond, _) => match cond {
                // Assuming the enum is at crate::player::TeamCondition
                crate::player::TeamCondition::Reflect => {
                    write!(f, "Reduces damage from physical attacks for several turns.")
                }
                crate::player::TeamCondition::LightScreen => {
                    write!(f, "Reduces damage from special attacks for several turns.")
                }
                crate::player::TeamCondition::Mist => {
                    write!(f, "Protects the user's team from having their stats lowered.")
                }
            },
            // --- UTILITY ---
            MoveEffect::CureStatus(target, status) => write!(f, "Cures the {} of {:?}.", target, status),
            MoveEffect::Ante(_) => write!(f, "Scatters coins around the field."),
        }
    }
}

impl fmt::Display for MoveData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Name and header
        writeln!(f, "{}", self.name)?;
        writeln!(f, "--------------------")?;

        // Core stats
        writeln!(f, "Type: {}", self.move_type)?;
        writeln!(f, "Category: {}", self.category)?;

        // Power, Accuracy, PP line
        let power_str = self.power.map_or("---".to_string(), |p| p.to_string());
        let accuracy_str = self.accuracy.map_or("---".to_string(), |a| a.to_string());
        writeln!(f, "Power: {} | Accuracy: {} | PP: {}", power_str, accuracy_str, self.max_pp)?;
        
        // Effects section
        if !self.effects.is_empty() {
            writeln!(f, "--------------------")?;
            writeln!(f, "Effects:")?;
            for effect in &self.effects {
                writeln!(f, "- {}", effect)?;
            }
        }
        
        Ok(())
    }
}
