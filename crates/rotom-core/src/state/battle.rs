use super::{FieldState, SideState};
use crate::{BattleError, PokemonPosition, SideId};
use serde::{Deserialize, Serialize};

/// The explicit point in the battle lifecycle at which choices or resolution occur.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BattlePhase {
    TeamPreview,
    TurnSelection,
    ResolvingTurn,
    ForcedSwitch,
    BattleEnd,
}

/// Complete authoritative state for one battle.
///
/// Queued actions and deterministic RNG state are added by their respective
/// milestones. They are intentionally not represented by placeholders here.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleState {
    pub turn: u32,
    pub picked_team_size: u8,
    pub sides: [SideState; 2],
    pub field: FieldState,
    pub phase: BattlePhase,
    pub winner: Option<SideId>,
}

impl BattleState {
    /// Creates a battle at team preview with an empty field.
    ///
    /// # Errors
    ///
    /// Returns an error if the format's picked-team size cannot supply two
    /// doubles leads, exceeds either loaded team, or either side is not in its
    /// initial pre-preview state.
    pub fn new(p1: SideState, p2: SideState, picked_team_size: usize) -> Result<Self, BattleError> {
        let picked_team_size = u8::try_from(picked_team_size).map_err(|_| {
            BattleError::InvalidState("picked-team size does not fit in a u8".to_owned())
        })?;
        if picked_team_size < 2 {
            return Err(BattleError::InvalidState(
                "a doubles battle must pick at least two Pokémon".to_owned(),
            ));
        }
        for (side_id, side) in [(SideId::P1, &p1), (SideId::P2, &p2)] {
            if side.team.members.len() < usize::from(picked_team_size) {
                return Err(BattleError::InvalidState(format!(
                    "{side_id:?} has fewer Pokémon than the picked-team size"
                )));
            }
            if side.active != [None, None] || !side.selected_team.is_empty() {
                return Err(BattleError::InvalidState(format!(
                    "{side_id:?} is not in its initial team-preview state"
                )));
            }
            if side.team.members.iter().any(|pokemon| {
                pokemon.position != PokemonPosition::Team
                    || pokemon.current_hp == 0
                    || pokemon.current_hp > pokemon.stats.hp
            }) {
                return Err(BattleError::InvalidState(format!(
                    "{side_id:?} has an invalid Pokémon state before team preview"
                )));
            }
        }

        Ok(Self {
            turn: 0,
            picked_team_size,
            sides: [p1, p2],
            field: FieldState::default(),
            phase: BattlePhase::TeamPreview,
            winner: None,
        })
    }

    /// Returns one side by its stable player identifier.
    #[must_use]
    pub const fn side(&self, side: SideId) -> &SideState {
        match side {
            SideId::P1 => &self.sides[0],
            SideId::P2 => &self.sides[1],
        }
    }

    /// Returns one mutable side by its stable player identifier.
    #[must_use]
    pub const fn side_mut(&mut self, side: SideId) -> &mut SideState {
        match side {
            SideId::P1 => &mut self.sides[0],
            SideId::P2 => &mut self.sides[1],
        }
    }

    /// Whether the battle ended without a winning side.
    #[must_use]
    pub const fn is_tie(&self) -> bool {
        matches!(self.phase, BattlePhase::BattleEnd) && self.winner.is_none()
    }
}
