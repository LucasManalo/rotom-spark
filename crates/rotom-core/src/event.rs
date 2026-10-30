use crate::{
    kernel::StateKernel, AbilityId, BattleError, BattleRng, BattleStat, BattleState, ConditionId,
    ConditionState, ItemId, MajorStatus, MoveCategory, MoveId, MoveInteractionFlags, PokemonIndex,
    PokemonPosition, PokemonType, PokemonTypes, RngReason, SideId, StatStageChange, StatusState,
    TerrainEnvironment, WeatherEnvironment,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Stable hook points through which battle effects intercept engine operations.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleEvent {
    BeforeTurn,
    BeforeMove,
    ModifyPriority,
    /// Relays sub-priority within one integer priority bracket.
    ModifyFractionalPriority,
    ModifySpeed,
    /// Relays the effective offensive stat after move-specific selection.
    ModifyAttack,
    /// Relays the effective Special Attack stat after move-specific selection.
    ModifySpecialAttack,
    /// Relays the effective defensive stat after move-specific selection.
    ModifyDefense,
    /// Relays the source's Defense when a move uses it offensively.
    ModifySourceDefense,
    /// Relays the effective Special Defense stat after move-specific selection.
    ModifySpecialDefense,
    /// Relays the source's Special Defense when a move uses it offensively.
    ModifySourceSpecialDefense,
    /// Relays base power after state-dependent move behavior.
    ModifyBasePower,
    /// Relays final numeric accuracy after stage and field modifiers.
    ModifyAccuracy,
    /// Relays the move's critical-hit stage before the RNG check.
    ModifyCritRatio,
    /// Confirms a rolled or forced critical hit against the target.
    CriticalHit,
    ModifyDamage,
    /// Relays a requested stat-stage delta before the kernel applies it.
    TryStatStageChange,
    /// Reports the kernel's actual applied stat-stage delta.
    AfterStatStageChange,
    TryHit,
    /// Relays move-specific immunity checks after `TryHit` and type immunity.
    TryImmunity,
    TrySetStatus,
    /// Runs after a major status is installed and move-scoped suppression ends.
    AfterSetStatus,
    Hit,
    AfterDamage,
    AfterMove,
    SwitchIn,
    SwitchOut,
    Faint,
    /// Relays whether the holder is affected by the active weather.
    WeatherImmunity,
    Residual,
}

impl BattleEvent {
    /// Declares the relay and cancellation semantics enforced by dispatch.
    #[must_use]
    pub const fn contract(self) -> EventContract {
        match self {
            Self::BeforeMove | Self::SwitchOut => EventContract::new(EventRelay::None, true, false),
            Self::TryHit | Self::TryImmunity | Self::TrySetStatus | Self::WeatherImmunity => {
                EventContract::new(EventRelay::Boolean, true, true)
            }
            Self::ModifyPriority
            | Self::ModifyFractionalPriority
            | Self::ModifySpeed
            | Self::ModifyAttack
            | Self::ModifySpecialAttack
            | Self::ModifyDefense
            | Self::ModifySourceDefense
            | Self::ModifySpecialDefense
            | Self::ModifySourceSpecialDefense
            | Self::ModifyBasePower
            | Self::ModifyAccuracy
            | Self::ModifyCritRatio
            | Self::ModifyDamage
            | Self::TryStatStageChange => EventContract::new(EventRelay::Integer, false, true),
            Self::CriticalHit => EventContract::new(EventRelay::Boolean, false, true),
            Self::AfterDamage | Self::AfterStatStageChange => {
                EventContract::new(EventRelay::Integer, false, false)
            }
            Self::BeforeTurn
            | Self::Hit
            | Self::AfterSetStatus
            | Self::AfterMove
            | Self::SwitchIn
            | Self::Faint
            | Self::Residual => EventContract::new(EventRelay::None, false, false),
        }
    }
}

/// Relay shape accepted by an event contract.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventRelay {
    None,
    Boolean,
    Integer,
}

/// Static rules for one event's relay and early termination behavior.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventContract {
    pub relay: EventRelay,
    pub cancellable: bool,
    pub mutable_relay: bool,
}

impl EventContract {
    #[must_use]
    pub const fn new(relay: EventRelay, cancellable: bool, mutable_relay: bool) -> Self {
        Self {
            relay,
            cancellable,
            mutable_relay,
        }
    }

    fn validate_value(self, event: BattleEvent, value: EventValue) -> Result<(), BattleError> {
        let valid = matches!(
            (self.relay, value),
            (EventRelay::None, EventValue::None)
                | (EventRelay::Boolean, EventValue::Boolean(_))
                | (EventRelay::Integer, EventValue::Integer(_))
        );
        if valid {
            Ok(())
        } else {
            Err(BattleError::InvalidState(format!(
                "{event:?} received an incompatible relay value"
            )))
        }
    }
}

/// Static identity of behavior that may register one or more event handlers.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum EffectSource {
    Battle,
    Ability(AbilityId),
    Item(ItemId),
    Move(MoveId),
    MajorStatus(MajorStatus),
    Condition(ConditionId),
}

/// A Pokémon participating in an event without borrowing battle state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PokemonRef {
    pub side: SideId,
    pub pokemon: PokemonIndex,
}

impl PokemonRef {
    #[must_use]
    pub const fn new(side: SideId, pokemon: PokemonIndex) -> Self {
        Self { side, pokemon }
    }
}

/// Runtime state holder from which a registered effect was collected.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EffectOwner {
    Battle,
    Field,
    Side { side: SideId },
    Pokemon { pokemon: PokemonRef },
    Move { pokemon: PokemonRef },
}

/// Central ordering metadata for effect handlers.
///
/// Handlers sort by lower `order`, higher `priority`, holder Speed, lower
/// `sub_order`, and finally registration order. This matches Showdown's normal
/// event-listener key direction while making the final tie-break deterministic.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EffectOrder {
    pub order: i16,
    pub priority: i16,
    pub sub_order: i16,
}

impl EffectOrder {
    #[must_use]
    pub const fn new(order: i16, priority: i16, sub_order: i16) -> Self {
        Self {
            order,
            priority,
            sub_order,
        }
    }
}

impl Default for EffectOrder {
    fn default() -> Self {
        Self {
            order: i16::MAX,
            priority: 0,
            sub_order: 0,
        }
    }
}

/// Actor, target, and move associated with one event dispatch.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)] // Independent event provenance flags are intentionally orthogonal.
pub struct EventContext {
    pub source: Option<PokemonRef>,
    pub target: Option<PokemonRef>,
    pub move_id: Option<MoveId>,
    /// Whether the current move ignores suppressible defensive handlers on its target's side.
    #[serde(default, skip_serializing_if = "is_false")]
    pub suppress_target_ability: bool,
    /// Whether Magic Room or Klutz suppresses the move source's held item.
    #[serde(default, skip_serializing_if = "is_false")]
    pub suppress_source_item: bool,
    /// Whether a species-restricted source item matches its current holder.
    #[serde(default, skip_serializing_if = "is_false")]
    pub source_item_species_match: bool,
    /// Whether an opposing active Unnerve holder prevents the source eating a Berry.
    #[serde(default, skip_serializing_if = "is_false")]
    pub source_berry_blocked: bool,
    /// Whether the source's Ripen doubles a Berry's effect.
    #[serde(default, skip_serializing_if = "is_false")]
    pub source_ripen: bool,
    /// Whether source Berry consumption requires a deferred cross-ability behavior.
    #[serde(default, skip_serializing_if = "is_false")]
    pub source_berry_interaction_unsupported: bool,
    /// Whether Magic Room or Klutz suppresses the move target's held item.
    #[serde(default, skip_serializing_if = "is_false")]
    pub suppress_target_item: bool,
    /// Whether an opposing active Unnerve holder prevents the target eating a Berry.
    #[serde(default, skip_serializing_if = "is_false")]
    pub target_berry_blocked: bool,
    /// Whether the target's active Ripen doubles a Berry's effect.
    #[serde(default, skip_serializing_if = "is_false")]
    pub target_ripen: bool,
    /// Whether Berry consumption would require a deferred cross-ability behavior.
    #[serde(default, skip_serializing_if = "is_false")]
    pub target_berry_interaction_unsupported: bool,
    /// Whether the current hit was confirmed as critical.
    #[serde(default, skip_serializing_if = "is_false")]
    pub critical: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_category: Option<MoveCategory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_type: Option<PokemonType>,
    #[serde(default, skip_serializing_if = "MoveInteractionFlags::is_empty")]
    pub move_flags: MoveInteractionFlags,
    /// Whether the source ability converted this move from Normal type.
    #[serde(default, skip_serializing_if = "is_false")]
    pub move_type_converted: bool,
    /// Whether Sheer Force removed this move's secondary effects.
    #[serde(default, skip_serializing_if = "is_false")]
    pub sheer_force_boosted: bool,
    /// Whether an active ally supplies Plus/Minus's shared Special Attack boost.
    #[serde(default, skip_serializing_if = "is_false")]
    pub plus_minus_active: bool,
    /// Whether Analytic sees no later queued action for this target.
    #[serde(default, skip_serializing_if = "is_false")]
    pub analytic_active: bool,
    /// Whether this stat-stage request was caused by Intimidate.
    #[serde(default, skip_serializing_if = "is_false")]
    pub intimidate: bool,
    /// Combined Showdown type modifier exponent: positive is weak, negative is resisted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_effectiveness: Option<i8>,
    /// Major status currently being offered to `TrySetStatus` handlers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_status: Option<MajorStatus>,
    /// Stat whose stage delta is currently being offered to handlers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_stat: Option<BattleStat>,
    /// Whether the queued move received Prankster's priority modifier.
    #[serde(default, skip_serializing_if = "is_false")]
    pub prankster_boosted: bool,
    /// Resolved target typing for typed immunity callbacks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_types: Option<PokemonTypes>,
    /// Whether the move target participates in grounded mechanics for this hit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_grounded: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weather: Option<WeatherEnvironment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain: Option<TerrainEnvironment>,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // Serde's skip predicate receives `&T`.
const fn is_false(value: &bool) -> bool {
    !*value
}

/// Typed relay value passed through an event's ordered handlers.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum EventValue {
    #[default]
    None,
    Boolean(bool),
    Integer(i64),
}

/// One collected handler, retained in dispatch traces for inspection.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EffectInvocation {
    pub source: EffectSource,
    pub owner: EffectOwner,
    pub order: EffectOrder,
    pub speed: u16,
}

/// Result and ordered trace of a completed event dispatch.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventDispatch {
    pub event: BattleEvent,
    pub context: EventContext,
    pub value: EventValue,
    pub cancelled: bool,
    /// Whether a `TryHit` handler rejected the move as an immunity.
    #[serde(default, skip_serializing_if = "is_false")]
    pub immune: bool,
    /// Whether `ModifyPriority` marked this move as Prankster-boosted.
    #[serde(default, skip_serializing_if = "is_false")]
    pub prankster_boosted: bool,
    pub invocations: Vec<EffectInvocation>,
}

impl EventDispatch {
    #[must_use]
    pub fn new(event: BattleEvent, context: EventContext, value: EventValue) -> Self {
        Self {
            event,
            context,
            value,
            cancelled: false,
            immune: false,
            prankster_boosted: false,
            invocations: Vec::new(),
        }
    }

    /// Stops the current event after the active handler returns.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Marks a rejected `TryHit` as an immunity for the move executor.
    pub fn mark_immune(&mut self) {
        self.immune = true;
    }

    /// Carries Prankster provenance from queue construction into hit resolution.
    pub fn mark_prankster_boosted(&mut self) {
        self.prankster_boosted = true;
    }
}

/// Controlled state access supplied to one effect callback.
///
/// Effects may inspect the complete battle and request typed, invariant-safe
/// mutations. The internal mutation kernel and unrestricted mutable battle
/// references are intentionally not exposed to registry consumers.
pub struct EffectState<'a> {
    battle: &'a mut BattleState,
}

impl EffectState<'_> {
    #[must_use]
    pub const fn battle(&self) -> &BattleState {
        self.battle
    }

    #[must_use]
    pub const fn rng(&self) -> &BattleRng {
        &self.battle.rng
    }

    /// Draws one mechanics-owned Bernoulli outcome.
    ///
    /// # Errors
    ///
    /// Returns an error when the probability is invalid.
    pub fn chance(
        &mut self,
        reason: RngReason,
        numerator: u64,
        denominator: u64,
    ) -> Result<bool, BattleError> {
        self.battle.rng.chance(reason, numerator, denominator)
    }

    /// Draws one uniformly distributed mechanics-owned integer.
    ///
    /// # Errors
    ///
    /// Returns an error when the range is empty or invalid.
    pub fn range(
        &mut self,
        reason: RngReason,
        range: std::ops::Range<u64>,
    ) -> Result<u64, BattleError> {
        self.battle.rng.range(reason, range)
    }

    /// Applies a coherent major status using the authoritative RNG stream.
    ///
    /// # Errors
    ///
    /// Returns an error when the target is invalid or status initialization fails.
    pub fn apply_major_status(
        &mut self,
        target: PokemonRef,
        status: MajorStatus,
    ) -> Result<(), BattleError> {
        self.battle.apply_major_status(target, status)
    }

    /// Applies clamped HP loss and returns the actual loss.
    ///
    /// # Errors
    ///
    /// Returns an error when the target is invalid.
    pub fn damage_hp(&mut self, target: PokemonRef, requested: u16) -> Result<u16, BattleError> {
        Ok(StateKernel::new(self.battle)
            .damage_hp(target, requested)?
            .applied)
    }

    /// Applies clamped HP recovery and returns the actual recovery.
    ///
    /// # Errors
    ///
    /// Returns an error when the target is invalid.
    pub fn heal_hp(&mut self, target: PokemonRef, requested: u16) -> Result<u16, BattleError> {
        Ok(StateKernel::new(self.battle)
            .heal_hp(target, requested)?
            .applied)
    }

    /// Applies a bounded stat-stage change.
    ///
    /// # Errors
    ///
    /// Returns an error when the target is invalid.
    pub fn change_stat_stage(
        &mut self,
        target: PokemonRef,
        stat: BattleStat,
        delta: i8,
    ) -> Result<StatStageChange, BattleError> {
        StateKernel::new(self.battle).change_stat_stage(target, stat, delta)
    }

    /// Installs a coherent major-status and counter state.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid target or incoherent status state.
    pub fn set_major_status(
        &mut self,
        target: PokemonRef,
        status: MajorStatus,
        state: StatusState,
    ) -> Result<(), BattleError> {
        StateKernel::new(self.battle).set_major_status(target, status, state)
    }

    /// Cures a major status and its associated counters.
    ///
    /// # Errors
    ///
    /// Returns an error when the target is invalid.
    pub fn cure_major_status(
        &mut self,
        target: PokemonRef,
    ) -> Result<Option<MajorStatus>, BattleError> {
        StateKernel::new(self.battle).cure_major_status(target)
    }

    /// Decrements a nonzero sleep counter.
    ///
    /// # Errors
    ///
    /// Returns an error when the target or counter state is invalid.
    pub fn decrement_sleep_turns(&mut self, target: PokemonRef) -> Result<u8, BattleError> {
        StateKernel::new(self.battle).decrement_sleep_turns(target)
    }

    /// Decrements a nonzero freeze counter.
    ///
    /// # Errors
    ///
    /// Returns an error when the target or counter state is invalid.
    pub fn decrement_freeze_turns(&mut self, target: PokemonRef) -> Result<u8, BattleError> {
        StateKernel::new(self.battle).decrement_freeze_turns(target)
    }

    /// Increments the toxic counter within its representation bounds.
    ///
    /// # Errors
    ///
    /// Returns an error when the target or status state is invalid.
    pub fn increment_toxic_counter(&mut self, target: PokemonRef) -> Result<u8, BattleError> {
        StateKernel::new(self.battle).increment_toxic_counter(target)
    }

    /// Adds one unique volatile condition.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid target or condition.
    pub fn add_volatile(
        &mut self,
        target: PokemonRef,
        condition: ConditionState,
    ) -> Result<(), BattleError> {
        StateKernel::new(self.battle).add_volatile(target, condition)
    }

    /// Removes one volatile condition when present.
    ///
    /// # Errors
    ///
    /// Returns an error when the target is invalid.
    pub fn remove_volatile(
        &mut self,
        target: PokemonRef,
        condition: ConditionId,
    ) -> Result<Option<ConditionState>, BattleError> {
        StateKernel::new(self.battle).remove_volatile(target, condition)
    }

    /// Replaces the held item and returns the previous value.
    ///
    /// # Errors
    ///
    /// Returns an error when the target is invalid.
    pub fn set_item(
        &mut self,
        target: PokemonRef,
        item: Option<ItemId>,
    ) -> Result<Option<ItemId>, BattleError> {
        StateKernel::new(self.battle).set_item(target, item)
    }

    /// Replaces one move slot's PP through the mutation kernel.
    ///
    /// # Errors
    ///
    /// Returns an error when the holder or move slot is invalid, or the value exceeds max PP.
    pub fn set_pp(
        &mut self,
        target: PokemonRef,
        slot: crate::MoveSlotIndex,
        value: u8,
    ) -> Result<(), BattleError> {
        StateKernel::new(self.battle).set_pp(target, slot, value)?;
        Ok(())
    }

    /// Replaces an ability identity and returns the previous identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the target is invalid.
    pub fn set_ability(
        &mut self,
        target: PokemonRef,
        ability: AbilityId,
    ) -> Result<AbilityId, BattleError> {
        StateKernel::new(self.battle).set_ability(target, ability)
    }

    /// Adds one unique field condition.
    ///
    /// # Errors
    ///
    /// Returns an error when the condition is invalid or duplicated.
    pub fn add_field_condition(&mut self, condition: ConditionState) -> Result<(), BattleError> {
        StateKernel::new(self.battle).add_field_condition(condition)
    }

    /// Replaces the current weather and returns the previous state.
    ///
    /// # Errors
    ///
    /// Returns an error for a zero-duration or zero-layer condition.
    pub fn set_weather(
        &mut self,
        weather: Option<ConditionState>,
    ) -> Result<Option<ConditionState>, BattleError> {
        StateKernel::new(self.battle).set_weather(weather)
    }

    /// Replaces the current terrain and returns the previous state.
    ///
    /// # Errors
    ///
    /// Returns an error for a zero-duration or zero-layer condition.
    pub fn set_terrain(
        &mut self,
        terrain: Option<ConditionState>,
    ) -> Result<Option<ConditionState>, BattleError> {
        StateKernel::new(self.battle).set_terrain(terrain)
    }

    /// Adds one unique condition to a side.
    ///
    /// # Errors
    ///
    /// Returns an error for a duplicate or invalid condition.
    pub fn add_side_condition(
        &mut self,
        side: SideId,
        condition: ConditionState,
    ) -> Result<(), BattleError> {
        StateKernel::new(self.battle).add_side_condition(side, condition)
    }

    pub fn remove_field_condition(&mut self, condition: ConditionId) -> Option<ConditionState> {
        StateKernel::new(self.battle).remove_field_condition(condition)
    }

    pub fn remove_side_condition(
        &mut self,
        side: SideId,
        condition: ConditionId,
    ) -> Option<ConditionState> {
        StateKernel::new(self.battle).remove_side_condition(side, condition)
    }
}

/// A stateless effect function registered in the engine behavior registry.
pub type EffectCallback = fn(
    &mut EffectState<'_>,
    &EffectInvocation,
    &EventContext,
    &mut EventDispatch,
) -> Result<(), BattleError>;

#[derive(Clone, Copy)]
struct EffectRegistration {
    event: BattleEvent,
    source: EffectSource,
    order: EffectOrder,
    callback: EffectCallback,
    registration_order: usize,
    suppressible: bool,
}

#[derive(Clone, Copy)]
struct CollectedEffect {
    invocation: EffectInvocation,
    callback: EffectCallback,
    registration_order: usize,
}

/// Static behavior registry kept outside serializable, cloneable battle state.
#[derive(Clone, Default)]
pub struct EffectRegistry {
    events: BTreeMap<BattleEvent, EventRegistrations>,
    registration_count: usize,
}

/// Registrations indexed first by event and then by concrete source identity.
/// Active holders are visited once per dispatch and probe these maps directly.
#[derive(Clone, Default)]
struct EventRegistrations {
    battle: Vec<EffectRegistration>,
    moves: BTreeMap<MoveId, Vec<EffectRegistration>>,
    abilities: BTreeMap<AbilityId, Vec<EffectRegistration>>,
    items: BTreeMap<ItemId, Vec<EffectRegistration>>,
    statuses: BTreeMap<MajorStatus, Vec<EffectRegistration>>,
    conditions: BTreeMap<ConditionId, Vec<EffectRegistration>>,
    registration_count: usize,
}

impl EffectRegistry {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            events: BTreeMap::new(),
            registration_count: 0,
        }
    }

    /// Registers one stateless callback for a typed effect and event.
    pub fn register(
        &mut self,
        event: BattleEvent,
        source: EffectSource,
        order: EffectOrder,
        callback: EffectCallback,
    ) {
        self.register_with_suppression(event, source, order, callback, false);
    }

    pub(crate) fn register_suppressible(
        &mut self,
        event: BattleEvent,
        source: EffectSource,
        order: EffectOrder,
        callback: EffectCallback,
    ) {
        self.register_with_suppression(event, source, order, callback, true);
    }

    fn register_with_suppression(
        &mut self,
        event: BattleEvent,
        source: EffectSource,
        order: EffectOrder,
        callback: EffectCallback,
        suppressible: bool,
    ) {
        let registration_order = self.registration_count;
        let registration = EffectRegistration {
            event,
            source,
            order,
            callback,
            registration_order,
            suppressible,
        };
        self.events.entry(event).or_default().insert(registration);
        self.registration_count += 1;
    }

    pub(crate) fn append(&mut self, other: &Self) {
        let mut registrations = other.registrations();
        registrations.sort_by_key(|registration| registration.registration_order);
        for registration in registrations {
            self.register_with_suppression(
                registration.event,
                registration.source,
                registration.order,
                registration.callback,
                registration.suppressible,
            );
        }
    }

    /// Collects active instances, orders them, and executes their callbacks.
    ///
    /// Applicable handlers are snapshotted before the first callback. A
    /// callback may update mutable relays, mutate battle state, cancel a
    /// cancellable event, or return a typed battle error. Cancellation stops
    /// later handlers; invalid relay or cancellation behavior is rejected.
    ///
    /// # Errors
    ///
    /// Propagates the first callback error.
    pub(crate) fn dispatch(
        &self,
        battle: &mut BattleState,
        event: BattleEvent,
        context: EventContext,
        value: EventValue,
    ) -> Result<EventDispatch, BattleError> {
        let contract = event.contract();
        contract.validate_value(event, value)?;
        let initial_value = value;
        let mut handlers = self.collect(battle, event, context)?;
        handlers.sort_by(compare_collected_effects);
        let mut dispatch = EventDispatch::new(event, context, value);
        for handler in handlers {
            dispatch.invocations.push(handler.invocation);
            let mut state = EffectState { battle };
            (handler.callback)(&mut state, &handler.invocation, &context, &mut dispatch)?;
            contract.validate_value(event, dispatch.value)?;
            if !contract.mutable_relay && dispatch.value != initial_value {
                return Err(BattleError::InvalidState(format!(
                    "{event:?} handlers cannot modify their relay value"
                )));
            }
            if dispatch.cancelled && !contract.cancellable {
                return Err(BattleError::InvalidState(format!(
                    "{event:?} is not a cancellable event"
                )));
            }
            if dispatch.immune
                && (!matches!(event, BattleEvent::TryHit | BattleEvent::TryImmunity)
                    || dispatch.value != EventValue::Boolean(false))
            {
                return Err(BattleError::InvalidState(format!(
                    "{event:?} handlers produced an invalid hit immunity"
                )));
            }
            if dispatch.prankster_boosted && event != BattleEvent::ModifyPriority {
                return Err(BattleError::InvalidState(format!(
                    "{event:?} handlers produced invalid Prankster provenance"
                )));
            }
            if dispatch.cancelled {
                break;
            }
        }
        Ok(dispatch)
    }

    fn collect(
        &self,
        battle: &BattleState,
        event: BattleEvent,
        context: EventContext,
    ) -> Result<Vec<CollectedEffect>, BattleError> {
        let Some(registrations) = self.events.get(&event) else {
            return Ok(Vec::new());
        };
        let mut collected = Vec::with_capacity(registrations.registration_count);
        collect_registrations(
            &mut collected,
            &registrations.battle,
            EffectOwner::Battle,
            0,
        );

        if let Some(move_id) = context.move_id {
            if let Some(move_handlers) = registrations.moves.get(&move_id) {
                let owner = context
                    .source
                    .map_or(EffectOwner::Battle, |pokemon| EffectOwner::Move { pokemon });
                collect_registrations(
                    &mut collected,
                    move_handlers,
                    owner,
                    owner_speed(battle, owner)?,
                );
            }
        }

        if registrations.has_conditions() {
            collect_field_conditions(battle, registrations, &mut collected);
        }
        if !registrations.has_active_sources() {
            return Ok(collected);
        }
        for side in [SideId::P1, SideId::P2] {
            if registrations.has_conditions() {
                collect_side_conditions(battle, registrations, side, &mut collected);
            }
            for pokemon in battle.side(side).active.into_iter().flatten() {
                let state = battle.side(side).pokemon(pokemon).ok_or_else(|| {
                    BattleError::InvalidState(
                        "active Pokémon index is out of bounds during effect collection".to_owned(),
                    )
                })?;
                let owner = EffectOwner::Pokemon {
                    pokemon: PokemonRef::new(side, pokemon),
                };
                let speed = state.effective_speed();
                if let Some(handlers) = registrations.abilities.get(&state.ability) {
                    collect_ability_registrations(&mut collected, handlers, owner, speed, context);
                }
                if let Some(handlers) = state.item.and_then(|item| registrations.items.get(&item)) {
                    collect_registrations(&mut collected, handlers, owner, speed);
                }
                if let Some(handlers) = state
                    .status
                    .and_then(|status| registrations.statuses.get(&status))
                {
                    collect_registrations(&mut collected, handlers, owner, speed);
                }
                for condition in &state.volatiles {
                    if let Some(handlers) = registrations.conditions.get(&condition.id) {
                        collect_registrations(&mut collected, handlers, owner, speed);
                    }
                }
            }
        }
        Ok(collected)
    }

    fn registrations(&self) -> Vec<EffectRegistration> {
        let mut registrations = Vec::with_capacity(self.registration_count);
        for event in self.events.values() {
            event.extend_registrations(&mut registrations);
        }
        registrations
    }
}

impl EventRegistrations {
    fn has_conditions(&self) -> bool {
        !self.conditions.is_empty()
    }

    fn has_active_sources(&self) -> bool {
        !self.abilities.is_empty()
            || !self.items.is_empty()
            || !self.statuses.is_empty()
            || self.has_conditions()
    }

    fn insert(&mut self, registration: EffectRegistration) {
        match registration.source {
            EffectSource::Battle => self.battle.push(registration),
            EffectSource::Move(id) => self.moves.entry(id).or_default().push(registration),
            EffectSource::Ability(id) => self.abilities.entry(id).or_default().push(registration),
            EffectSource::Item(id) => self.items.entry(id).or_default().push(registration),
            EffectSource::MajorStatus(status) => {
                self.statuses.entry(status).or_default().push(registration);
            }
            EffectSource::Condition(id) => {
                self.conditions.entry(id).or_default().push(registration);
            }
        }
        self.registration_count += 1;
    }

    fn extend_registrations(&self, output: &mut Vec<EffectRegistration>) {
        output.extend(self.battle.iter().copied());
        for registrations in self.moves.values() {
            output.extend(registrations.iter().copied());
        }
        for registrations in self.abilities.values() {
            output.extend(registrations.iter().copied());
        }
        for registrations in self.items.values() {
            output.extend(registrations.iter().copied());
        }
        for registrations in self.statuses.values() {
            output.extend(registrations.iter().copied());
        }
        for registrations in self.conditions.values() {
            output.extend(registrations.iter().copied());
        }
    }
}

fn collect_registrations(
    output: &mut Vec<CollectedEffect>,
    registrations: &[EffectRegistration],
    owner: EffectOwner,
    speed: u16,
) {
    for registration in registrations {
        push_collected(output, *registration, owner, speed);
    }
}

fn collect_ability_registrations(
    output: &mut Vec<CollectedEffect>,
    registrations: &[EffectRegistration],
    owner: EffectOwner,
    speed: u16,
    context: EventContext,
) {
    let target_side_is_suppressed = context.suppress_target_ability
        && context.target != context.source
        && matches!(
            (owner, context.target),
            (EffectOwner::Pokemon { pokemon }, Some(target)) if pokemon.side == target.side
        );
    for registration in registrations {
        if !(target_side_is_suppressed && registration.suppressible) {
            push_collected(output, *registration, owner, speed);
        }
    }
}

fn collect_field_conditions(
    battle: &BattleState,
    registrations: &EventRegistrations,
    output: &mut Vec<CollectedEffect>,
) {
    let field = &battle.field;
    let weather = field.weather.map(|state| state.id);
    if let Some(condition) = weather {
        collect_condition_handlers(registrations, condition, EffectOwner::Field, output);
    }
    let terrain = field.terrain.map(|state| state.id);
    if let Some(condition) = terrain.filter(|condition| Some(*condition) != weather) {
        collect_condition_handlers(registrations, condition, EffectOwner::Field, output);
    }
    for (index, condition) in field.conditions.iter().enumerate() {
        if Some(condition.id) != weather
            && Some(condition.id) != terrain
            && !field.conditions[..index]
                .iter()
                .any(|previous| previous.id == condition.id)
        {
            collect_condition_handlers(registrations, condition.id, EffectOwner::Field, output);
        }
    }
}

fn collect_condition_handlers(
    registrations: &EventRegistrations,
    condition: ConditionId,
    owner: EffectOwner,
    output: &mut Vec<CollectedEffect>,
) {
    if let Some(handlers) = registrations.conditions.get(&condition) {
        collect_registrations(output, handlers, owner, 0);
    }
}

fn collect_side_conditions(
    battle: &BattleState,
    registrations: &EventRegistrations,
    side: SideId,
    output: &mut Vec<CollectedEffect>,
) {
    for condition in &battle.side(side).side_conditions {
        collect_condition_handlers(
            registrations,
            condition.id,
            EffectOwner::Side { side },
            output,
        );
    }
}

fn push_collected(
    output: &mut Vec<CollectedEffect>,
    registration: EffectRegistration,
    owner: EffectOwner,
    speed: u16,
) {
    let mut order = registration.order;
    if order.sub_order == 0 {
        order.sub_order = default_sub_order(registration.source, owner);
    }
    output.push(CollectedEffect {
        invocation: EffectInvocation {
            source: registration.source,
            owner,
            order,
            speed,
        },
        callback: registration.callback,
        registration_order: registration.registration_order,
    });
}

const fn default_sub_order(source: EffectSource, owner: EffectOwner) -> i16 {
    match source {
        EffectSource::MajorStatus(_) => 2,
        EffectSource::Condition(_) => match owner {
            EffectOwner::Pokemon { .. } => 2,
            EffectOwner::Side { .. } => 4,
            EffectOwner::Field => 5,
            EffectOwner::Battle | EffectOwner::Move { .. } => 0,
        },
        EffectSource::Battle => 5,
        EffectSource::Ability(_) => 7,
        EffectSource::Item(_) => 8,
        EffectSource::Move(_) => 0,
    }
}

fn owner_speed(battle: &BattleState, owner: EffectOwner) -> Result<u16, BattleError> {
    let pokemon = match owner {
        EffectOwner::Pokemon { pokemon } | EffectOwner::Move { pokemon } => pokemon,
        EffectOwner::Battle | EffectOwner::Field | EffectOwner::Side { .. } => return Ok(0),
    };
    let state = battle
        .side(pokemon.side)
        .pokemon(pokemon.pokemon)
        .ok_or_else(|| {
            BattleError::InvalidState(
                "effect owner Pokémon index is out of bounds during collection".to_owned(),
            )
        })?;
    if !matches!(state.position, PokemonPosition::Active { .. }) {
        return Err(BattleError::InvalidState(
            "a Pokémon-owned active effect has an inactive holder".to_owned(),
        ));
    }
    Ok(state.effective_speed())
}

fn compare_collected_effects(left: &CollectedEffect, right: &CollectedEffect) -> Ordering {
    effective_order(left.invocation.order.order)
        .cmp(&effective_order(right.invocation.order.order))
        .then_with(|| {
            right
                .invocation
                .order
                .priority
                .cmp(&left.invocation.order.priority)
        })
        .then_with(|| right.invocation.speed.cmp(&left.invocation.speed))
        .then_with(|| {
            left.invocation
                .order
                .sub_order
                .cmp(&right.invocation.order.sub_order)
        })
        .then_with(|| left.registration_order.cmp(&right.registration_order))
}

const fn effective_order(order: i16) -> i16 {
    if order == 0 {
        i16::MAX
    } else {
        order
    }
}
