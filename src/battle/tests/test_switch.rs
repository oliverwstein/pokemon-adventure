#[cfg(test)]
mod tests {
    use crate::battle::action_stack::{ActionStack, BattleAction};
    use crate::battle::engine::execute_battle_action;
    use crate::battle::state::{BattleEvent, EventBus, TurnRng};
    use crate::battle::tests::common::{create_test_player, TestPokemonBuilder};
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use schema::Move;

    #[test]
    fn test_switch_event_shows_correct_pokemon() {
        // Arrange: Create a team with two different Pokemon
        let pokemon1 = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Tackle])
            .build();
        let pokemon2 = TestPokemonBuilder::new(Species::Charmander, 25)
            .with_moves(vec![Move::Scratch])
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![pokemon1, pokemon2]);

        let defender = TestPokemonBuilder::new(Species::Bulbasaur, 25)
            .with_moves(vec![Move::Tackle])
            .build();
        let player2 = create_test_player("p2", "Player 2", vec![defender]);

        let mut battle_state =
            crate::battle::state::BattleState::new("test".to_string(), player1, player2);

        let mut bus = EventBus::new();
        let mut action_stack = ActionStack::new();
        let mut rng = TurnRng::new_for_test(vec![]);

        // Verify initial state
        assert_eq!(
            battle_state.players[0].active_pokemon().unwrap().species,
            Species::Pikachu
        );

        // Act: Execute switch action (Player 1 switches from Pikachu to Charmander)
        execute_battle_action(
            BattleAction::Switch {
                player_index: 0,
                target_pokemon_index: 1,
            },
            &mut battle_state,
            &mut action_stack,
            &mut bus,
            &mut rng,
        );

        // Assert: Verify the switch occurred
        assert_eq!(
            battle_state.players[0].active_pokemon().unwrap().species,
            Species::Charmander
        );

        // Assert: Verify the correct switch event was generated
        bus.print_debug_with_message("Events for test_switch_event_shows_correct_pokemon:");

        let switch_events: Vec<_> = bus
            .events()
            .iter()
            .filter(|e| matches!(e, BattleEvent::PokemonSwitched { .. }))
            .collect();

        assert_eq!(
            switch_events.len(),
            1,
            "Should have exactly one switch event"
        );

        if let BattleEvent::PokemonSwitched {
            player_index,
            old_pokemon,
            new_pokemon,
        } = switch_events[0]
        {
            assert_eq!(*player_index, 0);
            assert_eq!(*old_pokemon, Species::Pikachu);
            assert_eq!(*new_pokemon, Species::Charmander);
            assert_ne!(
                *old_pokemon, *new_pokemon,
                "Old and new Pokemon should be different"
            );
        } else {
            panic!("Expected PokemonSwitched event");
        }
    }

    #[test]
    fn test_switch_between_same_species_different_individuals() {
        // Arrange: Create a team with two Pokemon of the same species
        let pokemon1 = TestPokemonBuilder::new(Species::Pikachu, 50)
            .with_moves(vec![Move::Tackle])
            .with_hp(50)
            .build();
        let pokemon2 = TestPokemonBuilder::new(Species::Pikachu, 50)
            .with_moves(vec![Move::Lightning])
            .with_hp(80)
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![pokemon1, pokemon2]);

        let defender = TestPokemonBuilder::new(Species::Bulbasaur, 45)
            .with_moves(vec![Move::Tackle])
            .build();
        let player2 = create_test_player("p2", "Player 2", vec![defender]);

        let mut battle_state =
            crate::battle::state::BattleState::new("test".to_string(), player1, player2);

        let mut bus = EventBus::new();
        let mut action_stack = ActionStack::new();
        let mut rng = TurnRng::new_for_test(vec![]);

        // Verify initial state
        assert_eq!(
            battle_state.players[0]
                .active_pokemon()
                .unwrap()
                .current_hp(),
            50
        );

        // Act: Execute switch action
        execute_battle_action(
            BattleAction::Switch {
                player_index: 0,
                target_pokemon_index: 1,
            },
            &mut battle_state,
            &mut action_stack,
            &mut bus,
            &mut rng,
        );

        // Assert: Verify the switch occurred (different HP values prove it's a different individual)
        assert_eq!(
            battle_state.players[0]
                .active_pokemon()
                .unwrap()
                .current_hp(),
            80
        );

        // Assert: Verify the switch event was generated (even for same species)
        bus.print_debug_with_message("Events for test_switch_between_same_species:");

        let switch_events: Vec<_> = bus
            .events()
            .iter()
            .filter(|e| matches!(e, BattleEvent::PokemonSwitched { .. }))
            .collect();

        assert_eq!(
            switch_events.len(),
            1,
            "Should have exactly one switch event"
        );

        if let BattleEvent::PokemonSwitched {
            player_index,
            old_pokemon,
            new_pokemon,
        } = switch_events[0]
        {
            assert_eq!(*player_index, 0);
            assert_eq!(*old_pokemon, Species::Pikachu);
            assert_eq!(*new_pokemon, Species::Pikachu);
            // In this case, old and new Pokemon are the same species, which is valid
        } else {
            panic!("Expected PokemonSwitched event");
        }
    }
}
