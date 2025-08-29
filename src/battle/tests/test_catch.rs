use crate::battle::action_stack::{ActionStack, BattleAction};
use crate::battle::catch::{calculate_catch_commands, can_attempt_catch, CatchError};
use crate::battle::engine::execute_battle_action;
use crate::battle::state::{BattleEvent, BattleState, BattleType, EventBus, TurnRng};
use crate::battle::tests::common::TestPokemonBuilder;
use crate::player::{BattlePlayer, PlayerType};
use crate::pokemon::StatusCondition;
use crate::species::Species;
use rstest::rstest;

fn create_wild_battle(player_team_size: usize, opponent_fainted: bool) -> BattleState {
    create_wild_battle_with_opponent(player_team_size, opponent_fainted, Species::Charmander)
}

fn create_wild_battle_with_opponent(
    player_team_size: usize,
    opponent_fainted: bool,
    opponent_species: Species,
) -> BattleState {
    // Create player with variable team size
    let mut player_team = vec![];
    for _i in 0..player_team_size {
        let pikachu = TestPokemonBuilder::new(Species::Pikachu, 25).build();
        player_team.push(pikachu);
    }

    // Create wild opponent
    let mut opponent_pokemon = TestPokemonBuilder::new(opponent_species, 25).build();

    if opponent_fainted {
        opponent_pokemon.take_damage(opponent_pokemon.current_hp());
    }

    let player = BattlePlayer::new_with_player_type(
        "player".to_string(),
        "Trainer".to_string(),
        player_team,
        PlayerType::Human,
    );

    let wild_opponent = BattlePlayer::new_with_player_type(
        "wild".to_string(),
        "Wild Pokémon".to_string(),
        vec![opponent_pokemon],
        PlayerType::NPC,
    );

    let mut battle_state = BattleState::new("catch_test".to_string(), player, wild_opponent);
    battle_state.battle_type = BattleType::Wild;
    battle_state
}

#[rstest]
#[case(
    BattleType::Wild,
    1,
    false,
    Ok(Species::Charmander),
    "Wild battle with space should succeed"
)]
#[case(
    BattleType::Safari,
    1,
    false,
    Ok(Species::Charmander),
    "Safari battle with space should succeed"
)]
#[case(BattleType::Trainer, 1, false, Err(CatchError::InvalidBattleType { battle_type: BattleType::Trainer }), "Trainer battle should fail")]
#[case(BattleType::Tournament, 1, false, Err(CatchError::InvalidBattleType { battle_type: BattleType::Tournament }), "Tournament battle should fail")]
#[case(
    BattleType::Wild,
    6,
    false,
    Err(CatchError::TeamFull),
    "Full team should fail"
)]
#[case(BattleType::Wild, 1, true, Err(CatchError::TargetFainted { pokemon: Species::Charmander }), "Fainted target should fail")]
fn test_catch_validation(
    #[case] battle_type: BattleType,
    #[case] team_size: usize,
    #[case] opponent_fainted: bool,
    #[case] expected: Result<Species, CatchError>,
    #[case] description: &str,
) {
    let mut battle_state = create_wild_battle(team_size, opponent_fainted);
    battle_state.battle_type = battle_type;

    let result = can_attempt_catch(&battle_state, 0);
    assert_eq!(result, expected, "{}", description);
}

#[rstest]
#[case(BattleType::Wild, 1, vec![1], true, "Low roll should succeed")]
#[case(BattleType::Wild, 1, vec![255], false, "High roll should fail")]
#[case(BattleType::Trainer, 1, vec![1], false, "Trainer battle should fail immediately")]
#[case(BattleType::Wild, 6, vec![1], false, "Full team should fail immediately")]
fn test_catch_commands(
    #[case] battle_type: BattleType,
    #[case] team_size: usize,
    #[case] rng_values: Vec<u8>,
    #[case] should_succeed: bool,
    #[case] description: &str,
) {
    let mut battle_state = create_wild_battle(team_size, false);
    battle_state.battle_type = battle_type;
    let mut rng = TurnRng::new_for_test(rng_values);

    let commands = calculate_catch_commands(0, Species::Charmander, &battle_state, &mut rng);

    if should_succeed {
        assert_eq!(
            commands.len(),
            2,
            "{}: Should have CatchAttempted + AttemptCatch",
            description
        );

        // First should be CatchAttempted event
        assert!(
            matches!(
                &commands[0],
                crate::battle::commands::BattleCommand::EmitEvent(
                    BattleEvent::CatchAttempted { .. }
                )
            ),
            "{}: First command should be CatchAttempted",
            description
        );

        // Second should be AttemptCatch command
        assert!(
            matches!(
                &commands[1],
                crate::battle::commands::BattleCommand::AttemptCatch { .. }
            ),
            "{}: Second command should be AttemptCatch",
            description
        );
    } else if battle_type == BattleType::Wild && team_size < 6 {
        // Failed roll case
        assert_eq!(
            commands.len(),
            2,
            "{}: Should have CatchAttempted + CatchFailed",
            description
        );

        assert!(
            matches!(
                &commands[1],
                crate::battle::commands::BattleCommand::EmitEvent(BattleEvent::CatchFailed { .. })
            ),
            "{}: Second command should be CatchFailed",
            description
        );
    } else {
        // Validation failure case
        assert_eq!(
            commands.len(),
            1,
            "{}: Should only have CatchFailed",
            description
        );

        assert!(
            matches!(
                &commands[0],
                crate::battle::commands::BattleCommand::EmitEvent(BattleEvent::CatchFailed { .. })
            ),
            "{}: Should have immediate CatchFailed",
            description
        );
    }
}

#[rstest]
#[case(vec![1], true, "Low roll should result in successful catch")]
#[case(vec![255], false, "High roll should result in failed catch")]
fn test_catch_action_execution(
    #[case] rng_values: Vec<u8>,
    #[case] should_succeed: bool,
    #[case] description: &str,
) {
    let mut battle_state = create_wild_battle(1, false);
    let mut action_stack = ActionStack::new();
    let mut event_bus = EventBus::new();
    let mut rng = TurnRng::new_for_test(rng_values);

    let catch_action = BattleAction::CatchAttempt { player_index: 0 };

    execute_battle_action(
        catch_action,
        &mut battle_state,
        &mut action_stack,
        &mut event_bus,
        &mut rng,
    );

    let events: Vec<_> = event_bus.events().iter().collect();
    assert!(!events.is_empty(), "{}: Should have events", description);

    // Should always have CatchAttempted event
    let catch_attempted = events
        .iter()
        .any(|e| matches!(e, BattleEvent::CatchAttempted { .. }));
    assert!(
        catch_attempted,
        "{}: Should have CatchAttempted event",
        description
    );

    let player = &battle_state.players[0];
    let team_count = player.team.iter().filter(|p| p.is_some()).count();

    if should_succeed {
        let catch_succeeded = events
            .iter()
            .any(|e| matches!(e, BattleEvent::CatchSucceeded { .. }));
        assert!(
            catch_succeeded,
            "{}: Should have CatchSucceeded event",
            description
        );

        assert_eq!(
            team_count, 2,
            "{}: Should have original + caught Pokemon",
            description
        );

        let caught_pokemon = player.team.iter().find(|p| {
            p.as_ref()
                .map_or(false, |pokemon| pokemon.species == Species::Charmander)
        });
        assert!(
            caught_pokemon.is_some(),
            "{}: Charmander should be in team",
            description
        );
    } else {
        let catch_failed = events
            .iter()
            .any(|e| matches!(e, BattleEvent::CatchFailed { .. }));
        assert!(
            catch_failed,
            "{}: Should have CatchFailed event",
            description
        );

        assert_eq!(
            team_count, 1,
            "{}: Should only have original Pokemon",
            description
        );
    }
}

#[rstest]
#[case(
    Some(StatusCondition::Sleep(3)),
    Species::Caterpie,
    50.0,
    "Sleep condition should significantly boost catch rate"
)]
#[case(
    Some(StatusCondition::Paralysis),
    Species::Caterpie,
    30.0,
    "Paralysis should moderately boost catch rate"
)]
#[case(
    None,
    Species::Caterpie,
    25.0,
    "High base catch rate should still be decent"
)]
fn test_catch_rates_with_modifiers(
    #[case] status: Option<StatusCondition>,
    #[case] species: Species,
    #[case] min_expected_rate: f32,
    #[case] description: &str,
) {
    let mut battle_state = create_wild_battle_with_opponent(1, false, species);

    // Apply status condition if specified
    if let Some(status_condition) = status {
        let opponent = &mut battle_state.players[1];
        if let Some(ref mut pokemon) = opponent.team[0] {
            pokemon.status = Some(status_condition);
        }
    }

    let mut rng = TurnRng::new_for_test(vec![100]);
    let commands = calculate_catch_commands(0, species, &battle_state, &mut rng);

    let catch_rate =
        if let crate::battle::commands::BattleCommand::EmitEvent(BattleEvent::CatchAttempted {
            catch_rate,
            ..
        }) = &commands[0]
        {
            *catch_rate
        } else {
            panic!("Expected CatchAttempted event for {}", description);
        };

    assert!(
        catch_rate >= min_expected_rate,
        "{}: got {}, expected >= {}",
        description,
        catch_rate,
        min_expected_rate
    );
}

#[test]
fn test_catch_rates_with_low_hp() {
    // Test that low HP significantly boosts catch rate
    let mut battle_state = create_wild_battle_with_opponent(1, false, Species::Caterpie);

    // Damage the target Pokemon to 1 HP
    let opponent = &mut battle_state.players[1];
    if let Some(ref mut pokemon) = opponent.team[0] {
        let damage = pokemon.current_hp() - 1;
        pokemon.take_damage(damage);
    }

    let mut rng = TurnRng::new_for_test(vec![100]);
    let commands = calculate_catch_commands(0, Species::Caterpie, &battle_state, &mut rng);

    let catch_rate =
        if let crate::battle::commands::BattleCommand::EmitEvent(BattleEvent::CatchAttempted {
            catch_rate,
            ..
        }) = &commands[0]
        {
            *catch_rate
        } else {
            panic!("Expected CatchAttempted event");
        };

    assert!(
        catch_rate > 50.0,
        "Low HP should significantly boost catch rate, got: {}",
        catch_rate
    );
}

#[test]
fn test_catch_event_formatting() {
    let battle_state = create_wild_battle(1, false);

    // Test CatchAttempted event formatting
    let catch_attempted = BattleEvent::CatchAttempted {
        player_index: 0,
        pokemon: Species::Charmander,
        catch_rate: 75.5,
    };

    let formatted = catch_attempted.format(&battle_state);
    assert!(formatted.is_some());
    let text = formatted.unwrap();
    assert!(text.contains("Trainer"));
    assert!(text.contains("Charmander"));
    assert!(text.contains("Poké Ball"));

    // Test CatchSucceeded event formatting
    let catch_succeeded = BattleEvent::CatchSucceeded {
        player_index: 0,
        pokemon: Species::Charmander,
    };

    let formatted = catch_succeeded.format(&battle_state);
    assert!(formatted.is_some());
    let text = formatted.unwrap();
    assert!(text.contains("Gotcha!"));
    assert!(text.contains("Charmander"));

    // Test CatchFailed event formatting
    let catch_failed = BattleEvent::CatchFailed {
        player_index: 0,
        pokemon: Species::Charmander,
        reason: crate::battle::state::CatchFailureReason::RollFailed { catch_rate: 75.5 },
    };

    let formatted = catch_failed.format(&battle_state);
    assert!(formatted.is_some());
    let text = formatted.unwrap();
    assert!(text.contains("broke free"));
    assert!(text.contains("Charmander"));
}
