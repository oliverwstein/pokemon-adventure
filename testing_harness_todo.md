# Refactor Testing Strategy

This guide outlines the steps to refactor the testing suite by creating a centralized test harness and introducing snapshot testing with `insta`.

## Part 1: Centralize Test Setup in a Helper Module

The goal is to eliminate boilerplate code from individual test files (`test_*.rs`).

### Task 1.1: Add `insta` Dependency

Add the `insta` crate as a development dependency.

- **File:** `pokemon-adventure/Cargo.toml`
- **Action:** Add `insta` to the `[dev-dependencies]` section.
- **Code:**
  ```toml
  # In pokemon-adventure/Cargo.toml

  [dev-dependencies]
  insta = { version = "1.34", features = ["yaml"] }
  # ... any other dev-dependencies
  ```

### Task 1.2: Create the Test Helper Module

- **File:** `src/battle/tests/helpers.rs`
- **Action:** Create this new file to house all common test setup logic.
- **Code:** Paste the following content into the file.

  ```rust
  // In src/battle/tests/helpers.rs

  use crate::battle::state::BattleState;
  use crate::moves::Move;
  use crate::player::BattlePlayer;
  use crate::pokemon::{get_species_data, PokemonInst};
  use crate::species::Species;

  /// A powerful builder for creating test `PokemonInst` objects.
  ///
  /// This allows for chaining methods to customize a Pokémon for a specific test scenario,
  /// reducing boilerplate in the test itself.
  pub struct PokemonBuilder {
      species: Species,
      level: u8,
      moves: Option<Vec<Move>>,
      hp: Option<u16>,
  }

  impl PokemonBuilder {
      pub fn new(species: Species) -> Self {
          Self {
              species,
              level: 50,
              moves: None,
              hp: None,
          }
      }

      pub fn level(mut self, level: u8) -> Self {
          self.level = level;
          self
      }

      pub fn moves(mut self, moves: Vec<Move>) -> Self {
          self.moves = Some(moves);
          self
      }
      
      pub fn hp(mut self, hp: u16) -> Self {
          self.hp = Some(hp);
          self
      }

      pub fn build(self) -> PokemonInst {
          let species_data = get_species_data(self.species)
              .unwrap_or_else(|| panic!("Failed to get species data for {:?}", self.species));
          
          let mut pokemon = PokemonInst::new(
              self.species,
              species_data,
              self.level,
              None, // Default IVs
              self.moves,
          );
          
          if let Some(hp) = self.hp {
              pokemon.set_hp(hp);
          }
          
          pokemon
      }
  }


  /// A high-level builder for setting up a full `BattleState`.
  ///
  /// This is the primary entry point for most tests. It simplifies the creation
  /// of a battle by taking two pre-built Pokémon.
  pub struct BattleBuilder {
      player1: PokemonInst,
      player2: PokemonInst,
      p1_conditions: Vec<crate::player::PokemonCondition>,
      p2_conditions: Vec<crate::player::PokemonCondition>,
      p1_status: Option<crate::pokemon::StatusCondition>,
      p2_status: Option<crate::pokemon::StatusCondition>,
  }

  impl BattleBuilder {
      pub fn new(player1_pokemon: PokemonInst, player2_pokemon: PokemonInst) -> Self {
          Self {
              player1: player1_pokemon,
              player2: player2_pokemon,
              p1_conditions: Vec::new(),
              p2_conditions: Vec::new(),
              p1_status: None,
              p2_status: None,
          }
      }

      pub fn p1_condition(mut self, condition: crate::player::PokemonCondition) -> Self {
          self.p1_conditions.push(condition);
          self
      }
      
      pub fn p2_condition(mut self, condition: crate::player::PokemonCondition) -> Self {
          self.p2_conditions.push(condition);
          self
      }

      pub fn p1_status(mut self, status: crate::pokemon::StatusCondition) -> Self {
          self.p1_status = Some(status);
          self
      }

      pub fn p2_status(mut self, status: crate::pokemon::StatusCondition) -> Self {
          self.p2_status = Some(status);
          self
      }

      pub fn build(self) -> BattleState {
          let mut player1_pokemon = self.player1;
          player1_pokemon.status = self.p1_status;
          
          let mut p1 = BattlePlayer::new("p1".to_string(), "Player 1".to_string(), vec![player1_pokemon]);
          for condition in self.p1_conditions {
              p1.add_condition(condition);
          }

          let mut player2_pokemon = self.player2;
          player2_pokemon.status = self.p2_status;

          let mut p2 = BattlePlayer::new("p2".to_string(), "Player 2".to_string(), vec![player2_pokemon]);
          for condition in self.p2_conditions {
              p2.add_condition(condition);
          }

          BattleState::new("test_battle".to_string(), p1, p2)
      }
  }
  ```

### Task 1.3: Link the New Helper Module

- **File:** `src/battle/tests/mod.rs`
- **Action:** Add `mod helpers;` to make the new functions available to all other test files in this directory.
- **Code:**
  ```rust
  // In src/battle/tests/mod.rs

  // This line makes the helpers module available to all other `test_*.rs` files
  mod helpers;

  // Keep all other `#[cfg(test)] mod test_...;` lines as they are for now.
  // We will refactor them in Part 2.
  #[cfg(test)]
  mod test_resolve_turn;
  // ... etc.
  ```

## Part 2: Refactor Existing Tests

Now, go through your `test_*.rs` files and replace the old setup code with the new, streamlined builders. Then, convert assertions to use snapshots.

### Task 2.1: Refactor `test_action_prevention.rs`

This is a good example to start with.

- **File:** `src/battle/tests/test_action_prevention.rs`
- **Action:** Delete the `INIT` static, `init_test_data()`, and `create_test_battle_state()` functions. Import and use the new builders. Convert one test to a snapshot test.
- **Code (showing `test_sleep_prevents_action` as an example):**

  ```rust
  // In src/battle/tests/test_action_prevention.rs

  // The old `tests` mod and setup functions are gone.
  // We are now at the top level of the file.

  use crate::battle::state::{BattleEvent, TurnRng};
  use crate::battle::turn_orchestrator::resolve_turn;
  use crate::moves::Move;
  use crate::player::PlayerAction;
  use crate::pokemon::StatusCondition;
  use crate::species::Species;
  use crate::battle::tests::helpers::{BattleBuilder, PokemonBuilder}; // NEW: Import builders

  #[test]
  fn test_sleep_prevents_action() {
      // OLD SETUP (to be deleted):
      // init_test_data();
      // let mut battle_state = create_test_battle_state(Some(StatusCondition::Sleep(2)), vec![]);

      // NEW SETUP:
      let mut battle_state = BattleBuilder::new(
          PokemonBuilder::new(Species::Pikachu).moves(vec![Move::Tackle]).build(),
          PokemonBuilder::new(Species::Charmander).moves(vec![Move::Tackle]).build(),
      )
      .p1_status(StatusCondition::Sleep(2))
      .build();

      battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
      battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
      
      let event_bus = resolve_turn(&mut battle_state, TurnRng::new_for_test(vec![128]));

      // OLD ASSERTION (to be deleted):
      // let events = bus.events();
      // assert_eq!(events.len(), 1);
      // assert!(matches!(events[0], BattleEvent::ActionFailed { ... }));
      
      // NEW SNAPSHOT ASSERTION:
      insta::assert_debug_snapshot!(event_bus.events());
  }

  // ... continue refactoring the other tests in this file ...
  ```

### Task 2.2: Run and Review Snapshots

1.  After refactoring a test, run `cargo test`.
2.  The test will fail with a message like `new snapshot saved, approve it with 'cargo insta review'`.
3.  Run `cargo insta review` in your terminal.
4.  An interactive prompt will show you the snapshot content. Press `a` to accept it.
5.  This creates a `snapshots` directory in your crate with a `.snap` file containing the approved output. Commit this file to git.

### Task 2.3: Continue Refactoring All Test Files

- **Action:** Systematically go through each file in `src/battle/tests/`.
- **For each `test_*.rs` file:**
    1.  Delete the local setup functions (`init_test_data`, `create_test_pokemon`, etc.).
    2.  Add `use crate::battle::tests::helpers::{BattleBuilder, PokemonBuilder};`.
    3.  Replace the test setup logic with the new builders.
    4.  Replace manual `assert!` chains on the `event_bus` with a single `insta::assert_debug_snapshot!(event_bus.events());`.
    5.  Run `cargo insta review` to approve the new snapshots.

### Task 2.4 (Optional but Recommended): Final Cleanup

- **File:** `src/battle/tests/mod.rs`
- **Action:** Once all test files are refactored, you can simplify this file.
- **Code:**
  ```rust
  // The new, simplified src/battle/tests/mod.rs
  
  // This helper module is available to all other test files.
  mod helpers;
  
  // The #[cfg(test)] attribute can be removed from each `mod test_*` line
  // because the parent module (`tests`) is already conditionally compiled.
  mod test_action_prevention;
  mod test_ante;
  mod test_condition_damage;
  mod test_critical_hits;
  mod test_cure_status;
  mod test_damage_effects;
  mod test_end_of_turn;
  mod test_fainting;
  mod test_haze;
  mod test_heal;
  mod test_metronome;
  mod test_mist;
  mod test_multi_attacks;
  mod test_nightmare;
  mod test_reckless;
  mod test_reflect_lightscreen;
  mod test_resolve_turn;
  mod test_rest;
  mod test_special_moves;
  mod test_status_moves;
  mod test_team_condition_expiry;
  mod test_team_condition_moves;