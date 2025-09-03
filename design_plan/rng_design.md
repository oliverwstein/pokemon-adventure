# RNG Design for Pokemon Battle System

## Overview

The Battle crate uses a distribution-based RNG system where external systems provide RNG instances through a clean trait interface. This design separates RNG distributions from game mechanics while enabling deterministic testing and maintaining battle calculation accuracy.

## RNG Trait Interface

```rust
#[derive(Debug, Clone, Copy)]
pub enum RngCategory {
    Uniform8,          // 0-255 uniform distribution
    Uniform16,         // 0-65535 uniform distribution  
    Percentage,        // 0-99 for percentage rolls (0-99%)
    DamageVariance,    // 0-15 for 85-100% damage range (specialized distribution)
    //....
}

pub trait BattleRng {
    fn roll(&mut self, category: RngCategory) -> u16;
}
```

## Usage in Battle Logic

### Critical Hits
```rust
let crit_roll = rng.roll(RngCategory::Percentage);
let is_critical = crit_roll < critical_hit_chance;
```

### Damage Calculation
```rust
let variance_roll = rng.roll(RngCategory::DamageVariance); // 0-15
let damage_multiplier = 0.85 + (variance_roll as f32 / 15.0) * 0.15; // 85-100%
```

### Accuracy Checks
```rust
let accuracy_roll = rng.roll(RngCategory::Percentage);
let move_hits = accuracy_roll < move_accuracy;
```

### Speed Tie Resolution
```rust
let speed_tie_roll = rng.roll(RngCategory::Uniform8);
let player1_goes_first = speed_tie_roll < 128; // 50% chance
```

### Status Infliction
```rust
let status_roll = rng.roll(RngCategory::Percentage);
let status_applied = status_roll < status_chance_percent;
```

## Implementation Types

### Production RNG
```rust
pub struct StandardBattleRng {
    rng: StdRng,
}

impl BattleRng for StandardBattleRng {
    fn roll(&mut self, category: RngCategory) -> u16 {
        match category {
            RngCategory::Uniform8 => self.rng.gen_range(0..256) as u16,
            RngCategory::Uniform16 => self.rng.gen::<u16>(),
            RngCategory::Percentage => self.rng.gen_range(0..100) as u16,
            RngCategory::DamageVariance => self.rng.gen_range(0..16) as u16,
        }
    }
}
```

### Test RNG with rstest Integration
```rust
#[cfg(test)]
pub struct MockBattleRng {
    overrides: HashMap<RngCategory, u16>,
    fallback: StdRng,
}

impl MockBattleRng {
    pub fn new() -> Self {
        Self {
            overrides: HashMap::new(),
            fallback: StdRng::seed_from_u64(0),
        }
    }
    
    pub fn set_fixed(mut self, category: RngCategory, value: u16) -> Self {
        self.overrides.insert(category, value);
        self
    }
    
    pub fn always_crit(self) -> Self {
        self.set_fixed(RngCategory::Percentage, 0) // Always < crit_chance
    }
    
    pub fn never_miss(self) -> Self {
        self.set_fixed(RngCategory::Percentage, 0) // Always < accuracy
    }
    
    pub fn min_damage(self) -> Self {
        self.set_fixed(RngCategory::DamageVariance, 0) // 85% damage
    }
    
    pub fn max_damage(self) -> Self {
        self.set_fixed(RngCategory::DamageVariance, 15) // 100% damage
    }
}

impl BattleRng for MockBattleRng {
    fn roll(&mut self, category: RngCategory) -> u16 {
        *self.overrides.get(&category)
            .unwrap_or_else(|| {
                // Fallback to seeded RNG if no override set
                match category {
                    RngCategory::Uniform8 => &(self.fallback.gen_range(0..256) as u16),
                    RngCategory::Uniform16 => &self.fallback.gen::<u16>(),
                    RngCategory::Percentage => &(self.fallback.gen_range(0..100) as u16),
                    RngCategory::DamageVariance => &(self.fallback.gen_range(0..16) as u16),
                }
            })
    }
}
```

## Testing Patterns

### rstest with Precise Control
```rust
#[rstest]
#[case(true, false, 85)]  // crit, no status, min damage
#[case(false, true, 100)] // no crit, status, max damage
fn test_move_outcomes(
    #[case] should_crit: bool, 
    #[case] should_inflict_status: bool,
    #[case] damage_percent: u8
) {
    let crit_roll = if should_crit { 0 } else { 99 };
    let status_roll = if should_inflict_status { 0 } else { 99 };
    let damage_roll = ((damage_percent - 85) * 15 / 15) as u16; // Convert % to 0-15
    
    let mut rng = MockBattleRng::new()
        .set_fixed(RngCategory::Percentage, crit_roll) // Used first for crit
        .set_fixed(RngCategory::DamageVariance, damage_roll);
        
    // Execute battle action with controlled outcomes
    let result = execute_move(&mut battle, &mut rng);
    
    assert_eq!(result.was_critical, should_crit);
    assert_eq!(result.damage_percent, damage_percent);
}
```

### Statistical Testing for Complex Scenarios
```rust
#[test]
fn test_critical_hit_rate_statistical() {
    let mut crits = 0;
    let trials = 1000;
    
    for seed in 0..trials {
        let mut rng = StandardBattleRng::new(seed);
        let mut battle = create_test_battle();
        
        if execute_move_and_check_crit(&mut battle, &mut rng) {
            crits += 1;
        }
    }
    
    let crit_rate = crits as f32 / trials as f32;
    assert!(crit_rate >= 0.055 && crit_rate <= 0.095); // ~6.25% ±3%
}
```

## Integration with Battle FSM

The Battle struct receives RNG through its advance method:

```rust
impl Battle {
    pub fn advance(&mut self, events: &mut EventBus, rng: &mut dyn BattleRng) -> GameState {
        // FSM execution with RNG passed to all calculation functions
    }
}
```

This design maintains the existing pattern of external RNG control while providing:
- **Type safety** through enum-based categories
- **Deterministic testing** through MockBattleRng
- **Distribution separation** from game mechanics
- **rstest compatibility** for comprehensive test coverage
- **Statistical validation** for complex scenarios

## Benefits

1. **No RNG Vector Counting**: Tests don't need to predict exact RNG consumption
2. **Robust to Changes**: Adding new random elements doesn't break existing tests  
3. **Clear Intent**: Battle code shows what distribution it needs
4. **Reusable Categories**: Same Percentage category for crits, accuracy, status
5. **Type Safe**: Enum prevents typos, compile-time validation
6. **rstest Integration**: Easy parametric testing with precise control