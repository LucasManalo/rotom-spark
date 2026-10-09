//! Core types and authoritative runtime state for the Rotom Spark battle engine.
//!
//! Turn resolution and battle mechanics are intentionally deferred to later
//! milestones. The state in this crate is structured, serializable, and
//! cloneable so those mechanics do not need to rely on text logs.

mod error;
mod id;
mod side;
mod state;
mod team;

pub use error::BattleError;
pub use id::{AbilityId, ConditionId, FormId, ItemId, MoveId, NatureId, SpeciesId};
pub use side::SideId;
pub use state::{
    ActiveSlot, BattlePhase, BattleState, BattleStats, ConditionState, FieldState, MajorStatus,
    MoveSlot, PokemonIndex, PokemonPosition, PokemonState, SideState, StatStages, StatusState,
    TeamState,
};
pub use team::{Gender, PokemonSet, StatAllocation, TeamDefinition};
