#[cfg(test)]
mod tests {
    use crate::battle::action_stack::ActionStack;
    use crate::battle::engine::execute_attack_hit;
    use crate::battle::state::{BattleEvent, EventBus, TurnRng};
    use crate::battle::tests::common::{create_test_battle, TestPokemonBuilder};
    use crate::pokemon::StatusCondition;
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use schema::Move;

    #[rstest]
    // --- BURN ---
    #[case("Ember succeeds on non-Fire type", Move::Ember, Species::Bulbasaur, None, 5, true)]
    #[case("Ember does not burn on high roll", Move::Ember, Species::Bulbasaur, None, 15, false)]
    #[case("Ember does not burn on Fire-type (immunity)", Move::Ember, Species::Arcanine, None, 5, false)]
    #[case("Ember does not burn on already-poisoned target", Move::Ember, Species::Bulbasaur, Some(StatusCondition::Poison(0)), 5, false)]
    // --- PARALYSIS ---
    #[case("Discharge succeeds on non-Electric type", Move::Discharge, Species::Squirtle, None, 5, true)]
    #[case("Discharge does not paralyze on high roll", Move::Discharge, Species::Squirtle, None, 35, false)]
    #[case("Discharge does not paralyze on Electric-type (immunity)", Move::Discharge, Species::Jolteon, None, 25, false)]
    #[case("Discharge does not paralyze on already-burned target", Move::Discharge, Species::Squirtle, Some(StatusCondition::Burn), 25, false)]
    // --- POISON ---
    #[case("Poison Sting succeeds on non-Poison type", Move::PoisonSting, Species::Rattata, None, 15, true)]
    #[case("Poison Sting does not poison on high roll", Move::PoisonSting, Species::Rattata, None, 25, false)]
    #[case("Poison Sting does not poison on Poison-type (immunity)", Move::PoisonSting, Species::Weezing, None, 15, false)]
    #[case("Poison Sting does not poison on already-sleeping target", Move::PoisonSting, Species::Rattata, Some(StatusCondition::Sleep(2)), 15, false)]
    // --- FREEZE ---
    #[case("Ice Beam succeeds on non-Ice type", Move::IceBeam, Species::Pidgey, None, 8, true)]
    #[case("Ice Beam does not freeze on high roll", Move::IceBeam, Species::Pidgey, None, 12, false)]
    #[case("Ice Beam does not freeze on Ice-type (immunity)", Move::IceBeam, Species::Lapras, None, 8, false)]
    #[case("Ice Beam does not freeze on already-paralyzed target", Move::IceBeam, Species::Pidgey, Some(StatusCondition::Paralysis), 8, false)]
    // --- SLEEP (Note: Ghosts are immune to sleep) ---
    #[case("Sing succeeds on target", Move::Sing, Species::Snorlax, None, 50, true)]
    #[case("Sing does not sedate on high roll", Move::Sing, Species::Snorlax, None, 60, false)]
    #[case("Sing does not sedate on Ghost-type (immunity)", Move::Sing, Species::Gengar, None, 8, false)]
    #[case("Sing fails on already-frozen target", Move::Sing, Species::Snorlax, Some(StatusCondition::Freeze), 50, false)]
    fn test_status_application_outcomes(
        #[case] desc: &str,
        #[case] attacker_move: Move,
        #[case] defender_species: Species,
        #[case] initial_defender_status: Option<StatusCondition>,
        #[case] rng_roll: u8,
        #[case] expect_status: bool,
    ) {
        // Arrange

        let attacker = TestPokemonBuilder::new(Species::Gengar, 5)
            .with_moves(vec![attacker_move])
            .build();

        let mut defender_builder = TestPokemonBuilder::new(defender_species, 50);
        if let Some(status) = initial_defender_status {
            defender_builder = defender_builder.with_status(status);
        }
        let defender = defender_builder.build();

        let mut battle_state = create_test_battle(attacker, defender);
        let mut bus = EventBus::new();
        let mut action_stack = ActionStack::new();
        // The first roll is for hit chance, the second is for the status effect.
        let mut rng = TurnRng::new_for_test(vec![50, rng_roll, 50, 50, 50, 50, 50, 50, 50, 50, 50]);
        // Act
        execute_attack_hit(
            0,
            1,
            attacker_move,
            0,
            &mut action_stack,
            &mut bus,
            &mut rng,
            &mut battle_state,
        );

        // Assert
        bus.print_debug_with_message(&format!("[{}]", desc));

        let final_status_is_some = battle_state.players[1].active_pokemon().unwrap().status.is_some();
        let status_applied_event_found = bus.events().iter().any(|e| matches!(e, BattleEvent::PokemonStatusApplied { .. }));

        if expect_status {
            assert!(final_status_is_some, "Defender should have a status condition");
            assert!(status_applied_event_found, "A PokemonStatusApplied event should have been emitted");
        } else {
            // If there was an initial status, the status should still be Some. Otherwise, it should be None.
            assert_eq!(final_status_is_some, initial_defender_status.is_some(), "Defender's status should not have changed");
            assert!(!status_applied_event_found, "No PokemonStatusApplied event should have been emitted");
        }
    }
}