use serde::{Deserialize, Serialize};

/// Tracks which Pokemon have faced each other during battle
/// participation[player][my_pokemon][opponent_pokemon] = true if they faced each other
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleParticipationTracker {
    participation: [[[bool; 6]; 6]; 2],
}

impl BattleParticipationTracker {
    pub fn new() -> Self {
        Self {
            participation: [[[false; 6]; 6]; 2],
        }
    }

    /// Record that the active Pokemon from each player faced each other
    pub fn record_participation(&mut self, p0_active: usize, p1_active: usize) {
        if p0_active < 6 && p1_active < 6 {
            self.participation[0][p0_active][p1_active] = true;
            self.participation[1][p1_active][p0_active] = true;
        }
    }

    /// Get all Pokemon from the opposing player who faced the specified opponent Pokemon
    pub fn get_participants_against(
        &self,
        opponent_player: usize,
        opponent_pokemon: usize,
    ) -> Vec<usize> {
        if opponent_player >= 2 || opponent_pokemon >= 6 {
            return Vec::new();
        }

        let participant_player = 1 - opponent_player;
        (0..6)
            .filter(|&pokemon_index| {
                self.participation[participant_player][pokemon_index][opponent_pokemon]
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_participation_tracking() {
        let mut tracker = BattleParticipationTracker::new();

        // Record that Pokemon 0 from player 0 faced Pokemon 1 from player 1
        tracker.record_participation(0, 1);

        // Check participants
        let participants = tracker.get_participants_against(1, 1); // Who faced player 1's Pokemon 1?
        assert_eq!(participants, vec![0]); // Player 0's Pokemon 0

        let participants2 = tracker.get_participants_against(0, 0); // Who faced player 0's Pokemon 0?
        assert_eq!(participants2, vec![1]); // Player 1's Pokemon 1
    }
}
