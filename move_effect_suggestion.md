This is the most impactful refactoring for the project's long-term health. Moving from a monolithic, direct-mutation style to a decoupled, intent-based pattern is a significant step up in software architecture.

Here is a robust, step-by-step guide in the format you requested. It breaks the process into manageable parts, starting with setting up the new architecture and then systematically migrating the logic, piece by piece.

---

# `TODO.md`: Refactor `turn_orchestrator` with the `EffectOutcome` Pattern

This guide details the process of refactoring `turn_orchestrator.rs`. The goal is to move from a system of direct, complex mutations to a cleaner, more maintainable pattern where move effects generate a list of "intents" (`EffectOutcome`s) that are then executed by a central applicator. This will improve code clarity, reduce bugs, simplify testing, and prepare the codebase for future concurrency.

### Guiding Principles

1.  **Separation of Concerns:** A `MoveEffect` should *describe* what it does, not *how* it does it.
2.  **Centralized Mutation:** All direct changes to `BattleState` should happen in one predictable place (the "applicator"), not scattered across dozens of `match` arms.
3.  **Clarity over Brevity:** The new system will have more lines of code initially (due to new structs/enums), but the logic in any given place will be far simpler and easier to understand.

---

## Part 1: Laying the Foundation

First, we will create the new files and data structures. The project will still compile and run after this part, but the new code won't be used yet.

### Task 1.1: Create `src/battle/effects.rs` and Define `EffectOutcome`

This new enum is the heart of the refactor. It represents every possible atomic change a move effect can request.

- **Action:** Create a new file at `src/battle/effects.rs`.
- **Code:** Paste the following content into the new file.

  ```rust
  // In src/battle/effects.rs

  use crate::moves::Move;
  use crate::player::{PokemonCondition, TeamCondition, StatType as PlayerStatType};
  use crate::move_data::{RampageEndCondition, StatusType, Target};
  use crate::pokemon::{PokemonType, StatusCondition};

  #[derive(Debug, Clone, PartialEq)]
  pub enum EffectOutcome {
      // Status & Condition Application
      ApplyStatus {
          target_index: usize,
          status: StatusCondition,
          chance: u8,
      },
      ApplyCondition {
          target_index: usize,
          condition: PokemonCondition,
          chance: u8,
      },

      // Status & Condition Removal
      CureStatus {
          target_index: usize,
          status_type: StatusType,
      },
      ClearAllUserConditions,
      ClearAllStatStages,

      // Stat Changes
      ChangeStatStage {
          target_index: usize,
          stat: PlayerStatType,
          stages: i8,
          chance: u8,
      },
      RaiseAllUserStats {
          chance: u8,
      },

      // HP & Damage
      HealUser { percentage: u8 },
      Recoil { percentage: u8 },
      Drain { percentage: u8 },

      // Team & Field
      SetTeamCondition {
          target_index: usize,
          condition: TeamCondition,
          turns: u8,
      },

      // Economic
      AddAnte { chance: u8 },

      // Special Move Mechanics (these don't have direct events but trigger logic)
      SkipTurn, // For ChargeUp, Fly, etc. on the first turn
      ExecuteSpecial, // A tag to run special logic like Transform, Bide, Metronome
  }
  ```

### Task 1.2: Integrate the `effects` Module

- **File:** `src/battle/mod.rs`
- **Action:** Add `pub mod effects;` to make the new module part of the `battle` crate.
- **Code:**
  ```rust
  // In src/battle/mod.rs

  pub mod effects;
  // ... rest of the file
  ```

---

## Part 2: The Core Migration - From Logic to Data

This is the main part of the work. We will create the "translator" and the "executor".

### Task 2.1: Create the `MoveEffect::to_outcomes` Translator

This function will be the "pure" part of our system. It reads a `MoveEffect` and returns a list of things that *should* happen, without actually doing them.

- **File:** `src/move_data.rs`
- **Action:** Add the new `use` statement and the `impl MoveEffect` block.
- **Code:**
  ```rust
  // In src/move_data.rs, after the MoveEffect enum definition

  use crate::battle::effects::EffectOutcome;
  use crate::player::{PokemonCondition, StatType as PlayerStatType, TeamCondition};
  use crate::pokemon::StatusCondition;

  impl MoveEffect {
      /// Describes the potential outcomes of this effect.
      pub fn to_outcomes(&self, attacker_index: usize, defender_index: usize) -> Vec<EffectOutcome> {
          let mut outcomes = Vec::new();
          let user = Target::User;
          let target = Target::Target;

          match self {
              // --- Direct translations from old `apply_move_effects` ---
              MoveEffect::Burn(chance) => outcomes.push(EffectOutcome::ApplyStatus { target_index: defender_index, status: StatusCondition::Burn, chance: *chance }),
              MoveEffect::Freeze(chance) => outcomes.push(EffectOutcome::ApplyStatus { target_index: defender_index, status: StatusCondition::Freeze, chance: *chance }),
              MoveEffect::Paralyze(chance) => outcomes.push(EffectOutcome::ApplyStatus { target_index: defender_index, status: StatusCondition::Paralysis, chance: *chance }),
              MoveEffect::Poison(chance) => outcomes.push(EffectOutcome::ApplyStatus { target_index: defender_index, status: StatusCondition::Poison(0), chance: *chance }),
              MoveEffect::Sedate(chance) => {
                  // Sleep needs a turn count, but we can model that in the applicator.
                  // For now, the outcome is simple.
                  outcomes.push(EffectOutcome::ApplyStatus { target_index: defender_index, status: StatusCondition::Sleep(0), chance: *chance });
              },
              MoveEffect::Flinch(chance) => outcomes.push(EffectOutcome::ApplyCondition { target_index: defender_index, condition: PokemonCondition::Flinched, chance: *chance }),
              MoveEffect::Confuse(chance) => outcomes.push(EffectOutcome::ApplyCondition { target_index: defender_index, condition: PokemonCondition::Confused { turns_remaining: 0 }, chance: *chance }),
              MoveEffect::Trap(chance) => outcomes.push(EffectOutcome::ApplyCondition { target_index: defender_index, condition: PokemonCondition::Trapped { turns_remaining: 0 }, chance: *chance }),
              MoveEffect::Exhaust(chance) => outcomes.push(EffectOutcome::ApplyCondition { target_index: attacker_index, condition: PokemonCondition::Exhausted { turns_remaining: 2 }, chance: *chance }),
              MoveEffect::StatChange(t, stat, stages, chance) => {
                  let target_index = if *t == user { attacker_index } else { defender_index };
                  // Conversion from data stat type to player stat type
                  let player_stat = /* ... convert MoveStatType to PlayerStatType ... */;
                  outcomes.push(EffectOutcome::ChangeStatStage { target_index, stat: player_stat, stages: *stages, chance: *chance });
              },
              MoveEffect::RaiseAllStats(chance) => outcomes.push(EffectOutcome::RaiseAllUserStats { chance: *chance }),
              MoveEffect::Heal(percentage) => outcomes.push(EffectOutcome::HealUser { percentage: *percentage }),
              MoveEffect::Haze(chance) => outcomes.push(EffectOutcome::ClearAllStatStages { chance: *chance }),
              MoveEffect::CureStatus(t, status_type) => {
                  let target_index = if *t == user { attacker_index } else { defender_index };
                  outcomes.push(EffectOutcome::CureStatus { target_index, status_type: *status_type });
              },
              MoveEffect::Reflect(rtype) => {
                  let condition = if *rtype == crate::move_data::ReflectType::Physical { TeamCondition::Reflect } else { TeamCondition::LightScreen };
                  outcomes.push(EffectOutcome::SetTeamCondition { target_index: attacker_index, condition, turns: 5 });
              },
              MoveEffect::Mist => outcomes.push(EffectOutcome::SetTeamCondition { target_index: attacker_index, condition: TeamCondition::Mist, turns: 5 }),
              MoveEffect::Ante(chance) => outcomes.push(EffectOutcome::AddAnte { chance: *chance }),
              
              // --- Translations from old `apply_on_damage_effects` ---
              MoveEffect::Recoil(percentage) => outcomes.push(EffectOutcome::Recoil { percentage: *percentage }),
              MoveEffect::Drain(percentage) => outcomes.push(EffectOutcome::Drain { percentage: *percentage }),

              // --- Translations from old `perform_special_move` ---
              MoveEffect::ChargeUp | MoveEffect::InAir | MoveEffect::Underground => outcomes.push(EffectOutcome::SkipTurn),
              MoveEffect::Rest(_) => {
                  // Rest is a great example of a composite effect.
                  outcomes.push(EffectOutcome::HealUser { percentage: 100 });
                  outcomes.push(EffectOutcome::ClearAllUserConditions);
                  outcomes.push(EffectOutcome::ApplyStatus { target_index: attacker_index, status: StatusCondition::Sleep(2), chance: 100 });
              },
              MoveEffect::Transform | MoveEffect::Conversion | MoveEffect::Substitute | MoveEffect::Counter | MoveEffect::Bide(_) | MoveEffect::MirrorMove | MoveEffect::Metronome => {
                  outcomes.push(EffectOutcome::ExecuteSpecial);
              },

              // These are handled by their own outcomes or are passive.
              _ => {}
          }

          outcomes
      }
  }
  ```

### Task 2.2: Create the `apply_outcomes` Executor

- **File:** `src/battle/effects.rs`
- **Action:** Add the executor function. This is the new home for all direct state mutation and RNG calls related to effects.
- **Code:** Add this function to `effects.rs`. You will need to move the logic from the old `match` arms into the corresponding arms here.

  ```rust
  // In src/battle/effects.rs

  pub fn apply_outcomes(
      outcomes: &[EffectOutcome],
      damage_dealt: u16, // Pass this in for Recoil/Drain
      attacker_index: usize,
      defender_index: usize,
      battle_state: &mut BattleState,
      bus: &mut EventBus,
      rng: &mut TurnRng,
  ) {
      for outcome in outcomes {
          match outcome {
              EffectOutcome::ApplyStatus { target_index, status, chance } => {
                  if rng.next_outcome() <= *chance {
                      // Logic to apply status, push event
                  }
              },
              EffectOutcome::Recoil { percentage } => {
                  if damage_dealt > 0 {
                      // Logic to calculate recoil, apply damage, handle fainting, push events
                  }
              },
              // ... one match arm for each outcome variant, containing the mutation logic.
          }
      }
  }
  ```

## Part 3: Integrating the New System

Finally, we'll replace the old, complex function calls in the main orchestrator with our new, clean pipeline.

### Task 3.1: Rewrite `execute_attack_hit`

- **File:** `src/battle/turn_orchestrator.rs`
- **Action:** This is the final and most critical step. Replace the body of `execute_attack_hit` with the new logic that uses our translator and executor.

- **Code:**
  ```rust
  // In src/battle/turn_orchestrator.rs

  // Make sure to add this at the top of the file:
  use crate::battle::effects::{apply_outcomes, EffectOutcome};

  pub fn execute_attack_hit( /* ... parameters ... */ ) {
      // ... (keep initial setup like getting attacker/defender, MoveUsed event) ...

      // --- NEW LOGIC ---

      let move_data = get_move_data(move_used).expect("Move data must exist");

      // 1. Generate all potential outcomes from the move's effects.
      let outcomes = move_data.effects.iter()
          .flat_map(|effect| effect.to_outcomes(attacker_index, defender_index))
          .collect::<Vec<_>>();

      // 2. Handle special, turn-altering effects first.
      if outcomes.contains(&EffectOutcome::SkipTurn) {
          // Handle logic for ChargeUp, Fly, etc. applying the condition
          // ...
          return; // This action is done for this turn.
      }
      if outcomes.contains(&EffectOutcome::ExecuteSpecial) {
          // Handle logic for Transform, Rest, Bide, Metronome, etc.
          // This will be a new match statement, but it only contains the truly unique moves.
          // ...
          return; // This action is fully handled here.
      }
      
      // 3. Proceed with a standard attack if not handled above.
      if !move_hits(...) {
          // ... (handle miss logic, including Reckless) ...
          return;
      }
      
      // 4. Calculate and apply damage.
      let damage_dealt = calculate_damage(...);
      if damage_dealt > 0 {
          // ... (apply damage, check for faint, handle Counter/Bide/Enraged triggers) ...
      }

      // 5. Apply all standard outcomes now that damage is known.
      apply_outcomes(
          &outcomes,
          damage_dealt,
          attacker_index,
          defender_index,
          battle_state,
          bus,
          rng,
      );
      
      // ... (handle multi-hit logic) ...
  }
  ```

### Task 3.2: Clean Up and Verify

- **Action:** Delete the old, now unused functions: `apply_move_effects`, `apply_on_damage_effects`, and `perform_special_move`.
- **Action:** Run `cargo build` and fix any compiler errors.
- **Action:** Run `cargo test`. Acknowledge that tests will fail. This is your guide to ensure the new logic is correct.
- **Action:** Go through each failing test. Run `cargo insta review` to see the difference between the old event sequence and the new one. Fix your new implementation in `effects.rs` until the snapshot matches the original, correct behavior. Accept the new snapshot once it's correct.

This detailed plan breaks the refactoring into manageable chunks. The key is to systematically move logic, one piece at a time, from the old monolith into the new, focused handler functions, using the `EffectOutcome` enum as the clean interface between them.