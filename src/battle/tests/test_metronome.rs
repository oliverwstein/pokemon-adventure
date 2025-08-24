#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{BattleEvent, TurnRng};
    use crate::battle::tests::common::{create_test_battle, TestPokemonBuilder};
    use crate::player::PlayerAction;
    use crate::species::Species;
    use pokemon_adventure_schema::Move;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[test]
    fn test_metronome_selects_and_uses_a_random_move() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Clefairy, 10)
            .with_moves(vec![Move::Metronome])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Pikachu, 10)
            .with_moves(vec![Move::Splash])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Metronome
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        // Use a deterministic RNG to ensure a predictable outcome.
        let test_rng = TurnRng::new_for_test(vec![50; 20]);

        // Act
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Assert
        event_bus.print_debug_with_message("Events for test_metronome_selects_random_move:");

        let p1_moves_used: Vec<Move> = event_bus
            .events()
            .iter()
            .filter_map(|event| {
                if let BattleEvent::MoveUsed {
                    player_index,
                    move_used,
                    ..
                } = event
                {
                    if *player_index == 0 {
                        Some(*move_used)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        // Metronome should generate two MoveUsed events for Player 1:
        // 1. The use of Metronome itself.
        // 2. The use of the randomly selected move.
        assert_eq!(
            p1_moves_used.len(),
            2,
            "Should have two MoveUsed events for the Metronome user"
        );
        assert!(
            p1_moves_used.contains(&Move::Metronome),
            "One event should be for Metronome"
        );
        assert!(
            p1_moves_used.iter().any(|&m| m != Move::Metronome),
            "The other event should be for the called move"
        );
    }

    #[rstest]
    #[case("seed 10", 10)]
    #[case("seed 25", 25)]
    #[case("seed 50", 50)]
    #[case("seed 75", 75)]
    #[case("seed 90", 90)]
    fn test_metronome_can_select_different_moves(#[case] desc: &str, #[case] rng_seed: u8) {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Clefairy, 10)
            .with_moves(vec![Move::Metronome])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Pikachu, 10)
            .with_moves(vec![Move::Splash])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        let test_rng = TurnRng::new_for_test(vec![rng_seed; 20]);

        // Act
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Assert
        event_bus.print_debug_with_message(&format!(
            "Events for test_metronome_can_select_different_moves [{}]:",
            desc
        ));

        let called_move = event_bus.events().iter().find_map(|event| {
            if let BattleEvent::MoveUsed {
                player_index,
                move_used,
                ..
            } = event
            {
                if *player_index == 0 && *move_used != Move::Metronome {
                    Some(*move_used)
                } else {
                    None
                }
            } else {
                None
            }
        });

        assert!(
            called_move.is_some(),
            "Metronome should have selected and used a move"
        );
    }

    #[test]
    fn test_metronome_fully_executes_a_damaging_move() {
        // Arrange: Use a specific RNG seed that is known to select a damaging move (Tackle).
        // This makes the test deterministic and not flaky.
        let p1_pokemon = TestPokemonBuilder::new(Species::Clefairy, 10)
            .with_moves(vec![Move::Metronome])
            .build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Pikachu, 10)
            .with_moves(vec![Move::Splash])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // This specific RNG seed will cause Metronome to select Tackle.
        let test_rng = TurnRng::new_for_test(vec![50, 50, 14, 50, 50, 50, 50]);

        // Act
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Assert
        event_bus
            .print_debug_with_message("Events for test_metronome_fully_executes_a_damaging_move:");

        let called_move_is_tackle = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::MoveUsed {
                    player_index: 0,
                    move_used: Move::Tackle,
                    ..
                }
            )
        });
        let damage_was_dealt = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::DamageDealt {
                    target: Species::Pikachu,
                    ..
                }
            )
        });

        assert!(
            called_move_is_tackle,
            "Metronome should have called Tackle with the given RNG seed"
        );
        assert!(
            damage_was_dealt,
            "The called move (Tackle) should have dealt damage to the target"
        );
    }
}
