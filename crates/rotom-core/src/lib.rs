//! Core types and authoritative runtime state for the Rotom Spark battle engine.
//!
//! Turn resolution and battle mechanics are intentionally deferred to later
//! milestones. The state in this crate is structured, serializable, and
//! cloneable so those mechanics do not need to rely on text logs.

mod action;
mod combat;
mod error;
mod event;
mod id;
mod rng;
mod side;
mod state;
mod team;

pub use action::{
    ActionTarget, LegalActions, MoveSlotIndex, MoveTarget, PokemonAction, SlotLegalActions,
    TurnChoice,
};
pub use combat::{
    calculate_damage, calculate_move_damage, CombatData, CombatMove, DamageResult, Modifier,
    MoveAccuracy, MoveCategory, MoveSupport, PokemonType, PokemonTypes, ResolvedAction,
    TurnResolution,
};
pub use error::BattleError;
pub use event::{
    BattleEvent, EffectCallback, EffectInvocation, EffectOrder, EffectOwner, EffectRegistry,
    EffectSource, EventContext, EventDispatch, EventValue, PokemonRef,
};
pub use id::{AbilityId, ConditionId, FormId, ItemId, MoveId, NatureId, SpeciesId};
pub use rng::{BattleRng, RngOperation, RngReason, RngTraceEntry};
pub use side::SideId;
pub use state::{
    ActionOrder, ActionQueue, ActiveSlot, BattlePhase, BattleState, BattleStats, ConditionState,
    FaintTarget, FieldState, MajorStatus, MoveSlot, PokemonIndex, PokemonPosition, PokemonState,
    QueuedAction, ReplacementRequest, SideState, StatStages, StatusState, TeamState,
};
pub use team::{Gender, PokemonSet, StatAllocation, TeamDefinition};
