use super::{FieldState, SideState};
use crate::SideId;
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
    pub sides: [SideState; 2],
    pub field: FieldState,
    pub phase: BattlePhase,
    pub winner: Option<SideId>,
}

impl BattleState {
    /// Creates a battle at team preview with an empty field.
    #[must_use]
    pub fn new(p1: SideState, p2: SideState) -> Self {
        Self {
            turn: 0,
            sides: [p1, p2],
            field: FieldState::default(),
            phase: BattlePhase::TeamPreview,
            winner: None,
        }
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
}
