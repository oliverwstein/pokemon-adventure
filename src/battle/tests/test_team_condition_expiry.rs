#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::{BattleEvent, BattleState};
    use crate::battle::tests::common::{create_test_player, predictable_rng, TestPokemonBuilder};
    use crate::moves::Move;
    use crate::player::{PlayerAction, TeamCondition};
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    // --- Unit Test for BattlePlayer Logic ---

    #[test]
    fn test_team_condition_ticking_unit() {
        // Arrange
        let pokemon = TestPokemonBuilder::new(Species::Alakazam, 10).with_moves(vec![Move::Splash]).build();
        let mut player = create_test_player("p1", "Player 1", vec![pokemon]);
        player.add_team_condition(TeamCondition::Reflect, 3);
        player.add_team_condition(TeamCondition::LightScreen, 1);

        // Act & Assert: First tick (Reflect: 3->2, Light Screen: 1->0)
        player.tick_team_conditions();
        assert_eq!(player.get_team_condition_turns(&TeamCondition::Reflect), Some(2));
        assert!(!player.has_team_condition(&TeamCondition::LightScreen));

        // Act & Assert: Second tick (Reflect: 2->1)
        player.tick_team_conditions();
        assert_eq!(player.get_team_condition_turns(&TeamCondition::Reflect), Some(1));

        // Act & Assert: Third tick (Reflect: 1->0)
        player.tick_team_conditions();
        assert!(!player.has_team_condition(&TeamCondition::Reflect));
    }

    // --- Integration Tests for Battle Engine Logic ---

    #[rstest]
    #[case("Reflect", TeamCondition::Reflect, 3)]
    #[case("Light Screen", TeamCondition::LightScreen, 2)]
    fn test_team_condition_expires_after_duration(
        #[case] desc: &str,
        #[case] condition: TeamCondition,
        #[case] duration: u8,
    ) {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Machamp, 10).with_moves(vec![Move::Tackle]).build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Alakazam, 10).with_moves(vec![Move::Splash]).build();
        
        let player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        let mut player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);
        player2.add_team_condition(condition, duration);
        
        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        // Act: Run the battle for the specified number of turns
        for i in 1..=duration {
            battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
            battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
            let bus = resolve_turn(&mut battle_state, predictable_rng());
            bus.print_debug_with_message(&format!("Events for {} expiry [Turn {}]:", desc, i));

            if i < duration {
                assert!(battle_state.players[1].has_team_condition(&condition), "Condition should be active before final turn");
            }
        }

        // Assert
        assert!(!battle_state.players[1].has_team_condition(&condition), "Condition should have expired after {} turns", duration);
    }
    
    #[test]
    fn test_multiple_team_conditions_expire_independently() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Alakazam, 10).with_moves(vec![Move::Confusion]).build();
        let p2_pokemon = TestPokemonBuilder::new(Species::Machamp, 10).with_moves(vec![Move::Splash]).build();

        let player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
        let mut player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);
        player2.add_team_condition(TeamCondition::Reflect, 2);      // Expires after turn 2
        player2.add_team_condition(TeamCondition::LightScreen, 1); // Expires after turn 1

        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        // Act - Turn 1
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        let bus1 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 1
        bus1.print_debug_with_message("Events for multiple expirations [Turn 1]:");
        assert!(battle_state.players[1].has_team_condition(&TeamCondition::Reflect), "Reflect should still be active");
        assert!(!battle_state.players[1].has_team_condition(&TeamCondition::LightScreen), "Light Screen should have expired");
        assert!(bus1.events().iter().any(|e| matches!(e, BattleEvent::TeamConditionExpired { condition: TeamCondition::LightScreen, .. })));

        // Act - Turn 2
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        let bus2 = resolve_turn(&mut battle_state, predictable_rng());

        // Assert - Turn 2
        bus2.print_debug_with_message("Events for multiple expirations [Turn 2]:");
        assert!(!battle_state.players[1].has_team_condition(&TeamCondition::Reflect), "Reflect should have expired");
        assert!(bus2.events().iter().any(|e| matches!(e, BattleEvent::TeamConditionExpired { condition: TeamCondition::Reflect, .. })));
    }

    #[test]
    fn test_effectiveness_changes_on_expiry() {
        // Arrange
        let attacker = TestPokemonBuilder::new(Species::Machamp, 50).with_moves(vec![Move::Tackle]).build();
        let defender = TestPokemonBuilder::new(Species::Alakazam, 50).with_moves(vec![Move::Splash]).build();

        let player1 = create_test_player("p1", "Player 1", vec![attacker]);
        let mut player2 = create_test_player("p2", "Player 2", vec![defender]);
        player2.add_team_condition(TeamCondition::Reflect, 1); // Expires after this turn

        let mut battle_state = BattleState::new("test".to_string(), player1, player2);

        // Act & Assert - Turn 1 (with Reflect)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        let bus1 = resolve_turn(&mut battle_state, predictable_rng());
        bus1.print_debug_with_message("Events for effectiveness change [Turn 1 - With Reflect]:");
        let damage1 = bus1.events().iter().find_map(|e| match e {
            BattleEvent::DamageDealt { damage, .. } => Some(*damage),
            _ => None,
        }).unwrap_or(0);
        assert!(!battle_state.players[1].has_team_condition(&TeamCondition::Reflect), "Reflect should expire after turn 1");

        // Act & Assert - Turn 2 (without Reflect)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        let bus2 = resolve_turn(&mut battle_state, predictable_rng());
        bus2.print_debug_with_message("Events for effectiveness change [Turn 2 - Without Reflect]:");
        let damage2 = bus2.events().iter().find_map(|e| match e {
            BattleEvent::DamageDealt { damage, .. } => Some(*damage),
            _ => None,
        }).unwrap_or(0);
        
        assert!(damage2 > damage1, "Damage should be higher after Reflect expires");
    }
}