use crate::{ActiveSlot, PokemonIndex};
use serde::{Deserialize, Serialize};

/// Static targeting behavior of a move in the pinned Showdown data.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MoveTarget {
    AdjacentAlly,
    AdjacentAllyOrSelf,
    AdjacentFoe,
    All,
    AllAdjacent,
    AllAdjacentFoes,
    Allies,
    AllySide,
    AllyTeam,
    Any,
    FoeSide,
    Normal,
    RandomNormal,
    Scripted,
    #[serde(rename = "self")]
    Self_,
}

/// A concrete target choice, or [`Self::Automatic`] for moves that do not
/// accept a player-selected target.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "slot", rename_all = "snake_case")]
pub enum ActionTarget {
    Automatic,
    #[serde(rename = "self")]
    Self_,
    Ally(ActiveSlot),
    Foe(ActiveSlot),
}

/// Stable zero-based index into an active Pokémon's move slots.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct MoveSlotIndex(u8);

impl MoveSlotIndex {
    #[must_use]
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// One active-slot decision supplied by a player.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PokemonAction {
    Move {
        move_slot: MoveSlotIndex,
        target: ActionTarget,
        mega: bool,
    },
    /// Used only when every ordinary move is disabled or out of PP.
    Struggle,
    Switch {
        pokemon: PokemonIndex,
    },
    /// Represents an active position for which no decision is possible.
    Pass,
}

/// The two ordered decisions for the left and right positions of one side.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TurnChoice {
    pub actions: [PokemonAction; 2],
}

/// Legal decisions for one active position.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SlotLegalActions {
    pub slot: ActiveSlot,
    pub actions: Vec<PokemonAction>,
}

/// Per-position actions and all valid combined doubles choices for one side.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LegalActions {
    pub slots: [SlotLegalActions; 2],
    pub choices: Vec<TurnChoice>,
}
