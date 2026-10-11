use super::{ActiveSlot, BattlePhase, BattleState, PokemonIndex, PokemonPosition, StatusState};
use crate::{BattleError, SideId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// One active Pokémon to resolve as fainted in an ordered faint batch.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaintTarget {
    pub side: SideId,
    pub slot: ActiveSlot,
}

impl FaintTarget {
    #[must_use]
    pub const fn new(side: SideId, slot: ActiveSlot) -> Self {
        Self { side, slot }
    }
}

/// The currently valid choices for a side that must replace fainted actives.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReplacementRequest {
    /// Empty active positions that may receive a replacement.
    pub slots: Vec<ActiveSlot>,
    /// Living selected-team members that may switch in.
    pub available: Vec<PokemonIndex>,
    /// Number of the empty positions that must be filled.
    pub required: usize,
}

impl BattleState {
    /// Submits one side's ordered team-preview selection.
    ///
    /// The first two selected Pokémon become the doubles leads once both sides
    /// have submitted. Unlike Showdown's text-command convenience behavior,
    /// this typed API requires the complete picked team instead of auto-filling
    /// omitted slots.
    ///
    /// # Errors
    ///
    /// Returns an error outside team preview, on a repeated submission, or if
    /// the selection has the wrong size, duplicates, or invalid team indexes.
    pub fn select_team(
        &mut self,
        side: SideId,
        selection: Vec<PokemonIndex>,
    ) -> Result<(), BattleError> {
        self.require_phase(BattlePhase::TeamPreview, "select a team")?;
        if !self.side(side).selected_team.is_empty() {
            return Err(BattleError::IllegalChoice(format!(
                "{side:?} already submitted a team-preview selection"
            )));
        }
        self.validate_selection(side, &selection)?;

        let opponent = side.opponent();
        if !self.side(opponent).selected_team.is_empty() {
            self.validate_selection(opponent, &self.side(opponent).selected_team)?;
        }

        self.side_mut(side).selected_team = selection;
        if self
            .sides
            .iter()
            .all(|state| state.selected_team.len() == usize::from(self.picked_team_size))
        {
            self.activate_initial_leads()?;
        }
        Ok(())
    }

    /// Applies a basic voluntary switch during turn selection.
    ///
    /// Action collection and ordering are added in later milestones. This
    /// transition updates only authoritative position state and switch-out
    /// state that always clears; switch event hooks are intentionally deferred.
    ///
    /// # Errors
    ///
    /// Returns an error outside turn selection or when the target is not a
    /// living, selected, inactive team member.
    pub fn switch_active(
        &mut self,
        side: SideId,
        slot: ActiveSlot,
        incoming: PokemonIndex,
    ) -> Result<(), BattleError> {
        self.require_phase(BattlePhase::TurnSelection, "switch")?;
        self.validate_switch_target(side, incoming)?;

        let slot_index = slot.index();
        let outgoing = self.side(side).active[slot_index].ok_or_else(|| {
            BattleError::IllegalChoice(format!(
                "{side:?} has no active Pokémon in the {slot:?} slot"
            ))
        })?;
        if outgoing == incoming {
            return Err(BattleError::IllegalChoice(
                "an active Pokémon cannot switch to itself".to_owned(),
            ));
        }

        let side_state = self.side_mut(side);
        let outgoing_state = side_state.pokemon_mut(outgoing).ok_or_else(|| {
            BattleError::InvalidState("active Pokémon index is out of bounds".to_owned())
        })?;
        if outgoing_state.is_fainted() || outgoing_state.current_hp == 0 {
            return Err(BattleError::InvalidState(
                "a fainted active Pokémon requires a forced replacement".to_owned(),
            ));
        }
        outgoing_state.clear_on_exit();
        outgoing_state.position = PokemonPosition::Team;

        let incoming_state = side_state.pokemon_mut(incoming).ok_or_else(|| {
            BattleError::InvalidState("switch target index became invalid".to_owned())
        })?;
        incoming_state.position = PokemonPosition::Active { slot };
        side_state.active[slot_index] = Some(incoming);
        Ok(())
    }

    /// Resolves an ordered batch of active Pokémon as fainted.
    ///
    /// Batch order matters for the pinned Gen 9 Showdown reference when both
    /// sides lose their final Pokémon: the side of the last fainted target wins.
    ///
    /// # Errors
    ///
    /// Returns an error during team preview or after battle end, for duplicate
    /// targets, or when a target does not identify a current active Pokémon.
    pub fn process_faints(&mut self, targets: &[FaintTarget]) -> Result<(), BattleError> {
        if matches!(
            self.phase,
            BattlePhase::TeamPreview | BattlePhase::BattleEnd
        ) {
            return Err(BattleError::IllegalChoice(format!(
                "cannot process faints during {:?}",
                self.phase
            )));
        }
        if targets.is_empty() {
            return Err(BattleError::IllegalChoice(
                "a faint batch must contain at least one target".to_owned(),
            ));
        }

        let mut seen_targets = HashSet::new();
        let mut seen_pokemon = HashSet::new();
        for target in targets {
            if !seen_targets.insert(*target) {
                return Err(BattleError::IllegalChoice(format!(
                    "duplicate faint target for {:?} {:?}",
                    target.side, target.slot
                )));
            }
            let index = self.side(target.side).active[target.slot.index()].ok_or_else(|| {
                BattleError::IllegalChoice(format!(
                    "{:?} has no active Pokémon in the {:?} slot",
                    target.side, target.slot
                ))
            })?;
            if !seen_pokemon.insert((target.side, index)) {
                return Err(BattleError::InvalidState(
                    "one Pokémon occupies multiple active slots".to_owned(),
                ));
            }
            let pokemon = self.side(target.side).pokemon(index).ok_or_else(|| {
                BattleError::InvalidState("active Pokémon index is out of bounds".to_owned())
            })?;
            if pokemon.is_fainted() {
                return Err(BattleError::IllegalChoice(
                    "a Pokémon cannot faint more than once".to_owned(),
                ));
            }
        }

        for target in targets {
            let slot_index = target.slot.index();
            let pokemon_index = self.side(target.side).active[slot_index].ok_or_else(|| {
                BattleError::InvalidState("validated active slot became empty".to_owned())
            })?;
            let side_state = self.side_mut(target.side);
            let pokemon = side_state.pokemon_mut(pokemon_index).ok_or_else(|| {
                BattleError::InvalidState("validated Pokémon index became invalid".to_owned())
            })?;
            pokemon.current_hp = 0;
            pokemon.status = None;
            pokemon.status_state = StatusState::default();
            pokemon.clear_on_exit();
            pokemon.position = PokemonPosition::Fainted;
            side_state.active[slot_index] = None;
        }

        let last_fainted_side = targets
            .last()
            .map(|target| target.side)
            .ok_or_else(|| BattleError::InvalidState("faint batch became empty".to_owned()))?;
        self.settle_after_faints(last_fainted_side)
    }

    /// Returns the side's current forced-replacement requirements.
    ///
    /// # Errors
    ///
    /// Returns an error when selected-team indexes do not refer to loaded team
    /// members, which indicates externally corrupted battle state.
    pub fn replacement_request(
        &self,
        side: SideId,
    ) -> Result<Option<ReplacementRequest>, BattleError> {
        if self.phase != BattlePhase::ForcedSwitch {
            return Ok(None);
        }
        let request = self.compute_replacement_request(side)?;
        Ok((request.required > 0).then_some(request))
    }

    /// Fills one empty active position during forced replacement.
    ///
    /// # Errors
    ///
    /// Returns an error outside forced switching or when the chosen slot or
    /// Pokémon is not present in the current replacement request.
    pub fn replace_fainted(
        &mut self,
        side: SideId,
        slot: ActiveSlot,
        incoming: PokemonIndex,
    ) -> Result<(), BattleError> {
        self.require_phase(BattlePhase::ForcedSwitch, "replace a fainted Pokémon")?;
        let request = self.replacement_request(side)?.ok_or_else(|| {
            BattleError::IllegalChoice(format!("{side:?} has no replacement due"))
        })?;
        if !request.slots.contains(&slot) {
            return Err(BattleError::IllegalChoice(format!(
                "the {slot:?} slot does not need a replacement"
            )));
        }
        if !request.available.contains(&incoming) {
            return Err(BattleError::IllegalChoice(
                "the selected Pokémon is not an available replacement".to_owned(),
            ));
        }

        let side_state = self.side_mut(side);
        let incoming_state = side_state.pokemon_mut(incoming).ok_or_else(|| {
            BattleError::InvalidState("replacement index became invalid".to_owned())
        })?;
        incoming_state.prepare_switch_in();
        incoming_state.position = PokemonPosition::Active { slot };
        side_state.active[slot.index()] = Some(incoming);

        if self.compute_replacement_request(SideId::P1)?.required == 0
            && self.compute_replacement_request(SideId::P2)?.required == 0
        {
            self.phase = BattlePhase::TurnSelection;
        }
        Ok(())
    }

    fn validate_selection(
        &self,
        side: SideId,
        selection: &[PokemonIndex],
    ) -> Result<(), BattleError> {
        if selection.len() != usize::from(self.picked_team_size) {
            return Err(BattleError::IllegalChoice(format!(
                "team preview requires exactly {} Pokémon",
                self.picked_team_size
            )));
        }
        let mut seen = HashSet::new();
        for &index in selection {
            if !seen.insert(index) {
                return Err(BattleError::IllegalChoice(format!(
                    "Pokémon index {} was selected more than once",
                    index.get()
                )));
            }
            let pokemon = self.side(side).pokemon(index).ok_or_else(|| {
                BattleError::IllegalChoice(format!(
                    "Pokémon index {} is outside {side:?}'s team",
                    index.get()
                ))
            })?;
            if pokemon.position != PokemonPosition::Team || pokemon.current_hp == 0 {
                return Err(BattleError::InvalidState(format!(
                    "Pokémon index {} is not healthy on the team during preview",
                    index.get()
                )));
            }
        }
        Ok(())
    }

    fn activate_initial_leads(&mut self) -> Result<(), BattleError> {
        for side in [SideId::P1, SideId::P2] {
            let leads = [
                self.side(side).selected_team[0],
                self.side(side).selected_team[1],
            ];
            for (slot, index) in ActiveSlot::ALL.into_iter().zip(leads) {
                let side_state = self.side_mut(side);
                let pokemon = side_state.pokemon_mut(index).ok_or_else(|| {
                    BattleError::InvalidState("selected lead index is out of bounds".to_owned())
                })?;
                pokemon.prepare_switch_in();
                pokemon.position = PokemonPosition::Active { slot };
                side_state.active[slot.index()] = Some(index);
            }
        }
        self.turn = 1;
        self.phase = BattlePhase::TurnSelection;
        Ok(())
    }

    fn validate_switch_target(
        &self,
        side: SideId,
        incoming: PokemonIndex,
    ) -> Result<(), BattleError> {
        let side_state = self.side(side);
        if !side_state.selected_team.contains(&incoming) {
            return Err(BattleError::IllegalChoice(
                "cannot switch to a Pokémon that was not picked at team preview".to_owned(),
            ));
        }
        let pokemon = side_state.pokemon(incoming).ok_or_else(|| {
            BattleError::InvalidState("selected switch target is out of bounds".to_owned())
        })?;
        if pokemon.position != PokemonPosition::Team || pokemon.current_hp == 0 {
            return Err(BattleError::IllegalChoice(
                "switch target must be a living inactive Pokémon".to_owned(),
            ));
        }
        Ok(())
    }

    fn settle_after_faints(&mut self, last_fainted_side: SideId) -> Result<(), BattleError> {
        let p1_remaining = self.remaining_selected(SideId::P1)?;
        let p2_remaining = self.remaining_selected(SideId::P2)?;
        self.winner = match (p1_remaining, p2_remaining) {
            (0, 0) => Some(last_fainted_side),
            (0, _) => Some(SideId::P2),
            (_, 0) => Some(SideId::P1),
            _ => None,
        };
        if self.winner.is_some() {
            self.phase = BattlePhase::BattleEnd;
            return Ok(());
        }

        let replacements_required = self.compute_replacement_request(SideId::P1)?.required > 0
            || self.compute_replacement_request(SideId::P2)?.required > 0;
        self.phase = if replacements_required {
            BattlePhase::ForcedSwitch
        } else {
            BattlePhase::TurnSelection
        };
        Ok(())
    }

    fn remaining_selected(&self, side: SideId) -> Result<usize, BattleError> {
        self.side(side)
            .selected_team
            .iter()
            .try_fold(0, |remaining, &index| {
                let pokemon = self.side(side).pokemon(index).ok_or_else(|| {
                    BattleError::InvalidState("selected-team index is out of bounds".to_owned())
                })?;
                Ok(remaining + usize::from(pokemon.current_hp > 0 && !pokemon.is_fainted()))
            })
    }

    fn compute_replacement_request(&self, side: SideId) -> Result<ReplacementRequest, BattleError> {
        let side_state = self.side(side);
        let slots = ActiveSlot::ALL
            .into_iter()
            .filter(|slot| side_state.active[slot.index()].is_none())
            .collect::<Vec<_>>();
        let available = side_state
            .selected_team
            .iter()
            .copied()
            .filter_map(|index| {
                side_state
                    .pokemon(index)
                    .filter(|pokemon| {
                        pokemon.current_hp > 0 && pokemon.position == PokemonPosition::Team
                    })
                    .map(|_| index)
            })
            .collect::<Vec<_>>();
        for &index in &side_state.selected_team {
            if side_state.pokemon(index).is_none() {
                return Err(BattleError::InvalidState(
                    "selected-team index is out of bounds".to_owned(),
                ));
            }
        }
        let required = slots.len().min(available.len());
        Ok(ReplacementRequest {
            slots,
            available,
            required,
        })
    }

    fn require_phase(&self, phase: BattlePhase, action: &str) -> Result<(), BattleError> {
        if self.phase == phase {
            Ok(())
        } else {
            Err(BattleError::IllegalChoice(format!(
                "cannot {action} during {:?}",
                self.phase
            )))
        }
    }
}
