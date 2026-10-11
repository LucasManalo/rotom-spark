use super::{ActiveSlot, BattlePhase, BattleState, PokemonIndex, PokemonPosition};
use crate::{
    ActionTarget, BattleError, BattleEvent, EffectRegistry, EventContext, EventDispatch,
    EventValue, PokemonAction, PokemonRef, RngReason, SideId, TurnChoice,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Showdown's major action-order bucket. Lower values execute first.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[repr(u16)]
#[serde(rename_all = "snake_case")]
pub enum ActionOrder {
    Switch = 103,
    Move = 200,
}

/// One validated active-Pokémon action with its resolved normal-order keys.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueuedAction {
    pub side: SideId,
    pub slot: ActiveSlot,
    pub pokemon: PokemonIndex,
    pub choice: PokemonAction,
    pub order: ActionOrder,
    pub priority: i8,
    pub speed: u16,
}

/// The inspectable execution order for one submitted doubles turn.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionQueue {
    pub actions: Vec<QueuedAction>,
    /// Queue-construction events such as priority and Speed modifiers.
    #[serde(default)]
    pub events: Vec<EventDispatch>,
}

impl BattleState {
    /// Validates both players' choices and builds the normal turn action queue.
    ///
    /// Voluntary switches use Showdown order 103 and moves use order 200.
    /// Within an order bucket, higher move priority and then higher current
    /// Speed execute first. Exact ties are Fisher–Yates shuffled using the
    /// battle-owned RNG. The resulting queue is stored on the battle for
    /// inspection and the phase advances to [`BattlePhase::ResolvingTurn`].
    ///
    /// Current `BattleStats::spe` values are the base effective speeds. The
    /// effect-aware entry point applies registered priority and Speed hooks;
    /// Trick Room remains deferred to its field-effect milestone.
    ///
    /// # Errors
    ///
    /// Returns an error unless the battle is selecting a turn, either choice
    /// is illegal, or active/move state is internally inconsistent. Choices
    /// are fully validated before RNG state, queue, or phase is changed.
    pub fn queue_turn(
        &mut self,
        p1: TurnChoice,
        p2: TurnChoice,
    ) -> Result<&ActionQueue, BattleError> {
        self.queue_turn_with_effects(p1, p2, &EffectRegistry::new())
    }

    /// Validates and queues a turn through priority and Speed effect hooks.
    ///
    /// Handler failures leave battle state and RNG unchanged.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::queue_turn`] and propagates effect
    /// handler errors or incompatible modifier relay values.
    pub fn queue_turn_with_effects(
        &mut self,
        p1: TurnChoice,
        p2: TurnChoice,
        effects: &EffectRegistry,
    ) -> Result<&ActionQueue, BattleError> {
        let mut next = self.clone();
        next.queue_turn_inner(p1, p2, effects)?;
        *self = next;
        Ok(&self.queue)
    }

    fn queue_turn_inner(
        &mut self,
        p1: TurnChoice,
        p2: TurnChoice,
        effects: &EffectRegistry,
    ) -> Result<(), BattleError> {
        if self.phase != BattlePhase::TurnSelection {
            return Err(BattleError::IllegalChoice(format!(
                "cannot queue a turn during {:?}",
                self.phase
            )));
        }
        self.validate_choice(SideId::P1, &p1)?;
        self.validate_choice(SideId::P2, &p2)?;

        let mut actions = Vec::with_capacity(4);
        let mut events = Vec::new();
        self.expand_choice(SideId::P1, p1, effects, &mut actions, &mut events)?;
        self.expand_choice(SideId::P2, p2, effects, &mut actions, &mut events)?;
        actions.sort_by(compare_actions);
        self.shuffle_speed_ties(&mut actions)?;

        self.queue = ActionQueue { actions, events };
        self.phase = BattlePhase::ResolvingTurn;
        Ok(())
    }

    fn expand_choice(
        &mut self,
        side: SideId,
        choice: TurnChoice,
        effects: &EffectRegistry,
        output: &mut Vec<QueuedAction>,
        events: &mut Vec<EventDispatch>,
    ) -> Result<(), BattleError> {
        for (slot, action) in ActiveSlot::ALL.into_iter().zip(choice.actions) {
            if action == PokemonAction::Pass {
                continue;
            }
            let pokemon = self.side(side).active[slot.index()].ok_or_else(|| {
                BattleError::InvalidState(format!(
                    "{side:?}'s {slot:?} queued action has no active Pokémon"
                ))
            })?;
            let (order, priority, speed, move_id) = {
                let pokemon_state = self.side(side).pokemon(pokemon).ok_or_else(|| {
                    BattleError::InvalidState(
                        "queued active Pokémon index is out of bounds".to_owned(),
                    )
                })?;
                if pokemon_state.position != (PokemonPosition::Active { slot })
                    || pokemon_state.current_hp == 0
                    || pokemon_state.is_fainted()
                {
                    return Err(BattleError::InvalidState(format!(
                        "{side:?}'s {slot:?} queued Pokémon is inconsistent"
                    )));
                }

                match action {
                    PokemonAction::Move { mega: true, .. } => {
                        return Err(BattleError::UnsupportedMechanic(
                            "Mega Evolution queue expansion is deferred".to_owned(),
                        ));
                    }
                    PokemonAction::Move {
                        move_slot,
                        mega: false,
                        ..
                    } => {
                        let move_data = pokemon_state
                            .moves
                            .get(usize::from(move_slot.get()))
                            .ok_or_else(|| {
                                BattleError::InvalidState(
                                    "queued move-slot index is out of bounds".to_owned(),
                                )
                            })?;
                        (
                            ActionOrder::Move,
                            move_data.priority,
                            pokemon_state.stats.spe,
                            Some(move_data.move_id),
                        )
                    }
                    PokemonAction::Struggle => {
                        (ActionOrder::Move, 0, pokemon_state.stats.spe, None)
                    }
                    PokemonAction::Switch { .. } => {
                        (ActionOrder::Switch, 0, pokemon_state.stats.spe, None)
                    }
                    PokemonAction::Pass => continue,
                }
            };
            let context = EventContext {
                source: Some(PokemonRef::new(side, pokemon)),
                target: action_target(self, side, slot, action),
                move_id,
            };
            let priority = if order == ActionOrder::Move {
                let priority_event = effects.dispatch(
                    self,
                    BattleEvent::ModifyPriority,
                    context,
                    EventValue::Integer(i64::from(priority)),
                )?;
                let priority = event_i8(&priority_event, BattleEvent::ModifyPriority)?;
                events.push(priority_event);
                priority
            } else {
                priority
            };
            let speed_event = effects.dispatch(
                self,
                BattleEvent::ModifySpeed,
                context,
                EventValue::Integer(i64::from(speed)),
            )?;
            let speed = event_u16(&speed_event, BattleEvent::ModifySpeed)?;
            events.push(speed_event);
            output.push(QueuedAction {
                side,
                slot,
                pokemon,
                choice: action,
                order,
                priority,
                speed,
            });
        }
        Ok(())
    }

    fn shuffle_speed_ties(&mut self, actions: &mut [QueuedAction]) -> Result<(), BattleError> {
        let mut group_start = 0;
        while group_start < actions.len() {
            let mut group_end = group_start + 1;
            while group_end < actions.len()
                && ordering_keys_match(actions[group_start], actions[group_end])
            {
                group_end += 1;
            }

            let mut index = group_start;
            while index + 1 < group_end {
                let selected = self.rng.range(
                    RngReason::SpeedTie,
                    u64::try_from(index).map_err(|_| {
                        BattleError::InvalidState("queue index does not fit in u64".to_owned())
                    })?..u64::try_from(group_end).map_err(|_| {
                        BattleError::InvalidState("queue length does not fit in u64".to_owned())
                    })?,
                )?;
                let selected = usize::try_from(selected).map_err(|_| {
                    BattleError::InvalidState("random queue index does not fit in usize".to_owned())
                })?;
                actions.swap(index, selected);
                index += 1;
            }
            group_start = group_end;
        }
        Ok(())
    }
}

fn event_i8(dispatch: &EventDispatch, event: BattleEvent) -> Result<i8, BattleError> {
    let EventValue::Integer(value) = dispatch.value else {
        return Err(BattleError::InvalidState(format!(
            "{event:?} replaced its integer relay with an incompatible value"
        )));
    };
    i8::try_from(value).map_err(|_| {
        BattleError::InvalidState(format!("{event:?} produced a value outside the i8 range"))
    })
}

fn event_u16(dispatch: &EventDispatch, event: BattleEvent) -> Result<u16, BattleError> {
    let EventValue::Integer(value) = dispatch.value else {
        return Err(BattleError::InvalidState(format!(
            "{event:?} replaced its integer relay with an incompatible value"
        )));
    };
    u16::try_from(value).map_err(|_| {
        BattleError::InvalidState(format!("{event:?} produced a value outside the u16 range"))
    })
}

fn action_target(
    battle: &BattleState,
    side: SideId,
    source_slot: ActiveSlot,
    action: PokemonAction,
) -> Option<PokemonRef> {
    let (target_side, target_slot) = match action {
        PokemonAction::Move { target, .. } => match target {
            ActionTarget::Self_ => (side, source_slot),
            ActionTarget::Ally(slot) => (side, slot),
            ActionTarget::Foe(slot) => (side.opponent(), slot),
            ActionTarget::Automatic => return None,
        },
        PokemonAction::Switch { pokemon } => return Some(PokemonRef::new(side, pokemon)),
        PokemonAction::Struggle | PokemonAction::Pass => return None,
    };
    battle.side(target_side).active[target_slot.index()]
        .map(|pokemon| PokemonRef::new(target_side, pokemon))
}

fn compare_actions(left: &QueuedAction, right: &QueuedAction) -> Ordering {
    left.order
        .cmp(&right.order)
        .then_with(|| right.priority.cmp(&left.priority))
        .then_with(|| right.speed.cmp(&left.speed))
}

fn ordering_keys_match(left: QueuedAction, right: QueuedAction) -> bool {
    left.order == right.order && left.priority == right.priority && left.speed == right.speed
}
