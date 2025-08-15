#[cfg(test)]
mod tests {
    use crate::battle::state::{BattleEvent, BattleState, TurnRng};
    use crate::battle::engine::resolve_turn;
    use crate::moves::Move;
    use crate::player::{BattlePlayer, PlayerAction, StatType, TeamCondition};
    use crate::pokemon::{MoveInstance, PokemonInst};
    use crate::species::Species;

    fn create_test_pokemon(species: Species, moves: Vec<Move>) -> PokemonInst {
        let mut pokemon_moves = [const { None }; 4];
        for (i, mv) in moves.into_iter().enumerate() {
            if i < 4 {
                pokemon_moves[i] = Some(MoveInstance { move_: mv, pp: 20 });
            }
        }

        let mut pokemon = PokemonInst::new_for_test(
            species,
            10,
            0,
            0, // Will be set below
            [15; 6],
            [0; 6],
            [100, 80, 80, 80, 80, 80],
            pokemon_moves,
            None,
        );
        pokemon.set_hp_to_max();
        pokemon
    }

    #[test]
    fn test_mist_prevents_enemy_stat_reduction() {
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Growl])], // Growl reduces Attack
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])],
        );

        // Add Mist protection
        player2.add_team_condition(TeamCondition::Mist, 3);

        // Check initial attack stat
        let initial_attack_stage = player2.get_stat_stage(StatType::Attack);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Growl (should be blocked by Mist), Player 2 uses Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Growl
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        println!("Mist prevents enemy stat reduction test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Attack stage should remain unchanged
        let final_attack_stage = battle_state.players[1].get_stat_stage(StatType::Attack);
        assert_eq!(
            initial_attack_stage, final_attack_stage,
            "Mist should prevent stat reduction"
        );

        // Should have a StatChangeBlocked event
        let blocked_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::StatChangeBlocked { reason, .. } if reason.contains("Mist")))
            .collect();
        assert!(
            !blocked_events.is_empty(),
            "Should have StatChangeBlocked event for Mist protection"
        );

        // Should NOT have StatStageChanged event for the target
        let stat_change_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::StatStageChanged {
                        target: Species::Machamp,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            stat_change_events.is_empty(),
            "Should not have stat change events when blocked by Mist"
        );
    }

    #[test]
    fn test_mist_allows_self_targeting_moves() {
        // Test that moves don't get blocked even if the user has Mist
        // (Mist only protects against enemy stat reductions, not moves in general)
         let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Alakazam,
                vec![Move::SwordsDance],
            )], // SwordsDance raises user's Attack
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])],
        );

        // Player 1 has Mist - but it shouldn't affect self-targeting moves
        player1.add_team_condition(TeamCondition::Mist, 3);

        // Check initial stat stages
        let initial_attack_stage = player1.get_stat_stage(StatType::Attack);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Swords Dance (should work normally), Player 2 uses Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Swords Dance
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print events for debugging
        println!("Mist allows self targeting moves test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Attack should be increased despite having Mist (because it's self-targeting)
        let final_attack_stage = battle_state.players[0].get_stat_stage(StatType::Attack);

        assert!(
            final_attack_stage > initial_attack_stage,
            "Self-targeting moves should work despite Mist"
        );

        // Should NOT have StatChangeBlocked events
        let blocked_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| matches!(event, BattleEvent::StatChangeBlocked { .. }))
            .collect();
        assert!(
            blocked_events.is_empty(),
            "Should not block self-targeting moves"
        );

        // Should have StatStageChanged events for the user
        let stat_change_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::StatStageChanged {
                        target: Species::Alakazam,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            !stat_change_events.is_empty(),
            "Should have stat change events for self-targeting moves"
        );
    }

    #[test]
    fn test_mist_allows_positive_enemy_stat_changes() {
        // This test verifies that Mist only blocks negative stat changes, not positive ones
        // Note: This would require a move that increases enemy stats, which is rare in Pokemon
        // For now, we'll test that Mist doesn't block moves that don't reduce stats
         let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Tackle])], // Tackle doesn't affect stats
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])],
        );

        // Add Mist protection
        player2.add_team_condition(TeamCondition::Mist, 3);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Tackle (no stat effects), Player 2 uses Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Should NOT have any StatChangeBlocked events from Mist (since no stat reduction attempted)
        let blocked_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::StatChangeBlocked { reason, .. } if reason.contains("Mist")))
            .collect();
        assert!(
            blocked_events.is_empty(),
            "Mist should not block moves that don't reduce stats"
        );

        // Should have normal damage events
        let damage_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::DamageDealt {
                        target: Species::Machamp,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            !damage_events.is_empty(),
            "Normal damage moves should work despite Mist"
        );
    }

    #[test]
    fn test_mist_blocks_multiple_stat_reductions() {
         let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Screech])], // Screech reduces Defense by 2 stages
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])],
        );

        // Add Mist protection
        player2.add_team_condition(TeamCondition::Mist, 3);

        // Check initial defense stat
        let initial_defense_stage = player2.get_stat_stage(StatType::Defense);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Screech (should be blocked by Mist), Player 2 uses Splash
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Screech
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Defense stage should remain unchanged
        let final_defense_stage = battle_state.players[1].get_stat_stage(StatType::Defense);
        assert_eq!(
            initial_defense_stage, final_defense_stage,
            "Mist should prevent multi-stage stat reduction"
        );

        // Should have a StatChangeBlocked event
        let blocked_events: Vec<_> = event_bus.events().iter()
            .filter(|event| matches!(event, BattleEvent::StatChangeBlocked { reason, .. } if reason.contains("Mist")))
            .collect();
        assert!(
            !blocked_events.is_empty(),
            "Should have StatChangeBlocked event for Mist protection against multi-stage reduction"
        );
    }

    #[test]
    fn test_mist_expires_normally() {
         let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Growl])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Splash])],
        );

        // Add Mist with only 1 turn remaining
        player2.add_team_condition(TeamCondition::Mist, 1);
        assert!(player2.has_team_condition(&TeamCondition::Mist));

        let mut battle_state =
            BattleState::new("test_battle".to_string(), player1.clone(), player2);

        // Turn 1: Mist should protect, then expire
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Growl
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let test_rng1 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus1 = resolve_turn(&mut battle_state, test_rng1);
        for event in event_bus1.events() {
            println!("  {:?}", event);
        }
        // Mist should have expired
        assert!(
            !battle_state.players[1].has_team_condition(&TeamCondition::Mist),
            "Mist should expire after 1 turn"
        );

        // Turn 2: Without Mist, stat reduction should work
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Growl
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Splash

        let initial_attack_stage = battle_state.players[1].get_stat_stage(StatType::Attack);

        let test_rng2 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus2 = resolve_turn(&mut battle_state, test_rng2);
        for event in event_bus2.events() {
            println!("  {:?}", event);
        }
        // Attack should now be reduced
        let final_attack_stage = battle_state.players[1].get_stat_stage(StatType::Attack);
        assert!(
            final_attack_stage < initial_attack_stage,
            "Stat reduction should work after Mist expires"
        );

        // Should NOT have StatChangeBlocked events in turn 2
        let blocked_events: Vec<_> = event_bus2
            .events()
            .iter()
            .filter(|event| matches!(event, BattleEvent::StatChangeBlocked { .. }))
            .collect();
        assert!(
            blocked_events.is_empty(),
            "Should not block stat changes after Mist expires"
        );
    }

    #[test]
    fn test_mist_ticking_in_isolation() {
        let mut player = BattlePlayer::new(
            "test".to_string(),
            "Test".to_string(),
            vec![create_test_pokemon(Species::Alakazam, vec![Move::Splash])],
        );

        // Add Mist with 2 turns
        player.add_team_condition(TeamCondition::Mist, 2);
        assert_eq!(
            player.get_team_condition_turns(&TeamCondition::Mist),
            Some(2)
        );

        // First tick: 2 -> 1
        player.tick_team_conditions();
        assert_eq!(
            player.get_team_condition_turns(&TeamCondition::Mist),
            Some(1)
        );
        assert!(player.has_team_condition(&TeamCondition::Mist));

        // Second tick: 1 -> 0 (removed)
        player.tick_team_conditions();
        assert_eq!(player.get_team_condition_turns(&TeamCondition::Mist), None);
        assert!(!player.has_team_condition(&TeamCondition::Mist));
    }
}
