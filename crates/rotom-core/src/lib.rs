//! Core types for the Rotom Spark battle engine.
//!
//! Battle state and mechanics are intentionally deferred to later milestones.

mod error;
mod id;
mod side;
mod team;

pub use error::BattleError;
pub use id::{AbilityId, FormId, ItemId, MoveId, NatureId, SpeciesId};
pub use side::SideId;
pub use team::{Gender, PokemonSet, StatAllocation, TeamDefinition};
