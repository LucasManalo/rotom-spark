mod validation;

use crate::{
    AbilityId, ActiveSlot, BattleError, BattleStat, BattleState, ConditionId, ConditionState,
    FormId, ItemId, MajorStatus, MegaEvolution, MoveSlotIndex, PokemonPosition, PokemonRef, SideId,
    StatStageChange, StatusState, SwitchRequest, SwitchRequestKind,
};

/// Atomic result of a form transformation applied by the kernel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MegaEvolutionChange {
    pub(crate) previous_form: Option<FormId>,
    pub(crate) current_form: FormId,
    pub(crate) previous_ability: AbilityId,
    pub(crate) current_ability: AbilityId,
    pub(crate) previous_max_hp: u16,
    pub(crate) current_max_hp: u16,
    pub(crate) previous_hp: u16,
    pub(crate) current_hp: u16,
}

/// The actual bounded result of one HP mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// PR B defines this result before PR D migrates production callers.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct HpChange {
    pub(crate) previous: u16,
    pub(crate) current: u16,
    pub(crate) requested: u16,
    pub(crate) applied: u16,
}

/// The previous and current PP after one bounded assignment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// PR B defines this result before PR D migrates production callers.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct PpChange {
    pub(crate) previous: u8,
    pub(crate) current: u8,
}

/// Internal, mechanically ignorant access to structurally safe state writes.
///
/// Callers decide why and when a change occurs. This type only resolves typed
/// references, applies the requested write, and preserves local representation
/// invariants. It never consumes RNG or dispatches effects.
// PR B defines the internal kernel before PR D migrates production callers.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct StateKernel<'a> {
    battle: &'a mut BattleState,
}

// PR B defines the internal kernel before PR D migrates production callers.
#[cfg_attr(not(test), allow(dead_code))]
impl<'a> StateKernel<'a> {
    pub(crate) const fn new(battle: &'a mut BattleState) -> Self {
        Self { battle }
    }

    pub(crate) fn damage_hp(
        &mut self,
        target: PokemonRef,
        requested: u16,
    ) -> Result<HpChange, BattleError> {
        let pokemon = self.pokemon_mut(target)?;
        if pokemon.current_hp > pokemon.stats.hp {
            return Err(BattleError::InvalidState(
                "cannot damage Pokémon whose HP exceeds max HP".to_owned(),
            ));
        }
        let previous = pokemon.current_hp;
        let current = previous.saturating_sub(requested);
        pokemon.current_hp = current;
        Ok(HpChange {
            previous,
            current,
            requested,
            applied: previous - current,
        })
    }

    pub(crate) fn heal_hp(
        &mut self,
        target: PokemonRef,
        requested: u16,
    ) -> Result<HpChange, BattleError> {
        let pokemon = self.pokemon_mut(target)?;
        if pokemon.current_hp > pokemon.stats.hp {
            return Err(BattleError::InvalidState(
                "cannot heal Pokémon whose HP exceeds max HP".to_owned(),
            ));
        }
        if pokemon.is_fainted() {
            return Err(BattleError::InvalidState(
                "cannot heal a structurally fainted Pokémon".to_owned(),
            ));
        }
        let previous = pokemon.current_hp;
        let current = previous.saturating_add(requested).min(pokemon.stats.hp);
        pokemon.current_hp = current;
        Ok(HpChange {
            previous,
            current,
            requested,
            applied: current - previous,
        })
    }

    pub(crate) fn set_pp(
        &mut self,
        target: PokemonRef,
        slot: MoveSlotIndex,
        value: u8,
    ) -> Result<PpChange, BattleError> {
        let move_slot = self
            .pokemon_mut(target)?
            .moves
            .get_mut(usize::from(slot.get()))
            .ok_or_else(|| BattleError::InvalidState("move slot is out of bounds".to_owned()))?;
        if value > move_slot.max_pp {
            return Err(BattleError::InvalidState(
                "move PP cannot exceed max PP".to_owned(),
            ));
        }
        let previous = move_slot.pp;
        move_slot.pp = value;
        Ok(PpChange {
            previous,
            current: value,
        })
    }

    pub(crate) fn set_item(
        &mut self,
        target: PokemonRef,
        item: Option<ItemId>,
    ) -> Result<Option<ItemId>, BattleError> {
        let pokemon = self.pokemon_mut(target)?;
        Ok(std::mem::replace(&mut pokemon.item, item))
    }

    /// Removes an item while preserving authoritative item history.
    pub(crate) fn remove_item(
        &mut self,
        target: PokemonRef,
        consumed: bool,
        activate_unburden: bool,
    ) -> Result<Option<ItemId>, BattleError> {
        let pokemon = self.pokemon_mut(target)?;
        let item = pokemon.item.take();
        if let Some(item) = item {
            if consumed {
                pokemon.last_item = Some(item);
                pokemon.item_used_this_turn = true;
            }
            pokemon.unburden_active |= activate_unburden;
        }
        Ok(item)
    }

    /// Transfers one held item atomically between two Pokémon.
    pub(crate) fn transfer_item(
        &mut self,
        from: PokemonRef,
        to: PokemonRef,
        activate_unburden: bool,
    ) -> Result<Option<ItemId>, BattleError> {
        if from == to {
            return Ok(None);
        }
        if self.pokemon(to)?.item.is_some() {
            return Err(BattleError::InvalidState(
                "item transfer target already holds an item".to_owned(),
            ));
        }
        let Some(item) = self.pokemon(from)?.item else {
            return Ok(None);
        };
        {
            let source = self.pokemon_mut(from)?;
            source.item = None;
            source.unburden_active |= activate_unburden;
        }
        self.pokemon_mut(to)?.item = Some(item);
        Ok(Some(item))
    }

    /// Exchanges held items atomically and records holders left itemless.
    pub(crate) fn swap_items(
        &mut self,
        left: PokemonRef,
        right: PokemonRef,
        left_unburden: bool,
        right_unburden: bool,
    ) -> Result<bool, BattleError> {
        if left == right {
            return Ok(false);
        }
        let left_item = self.pokemon(left)?.item;
        let right_item = self.pokemon(right)?.item;
        if left_item.is_none() && right_item.is_none() {
            return Ok(false);
        }
        {
            let pokemon = self.pokemon_mut(left)?;
            pokemon.item = right_item;
            pokemon.unburden_active |= left_unburden && right_item.is_none();
        }
        {
            let pokemon = self.pokemon_mut(right)?;
            pokemon.item = left_item;
            pokemon.unburden_active |= right_unburden && left_item.is_none();
        }
        Ok(true)
    }

    pub(crate) fn request_switch(
        &mut self,
        target: PokemonRef,
        kind: SwitchRequestKind,
    ) -> Result<bool, BattleError> {
        let pokemon = self.pokemon(target)?;
        let PokemonPosition::Active { slot } = pokemon.position else {
            return Err(BattleError::InvalidState(
                "a switch request requires an active Pokémon".to_owned(),
            ));
        };
        if pokemon.current_hp == 0 || pokemon.is_fainted() {
            return Err(BattleError::InvalidState(
                "a switch request requires a living Pokémon".to_owned(),
            ));
        }
        if self.battle.side(target.side).active[slot.index()] != Some(target.pokemon) {
            return Err(BattleError::InvalidState(
                "switch-request target disagrees with its active slot".to_owned(),
            ));
        }
        if self
            .battle
            .switch_requests
            .iter()
            .any(|request| request.side == target.side && request.slot == slot)
        {
            return Ok(false);
        }
        self.battle.switch_requests.push(SwitchRequest {
            side: target.side,
            slot,
            kind,
        });
        self.battle.phase = crate::BattlePhase::ForcedSwitch;
        Ok(true)
    }

    pub(crate) fn clear_switch_request(
        &mut self,
        side: SideId,
        slot: ActiveSlot,
    ) -> Result<SwitchRequest, BattleError> {
        let index = self
            .battle
            .switch_requests
            .iter()
            .position(|request| request.side == side && request.slot == slot)
            .ok_or_else(|| BattleError::InvalidState("switch request is missing".to_owned()))?;
        Ok(self.battle.switch_requests.remove(index))
    }

    pub(crate) fn consume_berry(
        &mut self,
        target: PokemonRef,
        berry: ItemId,
        schedule_cud_chew: bool,
    ) -> Result<(), BattleError> {
        let pokemon = self.pokemon_mut(target)?;
        if pokemon.item != Some(berry) {
            return Err(BattleError::InvalidState(
                "consumed Berry does not match the holder's item".to_owned(),
            ));
        }
        pokemon.item = None;
        pokemon.last_item = Some(berry);
        pokemon.item_used_this_turn = true;
        pokemon.last_consumed_berry = Some(berry);
        if schedule_cud_chew {
            pokemon.cud_chew_berry = Some(berry);
            pokemon.cud_chew_turns = 2;
        }
        Ok(())
    }

    pub(crate) fn tick_cud_chew(
        &mut self,
        target: PokemonRef,
    ) -> Result<Option<ItemId>, BattleError> {
        let pokemon = self.pokemon_mut(target)?;
        let Some(berry) = pokemon.cud_chew_berry else {
            return Ok(None);
        };
        pokemon.cud_chew_turns = pokemon.cud_chew_turns.saturating_sub(1);
        if pokemon.cud_chew_turns == 0 {
            pokemon.cud_chew_berry = None;
            Ok(Some(berry))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn restore_last_consumed_berry(
        &mut self,
        target: PokemonRef,
    ) -> Result<Option<ItemId>, BattleError> {
        let pokemon = self.pokemon_mut(target)?;
        if pokemon.item.is_some() {
            return Ok(None);
        }
        let berry = pokemon.last_consumed_berry.take();
        pokemon.item = berry;
        Ok(berry)
    }

    pub(crate) fn set_ability(
        &mut self,
        target: PokemonRef,
        ability: AbilityId,
    ) -> Result<AbilityId, BattleError> {
        let pokemon = self.pokemon_mut(target)?;
        Ok(std::mem::replace(&mut pokemon.ability, ability))
    }

    pub(crate) fn replace_ability(
        &mut self,
        target: PokemonRef,
        ability: AbilityId,
        temporary: bool,
    ) -> Result<AbilityId, BattleError> {
        let pokemon = self.pokemon_mut(target)?;
        let previous = pokemon.ability;
        if temporary && pokemon.ability_restore.is_none() {
            pokemon.ability_restore = Some(previous);
        }
        pokemon.ability = ability;
        pokemon.ability_suppressed = false;
        pokemon.unburden_active = false;
        Ok(previous)
    }

    pub(crate) fn swap_abilities(
        &mut self,
        left: PokemonRef,
        right: PokemonRef,
    ) -> Result<bool, BattleError> {
        if left == right {
            return Ok(false);
        }
        let left_ability = self.pokemon(left)?.ability;
        let right_ability = self.pokemon(right)?.ability;
        if left_ability == right_ability {
            return Ok(false);
        }
        {
            let pokemon = self.pokemon_mut(left)?;
            pokemon.ability_restore.get_or_insert(left_ability);
            pokemon.ability = right_ability;
            pokemon.ability_suppressed = false;
            pokemon.unburden_active = false;
        }
        {
            let pokemon = self.pokemon_mut(right)?;
            pokemon.ability_restore.get_or_insert(right_ability);
            pokemon.ability = left_ability;
            pokemon.ability_suppressed = false;
            pokemon.unburden_active = false;
        }
        Ok(true)
    }

    pub(crate) fn set_ability_suppressed(
        &mut self,
        target: PokemonRef,
        suppressed: bool,
    ) -> Result<bool, BattleError> {
        let pokemon = self.pokemon_mut(target)?;
        let changed = pokemon.ability_suppressed != suppressed;
        pokemon.ability_suppressed = suppressed;
        Ok(changed)
    }

    pub(crate) fn restore_ability(&mut self, target: PokemonRef) -> Result<bool, BattleError> {
        let pokemon = self.pokemon_mut(target)?;
        let Some(ability) = pokemon.ability_restore.take() else {
            pokemon.ability_suppressed = false;
            pokemon.unburden_active = false;
            return Ok(false);
        };
        pokemon.ability = ability;
        pokemon.ability_suppressed = false;
        pokemon.unburden_active = false;
        Ok(true)
    }

    pub(crate) fn reset_item_turn_history(&mut self) {
        for side in [SideId::P1, SideId::P2] {
            for pokemon in &mut self.battle.side_mut(side).team.members {
                pokemon.item_used_this_turn = false;
            }
        }
    }

    /// Applies a mechanics-approved Mega transformation atomically.
    ///
    /// Compatibility, timing, and once-per-battle eligibility are decided by
    /// mechanics. The kernel only validates the local representation, updates
    /// coupled form/stat/ability/HP fields, and consumes the side's prepared
    /// Mega descriptors without touching the held item.
    pub(crate) fn apply_mega_evolution(
        &mut self,
        target: PokemonRef,
        evolution: MegaEvolution,
    ) -> Result<MegaEvolutionChange, BattleError> {
        let side = self.battle.side(target.side);
        if side.mega_evolution_used {
            return Err(BattleError::InvalidState(
                "a side cannot Mega Evolve more than once".to_owned(),
            ));
        }
        let pokemon = self.pokemon(target)?;
        if !matches!(pokemon.position, PokemonPosition::Active { .. })
            || pokemon.current_hp == 0
            || pokemon.is_fainted()
        {
            return Err(BattleError::InvalidState(
                "Mega Evolution requires a living active Pokémon".to_owned(),
            ));
        }
        if pokemon.mega_evolution != Some(evolution) {
            return Err(BattleError::InvalidState(
                "Mega Evolution descriptor is stale or missing".to_owned(),
            ));
        }

        let previous_form = pokemon.form;
        let previous_ability = pokemon.ability;
        let previous_max_hp = pokemon.stats.hp;
        let previous_hp = pokemon.current_hp;
        let missing_hp = previous_max_hp.saturating_sub(previous_hp);
        let current_hp = evolution.stats.hp.saturating_sub(missing_hp).max(1);

        let side = self.battle.side_mut(target.side);
        let pokemon = side.pokemon_mut(target.pokemon).ok_or_else(|| {
            BattleError::InvalidState("Mega Evolution target is out of bounds".to_owned())
        })?;
        pokemon.form = Some(evolution.form);
        pokemon.stats = evolution.stats;
        pokemon.current_hp = current_hp;
        pokemon.ability = evolution.ability;
        pokemon.ability_restore = None;
        pokemon.ability_suppressed = false;
        pokemon.unburden_active = false;
        side.mega_evolution_used = true;
        for ally in &mut side.team.members {
            ally.mega_evolution = None;
        }

        Ok(MegaEvolutionChange {
            previous_form,
            current_form: evolution.form,
            previous_ability,
            current_ability: evolution.ability,
            previous_max_hp,
            current_max_hp: evolution.stats.hp,
            previous_hp,
            current_hp,
        })
    }

    pub(crate) fn change_stat_stage(
        &mut self,
        target: PokemonRef,
        stat: BattleStat,
        delta: i8,
    ) -> Result<StatStageChange, BattleError> {
        if self.pokemon(target)?.is_fainted() {
            return Err(BattleError::InvalidState(
                "cannot change stages on a structurally fainted Pokémon".to_owned(),
            ));
        }
        Ok(self.pokemon_mut(target)?.stat_stages.change(stat, delta))
    }

    pub(crate) fn set_major_status(
        &mut self,
        target: PokemonRef,
        status: MajorStatus,
        state: StatusState,
    ) -> Result<(), BattleError> {
        if self.pokemon(target)?.is_fainted() {
            return Err(BattleError::InvalidState(
                "cannot add a major status to a structurally fainted Pokémon".to_owned(),
            ));
        }
        self.pokemon_mut(target)?.set_major_status(status, state)
    }

    pub(crate) fn cure_major_status(
        &mut self,
        target: PokemonRef,
    ) -> Result<Option<MajorStatus>, BattleError> {
        Ok(self.pokemon_mut(target)?.cure_major_status())
    }

    pub(crate) fn decrement_sleep_turns(&mut self, target: PokemonRef) -> Result<u8, BattleError> {
        let pokemon = self.pokemon_mut(target)?;
        if pokemon.status != Some(MajorStatus::Sleep) {
            return Err(BattleError::InvalidState(
                "sleep counter mutation requires sleep status".to_owned(),
            ));
        }
        pokemon.status_state.sleep_turns = pokemon.status_state.sleep_turns.saturating_sub(1);
        let turns = pokemon.status_state.sleep_turns;
        if turns == 0 {
            pokemon.cure_major_status();
        }
        Ok(turns)
    }

    pub(crate) fn decrement_freeze_turns(&mut self, target: PokemonRef) -> Result<u8, BattleError> {
        let pokemon = self.pokemon_mut(target)?;
        if pokemon.status != Some(MajorStatus::Freeze) {
            return Err(BattleError::InvalidState(
                "freeze counter mutation requires freeze status".to_owned(),
            ));
        }
        pokemon.status_state.freeze_turns = pokemon.status_state.freeze_turns.saturating_sub(1);
        let turns = pokemon.status_state.freeze_turns;
        if turns == 0 {
            pokemon.cure_major_status();
        }
        Ok(turns)
    }

    pub(crate) fn increment_toxic_counter(
        &mut self,
        target: PokemonRef,
    ) -> Result<u8, BattleError> {
        let pokemon = self.pokemon_mut(target)?;
        if pokemon.status != Some(MajorStatus::BadlyPoisoned) {
            return Err(BattleError::InvalidState(
                "toxic counter mutation requires badly poisoned status".to_owned(),
            ));
        }
        pokemon.status_state.toxic_counter =
            pokemon.status_state.toxic_counter.saturating_add(1).min(15);
        Ok(pokemon.status_state.toxic_counter)
    }

    pub(crate) fn add_volatile(
        &mut self,
        target: PokemonRef,
        condition: ConditionState,
    ) -> Result<(), BattleError> {
        if self.pokemon(target)?.is_fainted() {
            return Err(BattleError::InvalidState(
                "cannot add a volatile to a structurally fainted Pokémon".to_owned(),
            ));
        }
        self.pokemon_mut(target)?.add_volatile(condition)
    }

    pub(crate) fn remove_volatile(
        &mut self,
        target: PokemonRef,
        condition: ConditionId,
    ) -> Result<Option<ConditionState>, BattleError> {
        Ok(self.pokemon_mut(target)?.remove_volatile(condition))
    }

    pub(crate) fn tick_volatile_durations(
        &mut self,
        target: PokemonRef,
    ) -> Result<Vec<ConditionId>, BattleError> {
        if self.pokemon(target)?.is_fainted() {
            return Err(BattleError::InvalidState(
                "cannot tick volatiles on a structurally fainted Pokémon".to_owned(),
            ));
        }
        Ok(self.pokemon_mut(target)?.tick_volatile_durations())
    }

    /// Replaces one occupied active position and applies structural switch
    /// cleanup. Eligibility and event ordering remain caller responsibilities.
    pub(crate) fn switch_active_position(
        &mut self,
        side: SideId,
        slot: ActiveSlot,
        incoming: crate::PokemonIndex,
    ) -> Result<crate::PokemonIndex, BattleError> {
        let slot_index = slot.index();
        let outgoing = self.battle.side(side).active[slot_index].ok_or_else(|| {
            BattleError::InvalidState("cannot switch an empty active position".to_owned())
        })?;
        if outgoing == incoming {
            return Err(BattleError::InvalidState(
                "incoming Pokémon already occupies the active position".to_owned(),
            ));
        }
        self.require_selected(side, outgoing)?;
        self.require_selected(side, incoming)?;
        self.require_position(side, outgoing, PokemonPosition::Active { slot })?;
        let outgoing_state = self.battle.side(side).pokemon(outgoing).ok_or_else(|| {
            BattleError::InvalidState("switch source is out of bounds".to_owned())
        })?;
        if outgoing_state.current_hp == 0 || outgoing_state.is_fainted() {
            return Err(BattleError::InvalidState(
                "outgoing Pokémon must be a healthy active Pokémon".to_owned(),
            ));
        }
        self.require_healthy_team_position(side, incoming)?;
        let effect_order = self.battle.next_effect_order;
        self.battle.next_effect_order = effect_order.checked_add(1).ok_or_else(|| {
            BattleError::InvalidState("active effect-order counter overflowed".to_owned())
        })?;

        let side_state = self.battle.side_mut(side);
        let outgoing_state = side_state.pokemon_mut(outgoing).ok_or_else(|| {
            BattleError::InvalidState("switch source is out of bounds".to_owned())
        })?;
        outgoing_state.clear_on_exit();
        outgoing_state.position = PokemonPosition::Team;
        let incoming_state = side_state.pokemon_mut(incoming).ok_or_else(|| {
            BattleError::InvalidState("switch target is out of bounds".to_owned())
        })?;
        incoming_state.prepare_switch_in();
        incoming_state.position = PokemonPosition::Active { slot };
        side_state.active[slot_index] = Some(incoming);
        self.battle.active_effect_orders[side_index(side)][slot_index] = Some(effect_order);
        Ok(outgoing)
    }

    /// Places a healthy benched Pokémon into an empty active slot.
    pub(crate) fn fill_active_position(
        &mut self,
        side: SideId,
        slot: ActiveSlot,
        incoming: crate::PokemonIndex,
    ) -> Result<(), BattleError> {
        if self.battle.side(side).active[slot.index()].is_some() {
            return Err(BattleError::InvalidState(
                "cannot fill an occupied active position".to_owned(),
            ));
        }
        self.require_selected(side, incoming)?;
        self.require_healthy_team_position(side, incoming)?;
        let effect_order = self.battle.next_effect_order;
        self.battle.next_effect_order = effect_order.checked_add(1).ok_or_else(|| {
            BattleError::InvalidState("active effect-order counter overflowed".to_owned())
        })?;
        let side_state = self.battle.side_mut(side);
        let incoming_state = side_state.pokemon_mut(incoming).ok_or_else(|| {
            BattleError::InvalidState("incoming Pokémon is out of bounds".to_owned())
        })?;
        incoming_state.prepare_switch_in();
        incoming_state.position = PokemonPosition::Active { slot };
        side_state.active[slot.index()] = Some(incoming);
        self.battle.active_effect_orders[side_index(side)][slot.index()] = Some(effect_order);
        Ok(())
    }

    /// Atomically establishes the structural faint representation for one
    /// occupied active slot. Event order and victory settlement remain caller
    /// responsibilities.
    pub(crate) fn mark_fainted(
        &mut self,
        side: SideId,
        slot: ActiveSlot,
    ) -> Result<crate::PokemonIndex, BattleError> {
        let slot_index = slot.index();
        let pokemon_index = self.battle.side(side).active[slot_index].ok_or_else(|| {
            BattleError::InvalidState("cannot faint an empty active position".to_owned())
        })?;
        self.require_selected(side, pokemon_index)?;
        self.require_position(side, pokemon_index, PokemonPosition::Active { slot })?;
        let side_state = self.battle.side_mut(side);
        let pokemon = side_state
            .pokemon_mut(pokemon_index)
            .ok_or_else(|| BattleError::InvalidState("faint target is out of bounds".to_owned()))?;
        pokemon.current_hp = 0;
        pokemon.status = None;
        pokemon.status_state = StatusState::default();
        pokemon.clear_on_exit();
        pokemon.position = PokemonPosition::Fainted;
        side_state.active[slot_index] = None;
        self.battle.active_effect_orders[side_index(side)][slot_index] = None;
        Ok(pokemon_index)
    }

    pub(crate) fn add_side_condition(
        &mut self,
        side: SideId,
        condition: ConditionState,
    ) -> Result<(), BattleError> {
        validate_new_condition(
            &self.battle.side(side).side_conditions,
            condition,
            "side condition",
        )?;
        self.battle.side_mut(side).side_conditions.push(condition);
        Ok(())
    }

    pub(crate) fn remove_side_condition(
        &mut self,
        side: SideId,
        condition: ConditionId,
    ) -> Option<ConditionState> {
        remove_condition(&mut self.battle.side_mut(side).side_conditions, condition)
    }

    pub(crate) fn add_field_condition(
        &mut self,
        condition: ConditionState,
    ) -> Result<(), BattleError> {
        validate_new_condition(&self.battle.field.conditions, condition, "field condition")?;
        self.battle.field.conditions.push(condition);
        Ok(())
    }

    pub(crate) fn remove_field_condition(
        &mut self,
        condition: ConditionId,
    ) -> Option<ConditionState> {
        remove_condition(&mut self.battle.field.conditions, condition)
    }

    pub(crate) fn set_weather(
        &mut self,
        weather: Option<ConditionState>,
    ) -> Result<Option<ConditionState>, BattleError> {
        validate_single_condition(weather, "weather")?;
        Ok(std::mem::replace(&mut self.battle.field.weather, weather))
    }

    pub(crate) fn set_terrain(
        &mut self,
        terrain: Option<ConditionState>,
    ) -> Result<Option<ConditionState>, BattleError> {
        validate_single_condition(terrain, "terrain")?;
        Ok(std::mem::replace(&mut self.battle.field.terrain, terrain))
    }

    pub(crate) fn tick_weather_duration(&mut self) -> Option<ConditionId> {
        tick_single_condition(&mut self.battle.field.weather)
    }

    pub(crate) fn tick_terrain_duration(&mut self) -> Option<ConditionId> {
        tick_single_condition(&mut self.battle.field.terrain)
    }

    pub(crate) fn tick_field_condition_durations(&mut self) -> Vec<ConditionId> {
        tick_condition_durations(&mut self.battle.field.conditions)
    }

    pub(crate) fn tick_side_condition_durations(&mut self, side: SideId) -> Vec<ConditionId> {
        tick_condition_durations(&mut self.battle.side_mut(side).side_conditions)
    }

    fn pokemon_mut(&mut self, target: PokemonRef) -> Result<&mut crate::PokemonState, BattleError> {
        self.battle
            .side_mut(target.side)
            .pokemon_mut(target.pokemon)
            .ok_or_else(|| {
                BattleError::InvalidState("Pokémon reference is out of bounds".to_owned())
            })
    }

    fn pokemon(&self, target: PokemonRef) -> Result<&crate::PokemonState, BattleError> {
        self.battle
            .side(target.side)
            .pokemon(target.pokemon)
            .ok_or_else(|| {
                BattleError::InvalidState("Pokémon reference is out of bounds".to_owned())
            })
    }

    fn require_selected(
        &self,
        side: SideId,
        pokemon: crate::PokemonIndex,
    ) -> Result<(), BattleError> {
        if self.battle.side(side).selected_team.contains(&pokemon) {
            Ok(())
        } else {
            Err(BattleError::InvalidState(
                "active-position mutation requires a selected Pokémon".to_owned(),
            ))
        }
    }

    fn require_position(
        &self,
        side: SideId,
        pokemon: crate::PokemonIndex,
        expected: PokemonPosition,
    ) -> Result<(), BattleError> {
        let state = self.battle.side(side).pokemon(pokemon).ok_or_else(|| {
            BattleError::InvalidState("Pokémon position reference is out of bounds".to_owned())
        })?;
        if state.position == expected {
            Ok(())
        } else {
            Err(BattleError::InvalidState(
                "Pokémon position does not match the active-slot mapping".to_owned(),
            ))
        }
    }

    fn require_healthy_team_position(
        &self,
        side: SideId,
        pokemon: crate::PokemonIndex,
    ) -> Result<(), BattleError> {
        let state = self.battle.side(side).pokemon(pokemon).ok_or_else(|| {
            BattleError::InvalidState("incoming Pokémon is out of bounds".to_owned())
        })?;
        if state.position != PokemonPosition::Team || state.current_hp == 0 || state.is_fainted() {
            return Err(BattleError::InvalidState(
                "incoming Pokémon must be healthy and on the team".to_owned(),
            ));
        }
        Ok(())
    }
}

const fn side_index(side: SideId) -> usize {
    match side {
        SideId::P1 => 0,
        SideId::P2 => 1,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn validate_new_condition(
    existing: &[ConditionState],
    condition: ConditionState,
    label: &str,
) -> Result<(), BattleError> {
    validate_single_condition(Some(condition), label)?;
    if existing.iter().any(|state| state.id == condition.id) {
        return Err(BattleError::InvalidState(format!(
            "duplicate {label} identity {}",
            condition.id.get()
        )));
    }
    Ok(())
}

#[cfg_attr(not(test), allow(dead_code))]
fn validate_single_condition(
    condition: Option<ConditionState>,
    label: &str,
) -> Result<(), BattleError> {
    if condition.is_some_and(|state| state.turns_remaining == Some(0) || state.layers == 0) {
        return Err(BattleError::InvalidState(format!(
            "{label} cannot have zero duration or layers"
        )));
    }
    Ok(())
}

fn tick_single_condition(condition: &mut Option<ConditionState>) -> Option<ConditionId> {
    let state = condition.as_mut()?;
    let turns = state.turns_remaining.as_mut()?;
    *turns = turns.saturating_sub(1);
    if *turns == 0 {
        let expired = state.id;
        *condition = None;
        Some(expired)
    } else {
        None
    }
}

fn tick_condition_durations(conditions: &mut Vec<ConditionState>) -> Vec<ConditionId> {
    let mut expired = Vec::new();
    for condition in conditions.iter_mut() {
        if let Some(turns) = &mut condition.turns_remaining {
            *turns = turns.saturating_sub(1);
            if *turns == 0 {
                expired.push(condition.id);
            }
        }
    }
    conditions.retain(|condition| condition.turns_remaining != Some(0));
    expired
}

#[cfg_attr(not(test), allow(dead_code))]
fn remove_condition(
    conditions: &mut Vec<ConditionState>,
    condition: ConditionId,
) -> Option<ConditionState> {
    let index = conditions.iter().position(|state| state.id == condition)?;
    Some(conditions.remove(index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AbilityId, BattlePhase, BattleStats, FormId, MegaEvolution, MoveId, MoveSlot, MoveTarget,
        PokemonIndex, PokemonState, SideState, SpeciesId, StatStages, TeamState,
    };
    use proptest::prelude::*;

    fn index(value: u8) -> PokemonIndex {
        PokemonIndex::new(value)
    }

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
            ability_restore: None,
            ability_suppressed: false,
            item: None,
            last_item: None,
            item_used_this_turn: false,
            unburden_active: false,
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
            mega_evolution: None,
            trapped: false,
            position: PokemonPosition::Team,
        }
    }

    fn battle() -> BattleState {
        let make_side = |offset: u32| {
            SideState::new(TeamState::new(
                (0_u32..4).map(|value| pokemon(offset + value)).collect(),
            ))
        };
        let mut battle = BattleState::new(make_side(1), make_side(101), 4).unwrap();
        let selected = vec![index(0), index(1), index(2), index(3)];
        battle.select_team(SideId::P1, selected.clone()).unwrap();
        battle.select_team(SideId::P2, selected).unwrap();
        battle
    }

    #[test]
    fn hp_pp_and_stages_are_bounded_and_report_actual_changes() {
        let mut battle = battle();
        let target = PokemonRef::new(SideId::P1, index(0));
        let mut kernel = StateKernel::new(&mut battle);

        let damage = kernel.damage_hp(target, 150).unwrap();
        assert_eq!(damage.applied, 100);
        assert_eq!(damage.current, 0);
        let healing = kernel.heal_hp(target, 30).unwrap();
        assert_eq!(healing.applied, 30);
        assert_eq!(healing.current, 30);
        let healing = kernel.heal_hp(target, 200).unwrap();
        assert_eq!(healing.applied, 70);
        assert_eq!(healing.current, 100);

        let pp = kernel.set_pp(target, MoveSlotIndex::new(0), 3).unwrap();
        assert_eq!(
            pp,
            PpChange {
                previous: 16,
                current: 3
            }
        );
        assert!(kernel.set_pp(target, MoveSlotIndex::new(0), 17).is_err());
        assert_eq!(
            kernel
                .battle
                .side(SideId::P1)
                .pokemon(index(0))
                .unwrap()
                .moves[0]
                .pp,
            3
        );
        let stage = kernel
            .change_stat_stage(target, BattleStat::Attack, 20)
            .unwrap();
        assert_eq!(stage.current, 6);
        assert_eq!(stage.applied_delta, 6);
    }

    #[test]
    fn status_volatile_and_condition_writes_preserve_local_invariants() {
        let mut battle = battle();
        let target = PokemonRef::new(SideId::P1, index(0));
        let condition = ConditionState::new(ConditionId::new(7), Some(2));
        let mut kernel = StateKernel::new(&mut battle);

        kernel
            .set_major_status(
                target,
                MajorStatus::Sleep,
                StatusState {
                    sleep_turns: 2,
                    ..StatusState::default()
                },
            )
            .unwrap();
        assert!(kernel
            .set_major_status(target, MajorStatus::Burn, StatusState::default())
            .is_err());
        assert_eq!(kernel.decrement_sleep_turns(target).unwrap(), 1);
        assert_eq!(kernel.decrement_sleep_turns(target).unwrap(), 0);
        assert_eq!(kernel.cure_major_status(target).unwrap(), None);
        kernel
            .set_major_status(
                target,
                MajorStatus::Freeze,
                StatusState {
                    freeze_turns: 3,
                    ..StatusState::default()
                },
            )
            .unwrap();
        assert_eq!(kernel.decrement_freeze_turns(target).unwrap(), 2);
        kernel.cure_major_status(target).unwrap();
        kernel
            .set_major_status(
                target,
                MajorStatus::BadlyPoisoned,
                StatusState {
                    toxic_counter: 14,
                    ..StatusState::default()
                },
            )
            .unwrap();
        assert_eq!(kernel.increment_toxic_counter(target).unwrap(), 15);
        assert_eq!(kernel.increment_toxic_counter(target).unwrap(), 15);
        kernel.cure_major_status(target).unwrap();

        kernel.add_volatile(target, condition).unwrap();
        assert!(kernel.add_volatile(target, condition).is_err());
        assert_eq!(kernel.tick_volatile_durations(target).unwrap(), Vec::new());
        assert_eq!(
            kernel.tick_volatile_durations(target).unwrap(),
            vec![condition.id]
        );
        kernel.add_volatile(target, condition).unwrap();
        assert_eq!(
            kernel.remove_volatile(target, condition.id).unwrap(),
            Some(condition)
        );

        kernel.add_side_condition(SideId::P1, condition).unwrap();
        assert!(kernel.add_side_condition(SideId::P1, condition).is_err());
        assert_eq!(
            kernel.remove_side_condition(SideId::P1, condition.id),
            Some(condition)
        );
        kernel.add_field_condition(condition).unwrap();
        assert!(kernel.add_field_condition(condition).is_err());
        assert_eq!(kernel.remove_field_condition(condition.id), Some(condition));
        assert!(kernel
            .set_weather(Some(ConditionState::new(ConditionId::new(8), Some(0))))
            .is_err());
        assert_eq!(
            kernel
                .set_terrain(Some(ConditionState::new(ConditionId::new(9), Some(5))))
                .unwrap(),
            None
        );
    }

    #[test]
    fn position_operations_update_both_sides_of_the_mapping_atomically() {
        let mut battle = battle();
        let original_left_order = battle.active_effect_orders[0][0].unwrap();
        let original_right_order = battle.active_effect_orders[0][1].unwrap();
        let before = battle.clone();
        let mut kernel = StateKernel::new(&mut battle);
        assert!(kernel
            .switch_active_position(SideId::P1, ActiveSlot::Left, index(99))
            .is_err());
        assert_eq!(battle, before);

        let mut kernel = StateKernel::new(&mut battle);
        kernel
            .pokemon_mut(PokemonRef::new(SideId::P1, index(0)))
            .unwrap()
            .stat_stages
            .atk = 3;
        kernel
            .pokemon_mut(PokemonRef::new(SideId::P1, index(0)))
            .unwrap()
            .add_volatile(ConditionState::new(ConditionId::new(11), Some(2)))
            .unwrap();
        kernel
            .pokemon_mut(PokemonRef::new(SideId::P1, index(2)))
            .unwrap()
            .set_major_status(
                MajorStatus::BadlyPoisoned,
                StatusState {
                    toxic_counter: 3,
                    ..StatusState::default()
                },
            )
            .unwrap();
        assert_eq!(
            kernel
                .switch_active_position(SideId::P1, ActiveSlot::Left, index(2))
                .unwrap(),
            index(0)
        );
        assert_eq!(battle.side(SideId::P1).active[0], Some(index(2)));
        assert!(battle.active_effect_orders[0][0].unwrap() > original_right_order);
        assert_ne!(battle.active_effect_orders[0][0], Some(original_left_order));
        assert_eq!(
            battle.side(SideId::P1).pokemon(index(0)).unwrap().position,
            PokemonPosition::Team
        );
        assert_eq!(
            battle
                .side(SideId::P1)
                .pokemon(index(0))
                .unwrap()
                .stat_stages,
            StatStages::default()
        );
        assert_eq!(
            battle.side(SideId::P1).pokemon(index(0)).unwrap().volatiles,
            Vec::new()
        );
        assert_eq!(
            battle.side(SideId::P1).pokemon(index(2)).unwrap().position,
            PokemonPosition::Active {
                slot: ActiveSlot::Left
            }
        );
        assert_eq!(
            battle
                .side(SideId::P1)
                .pokemon(index(2))
                .unwrap()
                .status_state
                .toxic_counter,
            0
        );

        let mut kernel = StateKernel::new(&mut battle);
        kernel.mark_fainted(SideId::P1, ActiveSlot::Right).unwrap();
        assert_eq!(battle.side(SideId::P1).active[1], None);
        assert_eq!(battle.active_effect_orders[0][1], None);
        let fainted = battle.side(SideId::P1).pokemon(index(1)).unwrap();
        assert_eq!(fainted.current_hp, 0);
        assert_eq!(fainted.position, PokemonPosition::Fainted);

        let mut kernel = StateKernel::new(&mut battle);
        kernel
            .fill_active_position(SideId::P1, ActiveSlot::Right, index(3))
            .unwrap();
        assert_eq!(battle.side(SideId::P1).active[1], Some(index(3)));
        assert!(battle.active_effect_orders[0][1].unwrap() > original_right_order);
        assert_eq!(battle.phase, BattlePhase::TurnSelection);
    }

    #[test]
    fn mega_evolution_updates_coupled_form_stats_hp_ability_and_side_state_atomically() {
        let mut battle = battle();
        let target = PokemonRef::new(SideId::P1, index(0));
        let evolution = MegaEvolution {
            form: FormId::new(7),
            stats: BattleStats {
                hp: 120,
                atk: 140,
                def: 110,
                spa: 90,
                spd: 100,
                spe: 130,
            },
            ability: AbilityId::new(9),
        };
        {
            let pokemon = battle.side_mut(SideId::P1).pokemon_mut(index(0)).unwrap();
            pokemon.current_hp = 83;
            pokemon.mega_evolution = Some(evolution);
        }

        let change = StateKernel::new(&mut battle)
            .apply_mega_evolution(target, evolution)
            .unwrap();
        assert_eq!(change.previous_form, None);
        assert_eq!(change.current_form, evolution.form);
        assert_eq!(change.previous_ability, AbilityId::new(1));
        assert_eq!(change.current_ability, evolution.ability);
        assert_eq!(change.previous_max_hp, 100);
        assert_eq!(change.current_max_hp, 120);
        assert_eq!(change.previous_hp, 83);
        assert_eq!(change.current_hp, 103);
        let transformed = battle.side(SideId::P1).pokemon(index(0)).unwrap();
        assert_eq!(transformed.form, Some(evolution.form));
        assert_eq!(transformed.stats, evolution.stats);
        assert_eq!(transformed.current_hp, 103);
        assert_eq!(transformed.ability, evolution.ability);
        assert!(battle.side(SideId::P1).mega_evolution_used);
        assert!(battle
            .side(SideId::P1)
            .team
            .members
            .iter()
            .all(|pokemon| pokemon.mega_evolution.is_none()));
    }

    proptest! {
        #[test]
        fn hp_mutations_are_bounded_and_report_exact_changes(
            damage in any::<u16>(),
            healing in any::<u16>(),
        ) {
            let mut battle = battle();
            let target = PokemonRef::new(SideId::P1, index(0));
            let mut kernel = StateKernel::new(&mut battle);

            let damaged = kernel.damage_hp(target, damage).unwrap();
            let expected_after_damage = 100_u16.saturating_sub(damage);
            prop_assert_eq!(damaged.previous, 100);
            prop_assert_eq!(damaged.current, expected_after_damage);
            prop_assert_eq!(damaged.applied, 100 - expected_after_damage);

            if expected_after_damage > 0 {
                let healed = kernel.heal_hp(target, healing).unwrap();
                let expected_after_healing = expected_after_damage.saturating_add(healing).min(100);
                prop_assert_eq!(healed.current, expected_after_healing);
                prop_assert_eq!(healed.applied, expected_after_healing - expected_after_damage);
            }
        }

        #[test]
        fn pp_stages_references_and_conditions_preserve_local_invariants(
            pp in any::<u8>(),
            delta in any::<i8>(),
            invalid_index in 4_u8..=u8::MAX,
            condition_id in 1_u32..=u32::MAX,
            duration in 1_u8..=u8::MAX,
        ) {
            let mut battle = battle();
            let target = PokemonRef::new(SideId::P1, index(0));
            let invalid_target = PokemonRef::new(SideId::P1, index(invalid_index));
            let condition = ConditionState::new(ConditionId::new(condition_id), Some(duration));
            let mut kernel = StateKernel::new(&mut battle);

            let pp_result = kernel.set_pp(target, MoveSlotIndex::new(0), pp);
            if pp <= 16 {
                prop_assert_eq!(pp_result.unwrap().current, pp);
            } else {
                prop_assert!(pp_result.is_err());
            }

            let stage = kernel.change_stat_stage(target, BattleStat::Attack, delta).unwrap();
            prop_assert!((-6..=6).contains(&stage.current));
            prop_assert!(kernel.damage_hp(invalid_target, 1).is_err());
            prop_assert_eq!(kernel.battle.side(SideId::P1).team.members.len(), 4);

            kernel.add_field_condition(condition).unwrap();
            prop_assert!(kernel.add_field_condition(condition).is_err());
            prop_assert_eq!(kernel.remove_field_condition(condition.id), Some(condition));
        }
    }
}
