use std::fmt;

// Re-export the Species enum from the schema crate
pub use schema::Species;

use crate::get_species_data;

/// Display detailed information about a species including stats and description
pub fn display_species_detailed(species: Species, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match get_species_data(species) {
        Ok(data) => {
            // --- 1. Name and Pokedex Number ---
            writeln!(f, "{} (#{:03})", data.name, data.pokedex_number)?;
            writeln!(f, "--------------------")?;

            // --- 2. Description ---
            writeln!(f, "{}", data.description)?;
            writeln!(f, "--------------------")?;

            // --- 3. Types ---
            write!(f, "Type(s): ")?;
            let type_names: Vec<String> = data.types.iter().map(|t| format!("{}", t)).collect();
            writeln!(f, "{}", type_names.join(" / "))?;
            writeln!(f, "--------------------")?;

            // --- 4. Base Stats ---
            writeln!(f, "Base Stats:")?;
            let base_stats = &data.base_stats;
            const LABEL_WIDTH: usize = 12;

            writeln!(f, "{:<LABEL_WIDTH$} : {}", "HP", base_stats.hp)?;
            writeln!(f, "{:<LABEL_WIDTH$} : {}", "Attack", base_stats.attack)?;
            writeln!(f, "{:<LABEL_WIDTH$} : {}", "Defense", base_stats.defense)?;
            writeln!(f, "{:<LABEL_WIDTH$} : {}", "Sp. Atk", base_stats.sp_attack)?;
            writeln!(f, "{:<LABEL_WIDTH$} : {}", "Sp. Def", base_stats.sp_defense)?;
            write!(f, "{:<LABEL_WIDTH$} : {}", "Speed", base_stats.speed)
        }
        Err(_) => {
            // Fallback if data loading fails, just print the name.
            write!(f, "{}", species.name())
        }
    }
}
