use super::{ActiveSlot, BattlePhase, BattleState, PokemonIndex, PokemonPosition};
use crate::{
    ActionTarget, BattleError, BattleEvent, CombatData, EventContext, EventDispatch, EventValue,
    Mechanics, PokemonAction, PokemonRef, PokemonState, RngReason, SideId, TurnChoice,
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
    /// Integer representation of Showdown's fractional priority key.
    #[serde(default)]
    pub fractional_priority: i8,
    pub speed: u16,
    /// Derived queue-time provenance used by the later Dark-type immunity check.
    #[serde(default)]
    pub prankster_boosted: bool,
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
    /// Returns the deterministic queue that a complete turn choice would use
    /// without mutating this battle or its authoritative RNG stream.
    ///
    /// Voluntary switches use Showdown order 103 and moves use order 200.
    /// Within an order bucket, higher move priority and then higher current
    /// Speed execute first. Exact ties are Fisher–Yates shuffled using the
    /// battle-owned RNG cloned for inspection.
    ///
    /// Current `BattleStats::spe` values are the base effective speeds. The
    /// effect-aware entry point applies registered priority and Speed hooks,
    /// followed by Tailwind and Trick Room field ordering.
    ///
    /// # Errors
    ///
    /// Returns an error unless the battle is selecting a turn, either choice
    /// is illegal, or active/move state is internally inconsistent.
    pub fn inspect_turn_queue<D: CombatData>(
        &self,
        p1: TurnChoice,
        p2: TurnChoice,
        mechanics: &Mechanics<'_, D>,
    ) -> Result<ActionQueue, BattleError> {
        self.validate_stable_state()?;
        let mut next = self.clone();
        next.queue_turn_inner(p1, p2, mechanics)?;
        Ok(next.queue)
    }

    pub(crate) fn queue_turn_inner<D: CombatData>(
        &mut self,
        p1: TurnChoice,
        p2: TurnChoice,
        mechanics: &Mechanics<'_, D>,
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
        self.expand_choice(SideId::P1, p1, mechanics, &mut actions, &mut events)?;
        self.expand_choice(SideId::P2, p2, mechanics, &mut actions, &mut events)?;
        actions.sort_by(compare_actions);
        self.shuffle_speed_ties(&mut actions)?;

        self.queue = ActionQueue { actions, events };
        self.phase = BattlePhase::ResolvingTurn;
        Ok(())
    }

    fn expand_choice<D: CombatData>(
        &mut self,
        side: SideId,
        choice: TurnChoice,
        mechanics: &Mechanics<'_, D>,
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
                base_action_ordering(pokemon_state, action)?
            };
            let context = EventContext {
                source: Some(PokemonRef::new(side, pokemon)),
                target: action_target(self, side, slot, action),
                move_id,
                move_category: move_id
                    .and_then(|move_id| mechanics.data().move_data(move_id))
                    .map(|move_data| move_data.category),
                move_type: move_id
                    .and_then(|move_id| mechanics.data().move_data(move_id))
                    .map(|move_data| move_data.move_type),
                weather: mechanics.field_effects().weather_environment(self),
                terrain: mechanics.field_effects().terrain_environment(self),
                ..EventContext::default()
            };
            let (priority, prankster_boosted) = if order == ActionOrder::Move {
                let priority_event = mechanics.effects().dispatch(
                    self,
                    BattleEvent::ModifyPriority,
                    context,
                    EventValue::Integer(i64::from(priority)),
                )?;
                let priority = event_i8(&priority_event, BattleEvent::ModifyPriority)?;
                let prankster_boosted = priority_event.prankster_boosted;
                events.push(priority_event);
                (priority, prankster_boosted)
            } else {
                (priority, false)
            };
            let fractional_priority = if order == ActionOrder::Move {
                let event = mechanics.effects().dispatch(
                    self,
                    BattleEvent::ModifyFractionalPriority,
                    context,
                    EventValue::Integer(0),
                )?;
                let value = event_i8(&event, BattleEvent::ModifyFractionalPriority)?;
                events.push(event);
                value
            } else {
                0
            };
            let speed_event = mechanics.effects().dispatch(
                self,
                BattleEvent::ModifySpeed,
                context,
                EventValue::Integer(i64::from(speed)),
            )?;
            let speed = mechanics.field_effects().action_speed(
                self,
                side,
                event_u16(&speed_event, BattleEvent::ModifySpeed)?,
            );
            events.push(speed_event);
            output.push(QueuedAction {
                side,
                slot,
                pokemon,
                choice: action,
                order,
                priority,
                fractional_priority,
                speed,
                prankster_boosted,
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

fn base_action_ordering(
    pokemon: &PokemonState,
    action: PokemonAction,
) -> Result<(ActionOrder, i8, u16, Option<crate::MoveId>), BattleError> {
    let speed = pokemon.staged_speed();
    match action {
        PokemonAction::Move { mega: true, .. } => Err(BattleError::UnsupportedMechanic(
            "Mega Evolution queue expansion is deferred".to_owned(),
        )),
        PokemonAction::Move {
            move_slot,
            mega: false,
            ..
        } => {
            let move_data = pokemon
                .moves
                .get(usize::from(move_slot.get()))
                .ok_or_else(|| {
                    BattleError::InvalidState("queued move-slot index is out of bounds".to_owned())
                })?;
            Ok((
                ActionOrder::Move,
                move_data.priority,
                speed,
                Some(move_data.move_id),
            ))
        }
        PokemonAction::Struggle => Ok((ActionOrder::Move, 0, speed, None)),
        PokemonAction::Switch { .. } => Ok((ActionOrder::Switch, 0, speed, None)),
        PokemonAction::Pass => Err(BattleError::InvalidState(
            "pass actions must be removed before queue expansion".to_owned(),
        )),
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
        .then_with(|| right.fractional_priority.cmp(&left.fractional_priority))
        .then_with(|| right.speed.cmp(&left.speed))
}

fn ordering_keys_match(left: QueuedAction, right: QueuedAction) -> bool {
    left.order == right.order
        && left.priority == right.priority
        && left.fractional_priority == right.fractional_priority
        && left.speed == right.speed
}
