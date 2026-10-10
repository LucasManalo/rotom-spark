use super::{ActiveSlot, BattlePhase, BattleState, PokemonIndex, PokemonPosition};
use crate::{BattleError, PokemonAction, RngReason, SideId, TurnChoice};
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
    /// Current `BattleStats::spe` values are treated as the effective speeds.
    /// Priority/speed modifiers and Trick Room are intentionally deferred to
    /// their mechanics milestones.
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
        if self.phase != BattlePhase::TurnSelection {
            return Err(BattleError::IllegalChoice(format!(
                "cannot queue a turn during {:?}",
                self.phase
            )));
        }
        self.validate_choice(SideId::P1, &p1)?;
        self.validate_choice(SideId::P2, &p2)?;

        let mut actions = Vec::with_capacity(4);
        self.expand_choice(SideId::P1, p1, &mut actions)?;
        self.expand_choice(SideId::P2, p2, &mut actions)?;
        actions.sort_by(compare_actions);
        self.shuffle_speed_ties(&mut actions)?;

        self.queue = ActionQueue { actions };
        self.phase = BattlePhase::ResolvingTurn;
        Ok(&self.queue)
    }

    fn expand_choice(
        &self,
        side: SideId,
        choice: TurnChoice,
        output: &mut Vec<QueuedAction>,
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
            let pokemon_state = self.side(side).pokemon(pokemon).ok_or_else(|| {
                BattleError::InvalidState("queued active Pokémon index is out of bounds".to_owned())
            })?;
            if pokemon_state.position != (PokemonPosition::Active { slot })
                || pokemon_state.current_hp == 0
                || pokemon_state.is_fainted()
            {
                return Err(BattleError::InvalidState(format!(
                    "{side:?}'s {slot:?} queued Pokémon is inconsistent"
                )));
            }

            let (order, priority) = match action {
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
                    (ActionOrder::Move, move_data.priority)
                }
                PokemonAction::Struggle => (ActionOrder::Move, 0),
                PokemonAction::Switch { .. } => (ActionOrder::Switch, 0),
                PokemonAction::Pass => continue,
            };
            output.push(QueuedAction {
                side,
                slot,
                pokemon,
                choice: action,
                order,
                priority,
                speed: pokemon_state.stats.spe,
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

fn compare_actions(left: &QueuedAction, right: &QueuedAction) -> Ordering {
    left.order
        .cmp(&right.order)
        .then_with(|| right.priority.cmp(&left.priority))
        .then_with(|| right.speed.cmp(&left.speed))
}

fn ordering_keys_match(left: QueuedAction, right: QueuedAction) -> bool {
    left.order == right.order && left.priority == right.priority && left.speed == right.speed
}
