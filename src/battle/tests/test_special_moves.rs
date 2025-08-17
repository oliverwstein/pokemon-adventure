#[cfg(test)]
mod tests {
    use crate::battle::conditions::PokemonCondition;
    use crate::battle::state::{BattleEvent, BattleState, TurnRng};
    use crate::battle::engine::{collect_npc_actions, resolve_turn, ready_for_turn_resolution};
    use crate::moves::Move;
    use crate::player::{BattlePlayer, PlayerAction};
    use crate::pokemon::{MoveInstance, PokemonInst};
    use crate::species::Species;

    fn create_test_pokemon(species: Species, moves: Vec<Move>) -> PokemonInst {
        let mut pokemon_moves = [const { None }; 4];
        for (i, mv) in moves.into_iter().enumerate() {
            if i < 4 {
                pokemon_moves[i] = Some(MoveInstance { move_: mv, pp: 20 }); // Increased PP to ensure tests work
            }
        }

        {
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
    }

    #[test]
    fn test_two_turn_move_charging() {
        // Test ChargeUp moves like Solar Beam
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Venusaur,
                vec![Move::SolarBeam], // move_index: 0
            )],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Charizard, vec![Move::TailWhip])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // --- TURN 1: Initiate Solar Beam ---
        // Player 1 uses Solar Beam, which should apply the Charging condition.
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        
        let npc_actions = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions {
            battle_state.action_queue[player_index] = Some(action);
        }

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);
        
        println!("Charging condition forcing behavior test events (turn 1):");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Assert that the Charging condition was applied after Turn 1.
        assert!(
            battle_state.players[0].has_condition(&PokemonCondition::Charging),
            "Player should be in a Charging state after the first turn."
        );

        // Assert that the last move was correctly recorded.
        assert_eq!(battle_state.players[0].last_move, Some(Move::SolarBeam));

        // --- REVISED TEST LOGIC FOR TURN 2 ---
        // With the new "End-of-Turn Injection" model, finalize_turn from Turn 1
        // should have already populated the action_queue for Turn 2.

        // NEW ASSERTION: The action queue for Player 0 should now be pre-filled with the forced move.
        assert!(
            battle_state.action_queue[0].is_some(),
            "Player 0's action queue should be PRE-FILLED with the forced SolarBeam action."
        );
        // NEW ASSERTION: Verify it's the correct action.
        assert_eq!(
            battle_state.action_queue[0],
            Some(PlayerAction::UseMove { move_index: 0 }),
            "The queued action for Player 0 should be SolarBeam."
        );

        // Collect actions for players who can still act (the opponent).
        let npc_actions_turn2 = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions_turn2 {
            battle_state.action_queue[player_index] = Some(action);
        }
        
        // The battle should now be ready for resolution as both queues are full.
        assert!(ready_for_turn_resolution(&battle_state), "Battle should be ready for Turn 2 resolution.");

        // Resolve Turn 2
        let test_rng2 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus2 = resolve_turn(&mut battle_state, test_rng2);
        
        println!("Charging condition forcing behavior test events (turn 2):");
        for event in event_bus2.events() {
            println!("  {:?}", event);
        }

        // Verify from the events that Solar Beam was used and dealt damage.
        let player_0_used_solar_beam = event_bus2.events().iter().any(|event| {
            matches!(
                event,
                BattleEvent::MoveUsed {
                    player_index: 0,
                    move_used: Move::SolarBeam,
                    ..
                }
            )
        });

        let opponent_took_damage = event_bus2.events().iter().any(|event| {
            matches!(
                event,
                BattleEvent::DamageDealt {
                    target: Species::Charizard,
                    ..
                }
            )
        });

        assert!(player_0_used_solar_beam, "Player 0 should have been forced to use Solar Beam on Turn 2.");
        assert!(opponent_took_damage, "Solar Beam should have dealt damage on Turn 2.");

        // Assert that the Charging condition was cleared after execution.
        assert!(
            !battle_state.players[0].has_condition(&PokemonCondition::Charging),
            "Charging condition should be cleared after the move executes."
        );
    }
    
     #[test]
    fn test_two_turn_move_fly() {
        // Test InAir moves like Fly
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Pidgeot, vec![Move::Fly])], // move_index: 0
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Rattata, vec![Move::Tackle])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // --- TURN 1: Initiate Fly ---
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        let npc_actions = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions {
            battle_state.action_queue[player_index] = Some(action);
        }
        
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        println!("Fly test events (Turn 1):");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Assert that the InAir condition was applied after Turn 1.
        assert!(
            battle_state.players[0].has_condition(&PokemonCondition::InAir),
            "Player should be InAir after first turn of Fly."
        );
        assert_eq!(
            battle_state.players[0].last_move,
            Some(Move::Fly),
            "Last move should be recorded as Fly."
        );

        // --- REVISED TEST LOGIC FOR TURN 2 ---
        // Assert that the action queue for Player 0 is now pre-filled with the forced move.
        assert!(
            battle_state.action_queue[0].is_some(),
            "Player 0's action queue should be PRE-FILLED with the forced Fly action."
        );
        assert_eq!(
            battle_state.action_queue[0],
            Some(PlayerAction::UseMove { move_index: 0 })
        );
        
        // Collect actions for the opponent.
        let npc_actions_2 = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions_2 {
            battle_state.action_queue[player_index] = Some(action);
        }
        
        assert!(ready_for_turn_resolution(&battle_state));
        
        // Resolve Turn 2
        let test_rng_2 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_2 = resolve_turn(&mut battle_state, test_rng_2);
        
        println!("\nFly test events (Turn 2):");
        for event in event_bus_2.events() {
            println!("  {:?}", event);
        }

        // Verify from the events that Fly was used and dealt damage.
        let player_0_used_fly = event_bus_2.events().iter().any(|event| {
            matches!(event, BattleEvent::MoveUsed { player_index: 0, move_used: Move::Fly, .. })
        });
        let opponent_took_damage = event_bus_2.events().iter().any(|event| {
            matches!(event, BattleEvent::DamageDealt { target: Species::Rattata, .. })
        });

        assert!(player_0_used_fly, "Player 0 should have been forced to use Fly on Turn 2.");
        assert!(opponent_took_damage, "Fly should have dealt damage on Turn 2.");

        // Assert that the InAir condition was cleared.
        assert!(
            !battle_state.players[0].has_condition(&PokemonCondition::InAir),
            "InAir condition should be cleared after Fly executes."
        );
    }

    #[test]
    fn test_two_turn_move_dig() {
        // Test Underground moves like Dig
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Sandslash, vec![Move::Dig])], // move_index: 0
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Geodude, vec![Move::RockThrow])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // --- TURN 1: Initiate Dig ---
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        let npc_actions = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions {
            battle_state.action_queue[player_index] = Some(action);
        }

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);
        
        println!("Dig test events (Turn 1):");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Assert that the Underground condition was applied after Turn 1.
        assert!(battle_state.players[0].has_condition(&PokemonCondition::Underground));
        assert_eq!(battle_state.players[0].last_move, Some(Move::Dig));

        // --- TEST LOGIC FOR TURN 2 ---
        // Assert that the action queue for Player 0 is now pre-filled with the forced move.
        assert!(
            battle_state.action_queue[0].is_some(),
            "Player 0's action queue should be PRE-FILLED with the forced Dig action."
        );
        assert_eq!(
            battle_state.action_queue[0],
            Some(PlayerAction::UseMove { move_index: 0 })
        );

        // Collect actions for the opponent.
        let npc_actions_2 = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions_2 {
            battle_state.action_queue[player_index] = Some(action);
        }
        
        assert!(ready_for_turn_resolution(&battle_state));
        
        // Resolve Turn 2
        let test_rng_2 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_2 = resolve_turn(&mut battle_state, test_rng_2);

        println!("\nDig test events (Turn 2):");
        for event in event_bus_2.events() {
            println!("  {:?}", event);
        }
        
        // Verify from the events that Dig was used and dealt damage.
        let player_0_used_dig = event_bus_2.events().iter().any(|event| {
            matches!(event, BattleEvent::MoveUsed { player_index: 0, move_used: Move::Dig, .. })
        });
        let opponent_took_damage = event_bus_2.events().iter().any(|event| {
            matches!(event, BattleEvent::DamageDealt { target: Species::Geodude, .. })
        });
        
        assert!(player_0_used_dig, "Player 0 should have been forced to use Dig on Turn 2.");
        assert!(opponent_took_damage, "Dig should have dealt damage on Turn 2.");

        // Assert that the Underground condition was cleared.
        assert!(
            !battle_state.players[0].has_condition(&PokemonCondition::Underground),
            "Underground condition should be cleared after Dig executes."
        );
    }

    #[test]
    fn test_rampage_move() {
        // Test Rampaging moves like Thrash
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Tauros, vec![Move::Thrash])], // move_index: 0
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Slowpoke, vec![Move::WaterGun])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // --- TURN 1: Initiate Thrash ---
        // Player 1 uses Thrash, which should apply the Rampaging condition.
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });

        let npc_actions = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions {
            battle_state.action_queue[player_index] = Some(action);
        }
        
        // The first RNG value determines rampage duration. <= 50 means 2 turns.
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);
        
        println!("Rampage test events (Turn 1):");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Assert that the Rampaging condition was applied after Turn 1.
        let has_rampage = battle_state.players[0]
            .active_pokemon_conditions
            .values()
            .any(|condition| matches!(condition, PokemonCondition::Rampaging { .. }));
        assert!(
            has_rampage,
            "Player should be Rampaging after using Thrash."
        );
        assert_eq!(
            battle_state.players[0].last_move,
            Some(Move::Thrash),
            "Last move should be recorded as Thrash."
        );

        // --- REVISED TEST LOGIC FOR TURN 2 ---
        // With the new model, finalize_turn from Turn 1 should have pre-filled the action queue.

        // NEW ASSERTION: The action queue for Player 0 should be pre-filled with the forced Thrash.
        assert!(
            battle_state.action_queue[0].is_some(),
            "Player 0's action queue should be PRE-FILLED with the forced Thrash action."
        );
        assert_eq!(
            battle_state.action_queue[0],
            Some(PlayerAction::UseMove { move_index: 0 })
        );
        
        // Collect actions for the opponent.
        let npc_actions_turn2 = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions_turn2 {
            battle_state.action_queue[player_index] = Some(action);
        }
        
        // The battle should now be ready for resolution.
        assert!(ready_for_turn_resolution(&battle_state));

        // Resolve Turn 2
        let test_rng_2 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_2 = resolve_turn(&mut battle_state, test_rng_2);
        
        println!("\nRampage test events (Turn 2):");
        for event in event_bus_2.events() {
            println!("  {:?}", event);
        }

        // Verify from the events that Thrash was used again by Player 0.
        let player_0_used_thrash = event_bus_2.events().iter().any(|event| {
            matches!(
                event,
                BattleEvent::MoveUsed {
                    player_index: 0,
                    move_used: Move::Thrash,
                    ..
                }
            )
        });

        assert!(
            player_0_used_thrash,
            "Player 0 should have been forced to use Thrash again on Turn 2"
        );
    }

    #[test]
    fn test_rampage_ends_with_confusion() {
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Meowth, vec![Move::Thrash])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Onix, vec![Move::Harden])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // --- Turn 1: Start the Rampage ---
        // Use an RNG value that will result in a 2-turn rampage for predictability.
        // The first roll is for Rampage duration (<= 50 means 2 turns).
        let turn_1_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        
        let npc_actions = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions {
            battle_state.action_queue[player_index] = Some(action);
        }
        let event_bus_1 = resolve_turn(&mut battle_state, turn_1_rng);
        for event in event_bus_1.events() {
            println!("  {:?}", event);
        }
        // Verify we are rampaging
        let is_rampaging = battle_state.players[0].active_pokemon_conditions.values().any(|c| matches!(c, PokemonCondition::Rampaging { turns_remaining: 1 }));
        assert!(is_rampaging, "Player 1 should be rampaging after Turn 1");
        
        // --- Turn 2: Continue Rampaging ---
        // The move is forced, so we only need to collect the AI's action.
        let npc_actions = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions {
            battle_state.action_queue[player_index] = Some(action);
        }
        let turn_2_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_2 = resolve_turn(&mut battle_state, turn_2_rng);

        // Verify the rampage counter is now 0, but the condition is still present until the end of the turn.
        let is_rampage_ending = battle_state.players[0].active_pokemon_conditions.values().any(|c| matches!(c, PokemonCondition::Rampaging { turns_remaining: 0 }));
        println!("\nRampage confusion test events (Turn 2):");
        for event in event_bus_2.events() {
            println!("  {:?}", event);
        }
        assert!(is_rampage_ending, "Rampage should be ending after Turn 2");

        // --- Turn 3: Rampage Ends, Confusion Begins ---
        let npc_actions = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions {
            battle_state.action_queue[player_index] = Some(action);
        }
        let turn_3_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]); // RNG for confusion check
        let event_bus_3 = resolve_turn(&mut battle_state, turn_3_rng);
        
        println!("\nRampage confusion test events (Turn 3):");
        for event in event_bus_3.events() {
            println!("  {:?}", event);
        }

        // Verify Rampaging condition is gone
        let is_still_rampaging = battle_state.players[0].active_pokemon_conditions.values().any(|c| matches!(c, PokemonCondition::Rampaging { .. }));
        assert!(!is_still_rampaging, "Rampaging condition should be removed after it ends.");

        // Verify Confused condition was applied
        let is_confused = battle_state.players[0].active_pokemon_conditions.values().any(|c| matches!(c, PokemonCondition::Confused { .. }));
        assert!(is_confused, "Player should become confused after rampage ends.");

        // Verify the correct event was emitted
        let confusion_applied_event = event_bus_3.events().iter().any(|event| {
            matches!(
                event,
                BattleEvent::StatusApplied {
                    target: Species::Meowth,
                    status: PokemonCondition::Confused { .. }
                }
            )
        });
        assert!(confusion_applied_event, "A StatusApplied event for Confusion should have been emitted.");
    }

    #[test]
    fn test_mirror_move_success() {
        // Test Mirror Move copying opponent's last move in a single turn
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Pidgeot,
                vec![Move::MirrorMove],
            )],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Lightning])],
        );

        // Set Player 2's last move to Lightning (as if they used it previously)
        player2.last_move = Some(Move::Lightning);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Mirror Move, Player 2 uses Lightning
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Mirror Move
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Lightning

        let test_rng = TurnRng::new_for_test(vec![
            50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50, 50,
        ]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Check what events were generated to debug the issue
        println!("Events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Mirror Move should have been executed (no ActionFailed events)
        let failed_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| matches!(event, BattleEvent::ActionFailed { .. }))
            .collect();

        // For debugging: if there are failed events, print them
        if !failed_events.is_empty() {
            println!("ActionFailed events found:");
            for event in &failed_events {
                println!("  {:?}", event);
            }
        }

        assert_eq!(
            failed_events.len(),
            0,
            "Mirror Move should not fail when copying a valid move"
        );
    }

    #[test]
    fn test_mirror_move_fail_mirror_move() {
        // Test Mirror Move failing when trying to copy Mirror Move
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Pidgeot,
                vec![Move::MirrorMove],
            )],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Fearow, vec![Move::MirrorMove])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Set Player 2's last move to Mirror Move
        battle_state.players[1].last_move = Some(Move::MirrorMove);

        // Player 1 uses Mirror Move
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Mirror Move
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Some other move

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Should have ActionFailed event
        let failed_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| matches!(event, BattleEvent::ActionFailed { .. }))
            .collect();
        assert!(
            !failed_events.is_empty(),
            "Mirror Move should fail when copying Mirror Move"
        );
    }

    #[test]
    fn test_mirror_move_fail_no_last_move() {
        // Test Mirror Move failing when opponent has no last move
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Pidgeot,
                vec![Move::MirrorMove],
            )],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Rattata, vec![Move::Tackle])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 2 has no last move (None)
        assert_eq!(battle_state.players[1].last_move, None);

        // Player 1 uses Mirror Move
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Mirror Move
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Should have ActionFailed event
        let failed_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| matches!(event, BattleEvent::ActionFailed { .. }))
            .collect();
        assert!(
            !failed_events.is_empty(),
            "Mirror Move should fail when no move to copy"
        );
    }

    #[test]
    fn test_explode_move() {
        // Test Explode effect - user faints then damage is dealt
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Electrode,
                vec![Move::Explosion],
            )],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Rest])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Explosion
        let npc_actions = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions {
            battle_state.action_queue[player_index] = Some(action);
        }
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Player 1 should have fainted
        assert!(
            battle_state.players[0]
                .active_pokemon()
                .unwrap()
                .is_fainted()
        );

        // Note: Explosion might miss, so we just check that the battle proceeded without error

        // Should have PokemonFainted event for Player 1
        let fainted_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::PokemonFainted {
                        player_index: 0,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            !fainted_events.is_empty(),
            "Player 1 should have fainted from Explosion"
        );
    }

    // === SPECIAL CONDITION TESTS ===
    // Tests for the 7 advanced special conditions implemented: Teleported, Transformed, Converted, Substitute, Countering, Enraged, Biding

    #[test]
    fn test_teleported_condition_causes_moves_to_miss() {
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Slowpoke, vec![Move::WaterGun])], // Slower Pokemon
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Abra, vec![Move::Tackle])], // Use a different move
        );

        // Apply Teleported condition to player 2 BEFORE the battle starts
        player2.add_condition(PokemonCondition::Teleported);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses WaterGun (has accuracy), Player 2 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // WaterGun
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        // Use RNG that would normally hit (low roll)
        let test_rng = TurnRng::new_for_test(vec![10, 10, 10, 10, 10, 10, 10, 10]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Teleported condition test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // WaterGun should miss because defender is Teleported
        let missed_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::MoveMissed {
                        attacker: Species::Slowpoke,
                        defender: Species::Abra,
                        move_used: Move::WaterGun
                    }
                )
            })
            .collect();
        assert!(
            !missed_events.is_empty(),
            "WaterGun should miss against Teleported Pokemon"
        );

        // Check that no damage was dealt to Abra from WaterGun
        let watergun_damage_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::DamageDealt {
                        target: Species::Abra,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            watergun_damage_events.is_empty(),
            "Teleported Pokemon should not take damage from missed moves"
        );

        // Teleported condition should expire at end of turn
        assert!(!battle_state.players[1].has_condition(&PokemonCondition::Teleported));
    }

    #[test]
    fn test_transformed_condition_uses_target_stats_and_types() {
        // Ditto vs Charizard - Ditto transforms into Charizard
        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Ditto, vec![Move::Transform])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Charizard, vec![Move::Ember])],
        );

        // Apply Transformed condition to Ditto (copying Charizard's stats/types)
        let charizard_inst = player2.active_pokemon().unwrap().clone();
        player1.add_condition(PokemonCondition::Transformed {
            target: charizard_inst,
        });

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Test that Ditto now gets STAB for Fire-type moves due to transformation
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Transform (shouldn't matter)
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Ember

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Transformed condition test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Test that transformed Ditto has Fire type
        let ditto_types = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .get_current_types(&battle_state.players[0]);
        assert!(
            ditto_types.contains(&crate::pokemon::PokemonType::Fire),
            "Transformed Ditto should have Fire type from Charizard"
        );
        assert!(
            ditto_types.contains(&crate::pokemon::PokemonType::Flying),
            "Transformed Ditto should have Flying type from Charizard"
        );
    }

    #[test]
    fn test_converted_condition_overrides_transform() {
        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Ditto, vec![Move::Transform])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Charizard, vec![Move::Ember])],
        );

        // Apply both Transformed and Converted conditions - Converted should take priority
        let charizard_inst = player2.active_pokemon().unwrap().clone();
        player1.add_condition(PokemonCondition::Transformed {
            target: charizard_inst,
        });
        player1.add_condition(PokemonCondition::Converted {
            pokemon_type: crate::pokemon::PokemonType::Electric,
        });

        let battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Test that Converted condition overrides Transformed
        let ditto_types = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .get_current_types(&battle_state.players[0]);
        assert_eq!(
            ditto_types.len(),
            1,
            "Converted Pokemon should have exactly one type"
        );
        assert_eq!(
            ditto_types[0],
            crate::pokemon::PokemonType::Electric,
            "Converted condition should override Transform - should be Electric type only"
        );
    }

    #[test]
    fn test_substitute_blocks_damage() {
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Lightning])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(
                Species::Alakazam,
                vec![Move::Substitute],
            )],
        );

        // Apply Substitute condition with 25 HP
        player2.add_condition(PokemonCondition::Substitute { hp: 25 });

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);
        let original_hp = battle_state.players[1]
            .active_pokemon()
            .unwrap()
            .current_hp();

        // Player 1 attacks with Lightning
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Lightning
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Substitute

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Substitute blocks damage test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Player 2's actual Pokemon should take no damage - Substitute should absorb it
        assert_eq!(
            battle_state.players[1]
                .active_pokemon()
                .unwrap()
                .current_hp(),
            original_hp,
            "Pokemon behind Substitute should take no damage"
        );
    }

    #[test]
    fn test_substitute_blocks_status_effects() {
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(
                Species::Pikachu,
                vec![Move::ThunderWave],
            )],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(
                Species::Alakazam,
                vec![Move::Substitute],
            )],
        );

        // Apply Substitute condition
        player2.add_condition(PokemonCondition::Substitute { hp: 25 });

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Thunder Wave (status move)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Thunder Wave
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Substitute

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Substitute blocks status effects test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Player 2's Pokemon should not be paralyzed
        let pokemon_status = battle_state.players[1].active_pokemon().unwrap().status;
        assert!(
            pokemon_status.is_none()
                || !matches!(
                    pokemon_status,
                    Some(crate::pokemon::StatusCondition::Paralysis)
                ),
            "Pokemon behind Substitute should not receive status effects"
        );
    }

    #[test]
    fn test_substitute_blocks_stat_decreases() {
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Pidgey, vec![Move::SandAttack])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(
                Species::Alakazam,
                vec![Move::Substitute],
            )],
        );

        // Apply Substitute condition
        player2.add_condition(PokemonCondition::Substitute { hp: 25 });

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);
        let original_accuracy =
            battle_state.players[1].get_stat_stage(crate::player::StatType::Accuracy);

        // Player 1 uses Sand Attack (lowers accuracy)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Sand Attack
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Substitute

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Substitute blocks stat decreases test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Player 2's accuracy should not be lowered
        let new_accuracy =
            battle_state.players[1].get_stat_stage(crate::player::StatType::Accuracy);
        assert_eq!(
            new_accuracy, original_accuracy,
            "Pokemon behind Substitute should not have stats lowered"
        );
    }

    #[test]
    fn test_substitute_blocks_active_conditions() {
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Hypno, vec![Move::ConfuseRay])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(
                Species::Alakazam,
                vec![Move::Substitute],
            )],
        );

        // Apply Substitute condition
        player2.add_condition(PokemonCondition::Substitute { hp: 25 });

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses ConfuseRay (causes Confused condition)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // ConfuseRay
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Substitute

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Substitute blocks active conditions test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Player 2 should not be confused
        assert!(
            !battle_state.players[1]
                .has_condition(&PokemonCondition::Confused { turns_remaining: 1 }),
            "Pokemon behind Substitute should not receive active conditions"
        );
    }

    #[test]
    fn test_countering_condition_immediate_retaliation() {
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Tackle])], // Physical move
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Hitmonlee, vec![Move::Counter])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);
        let initial_hp_p1 = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();

        // Turn 1: Player 2 uses Counter, Player 1 uses Tackle
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Counter

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Countering condition immediate retaliation test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Check that Countering condition was applied during the turn (visible in events)
        let status_applied_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::StatusApplied {
                        target: Species::Hitmonlee,
                        status: PokemonCondition::Countering { .. }
                    }
                )
            })
            .collect();
        assert!(
            !status_applied_events.is_empty(),
            "Countering condition should be applied when using Counter"
        );

        // Player 1 should have taken Counter damage (2x the physical damage dealt)
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
            "Should have Counter retaliation damage against Machamp"
        );

        // Check that Player 1's HP decreased (took Counter damage)
        let final_hp_p1 = battle_state.players[0]
            .active_pokemon()
            .unwrap()
            .current_hp();
        assert!(
            final_hp_p1 < initial_hp_p1,
            "Player 1 should have taken Counter retaliation damage"
        );

        // Countering condition should expire at end of turn (this is correct behavior)
        assert!(
            !battle_state.players[1].has_condition(&PokemonCondition::Countering { damage: 0 }),
            "Countering condition should expire at end of turn"
        );
    }

    #[test]
    fn test_counter_survival_requirement() {
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Machamp, vec![Move::Tackle])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Hitmonlee, vec![Move::Counter])],
        );

        // Set Player 2's HP to 1 so it will faint from Tackle
        player2.active_pokemon_mut().unwrap().set_hp(1);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 2 uses Counter, Player 1 uses Tackle (should KO Player 2)
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Counter

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Counter survival requirement test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Player 2 should be fainted
        assert!(
            battle_state.players[1]
                .active_pokemon()
                .unwrap()
                .is_fainted()
        );

        // Player 1 should not have taken Counter damage (since Player 2 fainted)
        let counter_damage_events: Vec<_> = event_bus
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

        // Should only have 1 damage event (from Tackle), no Counter retaliation
        assert_eq!(
            counter_damage_events.len(),
            0,
            "No Counter damage should occur if Countering Pokemon faints"
        );
    }

    #[test]
    fn test_enraged_condition_attack_increase() {
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Lightning])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Primeape, vec![Move::Rage])],
        );

        // Apply Enraged condition to Player 2
        player2.add_condition(PokemonCondition::Enraged);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);
        let original_attack_stage =
            battle_state.players[1].get_stat_stage(crate::player::StatType::Attack);

        // Player 1 attacks Player 2, Player 2 uses Rage
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Lightning
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Rage

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Enraged condition attack increase test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Player 2's attack should have increased after being hit while Enraged
        let new_attack_stage =
            battle_state.players[1].get_stat_stage(crate::player::StatType::Attack);
        assert!(
            new_attack_stage > original_attack_stage,
            "Enraged Pokemon should gain attack when hit (was {}, now {})",
            original_attack_stage,
            new_attack_stage
        );

        // Should have StatStageChanged event
        let stat_change_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::StatStageChanged {
                        target: Species::Primeape,
                        stat: crate::player::StatType::Attack,
                        ..
                    }
                )
            })
            .collect();
        assert!(
            !stat_change_events.is_empty(),
            "Should have StatStageChanged event for attack increase"
        );
    }

    #[test]
    fn test_enraged_condition_removed_when_using_non_rage_moves() {
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Lightning])],
        );

        let mut player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Primeape, vec![Move::Tackle])], // Non-Rage move
        );

        // Apply Enraged condition to Player 2
        player2.add_condition(PokemonCondition::Enraged);

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 2 uses Tackle (not Rage) - should remove Enraged condition
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Lightning
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Tackle

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Enraged removal test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Player 2 should no longer be Enraged
        assert!(
            !battle_state.players[1].has_condition(&PokemonCondition::Enraged),
            "Enraged condition should be removed when using non-Rage moves"
        );

        // Should have StatusRemoved event
        let status_removed_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    BattleEvent::StatusRemoved {
                        target: Species::Primeape,
                        status: PokemonCondition::Enraged
                    }
                )
            })
            .collect();
        assert!(
            !status_removed_events.is_empty(),
            "Should have StatusRemoved event for Enraged condition"
        );
    }

    #[test]
    fn test_biding_condition_forcing_behavior() {
        // Test that Bide forces the user to continue biding on subsequent turns.
        let player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Bide])], // move_index: 0
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Tackle])],
        );

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // --- TURN 1: Initiate Bide ---
        // Player 1 uses Bide, which should apply the Biding condition and start storing energy.
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        let npc_actions = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions {
            battle_state.action_queue[player_index] = Some(action);
        }
        
        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        println!("Biding condition forcing behavior test events (turn 1):");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Assert that the Biding condition was applied after Turn 1.
        let has_biding = battle_state.players[0]
            .active_pokemon_conditions
            .values()
            .any(|condition| matches!(condition, PokemonCondition::Biding { .. }));
        assert!(
            has_biding,
            "Player 1 should have Biding condition after using Bide"
        );

        // --- REVISED TEST LOGIC FOR TURN 2 ---
        // With the new model, finalize_turn from Turn 1 should have pre-filled the action queue.

        // NEW ASSERTION: The action queue for Player 0 should be pre-filled with the forced Bide.
        assert!(
            battle_state.action_queue[0].is_some(),
            "Player 0's action queue should be PRE-FILLED with the forced Bide action."
        );
        assert_eq!(
            battle_state.action_queue[0],
            Some(PlayerAction::UseMove { move_index: 0 })
        );
        
        // Collect actions for the opponent. Player 0 is already locked in.
        let npc_actions_2 = collect_npc_actions(&battle_state);
        for (player_index, action) in npc_actions_2 {
            battle_state.action_queue[player_index] = Some(action);
        }
        
        assert!(
            battle_state.action_queue[1].is_some(),
            "Player 2 (AI) should have a chosen action."
        );
        assert!(ready_for_turn_resolution(&battle_state));

        // Execute Turn 2.
        let test_rng_2 = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus_2 = resolve_turn(&mut battle_state, test_rng_2);
        
        println!("\nBiding condition forcing behavior test events (turn 2):");
        for event in event_bus_2.events() {
            println!("  {:?}", event);
        }

        // Verify from the events that Bide was used by Player 0 again.
        // This confirms that the engine's internal logic correctly executed the pre-filled forced move.
        let player_0_used_bide = event_bus_2.events().iter().any(|event| {
            matches!(
                event,
                BattleEvent::MoveUsed {
                    player_index: 0,
                    move_used: Move::Bide,
                    ..
                }
            )
        });

        assert!(player_0_used_bide, "Player 0 should have been forced to use Bide on Turn 2");
    }
    #[test]
    fn test_bide_execution_deals_double_stored_damage() {
        let mut player1 = BattlePlayer::new(
            "player1".to_string(),
            "Player 1".to_string(),
            vec![create_test_pokemon(Species::Snorlax, vec![Move::Bide])],
        );

        let player2 = BattlePlayer::new(
            "player2".to_string(),
            "Player 2".to_string(),
            vec![create_test_pokemon(Species::Pikachu, vec![Move::Lightning])],
        );

        // Apply Biding condition with 0 turns left (ready to execute) and some stored damage
        player1.add_condition(PokemonCondition::Biding {
            turns_remaining: 0,
            damage: 50,
        });

        let mut battle_state = BattleState::new("test_battle".to_string(), player1, player2);

        // Player 1 uses Bide (final turn), Player 2 attacks
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 }); // Bide
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 }); // Lightning

        let test_rng = TurnRng::new_for_test(vec![50, 50, 50, 50, 50, 50, 50, 50]);
        let event_bus = resolve_turn(&mut battle_state, test_rng);

        // Print all events for clarity
        println!("Bide execution deals double damage test events:");
        for event in event_bus.events() {
            println!("  {:?}", event);
        }

        // Biding condition should be gone after execution
        assert!(
            !battle_state.players[0]
                .active_pokemon_conditions
                .values()
                .any(|condition| { matches!(condition, PokemonCondition::Biding { .. }) }),
            "Biding condition should be removed after execution"
        );

        // Player 2 should have taken damage equal to 2x stored damage (100)
        let bide_damage_events: Vec<_> = event_bus
            .events()
            .iter()
            .filter_map(|event| match event {
                BattleEvent::DamageDealt {
                    target: Species::Pikachu,
                    damage,
                    ..
                } => Some(*damage),
                _ => None,
            })
            .collect::<Vec<_>>();

        // Should have damage events, and at least one should be the Bide retaliation (100 damage)
        assert!(
            !bide_damage_events.is_empty(),
            "Should have damage events from Bide execution"
        );

        // Look for the high damage value that indicates Bide retaliation (2x stored = 100)
        let has_bide_retaliation = bide_damage_events.iter().any(|&damage| damage >= 90); // Allow some variance for critical hits
        assert!(
            has_bide_retaliation,
            "Should have high damage from Bide retaliation (2x stored damage)"
        );
    }
}
