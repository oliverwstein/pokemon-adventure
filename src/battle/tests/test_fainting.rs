#[cfg(test)]
mod tests {
    use crate::battle::engine::{collect_npc_actions, resolve_turn};
    use crate::battle::state::{ActionFailureReason, BattleEvent, GameState};
    use crate::battle::tests::common::{
        TestPokemonBuilder, create_test_battle, create_test_player, predictable_rng,
    };
    use crate::moves::Move;
    use crate::player::PlayerAction;
    use crate::pokemon::StatusCondition;
    use crate::species::Species;
    use pretty_assertions::assert_eq;

    // --- Unit Tests for PokemonInst Fainting Logic ---

    #[test]
    fn test_pokemon_fainting_mechanics_unit() {
        // Arrange
        let mut pokemon = TestPokemonBuilder::new(Species::Pikachu, 10)
            .with_hp(20)
            .build();

        // Act & Assert: Damage without fainting
        let fainted1 = pokemon.take_damage(10);
        assert!(!fainted1);
        assert_eq!(pokemon.current_hp(), 10);
        assert!(!pokemon.is_fainted());

        // Act & Assert: Fatal damage
        let fainted2 = pokemon.take_damage(15);
        assert!(fainted2);
        assert_eq!(pokemon.current_hp(), 0);
        assert!(pokemon.is_fainted());
        assert_eq!(pokemon.status, Some(StatusCondition::Faint));
    }

    #[test]
    fn test_faint_replaces_other_statuses_unit() {
        // Arrange
        let mut pokemon = TestPokemonBuilder::new(Species::Pikachu, 10)
            .with_hp(10)
            .with_status(StatusCondition::Burn)
            .build();
        assert_eq!(pokemon.status, Some(StatusCondition::Burn));

        // Act
        let fainted = pokemon.take_damage(20);

        // Assert
        assert!(fainted);
        assert!(pokemon.is_fainted());
        assert_eq!(pokemon.status, Some(StatusCondition::Faint));
    }

    #[test]
    fn test_healing_and_revival_unit() {
        // Arrange
        let mut pokemon = TestPokemonBuilder::new(Species::Pikachu, 50)
            .with_hp(50)
            .build(); // Level 50 for higher HP

        // Damage and Heal
        pokemon.take_damage(30);
        assert_eq!(pokemon.current_hp(), 20);
        pokemon.heal(10);
        assert_eq!(pokemon.current_hp(), 30);

        // Faint and attempt to heal
        pokemon.take_damage(100);
        assert!(pokemon.is_fainted());
        pokemon.heal(20);
        assert_eq!(pokemon.current_hp(), 0);

        // Revive
        pokemon.revive(25);
        assert!(!pokemon.is_fainted());
        assert_eq!(pokemon.current_hp(), 25);
        assert_eq!(pokemon.status, None);
    }

    // --- Integration Tests for Fainting in Battle ---

    #[test]
    fn test_battle_with_fainting() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Pikachu, 10)
            .with_moves(vec![Move::Tackle])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Charmander, 5) // Low level ensures it's weaker
            .with_moves(vec![Move::Scratch])
            .with_hp(10) // Low HP to guarantee fainting
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_battle_with_fainting:");
        assert!(
            battle_state.players[1]
                .active_pokemon()
                .unwrap()
                .is_fainted()
        );
        let faint_event_found = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::PokemonFainted {
                    player_index: 1,
                    pokemon: Species::Charmander
                }
            )
        });
        assert!(
            faint_event_found,
            "A PokemonFainted event should have been emitted"
        );
    }

    #[test]
    fn test_fainted_pokemon_cannot_act() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Pikachu, 10)
            .with_moves(vec![Move::Tackle])
            .with_hp(0) // Starts fainted
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Charmander, 10)
            .with_moves(vec![Move::Scratch])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        // Manually set an attack action for the fainted Pokémon.
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_fainted_pokemon_cannot_act:");
        let action_failed_event = event_bus.events().iter().find(|e| {
            matches!(
                e,
                BattleEvent::ActionFailed {
                    reason: ActionFailureReason::PokemonFainted
                }
            )
        });
        assert!(
            action_failed_event.is_some(),
            "Action should fail because the Pokémon is fainted"
        );
    }

    #[test]
    fn test_forced_pokemon_replacement_after_fainting() {
        // Arrange
        let p1_active = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Tackle])
            .with_hp(1)
            .build();
        let p1_backup = TestPokemonBuilder::new(Species::Bulbasaur, 25)
            .with_moves(vec![Move::VineWhip])
            .build();
        let p2_active = TestPokemonBuilder::new(Species::Charizard, 25)
            .with_moves(vec![Move::Scratch])
            .build();

        // Manually create players and state for multi-pokemon teams
        let player1 = create_test_player("p1", "Player 1", vec![p1_active, p1_backup]);
        let player2 = create_test_player("p2", "Player 2", vec![p2_active]);
        let mut battle_state =
            crate::battle::state::BattleState::new("test_multi".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act: Resolve the turn where the fainting occurs.
        let faint_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert: Check the state after the faint.
        faint_bus.print_debug_with_message(
            "Events for test_forced_pokemon_replacement_after_fainting (Faint Turn):",
        );
        assert!(matches!(
            battle_state.game_state,
            GameState::WaitingForPlayer1Replacement
        ));

        // Act 2: Let the AI choose the replacement and resolve the replacement phase.
        let npc_actions = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions {
            battle_state.action_queue[player_index] = Some(action);
        }
        let switch_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert 2: Check the state after the switch.
        switch_bus.print_debug_with_message(
            "Events for test_forced_pokemon_replacement_after_fainting (Switch Turn):",
        );
        assert_eq!(battle_state.players[0].active_pokemon_index, 1);
        assert_eq!(
            battle_state.players[0].active_pokemon().unwrap().species,
            Species::Bulbasaur
        );
        assert!(matches!(
            battle_state.game_state,
            GameState::WaitingForActions
        ));
    }

    #[test]
    fn test_cannot_switch_to_fainted_pokemon() {
        // Arrange
        let p1_active = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Tackle])
            .build();
        let p1_fainted = TestPokemonBuilder::new(Species::Charmander, 25)
            .with_hp(0)
            .build();
        let p2_active = TestPokemonBuilder::new(Species::Squirtle, 25)
            .with_moves(vec![Move::Tackle])
            .build();

        // Manually create players and state for multi-pokemon teams
        let player1 = create_test_player("p1", "Player 1", vec![p1_active, p1_fainted]);
        let player2 = create_test_player("p2", "Player 2", vec![p2_active]);
        let mut battle_state =
            crate::battle::state::BattleState::new("test_multi".to_string(), player1, player2);

        // Player 1 attempts to switch to the fainted Pokémon at index 1.
        battle_state.action_queue[0] = Some(PlayerAction::SwitchPokemon { team_index: 1 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus.print_debug_with_message("Events for test_cannot_switch_to_fainted_pokemon:");
        let action_failed_event = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::ActionFailed {
                    reason: ActionFailureReason::PokemonFainted
                }
            )
        });
        assert!(
            action_failed_event,
            "Should fail when trying to switch to a fainted Pokémon"
        );
        assert_eq!(
            battle_state.players[0].active_pokemon_index, 0,
            "Player 1 should not have switched"
        );
    }
}
