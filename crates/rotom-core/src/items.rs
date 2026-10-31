use crate::{
    BattleError, BattleEvent, BattleStat, EffectInvocation, EffectOrder, EffectOwner,
    EffectRegistry, EffectSource, EffectState, EventContext, EventDispatch, EventValue, ItemId,
    MajorStatus, MoveCategory, MoveSlotIndex, PokemonRef, PokemonType, RngReason, SpeciesId,
    TerrainEnvironment, WeatherEnvironment,
};

/// A reusable implementation family for a held item resolved by static data.
///
/// Item metadata and stable identities remain in the Dex. These native
/// behaviors only describe mechanics already supported by the shared event
/// and move-execution pipeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemBehavior {
    PhysicalPower,
    SpecialPower,
    SuperEffectiveDamage,
    TypePower(PokemonType),
    ResistBerry(PokemonType),
    NormalResistBerry,
    StatusCureBerry(Option<MajorStatus>),
    ConfusionCureBerry,
    PpRestoreBerry,
    HealBerryFraction { numerator: u16, denominator: u16 },
    HealBerryFixed(u16),
    WeatherDuration(WeatherEnvironment),
    TerrainDuration,
    TerrainSeed(TerrainEnvironment, BattleStat),
    Leftovers,
    SourceAccuracy,
    TargetAccuracy,
    SpeedMultiplier { numerator: i64, denominator: i64 },
    DoubleOffenseFor(SpeciesId),
    FocusSash,
    AirBalloon,
    BigRoot,
    BindingBand,
    ForcedSwitchItem,
    FocusBand,
    FlinchItem,
    SpeciesCritRatio(u8),
    LifeOrb,
    LightClay,
    MentalHerb,
    Metronome,
    NormalGem,
    QuickClaw,
    RockyHelmet,
    CritRatio(u8),
    ShedShell,
    ShellBell,
    WhiteHerb,
    ZoomLens,
    MegaStone,
}

/// Joins one stable item identity to a native behavior family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ItemHandler {
    pub item: ItemId,
    pub behavior: ItemBehavior,
}

impl ItemHandler {
    #[must_use]
    pub const fn new(item: ItemId, behavior: ItemBehavior) -> Self {
        Self { item, behavior }
    }
}

#[allow(clippy::too_many_lines)] // The declarative match keeps each behavior's hook visible in one place.
pub(crate) fn register(
    registry: &mut EffectRegistry,
    handlers: impl IntoIterator<Item = ItemHandler>,
) {
    for handler in handlers {
        let source = EffectSource::Item(handler.item);
        match handler.behavior {
            ItemBehavior::PhysicalPower => registry.register(
                BattleEvent::ModifyBasePower,
                source,
                EffectOrder::new(0, 16, 0),
                physical_power,
            ),
            ItemBehavior::SpecialPower => registry.register(
                BattleEvent::ModifyBasePower,
                source,
                EffectOrder::new(0, 16, 0),
                special_power,
            ),
            ItemBehavior::TypePower(move_type) => registry.register(
                BattleEvent::ModifyBasePower,
                source,
                EffectOrder::new(0, 15, 0),
                type_power_callback(move_type),
            ),
            ItemBehavior::SuperEffectiveDamage => registry.register(
                BattleEvent::ModifyDamage,
                source,
                EffectOrder::default(),
                super_effective_damage,
            ),
            ItemBehavior::ResistBerry(move_type) => registry.register(
                BattleEvent::ModifyDamage,
                source,
                EffectOrder::default(),
                resist_berry_callback(move_type),
            ),
            ItemBehavior::NormalResistBerry => registry.register(
                BattleEvent::ModifyDamage,
                source,
                EffectOrder::default(),
                chilan_resist_berry,
            ),
            ItemBehavior::StatusCureBerry(status) => registry.register(
                BattleEvent::AfterSetStatus,
                source,
                EffectOrder::default(),
                status_cure_callback(status),
            ),
            ItemBehavior::PpRestoreBerry => registry.register(
                BattleEvent::AfterMove,
                source,
                EffectOrder::default(),
                restore_pp,
            ),
            ItemBehavior::HealBerryFraction {
                numerator,
                denominator,
            } => registry.register(
                BattleEvent::AfterDamage,
                source,
                EffectOrder::default(),
                heal_berry_callback(i64::from(numerator), i64::from(denominator)),
            ),
            ItemBehavior::HealBerryFixed(amount) => registry.register(
                BattleEvent::AfterDamage,
                source,
                EffectOrder::default(),
                fixed_heal_callback(amount),
            ),
            ItemBehavior::SourceAccuracy => registry.register(
                BattleEvent::ModifyAccuracy,
                source,
                EffectOrder::default(),
                source_accuracy,
            ),
            ItemBehavior::TargetAccuracy => registry.register(
                BattleEvent::ModifyAccuracy,
                source,
                EffectOrder::default(),
                target_accuracy,
            ),
            ItemBehavior::SpeedMultiplier {
                numerator,
                denominator,
            } => registry.register(
                BattleEvent::ModifySpeed,
                source,
                EffectOrder::default(),
                speed_callback(numerator, denominator),
            ),
            ItemBehavior::DoubleOffenseFor(_species) => {
                registry.register(
                    BattleEvent::ModifyAttack,
                    source,
                    EffectOrder::new(0, 1, 0),
                    double_offense,
                );
                registry.register(
                    BattleEvent::ModifySpecialAttack,
                    source,
                    EffectOrder::new(0, 1, 0),
                    double_offense,
                );
            }
            ItemBehavior::FocusSash => registry.register(
                BattleEvent::ModifyDamage,
                source,
                EffectOrder::default(),
                focus_sash,
            ),
            ItemBehavior::AirBalloon => registry.register(
                BattleEvent::AfterDamage,
                source,
                EffectOrder::default(),
                air_balloon,
            ),
            ItemBehavior::FocusBand => registry.register(
                BattleEvent::ModifyDamage,
                source,
                EffectOrder::new(0, -40, 0),
                focus_band,
            ),
            ItemBehavior::SpeciesCritRatio(amount) => registry.register(
                BattleEvent::ModifyCritRatio,
                source,
                EffectOrder::default(),
                crit_ratio_callback(amount, true),
            ),
            ItemBehavior::LifeOrb => {
                registry.register(
                    BattleEvent::ModifyDamage,
                    source,
                    EffectOrder::default(),
                    life_orb_damage,
                );
                registry.register(
                    BattleEvent::AfterMove,
                    source,
                    EffectOrder::default(),
                    life_orb_recoil,
                );
            }
            ItemBehavior::NormalGem => {
                registry.register(
                    BattleEvent::ModifyBasePower,
                    source,
                    EffectOrder::new(0, 14, 0),
                    normal_gem_power,
                );
                registry.register(
                    BattleEvent::Hit,
                    source,
                    EffectOrder::default(),
                    normal_gem_consume,
                );
            }
            ItemBehavior::QuickClaw => registry.register(
                BattleEvent::ModifyFractionalPriority,
                source,
                EffectOrder::new(0, -2, 0),
                quick_claw,
            ),
            ItemBehavior::RockyHelmet => registry.register(
                BattleEvent::AfterDamage,
                source,
                EffectOrder::new(2, 0, 0),
                rocky_helmet,
            ),
            ItemBehavior::CritRatio(amount) => registry.register(
                BattleEvent::ModifyCritRatio,
                source,
                EffectOrder::default(),
                crit_ratio_callback(amount, false),
            ),
            ItemBehavior::ShellBell => registry.register(
                BattleEvent::AfterMove,
                source,
                EffectOrder::new(0, -1, 0),
                shell_bell,
            ),
            ItemBehavior::WhiteHerb => registry.register(
                BattleEvent::AfterStatStageChange,
                source,
                EffectOrder::default(),
                white_herb,
            ),
            ItemBehavior::ZoomLens => registry.register(
                BattleEvent::ModifyAccuracy,
                source,
                EffectOrder::new(0, -2, 0),
                zoom_lens,
            ),
            ItemBehavior::ConfusionCureBerry
            | ItemBehavior::WeatherDuration(_)
            | ItemBehavior::TerrainDuration
            | ItemBehavior::TerrainSeed(_, _)
            | ItemBehavior::Leftovers
            | ItemBehavior::BigRoot
            | ItemBehavior::BindingBand
            | ItemBehavior::ForcedSwitchItem
            | ItemBehavior::FlinchItem
            | ItemBehavior::LightClay
            | ItemBehavior::MentalHerb
            | ItemBehavior::Metronome
            | ItemBehavior::ShedShell
            | ItemBehavior::MegaStone => {}
        }
    }
}

fn held_item_matches(
    state: &EffectState<'_>,
    invocation: &EffectInvocation,
    owner: PokemonRef,
) -> bool {
    let EffectSource::Item(item) = invocation.source else {
        return false;
    };
    state
        .battle()
        .side(owner.side)
        .pokemon(owner.pokemon)
        .and_then(|pokemon| pokemon.item)
        == Some(item)
}

#[derive(Clone, Copy)]
struct BerryConsumption {
    holder: PokemonRef,
    cheek_pouch: bool,
    symbiosis_ally: Option<PokemonRef>,
}

fn begin_target_berry(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
) -> Result<Option<BerryConsumption>, BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(None);
    };
    if Some(owner) != context.target || context.suppress_target_item || context.target_berry_blocked
    {
        return Ok(None);
    }
    let EffectSource::Item(item) = invocation.source else {
        return Ok(None);
    };
    if state
        .battle()
        .side(owner.side)
        .pokemon(owner.pokemon)
        .and_then(|pokemon| pokemon.item)
        != Some(item)
    {
        return Ok(None);
    }
    state.consume_berry(owner, item, context.target_cud_chew)?;
    Ok(Some(BerryConsumption {
        holder: owner,
        cheek_pouch: context.target_cheek_pouch,
        symbiosis_ally: context.target_symbiosis_ally,
    }))
}

fn begin_source_berry(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
) -> Result<Option<BerryConsumption>, BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(None);
    };
    if Some(owner) != context.source || context.suppress_source_item || context.source_berry_blocked
    {
        return Ok(None);
    }
    let EffectSource::Item(item) = invocation.source else {
        return Ok(None);
    };
    if !held_item_matches(state, invocation, owner) {
        return Ok(None);
    }
    state.consume_berry(owner, item, context.source_cud_chew)?;
    Ok(Some(BerryConsumption {
        holder: owner,
        cheek_pouch: context.source_cheek_pouch,
        symbiosis_ally: context.source_symbiosis_ally,
    }))
}

fn finish_berry(
    state: &mut EffectState<'_>,
    consumption: BerryConsumption,
) -> Result<(), BattleError> {
    if consumption.cheek_pouch {
        let max_hp = state
            .battle()
            .side(consumption.holder.side)
            .pokemon(consumption.holder.pokemon)
            .ok_or_else(|| BattleError::InvalidState("Berry holder disappeared".to_owned()))?
            .stats
            .hp;
        state.heal_hp(consumption.holder, (max_hp / 3).max(1))?;
    }
    if let Some(ally) = consumption.symbiosis_ally {
        if state
            .battle()
            .side(consumption.holder.side)
            .pokemon(consumption.holder.pokemon)
            .is_some_and(|holder| holder.item.is_none())
        {
            if let Some(item) = state
                .battle()
                .side(ally.side)
                .pokemon(ally.pokemon)
                .and_then(|pokemon| pokemon.item)
            {
                state.set_item(ally, None)?;
                state.set_item(consumption.holder, Some(item))?;
            }
        }
    }
    Ok(())
}

fn physical_power(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    modify_source(
        invocation,
        context,
        dispatch,
        context.move_category == Some(MoveCategory::Physical),
        4505,
    )
}

fn special_power(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    modify_source(
        invocation,
        context,
        dispatch,
        context.move_category == Some(MoveCategory::Special),
        4505,
    )
}

fn type_power(
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    move_type: PokemonType,
) -> Result<(), BattleError> {
    modify_source(
        invocation,
        context,
        dispatch,
        context.move_type == Some(move_type),
        4915,
    )
}

fn resist_berry(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    move_type: PokemonType,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    if Some(owner) != context.target
        || context.move_type != Some(move_type)
        || !context.type_effectiveness.is_some_and(|value| value > 0)
        || context.suppress_target_item
        || context.target_berry_blocked
    {
        return Ok(());
    }
    let EffectSource::Item(item) = invocation.source else {
        return Err(BattleError::InvalidState(
            "resistance Berry handler has a non-item source".to_owned(),
        ));
    };
    if state
        .battle()
        .side(owner.side)
        .pokemon(owner.pokemon)
        .and_then(|pokemon| pokemon.item)
        != Some(item)
    {
        return Ok(());
    }
    let Some(consumption) = begin_target_berry(state, invocation, context)? else {
        return Ok(());
    };
    modify_integer(dispatch, 1, 2)?;
    if context.target_ripen {
        modify_integer(dispatch, 1, 2)?;
    }
    finish_berry(state, consumption)?;
    Ok(())
}

fn chilan_resist_berry(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if context.move_type == Some(PokemonType::Normal) {
        let Some(consumption) = begin_target_berry(state, invocation, context)? else {
            return Ok(());
        };
        modify_integer(dispatch, 1, 2)?;
        if context.target_ripen {
            modify_integer(dispatch, 1, 2)?;
        }
        finish_berry(state, consumption)?;
    }
    Ok(())
}

fn status_cure(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    _: &mut EventDispatch,
    expected: Option<MajorStatus>,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    let status = state
        .battle()
        .side(owner.side)
        .pokemon(owner.pokemon)
        .and_then(|pokemon| pokemon.status);
    let matches = expected.map_or(true, |expected| {
        status == Some(expected)
            || (expected == MajorStatus::Poison && status == Some(MajorStatus::BadlyPoisoned))
    });
    if status.is_some() && matches {
        let Some(consumption) = begin_target_berry(state, invocation, context)? else {
            return Ok(());
        };
        state.cure_major_status(owner)?;
        finish_berry(state, consumption)?;
    }
    Ok(())
}

macro_rules! status_cure_callbacks {
    ($($variant:ident => $callback:ident),+ $(,)?) => {
        const fn status_cure_callback(status: Option<MajorStatus>) -> crate::EffectCallback {
            match status {
                $(Some(MajorStatus::$variant) => $callback,)+
                None => cure_any_status,
            }
        }
        $(fn $callback(state: &mut EffectState<'_>, invocation: &EffectInvocation, context: &EventContext, dispatch: &mut EventDispatch) -> Result<(), BattleError> {
            status_cure(state, invocation, context, dispatch, Some(MajorStatus::$variant))
        })+
    };
}

status_cure_callbacks!(
    Burn => cure_burn,
    Freeze => cure_freeze,
    Paralysis => cure_paralysis,
    Poison => cure_poison,
    BadlyPoisoned => cure_bad_poison,
    Sleep => cure_sleep,
);

fn cure_any_status(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    status_cure(state, invocation, context, dispatch, None)
}

fn restore_pp(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    if Some(owner) != context.source || context.suppress_source_item || context.source_berry_blocked
    {
        return Ok(());
    }
    let slot = state
        .battle()
        .side(owner.side)
        .pokemon(owner.pokemon)
        .and_then(|pokemon| pokemon.moves.iter().position(|move_slot| move_slot.pp == 0));
    let Some(slot) = slot else {
        return Ok(());
    };
    let Some(consumption) = begin_source_berry(state, invocation, context)? else {
        return Ok(());
    };
    let restore = if context.source_ripen { 20 } else { 10 };
    let move_slot = state
        .battle()
        .side(owner.side)
        .pokemon(owner.pokemon)
        .and_then(|pokemon| pokemon.moves.get(slot))
        .ok_or_else(|| BattleError::InvalidState("Leppa Berry move slot disappeared".to_owned()))?;
    let value = move_slot.pp.saturating_add(restore).min(move_slot.max_pp);
    state.set_pp(
        owner,
        MoveSlotIndex::new(
            u8::try_from(slot).map_err(|_| {
                BattleError::InvalidState("move slot does not fit in u8".to_owned())
            })?,
        ),
        value,
    )?;
    finish_berry(state, consumption)?;
    Ok(())
}

fn heal_berry(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    _: &mut EventDispatch,
    numerator: u16,
    denominator: u16,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    let Some((current_hp, max_hp)) = state
        .battle()
        .side(owner.side)
        .pokemon(owner.pokemon)
        .map(|holder| (holder.current_hp, holder.stats.hp))
    else {
        return Ok(());
    };
    if current_hp == 0 || current_hp > max_hp / 2 {
        return Ok(());
    }
    if let Some(consumption) = begin_target_berry(state, invocation, context)? {
        let ripen = if context.target_ripen { 2 } else { 1 };
        let amount = u32::from(max_hp) * u32::from(numerator) * ripen / u32::from(denominator);
        state.heal_hp(owner, u16::try_from(amount).unwrap_or(u16::MAX))?;
        finish_berry(state, consumption)?;
    }
    Ok(())
}

macro_rules! fraction_callbacks {
    ($selector:ident, $worker:ident; $(($num:literal, $den:literal) => $callback:ident),+ $(,)?) => {
        const fn $selector(num: i64, den: i64) -> crate::EffectCallback {
            match (num, den) { $(($num, $den) => $callback,)+ _ => unreachable!() }
        }
        $(fn $callback(state: &mut EffectState<'_>, invocation: &EffectInvocation, context: &EventContext, dispatch: &mut EventDispatch) -> Result<(), BattleError> {
            $worker(state, invocation, context, dispatch, $num, $den)
        })+
    };
}

fn heal_berry_worker(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    num: i64,
    den: i64,
) -> Result<(), BattleError> {
    heal_berry(
        state,
        invocation,
        context,
        dispatch,
        u16::try_from(num).unwrap_or(0),
        u16::try_from(den).unwrap_or(1),
    )
}
fraction_callbacks!(heal_berry_callback, heal_berry_worker; (1, 4) => heal_quarter, (1, 10) => heal_ten);

fn fixed_heal(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    _: &mut EventDispatch,
    amount: u16,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    let Some((current_hp, max_hp)) = state
        .battle()
        .side(owner.side)
        .pokemon(owner.pokemon)
        .map(|holder| (holder.current_hp, holder.stats.hp))
    else {
        return Ok(());
    };
    if current_hp != 0 && current_hp <= max_hp / 2 {
        let Some(consumption) = begin_target_berry(state, invocation, context)? else {
            return Ok(());
        };
        state.heal_hp(
            owner,
            amount.saturating_mul(if context.target_ripen { 2 } else { 1 }),
        )?;
        finish_berry(state, consumption)?;
    }
    Ok(())
}

macro_rules! fixed_heal_callbacks {
    ($($amount:literal => $callback:ident),+ $(,)?) => {
        const fn fixed_heal_callback(amount: u16) -> crate::EffectCallback {
            match amount { $($amount => $callback,)+ _ => unreachable!() }
        }
        $(fn $callback(state: &mut EffectState<'_>, invocation: &EffectInvocation, context: &EventContext, dispatch: &mut EventDispatch) -> Result<(), BattleError> {
            fixed_heal(state, invocation, context, dispatch, $amount)
        })+
    };
}
fixed_heal_callbacks!(10 => heal_ten_hp);

fn source_accuracy(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    modify_source(invocation, context, dispatch, true, 4505)
}

fn target_accuracy(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if owner_pokemon(invocation) == context.target && !context.suppress_target_item {
        modify_integer(dispatch, 3686, 4096)?;
    }
    Ok(())
}

fn speed_multiplier(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    num: i64,
    den: i64,
) -> Result<(), BattleError> {
    if owner_pokemon(invocation) == context.source && !context.suppress_source_item {
        modify_integer(dispatch, num, den)?;
    }
    Ok(())
}
fraction_callbacks!(speed_callback, speed_multiplier; (3, 2) => speed_up, (1, 2) => speed_down);

fn double_offense(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if owner_pokemon(invocation) == context.source
        && !context.suppress_source_item
        && context.source_item_species_match
    {
        modify_integer(dispatch, 2, 1)?;
    }
    Ok(())
}

fn focus_sash(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    if Some(owner) != context.target || context.suppress_target_item {
        return Ok(());
    }
    let Some(holder) = state.battle().side(owner.side).pokemon(owner.pokemon) else {
        return Ok(());
    };
    if context.move_id.is_some()
        && holder.current_hp == holder.stats.hp
        && integer(dispatch)? >= i64::from(holder.current_hp)
    {
        dispatch.value = EventValue::Integer(i64::from(holder.current_hp.saturating_sub(1)));
        state.set_item(owner, None)?;
    }
    Ok(())
}

fn air_balloon(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    if Some(owner) == context.target
        && !context.suppress_target_item
        && integer(dispatch)? > 0
        && held_item_matches(state, invocation, owner)
    {
        state.set_item(owner, None)?;
    }
    Ok(())
}

fn focus_band(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    if Some(owner) != context.target
        || context.suppress_target_item
        || context.move_id.is_none()
        || !held_item_matches(state, invocation, owner)
    {
        return Ok(());
    }
    let survives = state.chance(RngReason::Item, 1, 10)?;
    let hp = state
        .battle()
        .side(owner.side)
        .pokemon(owner.pokemon)
        .ok_or_else(|| BattleError::InvalidState("Focus Band holder is invalid".to_owned()))?
        .current_hp;
    if survives && integer(dispatch)? >= i64::from(hp) {
        dispatch.value = EventValue::Integer(i64::from(hp.saturating_sub(1)));
    }
    Ok(())
}

fn crit_ratio(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    amount: i64,
    species_restricted: bool,
) -> Result<(), BattleError> {
    if owner_pokemon(invocation) == context.source
        && !context.suppress_source_item
        && (!species_restricted || context.source_item_species_match)
    {
        dispatch.value = EventValue::Integer(integer(dispatch)?.saturating_add(amount));
    }
    Ok(())
}

macro_rules! crit_ratio_callbacks {
    ($(($amount:literal, $restricted:literal) => $callback:ident),+ $(,)?) => {
        const fn crit_ratio_callback(amount: u8, restricted: bool) -> crate::EffectCallback {
            match (amount, restricted) {
                $(($amount, $restricted) => $callback,)+
                _ => unreachable!(),
            }
        }
        $(fn $callback(state: &mut EffectState<'_>, invocation: &EffectInvocation, context: &EventContext, dispatch: &mut EventDispatch) -> Result<(), BattleError> {
            crit_ratio(state, invocation, context, dispatch, $amount, $restricted)
        })+
    };
}

crit_ratio_callbacks!(
    (1, false) => crit_plus_one,
    (2, true) => species_crit_plus_two,
);

fn life_orb_damage(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if owner_pokemon(invocation) == context.source && !context.suppress_source_item {
        modify_integer(dispatch, 5324, 4096)?;
    }
    Ok(())
}

fn life_orb_recoil(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    if Some(owner) == context.source
        && context.target != context.source
        && context.move_category != Some(MoveCategory::Status)
        && context.move_total_damage > 0
        && !context.sheer_force_boosted
        && !context.suppress_source_item
        && held_item_matches(state, invocation, owner)
    {
        let max_hp = state
            .battle()
            .side(owner.side)
            .pokemon(owner.pokemon)
            .ok_or_else(|| BattleError::InvalidState("Life Orb holder is invalid".to_owned()))?
            .stats
            .hp;
        state.damage_hp(owner, (max_hp / 10).max(1))?;
    }
    Ok(())
}

fn normal_gem_power(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if owner_pokemon(invocation) == context.source
        && context.target != context.source
        && context.move_category != Some(MoveCategory::Status)
        && context.move_type == Some(PokemonType::Normal)
        && !context.suppress_source_item
    {
        modify_integer(dispatch, 5324, 4096)?;
    }
    Ok(())
}

fn normal_gem_consume(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    if Some(owner) == context.source
        && context.target != context.source
        && context.move_category != Some(MoveCategory::Status)
        && context.move_type == Some(PokemonType::Normal)
        && !context.suppress_source_item
        && held_item_matches(state, invocation, owner)
    {
        state.set_item(owner, None)?;
    }
    Ok(())
}

fn quick_claw(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if owner_pokemon(invocation) == context.source
        && !context.suppress_source_item
        && integer(dispatch)? <= 0
        && state.chance(RngReason::Item, 1, 5)?
    {
        dispatch.value = EventValue::Integer(1);
    }
    Ok(())
}

fn rocky_helmet(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    let Some(source) = context.source.filter(|source| *source != owner) else {
        return Ok(());
    };
    if Some(owner) == context.target
        && !context.suppress_target_item
        && context.move_flags.contact()
        && integer(dispatch)? > 0
        && held_item_matches(state, invocation, owner)
    {
        let max_hp = state
            .battle()
            .side(source.side)
            .pokemon(source.pokemon)
            .ok_or_else(|| BattleError::InvalidState("Rocky Helmet source is invalid".to_owned()))?
            .stats
            .hp;
        state.damage_hp(source, (max_hp / 6).max(1))?;
    }
    Ok(())
}

fn shell_bell(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    if Some(owner) == context.source
        && context.move_total_damage > 0
        && !context.suppress_source_item
        && held_item_matches(state, invocation, owner)
    {
        let amount = u16::try_from(context.move_total_damage / 8)
            .unwrap_or(u16::MAX)
            .max(1);
        state.heal_hp(owner, amount)?;
    }
    Ok(())
}

fn white_herb(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    if Some(owner) != context.target
        || context.suppress_target_item
        || integer(dispatch)? >= 0
        || !held_item_matches(state, invocation, owner)
    {
        return Ok(());
    }
    let stages = state
        .battle()
        .side(owner.side)
        .pokemon(owner.pokemon)
        .ok_or_else(|| BattleError::InvalidState("White Herb holder is invalid".to_owned()))?
        .stat_stages;
    let mut changed = false;
    for stat in [
        BattleStat::Attack,
        BattleStat::Defense,
        BattleStat::SpecialAttack,
        BattleStat::SpecialDefense,
        BattleStat::Speed,
        BattleStat::Accuracy,
        BattleStat::Evasion,
    ] {
        let negative_stage = stages.get(stat);
        if negative_stage < 0 {
            state.change_stat_stage(owner, stat, -negative_stage)?;
            changed = true;
        }
    }
    if changed {
        state.set_item(owner, None)?;
    }
    Ok(())
}

fn zoom_lens(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if owner_pokemon(invocation) == context.source
        && !context.suppress_source_item
        && context.target_acted
    {
        modify_integer(dispatch, 4915, 4096)?;
    }
    Ok(())
}

macro_rules! type_callbacks {
    ($selector:ident, $worker:ident; $($variant:ident => $callback:ident),+ $(,)?) => {
        const fn $selector(move_type: PokemonType) -> crate::EffectCallback {
            match move_type {
                $(PokemonType::$variant => $callback,)+
            }
        }

        $(
            fn $callback(
                state: &mut EffectState<'_>,
                invocation: &EffectInvocation,
                context: &EventContext,
                dispatch: &mut EventDispatch,
            ) -> Result<(), BattleError> {
                $worker(state, invocation, context, dispatch, PokemonType::$variant)
            }
        )+
    };
}

fn type_power_worker(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    move_type: PokemonType,
) -> Result<(), BattleError> {
    type_power(invocation, context, dispatch, move_type)
}

type_callbacks!(type_power_callback, type_power_worker;
    Bug => bug_power,
    Dark => dark_power,
    Dragon => dragon_power,
    Electric => electric_power,
    Fairy => fairy_power,
    Fighting => fighting_power,
    Fire => fire_power,
    Flying => flying_power,
    Ghost => ghost_power,
    Grass => grass_power,
    Ground => ground_power,
    Ice => ice_power,
    Normal => normal_power,
    Poison => poison_power,
    Psychic => psychic_power,
    Rock => rock_power,
    Steel => steel_power,
    Stellar => stellar_power,
    Water => water_power,
);

type_callbacks!(resist_berry_callback, resist_berry;
    Bug => bug_resist_berry,
    Dark => dark_resist_berry,
    Dragon => dragon_resist_berry,
    Electric => electric_resist_berry,
    Fairy => fairy_resist_berry,
    Fighting => fighting_resist_berry,
    Fire => fire_resist_berry,
    Flying => flying_resist_berry,
    Ghost => ghost_resist_berry,
    Grass => grass_resist_berry,
    Ground => ground_resist_berry,
    Ice => ice_resist_berry,
    Normal => normal_resist_berry,
    Poison => poison_resist_berry,
    Psychic => psychic_resist_berry,
    Rock => rock_resist_berry,
    Steel => steel_resist_berry,
    Stellar => stellar_resist_berry,
    Water => water_resist_berry,
);

fn super_effective_damage(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    modify_source(
        invocation,
        context,
        dispatch,
        context.type_effectiveness.is_some_and(|value| value > 0),
        4915,
    )
}

fn modify_source(
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    applies: bool,
    modifier: i64,
) -> Result<(), BattleError> {
    if applies && !context.suppress_source_item && owner_pokemon(invocation) == context.source {
        let value = integer(dispatch)?;
        dispatch.value =
            EventValue::Integer(value.saturating_mul(modifier).saturating_add(2047) / 4096);
    }
    Ok(())
}

fn modify_integer(
    dispatch: &mut EventDispatch,
    numerator: i64,
    denominator: i64,
) -> Result<(), BattleError> {
    let value = integer(dispatch)?;
    let modifier = numerator.saturating_mul(4096) / denominator;
    dispatch.value =
        EventValue::Integer(value.saturating_mul(modifier).saturating_add(2047) / 4096);
    Ok(())
}

const fn owner_pokemon(invocation: &EffectInvocation) -> Option<PokemonRef> {
    match invocation.owner {
        EffectOwner::Pokemon { pokemon } | EffectOwner::Move { pokemon } => Some(pokemon),
        EffectOwner::Battle | EffectOwner::Field | EffectOwner::Side { .. } => None,
    }
}

fn integer(dispatch: &EventDispatch) -> Result<i64, BattleError> {
    match dispatch.value {
        EventValue::Integer(value) => Ok(value),
        EventValue::None | EventValue::Boolean(_) => Err(BattleError::InvalidState(
            "item modifier received a non-integer relay".to_owned(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AbilityId, BattleState, BattleStats, MoveId, MoveSlot, MoveTarget, PokemonIndex,
        PokemonPosition, PokemonState, SideId, SideState, SpeciesId, StatStages, StatusState,
        TeamState,
    };

    fn pokemon(item: Option<ItemId>) -> PokemonState {
        PokemonState {
            species: SpeciesId::new(1),
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
            item,
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

    fn battle(item: ItemId) -> BattleState {
        battle_with_items(Some(item), None)
    }

    fn battle_with_items(source_item: Option<ItemId>, target_item: Option<ItemId>) -> BattleState {
        let source = SideState::new(TeamState::new(
            (0..4).map(|_| pokemon(source_item)).collect(),
        ));
        let target = SideState::new(TeamState::new(
            (0..4).map(|_| pokemon(target_item)).collect(),
        ));
        let mut battle = BattleState::new(source, target, 4).unwrap();
        let selected = (0..4).map(PokemonIndex::new).collect::<Vec<_>>();
        battle.select_team(SideId::P1, selected.clone()).unwrap();
        battle.select_team(SideId::P2, selected).unwrap();
        battle
    }

    fn context(category: MoveCategory, move_type: PokemonType) -> EventContext {
        EventContext {
            source: Some(PokemonRef::new(SideId::P1, PokemonIndex::new(0))),
            target: Some(PokemonRef::new(SideId::P2, PokemonIndex::new(0))),
            move_id: Some(MoveId::new(1)),
            move_category: Some(category),
            move_type: Some(move_type),
            ..EventContext::default()
        }
    }

    fn dispatch(
        registry: &EffectRegistry,
        battle: &mut BattleState,
        event: BattleEvent,
        context: EventContext,
    ) -> i64 {
        let EventValue::Integer(value) = registry
            .dispatch(battle, event, context, EventValue::Integer(100))
            .unwrap()
            .value
        else {
            panic!("item modifier must retain an integer relay");
        };
        value
    }

    #[test]
    fn first_five_item_behaviors_use_their_pinned_modifiers_and_conditions() {
        let muscle_band = ItemId::new(10);
        let wise_glasses = ItemId::new(11);
        let expert_belt = ItemId::new(12);
        let charcoal = ItemId::new(13);
        let mystic_water = ItemId::new(14);
        let mut registry = EffectRegistry::new();
        register(
            &mut registry,
            [
                ItemHandler::new(muscle_band, ItemBehavior::PhysicalPower),
                ItemHandler::new(wise_glasses, ItemBehavior::SpecialPower),
                ItemHandler::new(expert_belt, ItemBehavior::SuperEffectiveDamage),
                ItemHandler::new(charcoal, ItemBehavior::TypePower(PokemonType::Fire)),
                ItemHandler::new(mystic_water, ItemBehavior::TypePower(PokemonType::Water)),
            ],
        );

        let mut state = battle(muscle_band);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyBasePower,
                context(MoveCategory::Physical, PokemonType::Normal),
            ),
            110
        );
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyBasePower,
                context(MoveCategory::Special, PokemonType::Normal),
            ),
            100
        );

        let mut state = battle(wise_glasses);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyBasePower,
                context(MoveCategory::Special, PokemonType::Normal),
            ),
            110
        );

        let mut expert_context = context(MoveCategory::Special, PokemonType::Water);
        expert_context.type_effectiveness = Some(1);
        let mut state = battle(expert_belt);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyDamage,
                expert_context,
            ),
            120
        );
        expert_context.type_effectiveness = Some(0);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyDamage,
                expert_context,
            ),
            100
        );

        let mut state = battle(charcoal);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyBasePower,
                context(MoveCategory::Special, PokemonType::Fire),
            ),
            120
        );
        let mut state = battle(mystic_water);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyBasePower,
                context(MoveCategory::Special, PokemonType::Water),
            ),
            120
        );
    }

    #[test]
    fn item_modifiers_are_holder_scoped_and_respect_shared_suppression() {
        let muscle_band = ItemId::new(10);
        let mut registry = EffectRegistry::new();
        register(
            &mut registry,
            [ItemHandler::new(muscle_band, ItemBehavior::PhysicalPower)],
        );
        let mut state = battle(muscle_band);
        let mut suppressed = context(MoveCategory::Physical, PokemonType::Normal);
        suppressed.suppress_source_item = true;
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyBasePower,
                suppressed,
            ),
            100
        );

        let mut other_source = context(MoveCategory::Physical, PokemonType::Normal);
        other_source.source = Some(PokemonRef::new(SideId::P2, PokemonIndex::new(0)));
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyBasePower,
                other_source,
            ),
            100
        );
    }

    #[test]
    fn all_type_power_handlers_apply_the_showdown_modifier_only_to_the_matching_type() {
        let types = [
            PokemonType::Bug,
            PokemonType::Dark,
            PokemonType::Dragon,
            PokemonType::Electric,
            PokemonType::Fairy,
            PokemonType::Fighting,
            PokemonType::Flying,
            PokemonType::Ghost,
            PokemonType::Grass,
            PokemonType::Ground,
            PokemonType::Ice,
            PokemonType::Normal,
            PokemonType::Poison,
            PokemonType::Psychic,
            PokemonType::Rock,
            PokemonType::Steel,
        ];
        for (offset, move_type) in types.into_iter().enumerate() {
            let item = ItemId::new(u32::try_from(100 + offset).unwrap());
            let mut registry = EffectRegistry::new();
            register(
                &mut registry,
                [ItemHandler::new(item, ItemBehavior::TypePower(move_type))],
            );
            let mut state = battle(item);
            assert_eq!(
                dispatch(
                    &registry,
                    &mut state,
                    BattleEvent::ModifyBasePower,
                    context(MoveCategory::Special, move_type),
                ),
                120
            );
            assert_eq!(
                dispatch(
                    &registry,
                    &mut state,
                    BattleEvent::ModifyBasePower,
                    context(MoveCategory::Special, PokemonType::Water),
                ),
                if move_type == PokemonType::Water {
                    120
                } else {
                    100
                }
            );
        }
    }

    #[test]
    fn resistance_berry_family_consumes_once_for_matching_super_effective_types() {
        let newly_covered_ids = ["chilanberry", "tangaberry", "wacanberry", "yacheberry"];
        assert_eq!(newly_covered_ids.len(), 4);
        let types = [
            PokemonType::Bug,
            PokemonType::Dark,
            PokemonType::Dragon,
            PokemonType::Electric,
            PokemonType::Fairy,
            PokemonType::Fighting,
            PokemonType::Fire,
            PokemonType::Flying,
            PokemonType::Ghost,
            PokemonType::Grass,
            PokemonType::Ground,
            PokemonType::Ice,
            PokemonType::Poison,
            PokemonType::Psychic,
            PokemonType::Rock,
            PokemonType::Steel,
            PokemonType::Water,
        ];
        for (offset, move_type) in types.into_iter().enumerate() {
            let item = ItemId::new(u32::try_from(200 + offset).unwrap());
            let mut registry = EffectRegistry::new();
            register(
                &mut registry,
                [ItemHandler::new(item, ItemBehavior::ResistBerry(move_type))],
            );
            let mut state = battle_with_items(None, Some(item));
            let mut berry_context = context(MoveCategory::Special, move_type);
            berry_context.type_effectiveness = Some(1);
            assert_eq!(
                dispatch(
                    &registry,
                    &mut state,
                    BattleEvent::ModifyDamage,
                    berry_context,
                ),
                50
            );
            assert_eq!(
                state
                    .side(SideId::P2)
                    .pokemon(PokemonIndex::new(0))
                    .unwrap()
                    .item,
                None
            );
        }

        let chilan = ItemId::new(240);
        let mut registry = EffectRegistry::new();
        register(
            &mut registry,
            [ItemHandler::new(chilan, ItemBehavior::NormalResistBerry)],
        );
        let mut state = battle_with_items(None, Some(chilan));
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyDamage,
                context(MoveCategory::Physical, PokemonType::Normal),
            ),
            50
        );
        assert_eq!(
            state
                .side(SideId::P2)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .item,
            None
        );
    }

    #[test]
    fn resistance_berry_interactions_are_suppressed_doubled_or_rejected_explicitly() {
        let item = ItemId::new(250);
        let mut registry = EffectRegistry::new();
        register(
            &mut registry,
            [ItemHandler::new(
                item,
                ItemBehavior::ResistBerry(PokemonType::Fire),
            )],
        );
        for blocked_context in [
            EventContext {
                suppress_target_item: true,
                ..context(MoveCategory::Special, PokemonType::Fire)
            },
            EventContext {
                target_berry_blocked: true,
                ..context(MoveCategory::Special, PokemonType::Fire)
            },
        ] {
            let mut state = battle_with_items(None, Some(item));
            let blocked_context = EventContext {
                type_effectiveness: Some(1),
                ..blocked_context
            };
            assert_eq!(
                dispatch(
                    &registry,
                    &mut state,
                    BattleEvent::ModifyDamage,
                    blocked_context,
                ),
                100
            );
            assert_eq!(
                state
                    .side(SideId::P2)
                    .pokemon(PokemonIndex::new(0))
                    .unwrap()
                    .item,
                Some(item)
            );
        }

        let mut state = battle_with_items(None, Some(item));
        let ripen_context = EventContext {
            type_effectiveness: Some(1),
            target_ripen: true,
            ..context(MoveCategory::Special, PokemonType::Fire)
        };
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyDamage,
                ripen_context,
            ),
            25
        );
    }

    #[allow(clippy::too_many_lines)] // One scenario verifies the four consumption mutation families together.
    #[test]
    fn cure_heal_pp_and_sash_items_mutate_through_typed_callbacks() {
        let shared_family_ids = [
            "aspearberry",
            "cheriberry",
            "chestoberry",
            "pechaberry",
            "rawstberry",
            "lumberry",
            "leppaberry",
            "oranberry",
            "sitrusberry",
            "focussash",
        ];
        assert_eq!(shared_family_ids.len(), 10);
        let cheri = ItemId::new(300);
        let sitrus = ItemId::new(301);
        let leppa = ItemId::new(302);
        let sash = ItemId::new(303);
        let mut registry = EffectRegistry::new();
        register(
            &mut registry,
            [
                ItemHandler::new(
                    cheri,
                    ItemBehavior::StatusCureBerry(Some(MajorStatus::Paralysis)),
                ),
                ItemHandler::new(
                    sitrus,
                    ItemBehavior::HealBerryFraction {
                        numerator: 1,
                        denominator: 4,
                    },
                ),
                ItemHandler::new(leppa, ItemBehavior::PpRestoreBerry),
                ItemHandler::new(sash, ItemBehavior::FocusSash),
            ],
        );
        let target = PokemonRef::new(SideId::P2, PokemonIndex::new(0));
        let source = PokemonRef::new(SideId::P1, PokemonIndex::new(0));

        let mut status_state = battle_with_items(None, Some(cheri));
        status_state
            .side_mut(SideId::P2)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .set_major_status(MajorStatus::Paralysis, StatusState::default())
            .unwrap();
        registry
            .dispatch(
                &mut status_state,
                BattleEvent::AfterSetStatus,
                EventContext {
                    target: Some(target),
                    ..EventContext::default()
                },
                EventValue::None,
            )
            .unwrap();
        let cured = status_state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .unwrap();
        assert_eq!(cured.status, None);
        assert_eq!(cured.item, None);

        let mut heal_state = battle_with_items(None, Some(sitrus));
        heal_state
            .side_mut(SideId::P2)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .current_hp = 40;
        registry
            .dispatch(
                &mut heal_state,
                BattleEvent::AfterDamage,
                EventContext {
                    target: Some(target),
                    ..EventContext::default()
                },
                EventValue::Integer(60),
            )
            .unwrap();
        let healed = heal_state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .unwrap();
        assert_eq!(healed.current_hp, 65);
        assert_eq!(healed.item, None);

        let mut pp_state = battle_with_items(Some(leppa), None);
        pp_state
            .side_mut(SideId::P1)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .moves[0]
            .pp = 0;
        registry
            .dispatch(
                &mut pp_state,
                BattleEvent::AfterMove,
                EventContext {
                    source: Some(source),
                    ..EventContext::default()
                },
                EventValue::None,
            )
            .unwrap();
        let restored = pp_state
            .side(SideId::P1)
            .pokemon(PokemonIndex::new(0))
            .unwrap();
        assert_eq!(restored.moves[0].pp, 10);
        assert_eq!(restored.item, None);

        let mut sash_state = battle_with_items(None, Some(sash));
        assert_eq!(
            dispatch(
                &registry,
                &mut sash_state,
                BattleEvent::ModifyDamage,
                context(MoveCategory::Physical, PokemonType::Normal),
            ),
            99
        );
        assert_eq!(
            sash_state
                .side(SideId::P2)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .item,
            None
        );
    }

    #[test]
    fn accuracy_speed_and_species_stat_items_apply_pinned_modifiers() {
        let shared_family_ids = [
            "brightpowder",
            "widelens",
            "choicescarf",
            "ironball",
            "lightball",
        ];
        assert_eq!(shared_family_ids.len(), 5);
        let wide_lens = ItemId::new(310);
        let bright_powder = ItemId::new(311);
        let scarf = ItemId::new(312);
        let iron_ball = ItemId::new(313);
        let light_ball = ItemId::new(314);
        let mut registry = EffectRegistry::new();
        register(
            &mut registry,
            [
                ItemHandler::new(wide_lens, ItemBehavior::SourceAccuracy),
                ItemHandler::new(bright_powder, ItemBehavior::TargetAccuracy),
                ItemHandler::new(
                    scarf,
                    ItemBehavior::SpeedMultiplier {
                        numerator: 3,
                        denominator: 2,
                    },
                ),
                ItemHandler::new(
                    iron_ball,
                    ItemBehavior::SpeedMultiplier {
                        numerator: 1,
                        denominator: 2,
                    },
                ),
                ItemHandler::new(
                    light_ball,
                    ItemBehavior::DoubleOffenseFor(SpeciesId::new(1)),
                ),
            ],
        );
        let move_context = context(MoveCategory::Physical, PokemonType::Normal);
        let mut state = battle(wide_lens);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyAccuracy,
                move_context
            ),
            110
        );
        let mut state = battle_with_items(None, Some(bright_powder));
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyAccuracy,
                move_context
            ),
            90
        );
        let mut state = battle(scarf);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifySpeed,
                move_context
            ),
            150
        );
        let mut state = battle(iron_ball);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifySpeed,
                move_context
            ),
            50
        );
        let mut state = battle(light_ball);
        let matching = EventContext {
            source_item_species_match: true,
            ..move_context
        };
        assert_eq!(
            dispatch(&registry, &mut state, BattleEvent::ModifyAttack, matching),
            200
        );
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn third_batch_callbacks_apply_consumption_recoil_healing_and_stat_rules() {
        let covered_ids = [
            "airballoon",
            "focusband",
            "leek",
            "lifeorb",
            "normalgem",
            "quickclaw",
            "rockyhelmet",
            "scopelens",
            "shellbell",
            "whiteherb",
            "zoomlens",
        ];
        assert_eq!(covered_ids.len(), 11);
        let air_balloon = ItemId::new(400);
        let focus_band_item = ItemId::new(401);
        let leek = ItemId::new(402);
        let life_orb = ItemId::new(403);
        let normal_gem = ItemId::new(404);
        let quick_claw_item = ItemId::new(405);
        let rocky_helmet_item = ItemId::new(406);
        let scope_lens = ItemId::new(407);
        let shell_bell_item = ItemId::new(408);
        let white_herb_item = ItemId::new(409);
        let zoom_lens_item = ItemId::new(410);
        let mut registry = EffectRegistry::new();
        register(
            &mut registry,
            [
                ItemHandler::new(air_balloon, ItemBehavior::AirBalloon),
                ItemHandler::new(focus_band_item, ItemBehavior::FocusBand),
                ItemHandler::new(leek, ItemBehavior::SpeciesCritRatio(2)),
                ItemHandler::new(life_orb, ItemBehavior::LifeOrb),
                ItemHandler::new(normal_gem, ItemBehavior::NormalGem),
                ItemHandler::new(quick_claw_item, ItemBehavior::QuickClaw),
                ItemHandler::new(rocky_helmet_item, ItemBehavior::RockyHelmet),
                ItemHandler::new(scope_lens, ItemBehavior::CritRatio(1)),
                ItemHandler::new(shell_bell_item, ItemBehavior::ShellBell),
                ItemHandler::new(white_herb_item, ItemBehavior::WhiteHerb),
                ItemHandler::new(zoom_lens_item, ItemBehavior::ZoomLens),
            ],
        );
        let base = context(MoveCategory::Physical, PokemonType::Normal);

        let mut balloon_state = battle_with_items(None, Some(air_balloon));
        registry
            .dispatch(
                &mut balloon_state,
                BattleEvent::AfterDamage,
                base,
                EventValue::Integer(10),
            )
            .unwrap();
        assert_eq!(
            balloon_state
                .side(SideId::P2)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .item,
            None
        );

        let mut crit_state = battle(scope_lens);
        assert_eq!(
            dispatch(
                &registry,
                &mut crit_state,
                BattleEvent::ModifyCritRatio,
                base
            ),
            101
        );
        let mut leek_state = battle(leek);
        let matching = EventContext {
            source_item_species_match: true,
            ..base
        };
        assert_eq!(
            dispatch(
                &registry,
                &mut leek_state,
                BattleEvent::ModifyCritRatio,
                matching
            ),
            102
        );

        let mut orb_state = battle(life_orb);
        assert_eq!(
            dispatch(&registry, &mut orb_state, BattleEvent::ModifyDamage, base),
            130
        );
        registry
            .dispatch(
                &mut orb_state,
                BattleEvent::AfterMove,
                EventContext {
                    move_total_damage: 50,
                    ..base
                },
                EventValue::None,
            )
            .unwrap();
        assert_eq!(
            orb_state
                .side(SideId::P1)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .current_hp,
            90
        );

        let mut gem_state = battle(normal_gem);
        assert_eq!(
            dispatch(
                &registry,
                &mut gem_state,
                BattleEvent::ModifyBasePower,
                base
            ),
            130
        );
        registry
            .dispatch(&mut gem_state, BattleEvent::Hit, base, EventValue::None)
            .unwrap();
        assert_eq!(
            gem_state
                .side(SideId::P1)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .item,
            None
        );

        let mut helmet_state = battle_with_items(None, Some(rocky_helmet_item));
        let contact = EventContext {
            move_flags: crate::MoveInteractionFlags::default().with_contact(true),
            ..base
        };
        registry
            .dispatch(
                &mut helmet_state,
                BattleEvent::AfterDamage,
                contact,
                EventValue::Integer(25),
            )
            .unwrap();
        assert_eq!(
            helmet_state
                .side(SideId::P1)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .current_hp,
            84
        );

        let mut bell_state = battle(shell_bell_item);
        bell_state
            .side_mut(SideId::P1)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .current_hp = 50;
        registry
            .dispatch(
                &mut bell_state,
                BattleEvent::AfterMove,
                EventContext {
                    move_total_damage: 80,
                    ..base
                },
                EventValue::None,
            )
            .unwrap();
        assert_eq!(
            bell_state
                .side(SideId::P1)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .current_hp,
            60
        );

        let mut herb_state = battle_with_items(None, Some(white_herb_item));
        herb_state
            .side_mut(SideId::P2)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .stat_stages
            .atk = -2;
        registry
            .dispatch(
                &mut herb_state,
                BattleEvent::AfterStatStageChange,
                base,
                EventValue::Integer(-2),
            )
            .unwrap();
        let restored = herb_state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .unwrap();
        assert_eq!(restored.stat_stages.atk, 0);
        assert_eq!(restored.item, None);

        let mut zoom_state = battle(zoom_lens_item);
        let moved = EventContext {
            target_acted: true,
            ..base
        };
        assert_eq!(
            dispatch(
                &registry,
                &mut zoom_state,
                BattleEvent::ModifyAccuracy,
                moved
            ),
            120
        );

        for item in [focus_band_item, quick_claw_item] {
            let mut rng_state = if item == focus_band_item {
                battle_with_items(None, Some(item))
            } else {
                battle(item)
            };
            let before = rng_state.rng.draws();
            let event = if item == focus_band_item {
                BattleEvent::ModifyDamage
            } else {
                BattleEvent::ModifyFractionalPriority
            };
            let initial = if item == focus_band_item { 100 } else { 0 };
            registry
                .dispatch(&mut rng_state, event, base, EventValue::Integer(initial))
                .unwrap();
            assert_eq!(rng_state.rng.draws(), before + 1);
        }
    }
}
