use crate::battle::state::{BattleState, TurnRng};
use crate::errors::BattleResult;
use crate::player::BattlePlayer;
use crate::pokemon::{PokemonInst, StatusCondition};
use crate::species::Species;
use pokemon_adventure_schema::Move;

/// A builder for creating test Pokemon instances with common defaults.
///
/// # Example
/// ```
/// let pokemon = TestPokemonBuilder::new(Species::Pikachu, 25)
///     .with_moves(vec![Move::Tackle])
///     .with_status(StatusCondition::Paralysis)
///     .build();
/// ```
pub struct TestPokemonBuilder {
    species: Species,
    level: u8,
    moves: Option<Vec<Move>>,
    status: Option<StatusCondition>,
    current_hp: Option<u16>,
}

impl TestPokemonBuilder {
    /// Creates a new builder for a given species and level.
    pub fn new(species: Species, level: u8) -> Self {
        Self {
            species,
            level,
            moves: None,
            status: None,
            current_hp: None,
        }
    }

    /// Sets the moves for the test Pokemon.
    pub fn with_moves(mut self, moves: Vec<Move>) -> Self {
        self.moves = Some(moves);
        self
    }

    /// Sets the status condition for the test Pokemon.
    pub fn with_status(mut self, status: StatusCondition) -> Self {
        self.status = Some(status);
        self
    }

    /// Sets the current HP for the test Pokemon. If not set, HP will be max.
    pub fn with_hp(mut self, hp: u16) -> Self {
        self.current_hp = Some(hp);
        self
    }

    /// Builds the `PokemonInst`.
    pub fn build(self) -> PokemonInst {
        let species_data = match crate::pokemon::get_species_data(self.species) {
            Ok(data) => data,
            Err(err) => panic!(
                "Failed to load species data for {:?}: {}",
                self.species, err
            ),
        };

        let mut pokemon = PokemonInst::new(
            self.species,
            &species_data,
            self.level,
            None, // Use default IVs for tests
            self.moves,
        );

        pokemon.status = self.status;

        if let Some(hp) = self.current_hp {
            pokemon.set_hp(hp);
        } else {
            pokemon.set_hp_to_max();
        }

        pokemon
    }
}

/// Creates a default test player with a given ID, name, and team.
pub fn create_test_player(id: &str, name: &str, team: Vec<PokemonInst>) -> BattlePlayer {
    BattlePlayer::new(id.to_string(), name.to_string(), team)
}

/// Creates a standard 1v1 battle state for testing.
pub fn create_test_battle(p1_pokemon: PokemonInst, p2_pokemon: PokemonInst) -> BattleState {
    let player1 = create_test_player("p1", "Player 1", vec![p1_pokemon]);
    let player2 = create_test_player("p2", "Player 2", vec![p2_pokemon]);

    BattleState::new("test_battle".to_string(), player1, player2)
}

/// Creates a `TurnRng` instance with a long list of default values (50).
/// Useful for tests where the specific RNG outcome is not important, preventing panics from exhaustion.
pub fn predictable_rng() -> TurnRng {
    TurnRng::new_for_test(vec![50; 100]) // Provide a generous buffer of RNG values
}

/// Helper function to assert that a Result is Ok and return the value.
/// Provides clear error messages in tests when functions unexpectedly fail.
pub fn assert_ok<T>(result: BattleResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => panic!("Expected Ok but got error: {}", err),
    }
}

/// Helper function to assert that a boolean Result is Ok and true.
/// Useful for testing boolean-returning battle functions.
pub fn assert_ok_true(result: BattleResult<bool>) -> bool {
    let value = assert_ok(result);
    assert!(value, "Expected true but got false");
    value
}

/// Helper function to assert that a boolean Result is Ok and false.
/// Useful for testing boolean-returning battle functions.
pub fn assert_ok_false(result: BattleResult<bool>) -> bool {
    let value = assert_ok(result);
    assert!(!value, "Expected false but got true");
    value
}
