#[cfg(test)]
mod tests {
    use crate::battle::conditions::PokemonCondition;
    use crate::battle::state::{BattleEvent, BattleState, EventBus, GameState, TurnRng};
    use crate::move_data::initialize_move_data;
    use crate::player::BattlePlayer;
    use crate::pokemon::{PokemonInst, StatusCondition, get_species_data, initialize_species_data};
    use crate::species::Species;
    use std::path::Path;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init_test_data() {
        INIT.call_once(|| {
            let data_path = Path::new("data");
            initialize_move_data(data_path).expect("Failed to initialize move data");
            initialize_species_data(data_path).expect("Failed to initialize species data");
        });
    }

    fn create_test_battle_state() -> BattleState {
        let pikachu_data = get_species_data(Species::Pikachu).expect("Failed to load Pikachu data");
        let charmander_data =
            get_species_data(Species::Charmander).expect("Failed to load Charmander data");

        let pikachu = PokemonInst::new(Species::Pikachu, &pikachu_data, 25, None, None);
        let charmander = PokemonInst::new(Species::Charmander, &charmander_data, 25, None, None);

        let player1 = BattlePlayer::new("p1".to_string(), "Player 1".to_string(), vec![pikachu]);
        let player2 = BattlePlayer::new("p2".to_string(), "Player 2".to_string(), vec![charmander]);

        BattleState {
            battle_id: "test".to_string(),
            players: [player1, player2],
            turn_number: 1,
            game_state: GameState::TurnInProgress,
            action_queue: [None, None],
        }
    }

    #[test]
    fn test_poison_status_damage() {
        init_test_data();

        // Create a test Pokemon with poison status
        let species_data =
            get_species_data(Species::Charmander).expect("Failed to load Charmander data");
        let mut pokemon = PokemonInst::new(Species::Charmander, &species_data, 50, None, None);

        let initial_hp = pokemon.max_hp();
        let max_hp = pokemon.max_hp();

        // Apply regular poison (severity 0)
        pokemon.status = Some(StatusCondition::Poison(0));

        // Deal status damage - should deal 1/16 of max HP damage
        let (damage, status_changed) = pokemon.deal_status_damage();

        assert_eq!(
            damage,
            (max_hp / 16).max(1),
            "Regular poison should deal 1/16 of max hp"
        );
        assert!(
            !status_changed,
            "Status should not change when dealing poison damage."
        );
        assert_eq!(
            pokemon.current_hp(),
            initial_hp - damage,
            "Hp should be decreased by {} damage",
            damage
        );
        assert!(
            matches!(pokemon.status, Some(StatusCondition::Poison(0))),
            "The pokemon should still be poisoned."
        );
    }

    #[test]
    fn test_badly_poisoned_status_damage() {
        init_test_data();

        // --- Setup ---
        let species_data =
            get_species_data(Species::Bulbasaur).expect("Failed to load Bulbasaur data");
        let mut pokemon = PokemonInst::new(Species::Bulbasaur, &species_data, 50, None, None);
        let initial_hp = pokemon.current_hp();
        let max_hp = pokemon.max_hp();
        pokemon.status = Some(StatusCondition::Poison(1)); // Start with "badly poisoned" severity 1

        // --- Action ---
        // Deal damage based on current severity (1)
        let (damage, status_changed) = pokemon.deal_status_damage();

        // --- Verification ---
        let expected_damage = (max_hp * 1 / 16).max(1); // Should be severity 1, not 2

        // If this fails, the test will print our custom message.
        assert_eq!(
            damage, expected_damage,
            "Damage dealt ({}) did not match the expected value ({}) for severity 1",
            damage, expected_damage
        );

        assert!(
            !status_changed,
            "Status should not change when dealing poison damage"
        );

        assert_eq!(
            pokemon.current_hp(),
            initial_hp - damage,
            "Pokémon's current HP is incorrect after taking damage"
        );

        // For `matches!`, you have to wrap it in `assert!` to add a message.
        assert!(
            matches!(pokemon.status, Some(StatusCondition::Poison(1))),
            "Pokémon status should still be Poison(1) after damage, but was {:?}",
            pokemon.status
        );
    }

    #[test]
    fn test_burn_status_damage() {
        init_test_data();

        // Create a test Pokemon with burn status
        let species_data =
            get_species_data(Species::Charmander).expect("Failed to load Charmander data");
        let mut pokemon = PokemonInst::new(Species::Charmander, &species_data, 50, None, None);

        let initial_hp = pokemon.max_hp();
        let max_hp = pokemon.max_hp();

        // Apply burn
        pokemon.status = Some(StatusCondition::Burn);

        // Deal status damage - should deal 1/8 of max HP damage
        let (damage, _) = pokemon.deal_status_damage();

        assert_eq!(
            damage,
            (max_hp / 8).max(1),
            "Damage should be at most 1/8 of max hp."
        );
        assert_eq!(
            pokemon.current_hp(),
            initial_hp - damage,
            "Hp was not updated correctly."
        );
        assert!(
            matches!(pokemon.status, Some(StatusCondition::Burn)),
            "The pokemon's status should remain burned."
        );
    }

    #[test]
    fn test_sleep_status_countdown() {
        init_test_data();

        // Create a test Pokemon with sleep status
        let species_data = get_species_data(Species::Snorlax).expect("Failed to load Snorlax data");
        let mut pokemon = PokemonInst::new(Species::Snorlax, &species_data, 50, None, None);

        let initial_hp = pokemon.max_hp();

        // Apply sleep for 3 turns
        pokemon.status = Some(StatusCondition::Sleep(3));

        // First progress update - should reduce to 2 turns, no wake up
        let (should_cure, status_changed) = pokemon.update_status_progress();
        assert!(!should_cure);
        assert!(status_changed);
        assert_eq!(pokemon.current_hp(), initial_hp); // No damage from sleep
        assert!(matches!(pokemon.status, Some(StatusCondition::Sleep(2))));

        // Second progress update - should reduce to 1 turn
        let (should_cure, status_changed) = pokemon.update_status_progress();
        assert!(!should_cure);
        assert!(status_changed);
        assert!(matches!(pokemon.status, Some(StatusCondition::Sleep(1))));

        // Third progress update - should reduce to 0 turn
        let (should_cure, status_changed) = pokemon.update_status_progress();
        assert!(!should_cure);
        assert!(status_changed);
        assert!(matches!(pokemon.status, Some(StatusCondition::Sleep(0))));

        // Fourth progress update - should wake up (starting at 0)
        let (should_cure, status_changed) = pokemon.update_status_progress();
        assert!(should_cure);
        assert!(status_changed);
        assert!(pokemon.status.is_none());
    }

    #[test]
    fn test_active_condition_timers() {
        init_test_data();

        let species_data = get_species_data(Species::Pikachu).expect("Failed to load Pikachu data");
        let pokemon = PokemonInst::new(Species::Pikachu, &species_data, 25, None, None);
        let mut player = BattlePlayer::new("test".to_string(), "Test".to_string(), vec![pokemon]);

        // Add some conditions with timers
        player.add_condition(PokemonCondition::Confused { turns_remaining: 3 });
        player.add_condition(PokemonCondition::Trapped { turns_remaining: 2 });
        player.add_condition(PokemonCondition::Flinched); // Should expire after 1 turn

        // First tick
        let expired = player.tick_active_conditions();
        assert_eq!(expired.len(), 1); // Flinched should expire
        assert!(expired.contains(&PokemonCondition::Flinched));

        // Check remaining conditions with actual values
        assert!(!player.has_condition(&PokemonCondition::Flinched));

        // Check confused condition has 2 turns remaining
        let confused_key = PokemonCondition::Confused { turns_remaining: 2 };
        match player.get_condition(&confused_key) {
            Some(condition) => {
                if let PokemonCondition::Confused { turns_remaining } = condition {
                    assert_eq!(*turns_remaining, 2);
                } else {
                    panic!("Expected Confused condition, got: {:?}", condition);
                }
            }
            None => {
                println!(
                    "Available conditions: {:?}",
                    player.active_pokemon_conditions
                );
                panic!("Confused condition should exist with 2 turns remaining");
            }
        }

        // Check trapped condition has 1 turn remaining
        if let Some(PokemonCondition::Trapped { turns_remaining }) =
            player.get_condition(&PokemonCondition::Trapped { turns_remaining: 1 })
        {
            assert_eq!(*turns_remaining, 1);
        } else {
            panic!("Trapped condition should exist with 1 turn remaining");
        }

        // Second tick - Trapped goes 1->0 but doesn't expire yet
        let expired = player.tick_active_conditions();
        assert_eq!(expired.len(), 0); // No conditions expire this tick

        // Check confused condition now has 1 turn remaining
        if let Some(PokemonCondition::Confused { turns_remaining }) =
            player.get_condition(&PokemonCondition::Confused { turns_remaining: 1 })
        {
            assert_eq!(*turns_remaining, 1);
        } else {
            panic!("Confused condition should exist with 1 turn remaining");
        }

        // Check trapped condition now has 0 turns remaining (but still active)
        if let Some(PokemonCondition::Trapped { turns_remaining }) =
            player.get_condition(&PokemonCondition::Trapped { turns_remaining: 0 })
        {
            assert_eq!(*turns_remaining, 0);
        } else {
            panic!("Trapped condition should exist with 0 turns remaining");
        }

        // Third tick - Trapped should expire (0 -> gone), Confused goes 1->0
        let expired = player.tick_active_conditions();
        assert_eq!(expired.len(), 1); // Trapped should expire
        assert!(expired.contains(&PokemonCondition::Trapped { turns_remaining: 0 }));

        // Check confused condition now has 0 turns remaining (but still active)
        if let Some(PokemonCondition::Confused { turns_remaining }) =
            player.get_condition(&PokemonCondition::Confused { turns_remaining: 0 })
        {
            assert_eq!(*turns_remaining, 0);
        } else {
            panic!("Confused condition should exist with 0 turns remaining");
        }

        // Fourth tick - Confused should expire (0 -> gone)
        let expired = player.tick_active_conditions();
        assert_eq!(expired.len(), 1); // Confused should expire
        assert!(expired.contains(&PokemonCondition::Confused { turns_remaining: 0 }));

        // Should have no active conditions now
        assert_eq!(player.active_pokemon_conditions.len(), 0);
    }

    #[test]
    fn test_status_damage_causes_fainting() {
        init_test_data();

        // Create a low HP Pokemon
        let species_data =
            get_species_data(Species::Magikarp).expect("Failed to load Magikarp data");
        let mut pokemon = PokemonInst::new(Species::Magikarp, &species_data, 5, None, None);

        // Set HP very low
        pokemon.set_hp(3);

        // Apply poison - damage should be enough to faint
        pokemon.status = Some(StatusCondition::Poison(0));
        let expected_damage = (pokemon.max_hp() / 16).max(1);

        // If damage >= current HP, should faint
        let (damage, status_changed) = pokemon.deal_status_damage();

        if expected_damage >= 3 {
            // Should faint
            assert_eq!(damage, expected_damage);
            assert!(status_changed); // Status changed to Faint
            assert_eq!(pokemon.current_hp(), 0);
            assert!(matches!(pokemon.status, Some(StatusCondition::Faint)));
        } else {
            // Should survive
            assert_eq!(pokemon.current_hp(), 3 - damage);
            assert!(matches!(pokemon.status, Some(StatusCondition::Poison(0))));
        }
    }

    #[test]
    fn test_frozen_defrost_25_percent_chance() {
        init_test_data();

        // Test successful defrost when Pokemon tries to act
        use crate::battle::engine::{ActionStack, BattleAction};
        use crate::moves::Move;

        let mut battle_state = create_test_battle_state();
        let pokemon = battle_state.players[0].team[battle_state.players[0].active_pokemon_index]
            .as_mut()
            .unwrap();
        pokemon.status = Some(StatusCondition::Freeze);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![24, 100, 100, 100]); // Below 25% threshold - should defrost, plus extra values for other RNG calls

        // Test defrost by trying to execute an attack (this will call check_action_preventing_conditions)
        let mut action_stack = ActionStack::new();
        crate::battle::engine::execute_battle_action(
            BattleAction::AttackHit {
                attacker_index: 0,
                defender_index: 1,
                move_used: Move::Tackle,
                hit_number: 0,
            },
            &mut battle_state,
            &mut action_stack,
            &mut bus,
            &mut rng,
        );

        // Pokemon should be defrosted
        let pokemon = battle_state.players[0].team[battle_state.players[0].active_pokemon_index]
            .as_ref()
            .unwrap();
        assert_eq!(pokemon.status, None);

        // Should have StatusRemoved event
        let events = bus.events();
        assert!(events.iter().any(|e| matches!(
            e,
            BattleEvent::PokemonStatusRemoved {
                target: Species::Pikachu,
                status: StatusCondition::Freeze
            }
        )));
    }

    #[test]
    fn test_frozen_no_defrost_75_percent_chance() {
        init_test_data();

        // Test failed defrost when Pokemon tries to act
        use crate::battle::engine::{ActionStack, BattleAction};
        use crate::moves::Move;

        let mut battle_state = create_test_battle_state();
        let pokemon = battle_state.players[0].team[battle_state.players[0].active_pokemon_index]
            .as_mut()
            .unwrap();
        pokemon.status = Some(StatusCondition::Freeze);

        let mut bus = EventBus::new();
        let mut rng = TurnRng::new_for_test(vec![25, 100, 100, 100]); // At 25% threshold - should remain frozen, plus extra values

        // Test freeze check by trying to execute an attack (this will call check_action_preventing_conditions)
        let mut action_stack = ActionStack::new();
        crate::battle::engine::execute_battle_action(
            BattleAction::AttackHit {
                attacker_index: 0,
                defender_index: 1,
                move_used: Move::Tackle,
                hit_number: 0,
            },
            &mut battle_state,
            &mut action_stack,
            &mut bus,
            &mut rng,
        );

        // Pokemon should still be frozen
        let pokemon = battle_state.players[0].team[battle_state.players[0].active_pokemon_index]
            .as_ref()
            .unwrap();
        assert_eq!(pokemon.status, Some(StatusCondition::Freeze));

        // Should not have StatusRemoved event for freeze
        let events = bus.events();
        assert!(!events.iter().any(|e| matches!(
            e,
            BattleEvent::PokemonStatusRemoved {
                status: StatusCondition::Freeze,
                ..
            }
        )));
    }
}
