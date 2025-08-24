#[cfg(test)]
mod tests {
    use crate::battle::action_stack::ActionStack;
    use crate::battle::engine::execute_attack_hit;
    use crate::battle::state::{BattleEvent, EventBus, TurnRng};
    use crate::battle::tests::common::{create_test_battle, TestPokemonBuilder};
    use crate::player::StatType;
    use crate::pokemon::StatusCondition;
    use crate::species::Species;
    use pokemon_adventure_schema::Move;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_swords_dance_raises_attack() {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Meowth, 10)
            .with_moves(vec![Move::SwordsDance])
            .build();
        let defender = TestPokemonBuilder::new(Species::Pidgey, 10).build();
        let mut battle_state = create_test_battle(attacker, defender);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50, 60, 70]); // Rolls to ensure success
        let mut action_stack = ActionStack::new();
        let initial_attack_stage = battle_state.players[0].get_stat_stage(StatType::Atk);
        assert_eq!(initial_attack_stage, 0);

        // Act
        execute_attack_hit(
            0,
            1,
            Move::SwordsDance,
            0,
            &mut action_stack,
            &mut bus,
            &mut rng,
            &mut battle_state,
        );

        // Assert
        bus.print_debug_with_message("Events for test_swords_dance_raises_attack:");

        let final_attack_stage = battle_state.players[0].get_stat_stage(StatType::Atk);
        assert_eq!(final_attack_stage, 2, "Attack stage should be raised by 2");

        let stat_change_event = bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::StatStageChanged {
                    stat: StatType::Atk,
                    old_stage: 0,
                    new_stage: 2,
                    ..
                }
            )
        });
        assert!(
            stat_change_event,
            "A StatStageChanged event should have been emitted"
        );
    }

    #[test]
    fn test_harden_raises_defense() {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Metapod, 10)
            .with_moves(vec![Move::Harden])
            .build();
        let defender = TestPokemonBuilder::new(Species::Pidgey, 10).build();
        let mut battle_state = create_test_battle(attacker, defender);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50, 60, 70]);
        let mut action_stack = ActionStack::new();
        let initial_defense_stage = battle_state.players[0].get_stat_stage(StatType::Def);
        assert_eq!(initial_defense_stage, 0);

        // Act
        execute_attack_hit(
            0,
            1,
            Move::Harden,
            0,
            &mut action_stack,
            &mut bus,
            &mut rng,
            &mut battle_state,
        );

        // Assert
        bus.print_debug_with_message("Events for test_harden_raises_defense:");

        let final_defense_stage = battle_state.players[0].get_stat_stage(StatType::Def);
        assert_eq!(
            final_defense_stage, 1,
            "Defense stage should be raised by 1"
        );

        let stat_change_event = bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::StatStageChanged {
                    stat: StatType::Def,
                    old_stage: 0,
                    new_stage: 1,
                    ..
                }
            )
        });
        assert!(
            stat_change_event,
            "A StatStageChanged event should have been emitted"
        );
    }

    #[test]
    fn test_thunder_wave_applies_paralysis() {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Pikachu, 10)
            .with_moves(vec![Move::ThunderWave])
            .build();
        let defender = TestPokemonBuilder::new(Species::Pidgey, 10).build();
        let mut battle_state = create_test_battle(attacker, defender);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![50, 60, 70]); // Rolls to ensure success
        let mut action_stack = ActionStack::new();
        assert!(battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .status
            .is_none());

        // Act
        execute_attack_hit(
            0,
            1,
            Move::ThunderWave,
            0,
            &mut action_stack,
            &mut bus,
            &mut rng,
            &mut battle_state,
        );

        // Assert
        bus.print_debug_with_message("Events for test_thunder_wave_applies_paralysis:");

        let final_status = battle_state.players[1].active_pokemon().unwrap().status;
        assert!(
            matches!(final_status, Some(StatusCondition::Paralysis)),
            "Defender should be paralyzed"
        );

        let status_applied_event = bus.events().iter().any(|e| {
            matches!(
                e,
                BattleEvent::PokemonStatusApplied {
                    status: StatusCondition::Paralysis,
                    ..
                }
            )
        });
        assert!(
            status_applied_event,
            "A PokemonStatusApplied event for Paralysis should have been emitted"
        );
    }
}
