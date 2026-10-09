use crate::ConditionId;
use serde::{Deserialize, Serialize};

/// State shared by timed weather, terrain, side, and volatile conditions.
///
/// `turns_remaining` is absent for effects whose duration is not fixed when
/// they are created. Effect behavior and duration updates are later milestones.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionState {
    pub id: ConditionId,
    pub turns_remaining: Option<u8>,
}

impl ConditionState {
    #[must_use]
    pub const fn new(id: ConditionId, turns_remaining: Option<u8>) -> Self {
        Self {
            id,
            turns_remaining,
        }
    }
}

/// Mutable effects applying to the whole battlefield.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldState {
    pub weather: Option<ConditionState>,
    pub terrain: Option<ConditionState>,
    pub conditions: Vec<ConditionState>,
}
