# **Implementation Roadmap: Command-Based Turn Orchestrator**

## **✅ COMPLETED**
1. **Initial Validation** - Defender fainted check (lines 2040-2050)
2. **MoveUsed Event** - First hit only (lines 2063-2070, now in calculator)  
3. **Hit/Miss Logic** - Accuracy calculation + events (lines 2070-2082, now in calculator)

**Status**: Basic scaffolding complete, hit/miss bridge working, all 129 tests passing.

---

## **🎯 REMAINING TO IMPLEMENT**

### **Iteration 3: Type Effectiveness & Critical Hits**
**Lines: 2088-2131**
- Type effectiveness calculation (`get_type_effectiveness`)
- Type effectiveness event emission  
- Critical hit calculation (`move_is_critical_hit`)
- Critical hit event emission
- Special damage vs normal damage branching

### **Iteration 4: Core Damage Calculation** 
**Lines: 2098-2131**
- Special attack damage (`calculate_special_attack_damage`)
- Normal attack damage (`calculate_attack_damage`) 
- Type effectiveness application to damage

### **Iteration 5: Substitute Damage Absorption**
**Lines: 2133-2175**
- Check for Substitute condition
- Substitute HP management
- Substitute destruction logic
- StatusRemoved event for destroyed substitute
- Damage routing (substitute vs pokemon)

### **Iteration 6: Normal Damage Application**
**Lines: 2175-2197**
- Pokemon damage application (`take_damage`)
- DamageDealt event emission
- Fainting detection and PokemonFainted event

### **Iteration 7: Counter Condition Logic**
**Lines: 2204-2241**
- Counter condition detection
- Physical move check for counter eligibility
- Counter damage calculation (2x damage)
- Counter damage application to attacker
- Counter fainting logic

### **Iteration 8: Bide Condition Logic**
**Lines: 2243-2267**
- Bide condition detection
- Damage accumulation in Bide condition
- Bide condition update

### **Iteration 9: Enraged Condition Logic**
**Lines: 2269-2285**
- Enraged condition detection
- Attack stat stage increase
- StatStageChanged event emission

### **Iteration 10: Move Effects Application**
**Lines: 2293-2309**
- Status/Other category move effect application
- Damage-dependent effect application
- Call to `apply_move_effects` (600+ lines itself!)

### **Iteration 11: Damage-Based Effects**
**Lines: 2311-2314**
- Recoil effects
- Drain effects  
- Other damage-dependent effects
- Call to `apply_on_damage_effects`

### **Iteration 12: Multi-Hit Logic**
**Lines: 2321-2350**
- Multi-hit effect detection
- Guaranteed hits vs probabilistic continuation
- Next hit action queuing
- Multi-hit termination logic

### **Iteration 13: Miss Effects (Reckless)**
**Lines: 2351-2393**
- Reckless effect on miss
- Miss damage calculation
- Miss damage application
- Miss fainting logic

---

## **🔥 MAJOR SUB-FUNCTIONS TO IMPLEMENT**

### **`apply_move_effects()` Function** (~600 lines)
This function handles dozens of move effects:
- Status conditions (Poison, Burn, Paralysis, Sleep, Freeze)
- Stat changes (Attack up, Defense down, etc.)
- Type conversion (Conversion)
- Transformation (Transform)
- Substitute creation
- Team conditions (Reflect, Light Screen, Mist)
- Special conditions (Confusion, Flinch, Disable)
- Healing effects
- PP restoration
- One-hit KO moves
- Fixed damage moves
- And many more...

### **`apply_on_damage_effects()` Function** (~200 lines)
- Recoil damage (Take damage from own move)
- Drain effects (Heal based on damage dealt)
- Other damage-dependent effects

---

## **📊 SCOPE ANALYSIS**

**Total Lines to Migrate**: ~1,400+ lines across 3 major functions
- `execute_attack_hit`: ~360 lines remaining
- `apply_move_effects`: ~600 lines  
- `apply_on_damage_effects`: ~200 lines
- Plus numerous helper functions

**Complexity Levels**:
- **Simple**: Type effectiveness, critical hits, basic damage
- **Medium**: Substitute logic, condition updates, multi-hit
- **Complex**: Move effects system, status conditions, special interactions

**Event Types to Generate**: 15+ different event types
**Command Types Needed**: 20+ command variants
**Special Cases**: Multi-hit, substitute, counter, bide, enraged, reckless

---

## **🎯 NEXT STEPS**

The next logical iteration is **Iteration 3: Type Effectiveness & Critical Hits** as it's:
- Self-contained logic
- Relatively simple compared to damage application
- Sets up foundation for damage calculation
- Has existing pure functions to leverage

**Current Progress**: 3/16 iterations complete (~19% of attack logic migrated)

This is a **comprehensive battle system** with authentic Generation 1 mechanics plus custom enhancements. The roadmap shows significant work ahead, but the incremental approach makes it very manageable!