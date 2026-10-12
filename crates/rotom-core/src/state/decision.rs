use super::{ActiveSlot, BattlePhase, BattleState, PokemonPosition, ReplacementRequest};
use crate::{
    ActionTarget, BattleError, LegalActions, MoveSlotIndex, MoveTarget, PokemonAction, SideId,
    SlotLegalActions, TurnChoice,
};
use std::collections::HashSet;

impl BattleState {
    /// Enumerates every legal doubles choice for one side in deterministic order.
    ///
    /// Move slots and switch targets retain team order. For each move target,
    /// the ordinary action precedes its Mega variant. Combined choices use the
    /// left action as the outer ordering key.
    ///
    /// # Errors
    ///
    /// Returns an error when the battle is not accepting turn or replacement
    /// choices, or when authoritative state is internally inconsistent.
    pub fn legal_actions(&self, side: SideId) -> Result<LegalActions, BattleError> {
        let slots = match self.phase {
            BattlePhase::TurnSelection => [
                self.turn_slot_actions(side, ActiveSlot::Left)?,
                self.turn_slot_actions(side, ActiveSlot::Right)?,
            ],
            BattlePhase::ForcedSwitch => self.forced_switch_actions(side)?,
            phase => {
                return Err(BattleError::IllegalChoice(format!(
                    "cannot request Pokémon actions during {phase:?}"
                )));
            }
        };

        let choices = combined_choices(&slots, self.phase);
        Ok(LegalActions { slots, choices })
    }

    /// Rejects a choice unless it is one of the currently enumerated choices.
    ///
    /// # Errors
    ///
    /// Returns [`BattleError::IllegalChoice`] for stale, malformed, or
    /// otherwise illegal choices, and propagates invalid-state errors found
    /// while enumerating the authoritative state.
    pub fn validate_choice(&self, side: SideId, choice: &TurnChoice) -> Result<(), BattleError> {
        let legal = self.legal_actions(side)?;
        if legal.choices.contains(choice) {
            Ok(())
        } else {
            Err(BattleError::IllegalChoice(format!(
                "choice is not legal for {side:?} during {:?}",
                self.phase
            )))
        }
    }

    fn turn_slot_actions(
        &self,
        side: SideId,
        slot: ActiveSlot,
    ) -> Result<SlotLegalActions, BattleError> {
        let side_state = self.side(side);
        let Some(pokemon_index) = side_state.active[slot.index()] else {
            return Ok(slot_actions(slot, vec![PokemonAction::Pass]));
        };
        let pokemon = side_state.pokemon(pokemon_index).ok_or_else(|| {
            BattleError::InvalidState("active Pokémon index is out of bounds".to_owned())
        })?;
        if pokemon.position != (PokemonPosition::Active { slot })
            || pokemon.current_hp == 0
            || pokemon.is_fainted()
        {
            return Err(BattleError::InvalidState(format!(
                "{side:?}'s {slot:?} active position is inconsistent"
            )));
        }

        let can_mega = pokemon.can_mega_evolve && !side_state.mega_evolution_used;
        let mut actions = Vec::new();
        for (raw_index, move_slot) in pokemon.moves.iter().enumerate() {
            if move_slot.disabled || move_slot.pp == 0 {
                continue;
            }
            let raw_index = u8::try_from(raw_index).map_err(|_| {
                BattleError::InvalidState("a Pokémon has more than 256 move slots".to_owned())
            })?;
            for target in targets_for(slot, move_slot.target) {
                actions.push(PokemonAction::Move {
                    move_slot: MoveSlotIndex::new(raw_index),
                    target,
                    mega: false,
                });
                if can_mega {
                    actions.push(PokemonAction::Move {
                        move_slot: MoveSlotIndex::new(raw_index),
                        target,
                        mega: true,
                    });
                }
            }
        }
        if actions.is_empty() {
            actions.push(PokemonAction::Struggle);
        }

        if !pokemon.trapped {
            actions.extend(
                available_switches(self, side)?
                    .into_iter()
                    .map(|pokemon| PokemonAction::Switch { pokemon }),
            );
        }
        Ok(slot_actions(slot, actions))
    }

    fn forced_switch_actions(&self, side: SideId) -> Result<[SlotLegalActions; 2], BattleError> {
        let request = self
            .replacement_request(side)?
            .unwrap_or(ReplacementRequest {
                slots: Vec::new(),
                available: Vec::new(),
                required: 0,
            });
        let may_pass = request.required < request.slots.len();
        Ok(ActiveSlot::ALL.map(|slot| {
            let mut actions = Vec::new();
            if request.slots.contains(&slot) {
                actions.extend(
                    request
                        .available
                        .iter()
                        .copied()
                        .map(|pokemon| PokemonAction::Switch { pokemon }),
                );
                if may_pass {
                    actions.push(PokemonAction::Pass);
                }
            } else {
                actions.push(PokemonAction::Pass);
            }
            slot_actions(slot, actions)
        }))
    }
}

fn slot_actions(slot: ActiveSlot, actions: Vec<PokemonAction>) -> SlotLegalActions {
    SlotLegalActions { slot, actions }
}

fn available_switches(
    state: &BattleState,
    side: SideId,
) -> Result<Vec<crate::PokemonIndex>, BattleError> {
    let side_state = state.side(side);
    side_state
        .selected_team
        .iter()
        .copied()
        .filter_map(|index| match side_state.pokemon(index) {
            Some(pokemon)
                if pokemon.current_hp > 0 && pokemon.position == PokemonPosition::Team =>
            {
                Some(Ok(index))
            }
            Some(_) => None,
            None => Some(Err(BattleError::InvalidState(
                "selected-team index is out of bounds".to_owned(),
            ))),
        })
        .collect()
}

fn targets_for(source: ActiveSlot, target: MoveTarget) -> Vec<ActionTarget> {
    let ally = match source {
        ActiveSlot::Left => ActiveSlot::Right,
        ActiveSlot::Right => ActiveSlot::Left,
    };
    match target {
        MoveTarget::AdjacentAlly => vec![ActionTarget::Ally(ally)],
        MoveTarget::AdjacentAllyOrSelf => {
            vec![ActionTarget::Self_, ActionTarget::Ally(ally)]
        }
        MoveTarget::AdjacentFoe => vec![
            ActionTarget::Foe(ActiveSlot::Left),
            ActionTarget::Foe(ActiveSlot::Right),
        ],
        MoveTarget::Any | MoveTarget::Normal => vec![
            ActionTarget::Ally(ally),
            ActionTarget::Foe(ActiveSlot::Left),
            ActionTarget::Foe(ActiveSlot::Right),
        ],
        MoveTarget::All
        | MoveTarget::AllAdjacent
        | MoveTarget::AllAdjacentFoes
        | MoveTarget::Allies
        | MoveTarget::AllySide
        | MoveTarget::AllyTeam
        | MoveTarget::FoeSide
        | MoveTarget::RandomNormal
        | MoveTarget::Scripted
        | MoveTarget::Self_ => vec![ActionTarget::Automatic],
    }
}

fn combined_choices(slots: &[SlotLegalActions; 2], phase: BattlePhase) -> Vec<TurnChoice> {
    let switch_required = phase == BattlePhase::ForcedSwitch
        && slots.iter().any(|slot| {
            slot.actions
                .iter()
                .any(|action| matches!(action, PokemonAction::Switch { .. }))
        });
    let mut choices = Vec::new();
    for &left in &slots[0].actions {
        for &right in &slots[1].actions {
            let choice = TurnChoice {
                actions: [left, right],
            };
            if combination_is_legal(choice, switch_required) {
                choices.push(choice);
            }
        }
    }
    choices
}

fn combination_is_legal(choice: TurnChoice, switch_required: bool) -> bool {
    let mut switch_ins = HashSet::new();
    let mut mega_count = 0;
    let mut switch_count = 0;
    for action in choice.actions {
        match action {
            PokemonAction::Move { mega: true, .. } => mega_count += 1,
            PokemonAction::Switch { pokemon } => {
                switch_count += 1;
                if !switch_ins.insert(pokemon) {
                    return false;
                }
            }
            PokemonAction::Move { mega: false, .. }
            | PokemonAction::Struggle
            | PokemonAction::Pass => {}
        }
    }
    mega_count <= 1 && (!switch_required || switch_count > 0)
}
