use crate::{
    state::validate_status_state, BattleError, BattlePhase, BattleState, ConditionState,
    PokemonIndex, PokemonPosition, SideId, StatStages, StatusState,
};
use std::collections::HashSet;

impl BattleState {
    /// Validates the representation invariants required at a stable public
    /// transition boundary.
    ///
    /// `ResolvingTurn` and a partially submitted team preview are deliberate
    /// migration-era internal/compatibility states and are rejected here.
    ///
    /// # Errors
    ///
    /// Returns [`BattleError::InvalidState`] for inconsistent references,
    /// bounds, counters, phase settlement, or terminal state.
    pub fn validate_stable_state(&self) -> Result<(), BattleError> {
        validate_stable_state(self)
    }
}

pub(super) fn validate_stable_state(battle: &BattleState) -> Result<(), BattleError> {
    if battle.picked_team_size < 2 {
        return invalid("picked-team size cannot supply doubles leads");
    }
    validate_conditions(&battle.field.conditions, "field conditions")?;
    validate_single_condition(battle.field.weather, "weather")?;
    validate_single_condition(battle.field.terrain, "terrain")?;
    validate_active_effect_orders(battle)?;

    for side in [SideId::P1, SideId::P2] {
        validate_side(battle, side)?;
    }

    if battle.phase == BattlePhase::ResolvingTurn {
        return invalid("ResolvingTurn is not a stable public boundary");
    }
    if !battle.queue.actions.is_empty() || !battle.queue.events.is_empty() {
        return invalid("a stable public state cannot retain a pending action queue");
    }

    match battle.phase {
        BattlePhase::TeamPreview => validate_team_preview(battle),
        BattlePhase::TurnSelection => {
            validate_started_battle(battle)?;
            if replacement_count(battle, SideId::P1)? > 0
                || replacement_count(battle, SideId::P2)? > 0
            {
                return invalid("TurnSelection cannot have a mandatory replacement pending");
            }
            Ok(())
        }
        BattlePhase::ForcedSwitch => {
            validate_started_battle(battle)?;
            if replacement_count(battle, SideId::P1)? == 0
                && replacement_count(battle, SideId::P2)? == 0
            {
                return invalid("ForcedSwitch requires at least one mandatory replacement");
            }
            Ok(())
        }
        BattlePhase::BattleEnd => validate_battle_end(battle),
        BattlePhase::ResolvingTurn => invalid("ResolvingTurn is not a stable public boundary"),
    }
}

fn validate_active_effect_orders(battle: &BattleState) -> Result<(), BattleError> {
    let mut orders = HashSet::new();
    for (side_index, side) in [SideId::P1, SideId::P2].into_iter().enumerate() {
        for slot in crate::ActiveSlot::ALL {
            let active = battle.side(side).active[slot.index()];
            let order = battle.active_effect_orders[side_index][slot.index()];
            if active.is_some() != order.is_some() {
                return invalid(format!(
                    "{side:?} {slot:?} active slot and effect order disagree"
                ));
            }
            if let Some(order) = order {
                if order >= battle.next_effect_order {
                    return invalid("active effect order is not below the next order counter");
                }
                if !orders.insert(order) {
                    return invalid("active effect orders must be unique");
                }
            }
        }
    }
    Ok(())
}

fn validate_side(battle: &BattleState, side: SideId) -> Result<(), BattleError> {
    let side_state = battle.side(side);
    if side_state.team.members.len() < usize::from(battle.picked_team_size) {
        return invalid(format!(
            "{side:?} has fewer loaded Pokémon than the picked-team size"
        ));
    }
    if side_state.team.members.len() > usize::from(u8::MAX) + 1 {
        return invalid(format!(
            "{side:?}'s team cannot be addressed by PokemonIndex"
        ));
    }

    let mut selected = HashSet::new();
    for &index in &side_state.selected_team {
        if !selected.insert(index) {
            return invalid(format!(
                "{side:?}'s selected team contains a duplicate index"
            ));
        }
        if side_state.pokemon(index).is_none() {
            return invalid(format!("{side:?}'s selected-team index is out of bounds"));
        }
    }
    validate_conditions(&side_state.side_conditions, "side conditions")?;

    let mut active = HashSet::new();
    for slot in crate::ActiveSlot::ALL {
        let Some(index) = side_state.active[slot.index()] else {
            continue;
        };
        if !active.insert(index) {
            return invalid(format!(
                "{side:?} maps one Pokémon to multiple active slots"
            ));
        }
        if !selected.contains(&index) {
            return invalid(format!("{side:?}'s active Pokémon was not selected"));
        }
        let pokemon = side_state.pokemon(index).ok_or_else(|| {
            BattleError::InvalidState(format!("{side:?}'s active index is out of bounds"))
        })?;
        if pokemon.position != (PokemonPosition::Active { slot }) {
            return invalid(format!(
                "{side:?}'s active slot and Pokémon position disagree"
            ));
        }
    }

    for (raw_index, pokemon) in side_state.team.members.iter().enumerate() {
        let raw_index = u8::try_from(raw_index).map_err(|_| {
            BattleError::InvalidState(format!("{side:?}'s team index does not fit PokemonIndex"))
        })?;
        let index = PokemonIndex::new(raw_index);
        validate_pokemon(pokemon, side, index)?;
        match pokemon.position {
            PokemonPosition::Active { slot } => {
                if side_state.active[slot.index()] != Some(index) {
                    return invalid(format!(
                        "{side:?}'s Pokémon position lacks its active-slot back-reference"
                    ));
                }
                if !selected.contains(&index) {
                    return invalid(format!("{side:?}'s active Pokémon was not selected"));
                }
            }
            PokemonPosition::Team => {
                if active.contains(&index) {
                    return invalid(format!(
                        "{side:?}'s benched Pokémon appears in an active slot"
                    ));
                }
            }
            PokemonPosition::Fainted => {
                if active.contains(&index) {
                    return invalid(format!(
                        "{side:?}'s fainted Pokémon appears in an active slot"
                    ));
                }
                if !selected.contains(&index) {
                    return invalid(format!("{side:?}'s unselected Pokémon cannot be fainted"));
                }
            }
        }
    }
    Ok(())
}

fn validate_pokemon(
    pokemon: &crate::PokemonState,
    side: SideId,
    index: PokemonIndex,
) -> Result<(), BattleError> {
    if pokemon.current_hp > pokemon.stats.hp {
        return invalid(format!("{side:?} Pokémon {} exceeds max HP", index.get()));
    }
    match pokemon.position {
        PokemonPosition::Fainted => {
            if pokemon.current_hp != 0
                || pokemon.status.is_some()
                || pokemon.status_state != StatusState::default()
                || pokemon.stat_stages != StatStages::default()
                || !pokemon.volatiles.is_empty()
                || pokemon.trapped
                || pokemon.moves.iter().any(|move_slot| move_slot.disabled)
            {
                return invalid(format!(
                    "{side:?} Pokémon {} has an inconsistent faint representation",
                    index.get()
                ));
            }
        }
        PokemonPosition::Team | PokemonPosition::Active { .. } => {
            if pokemon.current_hp == 0 {
                return invalid(format!(
                    "{side:?} Pokémon {} has zero HP without being fainted",
                    index.get()
                ));
            }
        }
    }

    match pokemon.status {
        Some(status) => validate_status_state(status, pokemon.status_state)?,
        None if pokemon.status_state != StatusState::default() => {
            return invalid(format!(
                "{side:?} Pokémon {} has status counters without a status",
                index.get()
            ));
        }
        None => {}
    }
    if pokemon.cud_chew_berry.is_some() != (pokemon.cud_chew_turns > 0) {
        return invalid(format!(
            "{side:?} Pokémon {} has inconsistent Cud Chew state",
            index.get()
        ));
    }
    for stage in [
        pokemon.stat_stages.atk,
        pokemon.stat_stages.def,
        pokemon.stat_stages.spa,
        pokemon.stat_stages.spd,
        pokemon.stat_stages.spe,
        pokemon.stat_stages.accuracy,
        pokemon.stat_stages.evasion,
    ] {
        if !(StatStages::MIN..=StatStages::MAX).contains(&stage) {
            return invalid(format!(
                "{side:?} Pokémon {} has an out-of-range stat stage",
                index.get()
            ));
        }
    }
    for move_slot in &pokemon.moves {
        if move_slot.pp > move_slot.max_pp {
            return invalid(format!(
                "{side:?} Pokémon {} has PP above max PP",
                index.get()
            ));
        }
    }
    validate_conditions(&pokemon.volatiles, "volatile conditions")
}

fn validate_team_preview(battle: &BattleState) -> Result<(), BattleError> {
    if battle.turn != 0 || battle.winner.is_some() {
        return invalid("TeamPreview requires turn zero and no winner");
    }
    for side in [SideId::P1, SideId::P2] {
        let side_state = battle.side(side);
        if !side_state.selected_team.is_empty() || side_state.active != [None, None] {
            return invalid("stable TeamPreview cannot contain partial choices or active Pokémon");
        }
    }
    Ok(())
}

fn validate_started_battle(battle: &BattleState) -> Result<(), BattleError> {
    if battle.turn == 0 || battle.winner.is_some() {
        return invalid("an active battle requires a positive turn and no winner");
    }
    for side in [SideId::P1, SideId::P2] {
        if battle.side(side).selected_team.len() != usize::from(battle.picked_team_size) {
            return invalid(format!("{side:?} does not have a complete selected team"));
        }
        if remaining_selected(battle, side)? == 0 {
            return invalid(format!(
                "{side:?} has no remaining selected Pokémon outside BattleEnd"
            ));
        }
    }
    Ok(())
}

fn validate_battle_end(battle: &BattleState) -> Result<(), BattleError> {
    if battle.turn == 0 {
        return invalid("BattleEnd requires a started battle");
    }
    for side in [SideId::P1, SideId::P2] {
        if battle.side(side).selected_team.len() != usize::from(battle.picked_team_size) {
            return invalid(format!("{side:?} does not have a complete selected team"));
        }
    }
    let p1 = remaining_selected(battle, SideId::P1)?;
    let p2 = remaining_selected(battle, SideId::P2)?;
    match battle.winner {
        Some(SideId::P1) if p2 == 0 => Ok(()),
        Some(SideId::P2) if p1 == 0 => Ok(()),
        None if p1 == 0 && p2 == 0 => Ok(()),
        Some(_) | None => invalid("BattleEnd winner disagrees with remaining selected Pokémon"),
    }
}

fn replacement_count(battle: &BattleState, side: SideId) -> Result<usize, BattleError> {
    let side_state = battle.side(side);
    let empty = side_state
        .active
        .iter()
        .filter(|slot| slot.is_none())
        .count();
    let mut available = 0;
    for &index in &side_state.selected_team {
        let pokemon = side_state.pokemon(index).ok_or_else(|| {
            BattleError::InvalidState("selected-team index is out of bounds".to_owned())
        })?;
        if pokemon.position == PokemonPosition::Team && pokemon.current_hp > 0 {
            available += 1;
        }
    }
    Ok(empty.min(available))
}

fn remaining_selected(battle: &BattleState, side: SideId) -> Result<usize, BattleError> {
    battle
        .side(side)
        .selected_team
        .iter()
        .try_fold(0, |remaining, &index| {
            let pokemon = battle.side(side).pokemon(index).ok_or_else(|| {
                BattleError::InvalidState("selected-team index is out of bounds".to_owned())
            })?;
            Ok(remaining + usize::from(pokemon.current_hp > 0 && !pokemon.is_fainted()))
        })
}

fn validate_conditions(states: &[ConditionState], label: &str) -> Result<(), BattleError> {
    let mut seen = HashSet::new();
    for state in states {
        validate_single_condition(Some(*state), label)?;
        if !seen.insert(state.id) {
            return invalid(format!("{label} contain a duplicate identity"));
        }
    }
    Ok(())
}

fn validate_single_condition(
    state: Option<ConditionState>,
    label: &str,
) -> Result<(), BattleError> {
    if state.is_some_and(|state| state.turns_remaining == Some(0) || state.layers == 0) {
        return invalid(format!("{label} cannot have zero duration or layers"));
    }
    Ok(())
}

fn invalid<T>(message: impl Into<String>) -> Result<T, BattleError> {
    Err(BattleError::InvalidState(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AbilityId, BattleStats, FaintTarget, MoveId, MoveSlot, MoveTarget, PokemonState, SideState,
        SpeciesId, TeamState,
    };

    fn pokemon(species: u32) -> PokemonState {
        PokemonState {
            species: SpeciesId::new(species),
            form: None,
            level: 50,
            stats: BattleStats {
                hp: 100,
                atk: 100,
                def: 100,
                spa: 100,
                spd: 100,
                spe: 100,
            },
            current_hp: 100,
            ability: AbilityId::new(1),
            item: None,
            last_consumed_berry: None,
            cud_chew_berry: None,
            cud_chew_turns: 0,
            moves: vec![MoveSlot {
                move_id: MoveId::new(1),
                target: MoveTarget::Normal,
                priority: 0,
                pp: 16,
                max_pp: 16,
                disabled: false,
            }],
            status: None,
            status_state: StatusState::default(),
            stat_stages: StatStages::default(),
            volatiles: Vec::new(),
            can_mega_evolve: false,
            trapped: false,
            position: PokemonPosition::Team,
        }
    }

    fn preview() -> BattleState {
        let make_side = |offset: u32| {
            SideState::new(TeamState::new(
                (0_u32..4).map(|value| pokemon(offset + value)).collect(),
            ))
        };
        BattleState::new(make_side(1), make_side(101), 4).unwrap()
    }

    fn started() -> BattleState {
        let mut battle = preview();
        let selected = (0..4).map(PokemonIndex::new).collect::<Vec<_>>();
        battle.select_team(SideId::P1, selected.clone()).unwrap();
        battle.select_team(SideId::P2, selected).unwrap();
        battle
    }

    #[test]
    fn valid_preview_turn_forced_switch_and_battle_end_states_pass() {
        preview().validate_stable_state().unwrap();
        let mut battle = started();
        battle.validate_stable_state().unwrap();
        battle
            .process_faints(&[FaintTarget::new(SideId::P1, crate::ActiveSlot::Left)])
            .unwrap();
        assert_eq!(battle.phase, BattlePhase::ForcedSwitch);
        battle.validate_stable_state().unwrap();

        let mut terminal = BattleState::new(
            SideState::new(TeamState::new(vec![pokemon(1), pokemon(2)])),
            SideState::new(TeamState::new(vec![pokemon(101), pokemon(102)])),
            2,
        )
        .unwrap();
        let selected = vec![PokemonIndex::new(0), PokemonIndex::new(1)];
        terminal.select_team(SideId::P1, selected.clone()).unwrap();
        terminal.select_team(SideId::P2, selected).unwrap();
        terminal
            .process_faints(&[
                FaintTarget::new(SideId::P2, crate::ActiveSlot::Left),
                FaintTarget::new(SideId::P2, crate::ActiveSlot::Right),
            ])
            .unwrap();
        terminal.validate_stable_state().unwrap();
    }

    #[test]
    fn validation_rejects_bounds_references_conditions_and_phase_mismatches() {
        let mut hp = started();
        hp.side_mut(SideId::P1)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .current_hp = 101;
        assert!(hp.validate_stable_state().is_err());

        let mut stage = started();
        stage
            .side_mut(SideId::P1)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .stat_stages
            .atk = 7;
        assert!(stage.validate_stable_state().is_err());

        let mut pp = started();
        pp.side_mut(SideId::P1)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .moves[0]
            .pp = 17;
        assert!(pp.validate_stable_state().is_err());

        let mut position = started();
        position.side_mut(SideId::P1).active[0] = Some(PokemonIndex::new(2));
        assert!(position.validate_stable_state().is_err());

        let mut missing_effect_order = started();
        missing_effect_order.active_effect_orders[0][0] = None;
        assert!(missing_effect_order.validate_stable_state().is_err());

        let mut duplicate_effect_order = started();
        duplicate_effect_order.active_effect_orders[0][1] =
            duplicate_effect_order.active_effect_orders[0][0];
        assert!(duplicate_effect_order.validate_stable_state().is_err());

        let mut duplicate = started();
        let condition = ConditionState::new(crate::ConditionId::new(9), Some(2));
        duplicate.field.conditions = vec![condition, condition];
        assert!(duplicate.validate_stable_state().is_err());

        let mut zero_layers = started();
        zero_layers.field.conditions = vec![ConditionState::with_layers(
            crate::ConditionId::new(10),
            None,
            0,
        )];
        assert!(zero_layers.validate_stable_state().is_err());

        let mut duplicate_volatile = started();
        duplicate_volatile
            .side_mut(SideId::P1)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .volatiles = vec![condition, condition];
        assert!(duplicate_volatile.validate_stable_state().is_err());

        let mut status_counters = started();
        status_counters
            .side_mut(SideId::P1)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .status_state
            .sleep_turns = 1;
        assert!(status_counters.validate_stable_state().is_err());

        let mut wrong_phase = started();
        wrong_phase.phase = BattlePhase::ForcedSwitch;
        assert!(wrong_phase.validate_stable_state().is_err());
    }
}
