//! Core types and authoritative runtime state for the Rotom Spark battle engine.
//!
//! The state in this crate is structured, serializable, and cloneable. The
//! phase-aware [`step`] function is the canonical deterministic advancement
//! API; its diagnostics never replace authoritative battle state.

mod action;
mod combat;
mod conditions;
mod error;
mod event;
mod field_effects;
mod id;
mod kernel;
mod rng;
mod side;
mod state;
mod team;
mod transition;

pub use action::{
    ActionTarget, LegalActions, MoveSlotIndex, MoveTarget, PokemonAction, SlotLegalActions,
    TurnChoice,
};
pub use combat::{
    calculate_damage, calculate_move_damage, CombatData, CombatMove, DamageResult, EffectFraction,
    ExpiredCondition, ExpiredVolatile, FieldHpChange, HitSequence, Modifier, MoveAccuracy,
    MoveCapabilities, MoveCapability, MoveCategory, MoveIntrinsic, MoveOperation, PokemonType,
    PokemonTypes, PrimaryMoveEffect, PriorMoveEffect, ResidualDamage, ResolvedAction,
    SecondaryMoveEffect, SelfMoveEffect, TargetOutcome, TargetResult, TurnResolution,
};
pub use error::BattleError;
pub use event::{
    BattleEvent, EffectCallback, EffectInvocation, EffectOrder, EffectOwner, EffectRegistry,
    EffectSource, EffectState, EventContext, EventContract, EventDispatch, EventRelay, EventValue,
    PokemonRef,
};
pub use field_effects::FieldEffectIds;
pub use id::{AbilityId, ConditionId, FormId, ItemId, MoveId, NatureId, SpeciesId};
pub use rng::{BattleRng, RngOperation, RngReason, RngTraceEntry};
pub use side::SideId;
pub use state::{
    ActionOrder, ActionQueue, ActiveSlot, BattlePhase, BattleStat, BattleState, BattleStateBuilder,
    BattleStats, ConditionState, FaintTarget, FieldState, MajorStatus, MoveSlot, PokemonIndex,
    PokemonPosition, PokemonState, QueuedAction, ReplacementRequest, SideState, StatStageChange,
    StatStages, StatusState, TeamState,
};
pub use team::{Gender, PokemonSet, StatAllocation, TeamDefinition};
pub use transition::{step, JointAction, Mechanics, StepDiagnostics, StepResult, TeamSelection};
