#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{BattleEvent, BattleState, GameState, TurnRng};
    use crate::battle::tests::common::{create_test_player, TestPokemonBuilder};
    use crate::moves::Move;
    use crate::player::PlayerAction;
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(
        "3 hits",
        // This RNG sequence forces 3 hits and then stops the multi-hit sequence.
        vec![50, 90, 95, 50, 90, 92, 40, 50, 90, 90, 90, 50, 90, 90],
        3
    )]
    // You could add other cases here, e.g., forcing 2, 4, or 5 hits.
    // #[case("2 hits", vec![50, 90, 95, 50, 90, 92, 60, ...], 2)]
    fn test_probabilistic_multi_hit_logic(
        #[case] desc: &str,
        #[case] rng_values: Vec<u8>,
        #[case] expected_hits: usize,
    ) {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Meowth, 10)
            .with_moves(vec![Move::FurySwipes])
            .build();
        let defender = TestPokemonBuilder::new(Species::Onix, 10)
            .with_moves(vec![Move::Tackle])
            .with_hp(100) // Ensure it can survive multiple hits
            .build();

        let player1 = create_test_player("p1", "Player 1", vec![attacker]);
        let player2 = create_test_player("p2", "Player 2", vec![defender]);
        let mut battle_state = BattleState::new("multi_hit_test".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Fury Swipes
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        let test_rng = TurnRng::new_for_test(rng_values);

        // Act
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Assert
        event_bus.print_debug_with_message(&format!(
            "Events for test_probabilistic_multi_hit_logic [{}]:",
            desc
        ));

        let fury_swipes_hits = event_bus
            .events()
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    BattleEvent::MoveHit {
                        move_used: Move::FurySwipes,
                        ..
                    }
                )
            })
            .count();

        // Total damage events = fury swipes hits + opponent's tackle
        let total_damage_events = event_bus
            .events()
            .iter()
            .filter(|e| matches!(e, BattleEvent::DamageDealt { .. }))
            .count();

        assert_eq!(
            fury_swipes_hits, expected_hits,
            "Fury Swipes should have hit the expected number of times"
        );
        assert_eq!(
            total_damage_events,
            expected_hits + 1,
            "Should be one damage event per hit + one for the opponent's move"
        );
        assert!(matches!(
            battle_state.game_state,
            GameState::WaitingForActions
        ));
    }

    #[test]
    fn test_multi_hit_stops_on_faint() {
        // Arrange: Defender has very low HP to ensure it faints on the first hit.
        let attacker = TestPokemonBuilder::new(Species::Meowth, 25)
            .with_moves(vec![Move::FurySwipes])
            .build();
        let defender = TestPokemonBuilder::new(Species::Pidgey, 5)
            .with_moves(vec![Move::Tackle])
            .with_hp(1)
            .build();
        let defender_backup = TestPokemonBuilder::new(Species::Rattata, 5).build();

        let player1 = create_test_player("p1", "Player 1", vec![attacker]);
        let player2 = create_test_player("p2", "Player 2", vec![defender, defender_backup]);
        let mut battle_state =
            BattleState::new("multi_hit_faint_test".to_string(), player1, player2);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // RNG to ensure the first hit connects and is powerful enough to KO.
        let test_rng = TurnRng::new_for_test(vec![50, 90, 100, 50, 90, 90]);

        // Act
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Assert
        event_bus.print_debug_with_message("Events for test_multi_hit_stops_on_faint:");

        let fury_swipes_hits = event_bus
            .events()
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    BattleEvent::MoveHit {
                        move_used: Move::FurySwipes,
                        ..
                    }
                )
            })
            .count();

        let faint_events = event_bus
            .events()
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    BattleEvent::PokemonFainted {
                        player_index: 1,
                        ..
                    }
                )
            })
            .count();

        assert_eq!(
            fury_swipes_hits, 1,
            "The multi-hit sequence should stop after the first fatal hit"
        );
        assert_eq!(faint_events, 1, "The defender should have fainted");
        assert!(
            matches!(
                battle_state.game_state,
                GameState::WaitingForPlayer2Replacement
            ),
            "Game should be waiting for replacement"
        );
    }
}
