#[cfg(test)]
mod tests {
    use crate::battle::engine::resolve_turn;
    use crate::battle::state::BattleEvent;
    use crate::battle::tests::common::{create_test_battle, predictable_rng, TestPokemonBuilder};
    use crate::player::PlayerAction;
    use crate::species::Species;
    use pretty_assertions::assert_eq;
    use schema::Move;

    #[test]
    fn test_pp_decrements_on_use() {
        // Arrange
        let p1_pokemon = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Tackle])
            .build();
        let initial_pp = p1_pokemon.moves[0].as_ref().unwrap().pp;

        let p2_pokemon = TestPokemonBuilder::new(Species::Charmander, 25)
            .with_moves(vec![Move::Scratch])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let _ = resolve_turn(&mut battle_state, predictable_rng());

        // Assert
        let final_pp = battle_state.players[0].active_pokemon().unwrap().moves[0]
            .as_ref()
            .unwrap()
            .pp;
        assert_eq!(
            final_pp,
            initial_pp - 1,
            "PP should decrement by 1 after a move is used."
        );
    }

    #[test]
    fn test_forced_struggle_when_out_of_pp() {
        // Arrange: Pikachu has only Tackle with 1 PP.
        let mut p1_pokemon = TestPokemonBuilder::new(Species::Pikachu, 25)
            .with_moves(vec![Move::Tackle])
            .build();
        p1_pokemon.moves[0].as_mut().unwrap().pp = 1;

        let p2_pokemon = TestPokemonBuilder::new(Species::Charmander, 25)
            .with_moves(vec![Move::Scratch])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        // --- Turn 1: Use the last PP ---
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        let _ = resolve_turn(&mut battle_state, predictable_rng());
        assert_eq!(
            battle_state.players[0].active_pokemon().unwrap().moves[0]
                .as_ref()
                .unwrap()
                .pp,
            0,
            "Tackle should have 0 PP after the first turn."
        );

        // --- Turn 2: Attempt to use the same move, expecting Struggle ---
        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });
        let bus2 = resolve_turn(&mut battle_state, predictable_rng());

        bus2.print_debug_with_message("Events for test_forced_struggle_when_out_of_pp:");

        let p1_used_struggle = bus2.events().iter().any(|e| {
            matches!(e, BattleEvent::MoveUsed { player_index: 0, move_used, .. } if *move_used == Move::Struggle)
        });
        let p1_used_tackle = bus2.events().iter().any(|e| {
            matches!(e, BattleEvent::MoveUsed { player_index: 0, move_used, .. } if *move_used == Move::Tackle)
        });

        assert!(
            p1_used_struggle,
            "Player 1 should have used Struggle after running out of PP."
        );
        assert!(
            !p1_used_tackle,
            "Player 1 should NOT have used Tackle on the second turn."
        );
    }

    #[test]
    fn test_struggle_mechanics_damage_and_recoil() {
        // Arrange: Attacker has 0 PP. Defender is a Ghost-type to test immunity bypass.
        let mut p1_pokemon = TestPokemonBuilder::new(Species::Snorlax, 50)
            .with_moves(vec![Move::HyperBeam])
            .build();
        p1_pokemon.moves[0].as_mut().unwrap().pp = 0;

        let p2_pokemon = TestPokemonBuilder::new(Species::Gastly, 50)
            .with_moves(vec![Move::Lick])
            .build();
        let mut battle_state = create_test_battle(p1_pokemon, p2_pokemon);

        battle_state.action_queue[0] = Some(PlayerAction::UseMove { move_index: 0 });
        battle_state.action_queue[1] = Some(PlayerAction::UseMove { move_index: 0 });

        // Act
        let event_bus = resolve_turn(&mut battle_state, predictable_rng());

        event_bus.print_debug_with_message("Events for test_struggle_mechanics_damage_and_recoil:");

        // Assert
        let mut damage_to_defender = 0;
        let mut recoil_to_attacker = 0;

        for event in event_bus.events() {
            if let BattleEvent::DamageDealt { target, damage, .. } = event {
                if *target == Species::Gastly {
                    damage_to_defender = *damage;
                } else if *target == Species::Snorlax {
                    recoil_to_attacker = *damage;
                }
            }
        }

        assert!(
            damage_to_defender > 0,
            "Struggle should deal damage to a Ghost-type."
        );
        assert!(
            recoil_to_attacker > 0,
            "Struggle should cause recoil damage to the user."
        );

        // Struggle recoil is 50% of damage dealt. Round up.
        println!("Damage to defender: {}, Recoil to attacker: {}", damage_to_defender, recoil_to_attacker);
        let expected_recoil = (damage_to_defender as f32 * 0.50).ceil() as u16;
        assert_eq!(
            recoil_to_attacker, expected_recoil,
            "Recoil damage should be 50% of the damage dealt."
        );
    }
}
