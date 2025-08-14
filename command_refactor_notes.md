## Analysis of `turn_orchestrator.rs`: Responsibilities & Refactoring Map

This document breaks down the major functions within `turn_orchestrator.rs`, identifies the specific game mechanics they are responsible for, and maps out how each piece of logic will be refactored into the new `Effect` trait and handler system.

### Core Data Structures and Contexts (For Refactoring)

Before we begin, let's define the target structures for our refactoring.

*   **`TurnContext`**: The global context for the turn. Holds `&mut BattleState`, `&mut EventBus`, etc.
*   **`EffectContext`**: The specific context for a move's effect. Holds the `TurnContext`, attacker/defender indices, the move being used, and damage dealt.
*   **`Effect` Trait**: The command interface. `fn apply(&self, context: &mut EffectContext) -> bool;`. A `true` return value means "I have handled everything; skip the standard damage phase."

---

### 1. `check_action_preventing_conditions`

This function is the "gatekeeper" that determines if a Pokémon can act at all.

#### Responsibilities:
*   **Status Condition Checks (Action Prevention):**
    *   **`Sleep`**: Checks if `turns_remaining > 0`. If so, prevents action. **Side Effect:** Ticks down the sleep counter.
    *   **`Freeze`**: Prevents action. **Side Effect:** Has a 25% chance to thaw the user, allowing the action to proceed.
    *   **`Paralysis`**: Has a 25% chance to prevent the action.
*   **Active `PokemonCondition` Checks (Action Prevention):**
    *   **`Flinched`**: Prevents the action.
    *   **`Exhausted`**: Prevents the action (e.g., the turn after `Hyper Beam`).
    *   **`Confused`**: Has a 50% chance to prevent the action. **Side Effect:** If the action is prevented, it queues up a self-attack (`HittingItself`).
    *   **`Disabled`**: Checks if the specific `move_used` matches the disabled move. If so, prevents the action.
*   **Move-Specific Pre-condition Checks:**
    *   **`Nightmare` (`Dream Eater`)**: Checks if the *target* is asleep. If not, prevents the action.

#### Refactoring Map:
This function's logic is distinct from move *effects* and should remain separate. However, it can still be refactored for better cohesion using a similar pattern.

1.  **Create a `Condition` Trait:**
    ```rust
    trait ConditionCheck {
        // Returns Some(reason) if the action should be blocked.
        fn check_before_action(&self, ctx: &mut TurnContext, player_idx: usize) -> Option<ActionFailureReason>;
    }
    ```
2.  **Implement `ConditionCheck` for `StatusCondition` and `PokemonCondition`:**
    *   `impl ConditionCheck for StatusCondition`: The `match` statement for `Sleep`, `Freeze`, `Paralysis` goes here.
    *   `impl ConditionCheck for PokemonCondition`: The `match` statement for `Flinched`, `Confused`, `Exhausted`, `Disabled` goes here.
3.  **Refactor the Main Function:** The body of `check_action_preventing_conditions` becomes a clean loop that iterates through the Pokémon's active status and conditions, calling `check_before_action` on each.
4.  The `Nightmare` check is an edge case. It's a property of the *move*, not the *user's status*. This check should be moved to the `execute_attack_hit` function as a "pre-execution validation" step before any other logic.

---

### 2. `apply_move_effects`

This is the largest and most complex function, handling chance-based effects that occur when a move hits.

#### Responsibilities:
*   **Substitute Check:** The very first thing it does is check if the defender has a `Substitute`. If so, it has a separate, smaller logic path for user-only effects and then returns early.
*   **Chance-based Status Application:**
    *   Handles `Burn`, `Paralyze`, `Freeze`, `Poison`, `Sedate` (Sleep). It rolls RNG against a `chance` value and applies the `StatusCondition` to the defender.
*   **Chance-based `PokemonCondition` Application:**
    *   Handles `Flinch`, `Confuse`, `Exhaust` (for the user), `Trap`, `Seed`. It rolls RNG and applies the appropriate `PokemonCondition`.
*   **Stat Changes:**
    *   Handles `StatChange` and `RaiseAllStats`. It checks the target (`User` or `Target`), rolls RNG, modifies the stat stage, and checks for `Mist` protection on the defender.
*   **Healing:**
    *   Handles `Heal` for the user.
*   **Field/Team Effects:**
    *   Handles `Haze`, `CureStatus`, `Reflect`, `LightScreen`, `Mist`.
*   **Economic Effects:**
    *   Handles `Ante` (`Pay Day`).

#### Refactoring Map:
This entire function will be replaced by a call to `dispatch_effect` (the `impl Effect for MoveEffect` block).

*   **Substitute Check**: This logic moves out of the effect system and into `execute_attack_hit`. It will happen *before* calling `apply_move_effects`.
*   **`Burn(chance)`**: Becomes `fn handle_burn(chance, ctx)`.
*   **`Paralyze(chance)`**: Becomes `fn handle_paralyze(chance, ctx)`.
*   ...and so on for every single `match` arm. Each one gets its own dedicated handler function in `effects.rs`.
*   **`StatChange(...)`**: The logic for checking `Mist` will live inside `handle_stat_change`. This co-locates the action (lowering a stat) with its counter-play (checking for Mist).

---

### 3. `apply_on_damage_effects`

This function handles effects that are directly proportional to the damage dealt by a move.

#### Responsibilities:
*   **`Recoil(percentage)`**: Calculates recoil damage based on `damage_dealt` and applies it to the attacker. Handles the attacker fainting from recoil.
*   **`Drain(percentage)`**: Calculates HP to drain based on `damage_dealt` and heals the attacker.

#### Refactoring Map:
*   This function is **deleted**.
*   The logic is moved into `handle_recoil` and `handle_drain` functions within `effects.rs`.
*   These handlers will receive the `damage_dealt` via the `EffectContext` and will start with a check: `if context.damage_dealt == 0 { return; }`.

---

### 4. `perform_special_move`

This function handles moves that have unique, turn-altering mechanics that often skip the standard damage phase.

#### Responsibilities:
*   **Two-Turn Moves:**
    *   `InAir` (`Fly`), `ChargeUp` (`SolarBeam`), `Underground` (`Dig`): On the first turn, it applies the condition and returns `true` (skip damage). On the second turn, it removes the condition and returns `false` (proceed to damage).
*   **State-Copying Moves:**
    *   `Transform`: Applies the `Transformed` condition to the user.
    *   `Conversion`: Applies the `Converted` condition to the user.
*   **Complex User Conditions:**
    *   `Substitute`: Creates a `Substitute` condition with HP based on the user's max HP.
    *   `Rest`: Heals the user to full, clears all active conditions, and applies a `Sleep` status.
*   **Retaliation Moves:**
    *   `Counter`: Applies the `Countering` condition for the turn.
    *   `Bide`: Manages the `Biding` state machine: starts the bide, waits for turns, and finally unleashes the stored damage.
*   **Multi-Turn Forced Moves:**
    *   `Rampage`: Applies the `Rampaging` condition, forcing the move for several turns.
    *   `Rage`: Applies the `Enraged` condition.
*   **Self-Fainting Moves:**
    *   `Explode`: Faints the user *before* damage calculation.
*   **Copycat/Random Moves:**
    *   `MirrorMove`: Retrieves the opponent's last move and queues it up for immediate execution.
    *   `Metronome`: Selects a random move and queues it up for immediate execution.

#### Refactoring Map:
*   This function is **deleted**.
*   Each `match` arm becomes a handler function in `effects.rs`.
*   The `Effect::apply` trait method will return `bool` to signal whether to skip the damage phase.
    *   `handle_fly` will return `true` on turn 1, `false` on turn 2.
    *   `handle_transform` will return `true`.
    *   `handle_explode` will return `false` (it faints the user, but the damage phase still needs to run).
    *   `handle_metronome` will recursively call `dispatch_effect` for the new move's effects and return `true`.

### Summary of the Transformation

| Old Function in `turn_orchestrator` | New Location / Refactoring                                                                                        |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `check_action_preventing_conditions`| Refactored to use a `ConditionCheck` trait. Logic for each condition moves to its own `impl`.                       |
| `apply_move_effects`                | **Deleted.** Replaced by a loop over `move.effects` that calls `effect.apply()`.                                    |
| `apply_on_damage_effects`           | **Deleted.** Its logic is merged into `handle_recoil` and `handle_drain` within the `Effect` system.                |
| `perform_special_move`              | **Deleted.** Its logic is merged into various `handle_*` functions within the `Effect` system.                      |
| `execute_attack_hit`                | Becomes the central, streamlined function that calls out to the new, specialized systems for each step of an attack. |

By executing this plan, you will deconstruct the monolithic orchestrator into a set of small, independent, and highly cohesive "command" objects. The orchestrator will be reduced to a simple conductor, telling each command *when* to execute, but not caring *how* it executes. This is the essence of good, maintainable design.