This guide outlines the steps to refactor the project to load all Pokémon and Move data at compile time using a `build.rs` script and `phf`.

## Part 1: Setup and Build Script Creation

### Task 1.1: Update `Cargo.toml` with Build Dependencies

Add the `[build-dependencies]` section to `pokemon-adventure/Cargo.toml`. This is separate from the existing `[dependencies]` section.

```toml
# In pokemon-adventure/Cargo.toml

[build-dependencies]
phf = { version = "0.11", features = ["macros"] }
phf_codegen = "0.11"
ron = "0.8"
serde = { version = "1.0", features = ["derive"] }
```

### Task 1.2: Add Required `derive` Traits to Enums

The `phf` crate requires specific traits to be derived for the keys of its maps.

- **File:** `src/moves.rs`
- **Action:** Add `use phf::macros::{FmtConst, PhfHash};` at the top. Modify the `derive` attribute for the `Move` enum.
- **Code:**
  ```rust
  use phf::macros::{FmtConst, PhfHash};
  use serde::{Deserialize, Serialize};

  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PhfHash, FmtConst)]
  pub enum Move {
      // ... enum variants remain the same
  }
  ```

- **File:** `src/species.rs`
- **Action:** Add `use phf::macros::{FmtConst, PhfHash};` at the top. Modify the `derive` attribute for the `Species` enum.
- **Code:**
  ```rust
  use phf::macros::{FmtConst, PhfHash};
  use serde::{Deserialize, Serialize};

  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PhfHash, FmtConst)]
  pub enum Species {
      // ... enum variants remain the same
  }
  ```

### Task 1.3: Temporarily Clean `use` Statements for the Build Script

The build script will `include!` several files. To prevent import conflicts during the build process, remove the `use` statements from the files it will include.

- **File:** `src/pokemon.rs`
- **Action:** Delete all `use` statements from the top of the file.

- **File:** `src/move_data.rs`
- **Action:** Delete all `use` statements from the top of the file.

### Task 1.4: Create and Populate `build.rs`

- **File:** `build.rs` (in the root of the `pokemon-adventure` crate)
- **Action:** Create this new file and paste the exact contents below into it.

```rust
use phf_codegen::Map;
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

// Include struct/enum definitions directly from the `src` directory.
// The order is important: dependencies must come before dependents.
include!("src/moves.rs");
include!("src/species.rs");
include!("src/move_data.rs");
include!("src/pokemon.rs");

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=data/moves/");
    println!("cargo:rerun-if-changed=data/pokemon/");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let data_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("data");

    process_moves(&data_dir, &out_dir);
    process_species(&data_dir, &out_dir);
}

/// Reads all move .ron files and generates a static phf::Map.
fn process_moves(data_dir: &PathBuf, out_dir: &PathBuf) {
    let moves_path = data_dir.join("moves");
    let generated_path = out_dir.join("generated_moves.rs");
    let mut file = BufWriter::new(File::create(generated_path).unwrap());

    let mut move_map = Map::new();

    for entry in fs::read_dir(moves_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |s| s == "ron") {
            let filename = path.file_stem().unwrap().to_str().unwrap();
            let move_enum: Move = filename.parse().expect(&format!("Failed to parse move: {}", filename));
            let content = fs::read_to_string(&path).unwrap();
            let move_data: MoveData = ron::from_str(&content).expect(&format!("Failed to parse RON for {}", filename));
            move_map.entry(move_enum, &format!("{:#?}", move_data));
        }
    }

    // Manually add data for Struggle and HittingItself
    let struggle_data = MoveData {
        name: "Struggle".to_string(),
        move_type: PokemonType::Typeless,
        power: Some(50),
        category: MoveCategory::Physical,
        accuracy: Some(90),
        max_pp: 0,
        effects: vec![MoveEffect::Recoil(25)],
    };
    let hitting_itself_data = MoveData {
        name: "Hit Itself".to_string(),
        move_type: PokemonType::Typeless,
        power: Some(40),
        category: MoveCategory::Physical,
        accuracy: None,
        max_pp: 0,
        effects: vec![],
    };

    move_map.entry(Move::Struggle, &format!("{:#?}", struggle_data));
    move_map.entry(Move::HittingItself, &format!("{:#?}", hitting_itself_data));

    writeln!(
        &mut file,
        "static MOVE_DATA: phf::Map<Move, MoveData> = {};",
        move_map.build()
    ).unwrap();
}

/// Reads all Pokémon .ron files and generates a static phf::Map.
fn process_species(data_dir: &PathBuf, out_dir: &PathBuf) {
    let species_path = data_dir.join("pokemon");
    let generated_path = out_dir.join("generated_species.rs");
    let mut file = BufWriter::new(File::create(generated_path).unwrap());

    let mut species_map = Map::new();

    for entry in fs::read_dir(species_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |s| s == "ron") {
            let filename = path.file_stem().unwrap().to_str().unwrap();
            let species_name_part = filename.splitn(2, '-').nth(1).unwrap();
            let species_enum: Species = species_name_part.parse().expect(&format!("Failed to parse species: {}", species_name_part));
            let content = fs::read_to_string(&path).unwrap();
            let species_data: PokemonSpecies = ron::from_str(&content).expect(&format!("Failed to parse RON for {}", filename));
            species_map.entry(species_enum, &format!("{:#?}", species_data));
        }
    }

    writeln!(
        &mut file,
        "static SPECIES_DATA: phf::Map<Species, PokemonSpecies> = {};",
        species_map.build()
    ).unwrap();
}
```

---

## Part 2: Refactor Runtime Code to Use Static Data

Now, modify the application source code to use the data generated by `build.rs`.

### Task 2.1: Update `src/move_data.rs`

- **Action:** Replace the old file-loading logic with the new static map.
- **Code:**
  ```rust
  // In src/move_data.rs

  use crate::moves::Move;
  use crate::pokemon::PokemonType;
  use serde::{Deserialize, Serialize};
  use std::str::FromStr;
  
  // NOTE: The structs and enums like MoveData, MoveCategory, etc., must remain here.
  // The 'use' statements for phf/HashMap/fs/etc. are no longer needed here.

  // --- START OF NEW CODE ---
  include!(concat!(env!("OUT_DIR"), "/generated_moves.rs"));

  /// Get move data for a specific move from the global static map.
  /// The returned data has a 'static lifetime.
  pub fn get_move_data(move_: Move) -> Option<&'static MoveData> {
      MOVE_DATA.get(&move_)
  }

  /// Get max PP for a specific move
  pub fn get_move_max_pp(move_: Move) -> u8 {
      get_move_data(move_).map(|data| data.max_pp).unwrap_or(30)
  }
  // --- END OF NEW CODE ---
  
  // The FromStr implementation for Move should be kept as it is used by the build script.
  // ... rest of the file ...
  ```
  **Important:** Delete the old `LazyLock` static variable and the `initialize_move_data` function.

### Task 2.2: Update `src/pokemon.rs`

- **Action:** Replace the file-loading logic with the new static map.
- **Code:**
  ```rust
  // In src/pokemon.rs

  // Add back necessary use statements for functions within this file.
  use crate::move_data::get_move_max_pp;
  use crate::moves::Move;
  use crate::species::Species;
  use serde::{Deserialize, Serialize};

  // NOTE: The structs and enums like PokemonSpecies, PokemonInst, etc., must remain here.
  // The 'use' statements for phf/HashMap/fs/etc. are no longer needed here.

  // --- START OF NEW CODE ---
  include!(concat!(env!("OUT_DIR"), "/generated_species.rs"));

  /// Get species data for a specific species from the global static map.
  /// The returned data has a 'static lifetime.
  pub fn get_species_data(species: Species) -> Option<&'static PokemonSpecies> {
      SPECIES_DATA.get(&species)
  }
  // --- END OF NEW CODE ---
  
  // ... rest of the file (impl PokemonInst, etc.) ...
  ```
  **Important:** Delete the old `LazyLock` static variable, the `initialize_species_data` function, and all related `PokemonSpecies::load_*` methods.

### Task 2.3: Update `src/main.rs`

- **Action:** Remove the calls to the now-deleted initialization functions.
- **Code:**
  ```rust
  // In src/main.rs
  
  // ...
  use pokemon::{get_species_data, PokemonInst};
  use species::Species;

  fn main() {
      // The two initialization blocks are now GONE. The data is already loaded.
      // All the example code that follows should work without any changes.
      
      if let Some(pikachu) = get_species_data(Species::Pikachu) {
          // ...
      }
      // ...
  }