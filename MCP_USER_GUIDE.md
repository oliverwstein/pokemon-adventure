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

### Testing the MCP Server

Before configuring with Claude, you can test the server directly:

```bash
# Test server initialization
echo '{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}' | cargo run --bin pokemon-adventure-mcp

# Test getting available tools
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}' | cargo run --bin pokemon-adventure-mcp

# Test getting available teams
echo '{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "get_available_teams", "arguments": {}}}' | cargo run --bin pokemon-adventure-mcp
```

### Configuration for Claude Code

**Option 1: Using Claude CLI (Recommended)**

First, ensure the MCP server binary is built:
```bash
cd pokemon-adventure
cargo build --bin pokemon-adventure-mcp --release
```

Then configure the MCP server. There are three approaches that work:

**Method A: Using pre-built binary (Most Reliable)**
```bash
claude mcp add pokemon-adventure "/full/path/to/pokemon-adventure/target/release/pokemon-adventure-mcp"
```

**Method B: Using cargo with directory change**
```bash
claude mcp add pokemon-adventure "cd /full/path/to/pokemon-adventure && cargo run --bin pokemon-adventure-mcp --release"
```

**Method C: Using cargo from correct directory**
```bash
cd pokemon-adventure
claude mcp add pokemon-adventure "cargo run --bin pokemon-adventure-mcp --release"
```

Replace `/full/path/to/pokemon-adventure` with the actual absolute path to your pokemon-adventure directory.

**Option 2: Manual Configuration**

**Step 1: Locate Configuration File**
The Claude Code MCP configuration file location depends on your operating system:
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **Linux**: `~/.config/Claude/claude_desktop_config.json`

**Step 2: Add MCP Server Configuration**
Add or update the configuration file with:
```json
{
  "mcpServers": {
    "pokemon-adventure": {
      "command": "cargo",
      "args": ["run", "--bin", "pokemon-adventure-mcp"],
      "cwd": "/full/path/to/pokemon-adventure"
    }
  }
}
```

Replace `/full/path/to/pokemon-adventure` with the actual absolute path to your pokemon-adventure directory.

**Step 3: Restart Claude Code**
After saving the configuration file, you MUST completely restart Claude Code for the MCP server to be loaded and the tools to become available.

**Step 4: Verify Configuration**
```bash
claude mcp list
```

Should show:
```
pokemon-adventure: /path/to/pokemon-adventure/target/release/pokemon-adventure-mcp - ✓ Connected
```

## Troubleshooting Connection Issues

If the connection shows `✗ Failed to connect`, try these solutions:

### Common Issues and Solutions

1. **Binary not found**: 
   - Verify the binary exists: `ls -la target/release/pokemon-adventure-mcp`
   - Build it first: `cargo build --bin pokemon-adventure-mcp --release`

2. **Wrong binary name**:
   - The correct binary name is `pokemon-adventure-mcp` (not `mcp_client_server`)
   - Check available binaries: `cargo build --bin pokemon-adventure-mcp 2>&1 | grep "Available bin targets"`

3. **Working directory issues**:
   - Use absolute paths in the configuration
   - Method A (pre-built binary) avoids working directory issues entirely

4. **Cargo command not found**:
   - Ensure Rust and Cargo are installed and in PATH
   - Use Method A (pre-built binary) to avoid cargo dependencies

5. **Connection timeout**:
   - The MCP server needs proper JSON-RPC initialization
   - Direct binary execution (Method A) is most reliable

### Verification Steps

After successful configuration, test the connection:
```bash
# Check server status
claude mcp list

# Get server details
claude mcp get pokemon-adventure

# Should show "✓ Connected" status
```

If still having issues:
1. Remove and re-add the server: `claude mcp remove pokemon-adventure`
2. Use Method A (pre-built binary) for most reliable connection
3. Ensure the pokemon-adventure directory has proper permissions
4. Restart Claude Code completely after configuration changes

## Available Tools

The MCP server provides 8 tools that cover all aspects of Pokemon battling:

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

### 8. `forfeit_battle`
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

## Verified Functionality

The MCP server has been thoroughly tested and verified to work correctly:

### ✅ Server Initialization
- Responds properly to JSON-RPC `initialize` requests
- Returns correct server info and capabilities
- Supports all standard MCP protocol methods

### ✅ Tool Discovery
- Exposes exactly 8 Pokemon battle tools
- Each tool has proper JSON schema validation
- All parameter requirements are correctly defined

### ✅ Battle Functionality
- **Team Selection**: Successfully lists 3 pre-built teams (Venusaur, Blastoise, Charizard)
- **Battle Initialization**: Properly starts battles with chosen teams
- **State Management**: Maintains battle state across tool calls
- **JSON-RPC Compliance**: All responses follow proper JSON-RPC 2.0 format

### ✅ Integration Ready
- Compatible with Claude Code MCP configuration
- Works with both CLI (`claude mcp add`) and manual JSON configuration
- Requires proper working directory (`cwd`) for successful connection
- Tested command: `cargo run --bin pokemon-adventure-mcp`

---

Enjoy battling with the Pokemon Adventure MCP Server! The server provides rich, natural language responses perfect for LLM interaction while maintaining all the strategic depth of authentic Pokemon battles.