#[cfg(test)]
mod tests {
    use crate::battle::action_stack::{ActionStack, BattleAction};
    use crate::battle::conditions::{PokemonCondition, PokemonConditionType};
    use crate::battle::engine::{execute_battle_action, resolve_turn};
    use crate::battle::state::{BattleEvent, EventBus, TurnRng};
    use crate::battle::tests::common::{create_test_battle, predictable_rng, TestPokemonBuilder};
    use crate::moves::Move;
    use crate::player::PlayerAction;
    use crate::pokemon::{StatusCondition};
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    // --- Unit Tests for PokemonInst Methods ---

    #[rstest]
    #[case("regular poison", StatusCondition::Poison(0), 1.0/16.0)]
    #[case("badly poisoned (toxic)", StatusCondition::Poison(1), 1.0/16.0)] // Severity 1 means 1/16th damage
    #[case("burn", StatusCondition::Burn, 1.0/8.0)]
    fn test_status_damage_calculation(#[case] desc: &str, #[case] status: StatusCondition, #[case] fraction: f32) {
        // Arrange
        let pokemon = TestPokemonBuilder::new(Species::Charmander, 50)
            .with_status(status)
            .build();
        let max_hp = pokemon.max_hp();
        let expected_damage = (max_hp as f32 * fraction).max(1.0) as u16;

        // Act
        let damage = pokemon.calculate_status_damage();

        // Assert
        assert_eq!(damage, expected_damage, "Failed case: {}", desc);
    }

    #[test]
    fn test_sleep_countdown_unit() {
        // Arrange
        let mut pokemon = TestPokemonBuilder::new(Species::Snorlax, 50)
            .with_status(StatusCondition::Sleep(3))
            .build();

        // Act & Assert - Turn 1 (Sleep 3 -> 2)
        let (should_cure, _) = pokemon.update_status_progress();
        assert!(!should_cure);
        assert_eq!(pokemon.status, Some(StatusCondition::Sleep(2)));

        // Act & Assert - Turn 2 (Sleep 2 -> 1)
        let (should_cure, _) = pokemon.update_status_progress();
        assert!(!should_cure);
        assert_eq!(pokemon.status, Some(StatusCondition::Sleep(1)));
        
        // Act & Assert - Turn 3 (Sleep 1 -> 0)
        let (should_cure, _) = pokemon.update_status_progress();
        assert!(!should_cure);
        assert_eq!(pokemon.status, Some(StatusCondition::Sleep(0)));

        // Act & Assert - Turn 4 (Sleep 0 -> Wakes up)
        let (should_cure, _) = pokemon.update_status_progress();
        assert!(should_cure);
        assert_eq!(pokemon.status, None);
    }

    // --- Integration Tests for End-of-Turn Engine Logic ---

    #[test]
    fn test_end_of_turn_integration_damage() {
        // Arrange: Test that Poison and Burn damage are applied at the end of a turn.
        let p1_pokemon = TestPokemonBuilder::new(Species::Charmander, 25)
            .with_moves(vec![Move::Splash])
            .with_status(StatusCondition::Poison(0))
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Bulbasaur, 25)
            .with_moves(vec![Move::Splash])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);
        
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_end_of_turn_integration_damage:");
        let poison_damage_event = event_bus.events().iter().any(|e| {
            matches!(e, BattleEvent::PokemonStatusDamage { target: Species::Charmander, status: StatusCondition::Poison(0), .. })
        });
        assert!(poison_damage_event, "Poison damage should be applied at the end of the turn");
    }

    #[test]
    fn test_active_condition_timers_decrement() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Pikachu, 25).with_moves(vec![Move::Splash]).build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Rattata, 25).with_moves(vec![Move::Splash]).build();
        
        let mut player1 = crate::battle::tests::common::create_test_player("p1", "Player 1", vec![p1_pokemon]);
        player1.add_condition(PokemonCondition::Confused { turns_remaining: 3 });
        player1.add_condition(PokemonCondition::Trapped { turns_remaining: 2 });
        player1.add_condition(PokemonCondition::Flinched); // Should expire this turn

        let player2 = crate::battle::tests::common::create_test_player("p2", "Player 2", vec![p2_pokemon]);
        let mut battle_state = crate::battle::state::BattleState::new("test".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_active_condition_timers_decrement:");

        // Check that Flinched was removed
        assert!(!battle_state.players[0].has_condition_type(PokemonConditionType::Flinched));
        assert!(event_bus.events().iter().any(|e| matches!(e, BattleEvent::ConditionExpired { condition: PokemonCondition::Flinched, .. })));

        // Check that other timers decremented correctly
        let confusion = battle_state.players[0].active_pokemon_conditions.get(&PokemonConditionType::Confused);
        let trapped = battle_state.players[0].active_pokemon_conditions.get(&PokemonConditionType::Trapped);
        
        assert!(matches!(confusion, Some(PokemonCondition::Confused { turns_remaining: 2 })));
        assert!(matches!(trapped, Some(PokemonCondition::Trapped { turns_remaining: 1 })));
    }

    #[test]
    fn test_status_damage_causes_fainting() {
        // Arrange: Level 50 Magikarp's HP is low enough to be KO'd by poison.
        let template_pokemon = TestPokemonBuilder::new(Species::Magikarp, 50).build();
        let max_hp = template_pokemon.max_hp();
        let poison_damage = (max_hp as f32 / 16.0).max(1.0) as u16;

        let p1_pokemon = TestPokemonBuilder::new(Species::Magikarp, 50)
            .with_moves(vec![Move::Splash])
            .with_status(StatusCondition::Poison(0))
            .with_hp(poison_damage)
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Pikachu, 50).with_moves(vec![Move::Splash]).build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_status_damage_causes_fainting:");
        assert!(battle_state.players[0].active_pokemon().unwrap().is_fainted());
        assert!(event_bus.events().iter().any(|e| matches!(e, BattleEvent::PokemonFainted { player_index: 0, pokemon: Species::Magikarp })));
    }

    #[rstest]
    #[case("thaws on low roll", 24, true)]
    #[case("stays frozen on high roll", 25, false)]
    fn test_freeze_thaw_outcomes(#[case] desc: &str, #[case] rng_val: u8, #[case] should_thaw: bool) {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Tackle])
            .with_status(StatusCondition::Freeze)
            .build();
        let defender = TestPokemonBuilder::new(Species::Charmander, 25).build();
        let mut battle_state = create_test_battle(attacker, defender);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![rng_val, 100, 100, 100]);
        let mut action_stack = ActionStack::new();

        // Act
        execute_battle_action(
            BattleAction::AttackHit { attacker_index: 0, defender_index: 1, move_used: Move::Tackle, hit_number: 0 },
            &mut battle_state, &mut action_stack, &mut bus, &mut rng,
        );

        // Assert
        bus.print_debug_with_message(&format!("Events for test_freeze_thaw_outcomes [{}]:", desc));
        
        let final_status = battle_state.players[0].active_pokemon().unwrap().status;
        let thaw_event_found = bus.events().iter().any(|e| matches!(e, BattleEvent::PokemonStatusRemoved { status: StatusCondition::Freeze, .. }));

        if should_thaw {
            assert_eq!(final_status, None, "Pokemon should have thawed and have no status");
            assert!(thaw_event_found, "A PokemonStatusRemoved event should have been emitted for the thaw");
        } else {
            assert_eq!(final_status, Some(StatusCondition::Freeze), "Pokemon should remain frozen");
            assert!(!thaw_event_found, "No thaw event should have been emitted");
        }
    }
}