use crate::ConditionId;
use serde::{Deserialize, Serialize};

/// State shared by timed weather, terrain, side, and volatile conditions.
///
/// `turns_remaining` is absent for effects whose duration is not fixed when
/// they are created. Timed conditions are decremented during residual upkeep.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionState {
    pub id: ConditionId,
    pub turns_remaining: Option<u8>,
    /// Stack count for layered conditions such as entry hazards.
    ///
    /// Ordinary weather, terrain, rooms, screens, and volatiles use one.
    #[serde(default = "default_condition_layers")]
    pub layers: u8,
}

impl ConditionState {
    #[must_use]
    pub const fn new(id: ConditionId, turns_remaining: Option<u8>) -> Self {
        Self {
            id,
            turns_remaining,
            layers: 1,
        }
    }

    /// Creates a layered condition. Mechanics decide the permitted maximum;
    /// the state representation only requires a nonzero layer count.
    #[must_use]
    pub const fn with_layers(id: ConditionId, turns_remaining: Option<u8>, layers: u8) -> Self {
        Self {
            id,
            turns_remaining,
            layers,
        }
    }
}

const fn default_condition_layers() -> u8 {
    1
}

/// Mutable effects applying to the whole battlefield.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldState {
    pub weather: Option<ConditionState>,
    pub terrain: Option<ConditionState>,
    pub conditions: Vec<ConditionState>,
}
