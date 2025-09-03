# Pokemon Battle Finite State Machine Notes

## Introduction 

Pokemon-Adventure contains a largely-complete Rust implementation of a Pokemon battle engine, which handles battles well. There are a few edge cases that aren't properly implemented, but they can be fixed. However, I am also seeking to expand the engine so that it handles things like levelling up, evolution, catching pokemon, etc., to be able to support a story mode. It is currently built around a resolve_turn() function. CLAUDE.md describes it. In the process of writing the repo, I realized that I could improve the design by making it a more robust state machine by leveraging a GameState enum, a CommandStack, and a few other things. 

It works quite well for handling handling tournament battles in a vacuum, but the system design is not readily extensible for a single-player story mode as the current system design which based around the assumption that, on each turn, two players each input one PlayerAction and then the whole turn can be executed. This structure is frustrating because there are, of course, a number of edge cases requiring secondary user inputs. The most obvious of these is that, when a pokemon faints, the owner must send out a new pokemon if they can, or else the battle ends. I think a proper Finite State Machine would work better, because we would fundamentally have two Game States: Awaiting Input and Advancing. The problem is mostly one of expanding the engine to handle story considerations like capturing, leveling up, learning moves, and evolving. 

So it behooves us to design the Pokemon Finite State Machine (PFSM), in which we are either Advancing or Awaiting Input. 
This is a pure calculation engine, built with the assumption that any GUI or CLI would be an observer that relies on a calculation engine that can run independently of the UI. We just need an interface that allows us to inject inputs when the game state requires it.

The PFSM's State consists of the GameState itself, a binary enum of `{ Advancing, AwaitingInput }`
```rust
pub enum GameState {
    Advancing,
    AwaitingInput,
}
```
As well as the Battle struct, which handles the complex state machine transitions during battle:
```rust 
pub struct Battle {
    pub battle_type: BattleType, // Tournament, Trainer, Wild, Safari
    pub players: [BattlePlayer; 2],
    pub battle_commands: [Option<BattleCommand>; 2],
    pub action_stack: Vec<BattleAction>, // LIFO Stack
}
```
### Inside the Battle Struct:
- `battle_type`: a value of the `BattleType` enum:  `{ Tournament, Trainer, Wild, Safari}`. Does not change during battle and determines which `BattleAction`s are available. Also controls relevant parts of `BattleCommand` execution.
- `players`: an array with two `BattlePlayer` structs. The BattlePlayers contain almost all the complex data that is mutated during battle.
- `battle_commands`: an array of two `Option<BattleCommand>`, representing the queued actions for the players to do. User inputs are converted into commands, but commands can also be provided directly by NPC players (`BattlePlayer` specifies `Human` or `NPC`) or carried forward from the previous turn (for multi-turn moves like SolarBeam and Fly). 
    ```rust
    pub enum BattleCommand {
        SwitchPokemon { team_index: usize },
        UseMove { team_index: usize, chosen_move: Move },
        UseBall { ball: PokeballType },
        Forfeit,
        AcceptEvolution { accept: bool },
        ChooseMoveToForget { move_index: usize },
    }
    ```
- `action_stack`: a `BattleAction` stack. **All** mutations of `players` and `GameState` come from `BattleAction` execution **exclusively**. `BattleCommand` just adds to the stack. Executing an action returns a `Vec<BattleAction>` that are then pushed to the stack. There are a lot of these. Also responsible for publishing events.
```rust
pub enum BattleAction {
    // Actions that can trigger Awaiting
    RequestBattleCommands,
    RequestNextPokemon {p1: bool, p2: bool},
    OfferMove {player_index: bool, team_index: usize, new_move: Move},
    OfferEvolution {player_index: bool, team_index: usize, species: Species},

    // Actions that are put on the action_stack from BattleCommands
    DoSwitch { player_index: bool, team_index: usize},
    DoMove {player_index: bool, team_index: usize, move_index: usize},
    DoForfeit { player_index: bool }, // Forfeit beomes Run for Wild and Safari battles
    ThrowBall { ball: PokeballType }, // Wild and Safari battles only

    EndTurn,
    EndBattle {outcome: BattleResolution},
    // Everything else that can happen in a battle, generically
    // For offensive moves that can miss
    StrikeAction {player_index: bool, team_index: usize, target_team_index: usize, use_move: Move},
    // For status moves that don't affect the opponent directly (Reflect, Swords Dance, Rain Dance, etc.)
    PassiveAction {player_index: bool, team_index: usize, use_move: Move}, // Would love a better name

    // For missing, which emits an event and can trigger other things
    Miss {player_index: bool, team_index: usize, use_move: Move},

    Damage {player_index: bool, team_index: usize, amount: u16},
    Heal {player_index: bool, team_index: usize, amount: u16},

    Knockout {player_index: bool, target_team_index: usize},

    ModifyStatStage {player_index: bool, target_team_index: usize, stat: Stat, delta: i8},
    ResetStatChanges {player_index: bool, target_team_index: usize},

    ApplyStatus {player_index: bool, target_team_index: usize, status: Status {..}},
    RemoveStatus {player_index: bool, target_team_index: usize, status: Status {..}},

    ApplyCondition {player_index: bool, target_team_index: usize, condition: Condition {..}},
    RemoveCondition {player_index: bool, target_team_index: usize, condition: Condition {..}},
    RemoveAllConditions {player_index: bool, target_team_index: usize},
    
    ApplyTeamCondition {player_index: bool, condition: TeamCondition {..}},
}

pub enum BattleResolution {Player1Wins, Player2Wins, Draw}
```

## Converting BattleCommands to BattleActions
Four `BattleAction`s depend on `battle_commands` to execute. 
When `GameState==Advancing`, executing any of `RequestBattleCommands`, `RequestNextPokemon`, `OfferMove`, or `OfferEvolution` requires checking the battle_commands. 
- `RequestBattleCommands` accepts `SwitchPokemon`, `UseMove`, `UseBall` (When `BattleType` is `Safari` or `Wild`), and `Forfeit`
- Note: `RequestBattleCommands` requires a BattleCommand from both players.
- `RequestNextPokemon` specify which of `battle_commands` must be non-empty.
It can require one or both players to provide input.
    - `RequestNextPokemon` accepts `SwitchPokemon` and `Forfeit`
- `OfferMove` only ever applies to one player, and accepts only `ChooseMoveToForget`
- `OfferEvolution` only ever applies to one player, and accepts only `EvolutionResponse`

- If a command is missing for any player (Human or NPC):
    1. the `GameState` is set to `AwaitingInput`
    2. a copy of the inciting `BattleAction` is put back on the stack
- External systems (BattleRunner) provide commands via UI for humans or AI for NPCs

Note that the PFSM must be designed such that there are never 'outdated' BattleCommands in `battle_commands`. Consequently, care must be paid with the situations where actions are requested from both players. 
- A weird edge case might involve confusion:
    1. player2 uses QuickAttack, but his pokemon hits itself in confusion and faints.
    2. The `BattleType` is `Trainer`, so player1, who is `PlayerType::Human`, gains experience.
    3. The experience is enough to level up, and does so, at which point it can learn a new move.
    4. player1 still has the `BattleCommand` for the move they ordered in `battle_commands`, but now must handle OfferMove
This situation is pre-empted by guaranteeing that, whenever both players have BattleCommands, their priority is worked out and then BOTH commands are converted to Actions and placed on the `action_stack`. 

There is a related edge case wherein the player could overwrite the move they have queued. 
In the actual Pokemon games, this created an edge case where the pokemon would use the new move instead, because the cached action specified the move_index rather than the actual move. Our system faithfully reflects that silly edge case by having DoMove take the move_index rather than the Move itself.

## Battle FSM Terminal Interface

The Battle FSM provides a clean terminal interface with three core methods:

```rust
pub struct Battle {
    pub battle_type: BattleType, // Tournament, Trainer, Wild, Safari
    pub players: [BattlePlayer; 2],
    pub battle_commands: [Option<BattleCommand>; 2],
    pub action_stack: Vec<BattleAction>, // LIFO Stack
    pub turn: u8,
}

impl Battle {
    pub fn new(battle_type: BattleType, players: [BattlePlayer; 2]) -> Self {
        let mut battle = Battle {
            battle_type,
            players,
            battle_commands: [None, None],
            action_stack: Vec::new(),
        };
        
        // Initialize with first action
        battle.action_stack.push(BattleAction::RequestBattleCommands);
        battle
    }

    pub fn advance(&mut self, events: &mut EventBus) -> GameState {
        // Pop an action. If the stack is empty, this shouldn't happen - generate emergency EndBattle
        if let Some(action) = self.action_stack.pop() {
            // Execute the action, passing a mutable reference to the entire battle
            // so that the action can modify it.
            match action.execute(self, events) {
                Ok(next_state) => next_state,
                Err(_) => GameState::AwaitingInput, // Or some other error state
            }
        } else {
            // Stack should never be empty - this is an error condition
            // Generate emergency EndBattle and set to AwaitingInput
            self.action_stack.push(BattleAction::EndBattle { outcome: BattleResolution::Draw });
            GameState::AwaitingInput
        }
    }

    pub fn submit_commands(&mut self, commands: [Option<BattleCommand>; 2]) -> Result<(), BattleError> {
        // Validate and update battle_commands array
        // This method provides commands to satisfy InputRequest requirements
        self.battle_commands = commands;
        Ok(())
    }

    pub fn get_input_request(&self) -> Option<InputRequest> {
        // The action that paused the engine is the last one pushed onto the stack.
        let waiting_action = self.action_stack.last()?; // Return None if the stack is empty.

        match waiting_action {
            BattleAction::RequestBattleCommands => {
                // Find which human player, if any, still needs to provide a command.
                for i in 0..2 {
                    if self.players[i].player_type == PlayerType::Human && self.battle_commands[i].is_none() {
                        return Some(InputRequest::ForTurnActions { player_index: i });
                    }
                }
                None // Should not happen if state is AwaitingInput, but good to be safe.
            }

            BattleAction::RequestNextPokemon { p1, p2 } => {
                // Check the specific players flagged as needing replacements
                if *p1 && self.players[0].player_type == PlayerType::Human && self.battle_commands[0].is_none() {
                    return Some(InputRequest::ForNextPokemon { player_index: 0 });
                }
                if *p2 && self.players[1].player_type == PlayerType::Human && self.battle_commands[1].is_none() {
                    return Some(InputRequest::ForNextPokemon { player_index: 1 });
                }
                None // Neither flagged player is a human who needs to act right now.
            }

            BattleAction::OfferMove { player_index, team_index, new_move } => {
                // Check if the specified player is a human needing to act.
                if self.players[*player_index].player_type == PlayerType::Human && self.battle_commands[*player_index].is_none() {
                    return Some(InputRequest::ForMoveToForget {
                        player_index: *player_index,
                        team_index: *team_index,
                        new_move: *new_move,
                    });
                }
                None
            }

            BattleAction::OfferEvolution { player_index, team_index, species } => {
                // Check if the specified player is a human needing to act.
                if self.players[*player_index].player_type == PlayerType::Human && self.battle_commands[*player_index].is_none() {
                    return Some(InputRequest::ForEvolution {
                        player_index: *player_index,
                        team_index: *team_index,
                        new_species: *species,
                    });
                }
                None
            }

            BattleAction::EndBattle { outcome } => {
                // Battle is complete - provide the resolution to external systems
                Some(InputRequest::ForBattleComplete { 
                    resolution: *outcome 
                })
            }

            // For any other action, the engine should not be waiting for input.
            _ => None,
        }
    }
}

// On the BattleAction enum
impl BattleAction {
    pub fn execute(
        &self, 
        battle: &mut Battle, 
        events: &mut EventBus
    ) -> Result<GameState, BattleError> {
        // match self { ... }
    }
}

/// A request from the battle engine for a specific piece of input from a player.
/// This is the primary contract between the engine and external systems when the GameState is AwaitingInput.
/// External systems provide commands via UI (humans) or AI (NPCs). It is a handshake to the BattleActions that require input.
#[derive(Debug, Clone)]
pub enum InputRequest {
    /// The engine is waiting for a player to choose their primary action for the turn.
    ForTurnActions { // For RequestBattleCommands
        player_index: usize,
    },

    /// A player's active Pokémon has fainted, and they must choose a replacement.
    ForNextPokemon { // For RequestNextPokemon
        player_index: usize,
    },

    /// A Pokémon is being offered a new move, but its moveset is full.
    ForMoveToForget { // For OfferMove
        player_index: usize,
        team_index: usize,
        new_move: Move,
    },

    /// A Pokémon has met the criteria to evolve.
    ForEvolution { // For OfferEvolution
        player_index: usize,
        team_index: usize,
        new_species: Species,
    },

    /// A Pokémon battle has concluded with the given resolution.
    ForBattleComplete { // For EndBattle
        resolution: BattleResolution,
    },
}

impl InputRequest {
    /// A convenient helper method to get the relevant player index from any request type.
    /// Returns None for battle completion since no specific player is involved.
    pub fn player_index(&self) -> Option<usize> {
        match self {
            InputRequest::ForTurnActions { player_index } => Some(*player_index),
            InputRequest::ForNextPokemon { player_index, .. } => Some(*player_index),
            InputRequest::ForMoveToForget { player_index, .. } => Some(*player_index),
            InputRequest::ForEvolution { player_index, .. } => Some(*player_index),
            InputRequest::ForBattleComplete { .. } => None,
        }
    }
}

```

## Battle Lifecycle

The Battle FSM operates as a pure terminal interface:

1. **External systems** create a Battle instance and initialize it with `RequestBattleCommands` on the action stack
2. **External systems** call `advance()` repeatedly until `GameState::AwaitingInput` is returned
3. **When AwaitingInput**: External systems use `get_input_request()` to determine what input is needed
4. **Input provision**: External systems call `submit_commands()` to provide the required commands:
   - For human players: Commands obtained via UI interaction
   - For NPC players: Commands generated by AI systems (AI modules are part of Battle crate but called by external systems)
5. **Resume**: Return to step 2 until battle completes

**Battle Completion**: When `EndBattle` action executes, the battle is complete. The FSM sets `GameState::AwaitingInput` to prevent further advancement, and external systems can read the final `BattleResolution` from the `InputRequest::ForBattleComplete`.

The `BattleAction` enum implements `execute()` for each `BattleAction`, which takes the Battle struct and can mutate `players`, `battle_commands`, and `action_stack`, and returns a GameState.

### Starting a Battle
Each battle begins with a single `RequestBattleCommands` action on the stack and an empty `battle_commands` array. 
The system remains in the `Advancing` state as it processes this initial action. 

### RequestBattleCommands Execution

When `RequestBattleCommands` executes, it orchestrates the collection of player commands through the following process:

**Phase 1: Command Collection**

For each player without a command:
- If the player has a forced move (e.g., locked into Solar Beam), the corresponding command is generated automatically
- If either player (Human or NPC) lacks a command, their slot remains empty

**Phase 2: Resolution**

If both command slots are filled:
- **Push an `EndTurn` action on the stack**
- **Convert both commands into their corresponding `BattleAction` objects**
    - These can be (In base-priority order):
        1. `DoForfeit`
        2. `ThrowBall`
        3. `DoSwitch`
        4. `DoMove`
    - If both commands are `DoSwitch`, they execute simultaneously.
    - If both commands are `DoMove`, Determine action priority based on:
        - Move priority values
        - Pokemon speed stats  
        - Random resolution for ties
- Push these actions onto the stack in reverse priority order
- Clear both command slots
- **Continue in `Advancing` state**

If any command slot remains empty (input needed):
- **Push `RequestBattleCommands` back onto the stack**
- **Transition to `AwaitingInput` state**
- Wait for external input (UI for humans, AI for NPCs)

You're right - players should be able to forfeit when asked for a replacement. Here's the corrected version:

### RequestNextPokemon Execution

When `RequestNextPokemon` executes, it manages Pokemon replacement after fainting. The action includes boolean flags `{p1: bool, p2: bool}` indicating which players need to send out new Pokemon.

**Phase 1: Command Collection**

For each player flagged as needing a replacement:
- If the player has no remaining conscious Pokemon, mark them as unable to switch
- If the player (Human or NPC) needs to provide a replacement, their slot remains empty pending input

**Phase 2: Resolution**

If all required command slots are filled (or players cannot switch):
- For each player:
  - If they have no conscious Pokemon: push `EndBattle` with appropriate winner
  - If they provided Forfeit:
        - In Wild battles: push EndBattle {Draw}
        - Otherwise: push EndBattle with them as loser
  - If they provided `SwitchPokemon`: generate a `DoSwitch` action
- If the battle hasn't ended:
  - Push all `DoSwitch` actions onto the stack
  - Clear the relevant command slots
- **Continue in `Advancing` state**

If any required command slot remains empty (input needed):
- **Push `RequestNextPokemon` back onto the stack** with updated flags
- **Transition to `AwaitingInput` state**
- Wait for external input (UI for humans, AI for NPCs)

**Special Cases:**
- If both players need replacements (e.g., from Explosion), both provide input before any switches execute
- This action only accepts `SwitchPokemon` or `Forfeit` commands
- Invalid switches (fainted Pokemon, out-of-range index) are rejected, keeping the state as `AwaitingInput`
