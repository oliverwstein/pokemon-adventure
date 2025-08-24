use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveCategory {
    Physical,
    Special,
    Other,
    Status,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatType {
    Atk,
    Def,
    SpAtk,
    SpDef,
    Spe,
    Acc,
    Eva,
    Crit,
}

impl fmt::Display for StatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StatType::Atk => write!(f, "Attack"),
            StatType::Def => write!(f, "Defense"),
            StatType::SpAtk => write!(f, "Special Attack"),
            StatType::SpDef => write!(f, "Special Defense"),
            StatType::Spe => write!(f, "Speed"),
            StatType::Acc => write!(f, "Accuracy"),
            StatType::Eva => write!(f, "Evasion"),
            StatType::Crit => write!(f, "Focus"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    User,
    Target,
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Target::User => write!(f, "User"),
            Target::Target => write!(f, "Target"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusType {
    Sleep,
    Poison,
    Burn,
    Freeze,
    Paralysis,
}
