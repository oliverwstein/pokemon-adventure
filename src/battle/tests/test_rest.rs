#[cfg(test)]
mod tests {
    use crate::battle::conditions::PokemonCondition;
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{ActionFailureReason, BattleEvent, BattleState};
    use crate::battle::tests::common::{
        create_test_battle, create_test_player, predictable_rng, TestPokemonBuilder,
    };
    use crate::player::PlayerAction;
    use crate::pokemon::StatusCondition;
    use crate::species::Species;
    use pokemon_adventure_schema::Move;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case("heals when damaged", 50, true)]
    #[case("does not heal at full hp", 100, false)]
    fn test_rest_healing_logic(
        #[case] desc: &str,
        #[case] start_hp_percent: u16,
        #[case] should_heal: bool,
    ) {
        // Arrange
        let template = TestPokemonBuilder::new(Species::Snorlax, 10).build();
        let max_hp = template.max_hp();
        let start_hp = (max_hp * start_hp_percent) / 100;

        let p1_pokemon = TestPokemonBuilder::new(Species::Snorlax, 10)
            .with_moves(vec![Move::Rest])
            .with_hp(start_hp)
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Pikachu, 10)
            .with_moves(vec![Move::Splash])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus
            .print_debug_with_message(&format!("Events for test_rest_healing_logic [{}]:", desc));

        let final_pokemon = battle_state.players[0].active_pokemon().unwrap();
        assert_eq!(
            final_pokemon.current_hp(),
            final_pokemon.max_hp(),
            "HP should be full after Rest"
        );
        assert!(
            matches!(final_pokemon.status, Some(StatusCondition::Sleep(2))),
            "Pokemon should be asleep"
        );

        let heal_event_found = event_bus
            .events()
            .iter()
            .any(|e| matches!(e, BattleEvent::PokemonHealed { .. }));
        assert_eq!(
            heal_event_found, should_heal,
            "Heal event expectation mismatch"
        );
    }

    #[test]
    fn test_rest_clears_status_but_not_conditions() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Snorlax, 10)
            .with_moves(vec![Move::Rest])
            .with_status(StatusCondition::Poison(1)) // Existing status
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Pikachu, 10)
            .with_moves(vec![Move::Splash])
            .build();

        let mut player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        // Add a variety of active conditions
        player1.add_condition(PokemonCondition::Confused { turns_remaining: 2 });
        player1.add_condition(PokemonCondition::Enraged);
        player1.add_condition(PokemonCondition::Substitute { hp: 20 });

        let player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);
        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus
            .print_debug_with_message("Events for test_rest_clears_status_but_not_conditions:");

        let final_pokemon = battle_state.players[0].active_pokemon().unwrap();
        let final_player = &battle_state.players[0];

        // Check final status and conditions
        assert!(
            matches!(final_pokemon.status, Some(StatusCondition::Sleep(2))),
            "Final status should be Sleep"
        );
        assert!(
            !final_player.active_pokemon_conditions.is_empty(),
            "Active conditions should remain"
        );
        assert_eq!(
            final_player.active_pokemon_conditions.len(),
            3,
            "All 3 conditions should still be present"
        );

        // Check for events
        let status_applied_event = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::PokemonStatusApplied {
                    status: StatusCondition::Sleep(2),
                    ..
                }
            )
        });

        assert!(
            status_applied_event,
            "Should emit an event for applying sleep"
        );
    }

    #[test]
    fn test_rest_prevents_action_on_subsequent_turn() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Snorlax, 10)
            .with_moves(vec![Move::Rest, Move::Tackle])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Pikachu, 10)
            .with_moves(vec![Move::Splash])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        // Act - Turn 1: Use Rest
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Rest
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash
        let bus1 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 1
        bus1.print_debug_with_message("Events for test_rest_prevents_action (Turn 1):");
        assert!(matches!(
            battle_state.players[0].active_pokemon().unwrap().status,
            Some(StatusCondition::Sleep(2))
        ));

        // Act - Turn 2: Try to use Tackle while asleep
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 1 }); // Tackle
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash
        let bus2 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 2
        bus2.print_debug_with_message("Events for test_rest_prevents_action (Turn 2):");
        let action_failed_event = bus2.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::ActionFailed {
                    reason: ActionFailureReason::IsAsleep { .. }
                }
            )
        });
        let damage_dealt_event = bus2.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::DamageDealt {
                    target: Species::Pikachu,
                    ..
                }
            )
        });

        assert!(action_failed_event, "Action should fail due to sleep");
        assert!(
            !damage_dealt_event,
            "No damage should be dealt to the opponent"
        );
    }
}
