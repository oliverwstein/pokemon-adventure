The `BattleRunner` as a long-lived struct was a stepping stone, but now we have to refactor it to account for the **stateless request/response model** we have planned. 

### Desired Architecture Summary

A **stateless request/response model** where the database is the single source of truth. The "runner" logic isn't a long-lived struct, but a set of functions or a temporary orchestrator that is created for the duration of a single API request.

Here are the key principles:

1.  **DB is Truth:** The primary entities stored in the database are `BattleState` and a log. Everything needed to resume a battle is in the `BattleState`.
2.  **Stateless Handlers:** The Lambda functions are stateless. Each invocation loads the necessary data, performs its logic, and saves the new state. There is no "running" `BattleRunner` instance between API calls.
3.  **The "Game Tick" Loop:** the "loop" happens *within a single API request*. When a player's action is the last piece of information needed to proceed, the handler should continue to "tick" the game forward as many times as possible until it requires input from an API POST again.

Here is the outline of the API endpoints and flow I have in mind:

#### **API Endpoints & Flow**

*   **`GET /available_teams`**:
    *   **Action:** Returns a list of available prefab teams.
    *   **Purpose:** Allows a player to see what teams they can choose from.
    *   **Notes:** Not associated with a specific battle--this should draw on the db to get the data stored in a prefab_teams table, each with a team_id. 

*   **`GET /npc_opponents`**:
    *   **Action:** Returns a list of npc opponents, with info on each one.
    *   **Purpose:** Allows a player to see what NPCs they can fight.
    *   **Notes:** like available_teams, this allows the player to see the npc_opponents they can fight by reading from a table in the db.


*   **`POST /battles` (Create Battle):**
    *   **Request Body:** `{ "player_name": "Ash", "team_id": "venusaur_team", "opponent": "npc_id" }`
    *   **Logic:**
        1.  Validate input (team_id exists, npc_id exists).
        2.  Create a `BattlePlayer` for the human using the chosen prefab team.
        3.  Create an NPC `BattlePlayer`.
        4.  Instantiate a new `BattleState`.
        5.  **Game Tick:** Create a temporary `BattleRunner` for this request.
        6.  Save the new `BattleState` to the DB.
        7.  Save an initial set of events (e.g., "Battle Started") to the event log.
        8.  **Respond:** `{ "battle_id": "...", "status": "waiting_for_player_0_action", ... }`
    *   **Notes:** This creates the BattleState, and should probably return the intro of the NPC they are facing and the output of `GET /battles/{battle_id}/state`.

*   **`GET /battles/{battle_id}/log`**:
    *   **Action:** Returns the current public state of the battle and the full event log.
    *   **Purpose:** Allows a client to sync up, view history, and see whose turn it is.

*   **`GET /battles/{battle_id}/state`**:
    *   **Action:** Returns the current public state of the battle: the player's active pokemon, its moves (with PP) and stats, the NPC's first pokemon, and its level/HP. Oh, and all the statuses and conditions that apply. Should also note how many non-fainted pokemon each player has. Should also note whether the game is waiting for input from the user.
    *   **Purpose:** Allows a user to see the current state of the battle.

*   **`GET /battles/{battle_id}/team_info`**:
    *   **Action:** Returns the state of all the pokemon on the player's team, their moves, statuses, and which pokemon is active.
    *   **Purpose:** Allows a user to see the current state of their team

*   **`GET /battles/{battle_id}/valid_actions`**:
    *   **Action:** Returns a list of actions the player can take at the given time. Their available moves (with PP), the pokemon they can switch to (if any), and, of course, the option to forfeit. 
    *   **Purpose:** Allows a user to see what they can choose.

*   **`POST /battles/{battle_id}/action`**:
    *   **Request Body:** `{ "player_id": "auth_token_or_id", "action": { "type": "use_move", "move_index": 0 } }`
    *   **This is the core of the game tick.**
    *   **Logic:**
        1.  Load `BattleState` from DB.
        2.  Create a temporary `BattleRunner` for this request, attaching the appropriate controllers (e.g., `Human` for the acting player, `SimpleAI` for the NPC opponent).
        3.  Call `runner.submit_action(player_index, action)`.
        4.  This method will internally queue the human's action and then **loop**, as you described. It will see the opponent is an AI, resolve its action, run the turn, accumulate events, and then check again. If the human's next move is forced (Solar Beam), it will resolve the AI's next move and run *another* turn. This loop continues until the runner is waiting for a non-forced human action or the battle ends.
        5.  The loop finishes.
        6.  Take the final, mutated `BattleState` from the runner and save it to the DB.
        7.  Append all the newly generated events to the event log in the DB.
        8.  **Respond:** `{ "status": "ok", "new_events": [...], "new_battle_state": {...} }`
    *   **Notes:** Ideally we should be able to accept move_index or move name. 
        * For now the valid action types should be:
            * `use_move`: the move_index (1, 2, 3, or 4)
            * `switch`: the team_index of the pokemon to switch to (1, 2, 3, 4, 5, or 6)
            * `forfeit`: no parameter needed.
 
### Architectural Deep Dive: Authentication and Authorization

*   **The Core Problem:** Several `GET` endpoints (`/battles/{id}`, `/battles/{id}/valid_actions`, etc.) need to return data specific to a user. A standard `GET` request doesn't have a body, so how does the server know who is asking? Furthermore, how can we ensure that "Ash" can't see "Gary's" options or submit an action on his behalf?

*   **The Solution: Token-Based Authentication via Headers**
    *   The industry-standard solution is to handle identity outside the immediate request path or body. The client sends a secret token in a standard HTTP header with every request that requires authentication.
    *   **Header:** `Authorization: Bearer <some_secret_token>`
    *   This token is issued to the user when they log in. It proves their identity for a limited time.

*   **Implementation with AWS API Gateway & Lambda Authorizers**
    *   Our stateless Lambda handlers should not be responsible for the complex logic of parsing and validating these tokens. This is a separate concern.
    *   We configure API Gateway to protect our sensitive endpoints. When a request comes in to a protected endpoint (like `POST /battles/{id}/action`), API Gateway first routes it to a special, dedicated "Lambda Authorizer" function.
    *   **The Authorizer's Job:**
        1.  Receive the request and extract the token from the `Authorization` header.
        2.  Validate the token (check its signature, expiration date, and that it hasn't been revoked).
        3.  If the token is valid, look up the associated `user_id` (e.g., "ash_ketchum_123").
        4.  Return an "Allow" policy to API Gateway. Crucially, it can also pass a `context` object containing the `user_id`.
        5.  If the token is invalid, it returns a "Deny" policy, and API Gateway immediately sends a `401 Unauthorized` or `403 Forbidden` response. The main handler is never even invoked.

*   **The Main Handler's Role:**
    *   Because the Authorizer has already handled security, our main `lambda_handler` can trust that any request it receives is from a valid, authenticated user.
    *   It can access the user's ID directly from the `event` payload passed by the authorizer.
    *   **Example Logic for `GET /battles/{id}/state`:**
        1.  Receive the request. The authorizer has already validated the token and added `user_id` to the event context.
        2.  Extract `battle_id` from the URL path.
        3.  Extract `user_id` from the event context.
        4.  Load the `BattleState` from the database using the `battle_id`.
        5.  **Perform Authorization:** Check if `loaded_state.players[0].player_id == user_id` OR `loaded_state.players[1].player_id == user_id`.
        6.  If the user is not a participant, return a `403 Forbidden` error. They are authenticated, but not *authorized* to see this specific battle.
        7.  If they are a participant, construct and return the detailed response, showing their own private info (full team) and the opponent's public info.
