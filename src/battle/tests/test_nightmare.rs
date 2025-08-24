#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{ActionFailureReason, BattleEvent};
    use crate::battle::tests::common::{create_test_battle, predictable_rng, TestPokemonBuilder};
    use crate::player::PlayerAction;
    use crate::pokemon::StatusCondition;
    use crate::species::Species;
    use pokemon_adventure_schema::Move;
    use rstest::rstest;

    #[rstest]
    #[case("succeeds on sleeping target", Some(StatusCondition::Sleep(2)), true)]
    #[case("fails on awake target", None, false)]
    #[case("fails on paralyzed target", Some(StatusCondition::Paralysis), false)]
    fn test_dream_eater_outcomes(
        #[case] desc: &str,
        #[case] target_status: Option<StatusCondition>,
        #[case] should_succeed: bool,
    ) {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Hypno, 10)
            .with_moves(vec![Move::DreamEater])
            .build();

        let mut p2_builder =
            TestPokemonBuilder::new(Species::Snorlax, 10).with_moves(vec![Move::Tackle]);

        if let Some(status) = target_status {
            p2_builder = p2_builder.with_status(status);
        }

        let mut battle_state = create_test_battle(p1_pokemon, p2_builder.build());

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Dream Eater
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        event_bus
            .print_debug_with_message(&format!("Events for test_dream_eater_outcomes [{}]:", desc));

        let move_failed = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::ActionFailed {
                    reason: ActionFailureReason::MoveFailedToExecute { .. }
                }
            )
        });

        let damage_dealt = event_bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::DamageDealt {
                    target: Species::Snorlax,
                    ..
                }
            )
        });

        if should_succeed {
            assert!(
                !move_failed,
                "Dream Eater should have succeeded but it failed"
            );
            assert!(damage_dealt, "Dream Eater should have dealt damage");
        } else {
            assert!(
                move_failed,
                "Dream Eater should have failed but it succeeded"
            );
            assert!(
                !damage_dealt,
                "Dream Eater should not have dealt damage when it fails"
            );
        }
    }
}
