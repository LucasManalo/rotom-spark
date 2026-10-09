//! Core types for the Rotom Spark battle engine.
//!
//! Battle state and mechanics are intentionally deferred to later milestones.

mod error;
mod id;
mod side;

pub use error::BattleError;
pub use id::{AbilityId, FormId, ItemId, MoveId, SpeciesId};
pub use side::SideId;
