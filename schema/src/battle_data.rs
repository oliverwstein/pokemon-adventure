use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum TeamCondition {
    Reflect,
    LightScreen,
    Mist,
}

impl fmt::Display for TeamCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // We match on `self` to get the specific variant and write its
        // human-readable name to the formatter.
        let display_name = match self {
            TeamCondition::Reflect => "Reflect",
            TeamCondition::LightScreen => "Light Screen", // Use a space for better readability
            TeamCondition::Mist => "Mist",
        };

        // The write! macro handles writing the string to the output.
        write!(f, "{}", display_name)
    }
}