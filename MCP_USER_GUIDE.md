# Pokemon Adventure MCP Server - User Guide

## Overview

The Pokemon Adventure MCP Server is a Model Context Protocol (MCP) server that provides LLMs with access to a complete Pokemon battle engine featuring authentic Generation 1 mechanics. This guide explains how to use the various tools available through the MCP server.

## Installation & Setup

### Prerequisites
- Rust toolchain installed
- Clone the Pokemon Adventure repository

### Building the MCP Server
```bash
cd pokemon-adventure
cargo build --bin pokemon-adventure-mcp
```

The binary will be located at `target/debug/pokemon-adventure-mcp` (or `target/release/pokemon-adventure-mcp` for release builds).

### Configuration for Claude Code
Add to your MCP configuration:
```json
{
  "mcpServers": {
    "pokemon-adventure": {
      "command": "/path/to/pokemon-adventure/target/debug/pokemon-adventure-mcp"
    }
  }
}
```

## Available Tools

The MCP server provides 9 tools that cover all aspects of Pokemon battling:

### 1. `get_available_teams`
**Purpose**: List all available pre-built Pokemon teams  
**Parameters**: None  
**Example Usage**: "What teams can I choose from?"
```
Available Teams:
  1. Venusaur Team - Elite team featuring Venusaur with diverse type coverage and strategic options
  2. Blastoise Team - Balanced team featuring Blastoise with excellent type diversity and control options  
  3. Charizard Team - Aggressive team featuring Charizard with high offensive potential and versatility
```

### 2. `start_battle`
**Purpose**: Begin a new Pokemon battle with your chosen team  
**Parameters**: 
- `team_choice` (number): Team to select (1-3)

**Example Usage**: "I want to start a battle with the Blastoise team"
```
🔥 Welcome to the Pokémon Adventure Battle Engine! 🔥

You chose the Blastoise Team!

💥 A wild trainer challenges you to a battle! 💥
You sent out Blastoise!
AI Trainer sends out Charizard!
```

### 3. `get_battle_state`
**Purpose**: View the current battle situation and status  
**Parameters**: None  
**Example Usage**: "What's the current battle status?"
```
⚔️  Battle in Progress ⚔️

=== BATTLE STATUS ===
Turn: 3 | Battle: mcp_battle

🔵 Player (Player)
  └─ Blastoise (Active) ⚡ HP: 142/158
     Status: Healthy | Level: 60

🔴 AI Trainer (AI Trainer)  
  └─ Charizard (Active) ⚡ HP: 98/153
     Status: Healthy | Level: 60
```

### 4. `use_move`
**Purpose**: Execute a Pokemon move in battle  
**Parameters**:
- `move_name` (string): Name of the move to use

**Example Usage**: "I want to use Hydro Pump"
```
Blastoise used Hydro Pump!
It's super effective!
Charizard took 67 damage!

AI Trainer's Charizard used Flamethrower!
It's not very effective...
Blastoise took 23 damage!
```

### 5. `switch_pokemon`
**Purpose**: Switch to a different Pokemon on your team  
**Parameters**:
- `pokemon_number` (number): Pokemon slot to switch to (1-6)

**Example Usage**: "Switch to my second Pokemon"
```
You switched Blastoise for Wartortle!
Wartortle, I choose you!

AI Trainer's Charizard used Dragon Rage!
Wartortle took 40 damage!
```

### 6. `check`
**Purpose**: Examine detailed status of Pokemon  
**Parameters**:
- `target` (string): What to check
  - `"self"` - Your active Pokemon
  - `"opponent"` - Opponent's active Pokemon  
  - `"team"` - Your entire team overview
  - `"team 3"` - Specific team member details

**Example Usage**: "Check my active Pokemon"
```
--- Your Active Pokémon ---
Blastoise ⚡ HP: 142/158 (Level 60)
Status: Healthy
Types: Water

Stats: ATK 83 | DEF 100 | SPC 85 | SPE 78

Moves:
  1. Hydro Pump (Water) - Power: 120, PP: 5/5
  2. Ice Beam (Ice) - Power: 95, PP: 10/10  
  3. Earthquake (Ground) - Power: 100, PP: 10/10
  4. Body Slam (Normal) - Power: 85, PP: 15/15
```

### 7. `lookup_move`
**Purpose**: Get detailed information about any Pokemon move  
**Parameters**:
- `move_name` (string): Name of the move to research

**Example Usage**: "What does Solar Beam do?"
```
--- Move Details ---
Solar Beam (Grass)
Power: 120 | Accuracy: 100% | PP: 10
Category: Special | Priority: 0

A two-turn attack. The user gathers light on the first turn, then blasts a bundled beam on the next turn.

Effects: Two-turn move - charges first turn, attacks second turn
```

### 8. `lookup_pokemon`
**Purpose**: Research detailed information about Pokemon species  
**Parameters**:
- `species_name` (string): Name of the Pokemon to look up

**Example Usage**: "Tell me about Charizard"
```
--- Pokemon Details ---
Charizard (#006) - Fire/Flying Type
A Fire-type Pokemon. Its wings can carry it close to an altitude of 4,600 feet.

Base Stats:
  HP: 78 | Attack: 84 | Defense: 78
  Special: 85 | Speed: 100

Evolution: Evolves from Charmeleon at level 36
```

### 9. `forfeit_battle`
**Purpose**: Give up and end the current battle  
**Parameters**: None  
**Example Usage**: "I give up, forfeit the battle"
```
Player forfeited the battle!

💀 You lost the battle! 💀
```

## Battle Flow

### Starting a Battle
1. Use `get_available_teams` to see your options
2. Use `start_battle` with your team choice (1-3)
3. Use `get_battle_state` to see the initial setup

### During Battle  
1. Use `check` tools to assess the situation
2. Use `use_move` or `switch_pokemon` to take actions
3. Use `lookup_move` or `lookup_pokemon` for strategic information
4. Use `get_battle_state` to see results after each turn

### Battle End
- Battles end automatically when all Pokemon on one side faint
- Use `forfeit_battle` to quit early if needed
- Start a new battle anytime with `start_battle`

## Tips for LLM Users

### Strategic Information Gathering
- **Research moves**: Use `lookup_move` to understand power, accuracy, and special effects
- **Know your opponent**: Use `check opponent` to see their Pokemon's status and type
- **Plan switches**: Use `check team` to see your available Pokemon and their conditions

### Effective Battle Commands
- **Be specific with move names**: "Use Hydro Pump" rather than "use a water attack"
- **Use exact Pokemon numbers**: "Switch to Pokemon 3" rather than "switch to another Pokemon" 
- **Check status regularly**: Use `get_battle_state` to stay informed about HP and status conditions

### Example Battle Session
```
LLM: "What teams are available?"
→ get_available_teams

LLM: "I'll choose the Venusaur team to start a battle"  
→ start_battle with team_choice: 1

LLM: "What's my Pokemon's current status?"
→ check with target: "self"

LLM: "Let me see what moves my opponent might use"
→ lookup_pokemon with species_name: "Charizard"

LLM: "I'll use Sleep Powder to put them to sleep"
→ use_move with move_name: "Sleep Powder"

LLM: "How did that turn go?"
→ get_battle_state
```

## Battle Mechanics

### Authentic Generation 1 Rules
- **Type effectiveness**: Super effective (2x), not very effective (0.5x), no effect (0x)
- **Status conditions**: Sleep, poison, burn, paralysis, freeze
- **Critical hits**: Speed-based with high-crit moves
- **Stat stages**: ±6 modifications affect all stats

### Advanced Features
- **Two-turn moves**: Solar Beam, Dig, Fly require charging
- **Multi-hit moves**: Some moves hit 2-5 times randomly
- **Recoil moves**: Take damage from your own attacks
- **Status moves**: Stat boosts, healing, protection effects

### Team Management
- **6 Pokemon maximum** per team (uses pre-built teams)
- **4 moves per Pokemon** with PP (Power Points) limitations
- **Forced switching** when Pokemon faint
- **No items or abilities** (pure Generation 1 mechanics)

## Error Handling

The MCP server provides helpful error messages:

- **Invalid move names**: "Thunderbolt is not a valid move for your active Pokemon"
- **Invalid switches**: "Cannot switch to fainted Pokemon"  
- **No active battle**: "No battle is currently active. Use 'start_battle' to begin"
- **Invalid parameters**: Clear guidance on correct parameter formats

## Session Management

- **One battle per session**: Each MCP server instance handles one battle
- **Persistent state**: Battle continues across multiple tool calls
- **Fresh starts**: Use `start_battle` anytime to begin a new battle
- **Stateful tracking**: Server remembers Pokemon HP, status, and battle progress

---

Enjoy battling with the Pokemon Adventure MCP Server! The server provides rich, natural language responses perfect for LLM interaction while maintaining all the strategic depth of authentic Pokemon battles.