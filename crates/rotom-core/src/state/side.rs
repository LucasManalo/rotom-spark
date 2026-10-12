use super::{ConditionState, PokemonIndex, PokemonState, TeamState};
use serde::{Deserialize, Serialize};

/// Complete mutable state belonging to one player.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SideState {
    pub team: TeamState,
    pub active: [Option<PokemonIndex>; 2],
    pub selected_team: Vec<PokemonIndex>,
    pub side_conditions: Vec<ConditionState>,
    /// True after this side has completed its once-per-battle Mega Evolution.
    pub mega_evolution_used: bool,
}

impl SideState {
    /// Creates a side before team-preview choices have been made.
    #[must_use]
    pub fn new(team: TeamState) -> Self {
        Self {
            team,
            active: [None, None],
            selected_team: Vec::new(),
            side_conditions: Vec::new(),
            mega_evolution_used: false,
        }
    }

    #[must_use]
    pub fn pokemon(&self, index: PokemonIndex) -> Option<&PokemonState> {
        self.team.pokemon(index)
    }

    #[must_use]
    pub(crate) fn pokemon_mut(&mut self, index: PokemonIndex) -> Option<&mut PokemonState> {
        self.team.pokemon_mut(index)
    }
}
