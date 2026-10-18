use crate::{
    kernel::StateKernel, AbilityId, BattleError, BattleRng, BattleStat, BattleState, ConditionId,
    ConditionState, ItemId, MajorStatus, MoveId, PokemonIndex, PokemonPosition, RngReason, SideId,
    StatStageChange, StatusState,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Stable hook points through which battle effects intercept engine operations.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleEvent {
    BeforeTurn,
    BeforeMove,
    ModifyPriority,
    ModifySpeed,
    /// Relays the effective offensive stat after move-specific selection.
    ModifyAttack,
    /// Relays the effective defensive stat after move-specific selection.
    ModifyDefense,
    /// Relays base power after state-dependent move behavior.
    ModifyBasePower,
    ModifyDamage,
    TryHit,
    TrySetStatus,
    Hit,
    AfterDamage,
    AfterMove,
    SwitchIn,
    SwitchOut,
    Faint,
    Residual,
}

impl BattleEvent {
    /// Declares the relay and cancellation semantics enforced by dispatch.
    #[must_use]
    pub const fn contract(self) -> EventContract {
        match self {
            Self::BeforeMove | Self::SwitchOut => EventContract::new(EventRelay::None, true, false),
            Self::TryHit | Self::TrySetStatus => {
                EventContract::new(EventRelay::Boolean, true, true)
            }
            Self::ModifyPriority
            | Self::ModifySpeed
            | Self::ModifyAttack
            | Self::ModifyDefense
            | Self::ModifyBasePower
            | Self::ModifyDamage => EventContract::new(EventRelay::Integer, false, true),
            Self::AfterDamage => EventContract::new(EventRelay::Integer, false, false),
            Self::BeforeTurn
            | Self::Hit
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
pub struct EventContext {
    pub source: Option<PokemonRef>,
    pub target: Option<PokemonRef>,
    pub move_id: Option<MoveId>,
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
            invocations: Vec::new(),
        }
    }

    /// Stops the current event after the active handler returns.
    pub fn cancel(&mut self) {
        self.cancelled = true;
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
    registrations: Vec<EffectRegistration>,
}

impl EffectRegistry {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            registrations: Vec::new(),
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
        let registration_order = self.registrations.len();
        self.registrations.push(EffectRegistration {
            event,
            source,
            order,
            callback,
            registration_order,
        });
    }

    pub(crate) fn append(&mut self, other: &Self) {
        for registration in &other.registrations {
            self.register(
                registration.event,
                registration.source,
                registration.order,
                registration.callback,
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
        let mut collected = Vec::new();
        for registration in self
            .registrations
            .iter()
            .copied()
            .filter(|registration| registration.event == event)
        {
            Self::collect_registration(battle, context, registration, &mut collected)?;
        }
        Ok(collected)
    }

    fn collect_registration(
        battle: &BattleState,
        context: EventContext,
        registration: EffectRegistration,
        output: &mut Vec<CollectedEffect>,
    ) -> Result<(), BattleError> {
        match registration.source {
            EffectSource::Battle => push_collected(output, registration, EffectOwner::Battle, 0),
            EffectSource::Move(move_id) => {
                if context.move_id == Some(move_id) {
                    let owner = context
                        .source
                        .map_or(EffectOwner::Battle, |pokemon| EffectOwner::Move { pokemon });
                    let speed = owner_speed(battle, owner)?;
                    push_collected(output, registration, owner, speed);
                }
            }
            EffectSource::Condition(condition_id) => {
                collect_condition(battle, registration, condition_id, output)?;
            }
            EffectSource::Ability(ability_id) => {
                collect_active_pokemon(battle, registration, output, |pokemon| {
                    pokemon.ability == ability_id
                })?;
            }
            EffectSource::Item(item_id) => {
                collect_active_pokemon(battle, registration, output, |pokemon| {
                    pokemon.item == Some(item_id)
                })?;
            }
            EffectSource::MajorStatus(status) => {
                collect_active_pokemon(battle, registration, output, |pokemon| {
                    pokemon.status == Some(status)
                })?;
            }
        }
        Ok(())
    }
}

fn collect_active_pokemon(
    battle: &BattleState,
    registration: EffectRegistration,
    output: &mut Vec<CollectedEffect>,
    predicate: impl Fn(&crate::PokemonState) -> bool,
) -> Result<(), BattleError> {
    for side in [SideId::P1, SideId::P2] {
        for pokemon in battle.side(side).active.into_iter().flatten() {
            let state = battle.side(side).pokemon(pokemon).ok_or_else(|| {
                BattleError::InvalidState(
                    "active Pokémon index is out of bounds during effect collection".to_owned(),
                )
            })?;
            if predicate(state) {
                push_collected(
                    output,
                    registration,
                    EffectOwner::Pokemon {
                        pokemon: PokemonRef::new(side, pokemon),
                    },
                    state.effective_speed(),
                );
            }
        }
    }
    Ok(())
}

fn collect_condition(
    battle: &BattleState,
    registration: EffectRegistration,
    condition_id: ConditionId,
    output: &mut Vec<CollectedEffect>,
) -> Result<(), BattleError> {
    let field = &battle.field;
    let field_matches = field.weather.is_some_and(|state| state.id == condition_id)
        || field.terrain.is_some_and(|state| state.id == condition_id)
        || field
            .conditions
            .iter()
            .any(|state| state.id == condition_id);
    if field_matches {
        push_collected(output, registration, EffectOwner::Field, 0);
    }
    for side in [SideId::P1, SideId::P2] {
        if battle
            .side(side)
            .side_conditions
            .iter()
            .any(|state| state.id == condition_id)
        {
            push_collected(output, registration, EffectOwner::Side { side }, 0);
        }
        for pokemon in battle.side(side).active.into_iter().flatten() {
            let state = battle.side(side).pokemon(pokemon).ok_or_else(|| {
                BattleError::InvalidState(
                    "active Pokémon index is out of bounds during condition collection".to_owned(),
                )
            })?;
            if state.volatiles.iter().any(|state| state.id == condition_id) {
                push_collected(
                    output,
                    registration,
                    EffectOwner::Pokemon {
                        pokemon: PokemonRef::new(side, pokemon),
                    },
                    state.effective_speed(),
                );
            }
        }
    }
    Ok(())
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
