### **Design Document: Pokemon Arena Turn Resolution & Event System**

**Version:** 1.0
**Author:** [Oliver Stein]
**Status:** Proposed

#### **1. Overview & Core Philosophy**

This document outlines the architecture for the turn resolution and event handling system within the PokéBot Arena engine. The primary goal is to create a system that is **deterministic, testable, robust, and communicative**.

The core philosophy is to treat the game engine as a **Transactional State Machine with a high-fidelity Event Log**.

*   **State Machine:** The `BattleState` struct is the single, authoritative representation of a battle at any given moment.
*   **Transactional:** The `resolve_turn` function is a discrete, atomic operation that transitions the `BattleState` from the beginning of a turn to the beginning of the next.
*   **Event Log:** The engine produces a structured, immutable log of `BattleEvent`s for every decision and state mutation that occurs during a turn. This log is the primary output used to communicate the turn's results.

#### **2. The Challenge: Managing Determinism and Communication**

A Pokémon battle contains stochastic elements (randomness) such as accuracy checks, critical hits, and secondary effect chances. A naive implementation would make the engine non-deterministic, rendering it nearly impossible to test reliably. Furthermore, the system must communicate the complex sequence of events within a turn to external observers (the API clients) in a clear and structured way.

Our design directly addresses these two challenges through two key architectural patterns: **RNG Abstraction** and an **Event-Driven Internal Architecture**.

#### **3. System Components & Design Rationale**

**3.1. The `TurnRng` Oracle: Decoupling Randomness from Logic**

*   **Component:** A `TurnRng` struct.
*   **Design:**
    *   Instead of calling a random number generator directly within the game logic, the `resolve_turn` function will be *given* a pre-populated `TurnRng` instance as an input.
    *   This "Oracle" will contain queues of all random numbers needed for the turn (e.g., `outcomes: Vec<u8>` for 1-100 rolls), which can be consumed as needed.
    *   The engine consumes numbers from the oracle sequentially for every stochastic check (accuracy, effect chance, damage variance).
*   **Rationale (The "Why"):**
    *   **To Achieve Determinism for Testing:** This is the most critical reason. For a unit test, we can instantiate `TurnRng` with a fixed, predictable sequence of numbers (`TurnRng::new_for_test(vec![90, 15, ...])`). This forces specific outcomes (a move hits, its effect triggers), allowing us to assert the exact resulting `BattleState`. This makes the engine 100% deterministic in a test environment.
    *   **To Enable Probabilistic Analysis:** We can run the engine thousands of times with a randomly generated `TurnRng` each time, then analyze the output to verify that effects like critical hits occur at the correct statistical rate.
    *   **To Isolate Side Effects:** Random number generation is a side effect. By isolating it at the entry point to the engine, the core game logic inside `resolve_turn` becomes a pure, predictable function of its inputs (State, Actions, RNG).

**3.2. The `EventBus` and `BattleEvent` Enum: Decoupling Logic from Communication**

*   **Component:** An `EventBus` struct and a comprehensive `BattleEvent` enum.
*   **Design:**
    *   The `resolve_turn` function will take a mutable reference to an `EventBus`.
    *   The engine's primary output is not the mutated state itself, but a series of structured `BattleEvent`s pushed onto the bus.
    *   A strict coding discipline will be followed: **every logical decision and state mutation within the engine MUST be preceded by pushing a corresponding event to the bus.**
*   **Rationale (The "Why"):**
    *   **To Separate Logic from Presentation:** The engine's job is to simulate the battle and report *what happened* in a structured, machine-readable format (`BattleEvent`). The API handler's job is to take that structured report and *translate it* into a human-readable format (`Vec<String>`). This separation is clean and robust.
    *   **To Provide a Complete Audit Trail:** The event stream is the definitive, immutable record of a turn. This is invaluable for debugging, creating replays, and understanding complex interactions.
    *   **To Include Informational Events:** As identified, not all events mutate the state. The `EventBus` allows us to capture informational events (`MoveMissed`, `ActionFailed::IsAsleep`) alongside state-changing events (`DamageDealt`, `StatusApplied`). This creates a complete narrative of the turn, explaining not only what happened but *why* it happened.

**3.3. Direct State Mutation (vs. Event Sourcing)**

*   **Component:** The `resolve_turn` function.
*   **Design:** The function will mutate the `BattleState` struct directly *after* pushing the corresponding event to the `EventBus`. We are deliberately **not** choosing a full Event Sourcing pattern where the state is mutated by re-applying the generated events.
*   **Rationale (The "Why"):**
    *   **To Prioritize Development Velocity:** A full Event Sourcing model requires separate logic for event generation and event application, significantly increasing code complexity and initial development time. The direct mutation model is simpler and faster to implement.
    *   **To Gain "Good Enough" Benefits:** By strictly tying every mutation to a preceding event, we gain the most critical benefits of Event Sourcing (a high-fidelity audit log for testing and communication) without incurring its full complexity.
    *   **To Maintain Performance:** This model avoids the overhead of replaying events to reconstruct state. The `BattleState` stored in the database is always a ready-to-use snapshot, ensuring fast load times for the Lambda function.

#### **4. The Turn Resolution Flow**

The `resolve_turn` function will execute the following sequence:

1.  **Initialization:**
    *   Set `game_state` to `TurnInProgress`.
    *   Clear the previous `turn_log`.
    *   Create and initialize the `EventBus`.

2.  **Action Prioritization:**
    *   Examine the `PlayerAction`s in the `action_queue`.
    *   First execute any Switch actions, then any Item actions, then Move actions.
    *   To handle move sequencing, first determine the priority of each move.
    *   If the moves have the same priority (which will be most of the time), 
            determine the effective speed of each `PokemonInst` and order them accordingly.
    *   Determine the final action order (e.g., `[player_1_index, player_2_index]`).

3.  **Main Action Loop (Iterate through ordered players):**
    *   If `SwitchPokemon`: ... (emit events and mutate state).
    *   If it is an attack:
        a.  **Pre-Action Check:** Check for non-volatile statuses (sleep, paralysis, freeze, fainted) that would prevent action. If the action is prevented, `bus.push(BattleEvent::ActionFailed)` and proceed to the next player.
        b.  **Volatile Status Check:** Check for volatile statuses (confusion, flinch). If the action is prevented, `bus.push(BattleEvent::ActionFailed)` and proceed. Note that the ActionFailed event should record why.
        c.  **Valid Target Check:** If the move is not a status move (that is, if it does not only affect the user), check if the target has fainted. If it has, cancel the attack and `bus.push(BattleEvent::ActionFailed)` 
        d.  **Execute Action:**
            *   If `UseMove`:
                i.  `bus.push(BattleEvent::MoveUsed)`
                ii.  Perform accuracy check using the `TurnRng` oracle. If it fails, `bus.push(BattleEvent::MoveMissed)` and the action ends.
                iii. Calculate damage, critical hits, and type effectiveness using the `TurnRng` oracle.
                iv. `bus.push(BattleEvent::DamageDealt)`.
                v.  **Mutate** the target's HP in the `BattleState`.
                vi. Iterate through the move's `effects` list. For each effect:
                    - Check if the effect can apply (e.g., can't poison a Poison-type).
                    - Use the `TurnRng` oracle to check the probability.
                    - If it succeeds, `bus.push(BattleEvent::EffectApplied)` and **mutate** the target's state.
                    - If it fails, `bus.push(BattleEvent::EffectFailed)`.
    *   **Post-Action Check:** Check if the target Pokémon has fainted. If so, `bus.push(BattleEvent::PokemonFainted)`. 

4.  **End-of-Turn Phase:**
    *   Iterate through both active Pokémon.
    *   Apply damage from status effects like Poison and Burn, pushing `DamageDealt` events for each.
    *   Handle passive recovery or other end-of-turn abilities.
    *   Decrement timers on volatile and field conditions.

5.  **Cleanup & Finalization:**
    *   Check for win/loss conditions (if an entire team is fainted). Set `game_state` accordingly.
    *   If the battle is ongoing, set `game_state` to `WaitingForBothActions`.
    *   Clear the `action_queue`.
    *   Increment the `turn_number`.
    *   The function concludes, leaving the `BattleState` ready for the next turn and having filled the `EventBus` with a complete record of the turn's events.