use crate::{
    AbilityId, BattleError, BattleEvent, EffectInvocation, EffectOrder, EffectOwner,
    EffectRegistry, EffectSource, EffectState, EventContext, EventDispatch, EventValue,
    MoveCategory, PokemonRef, TerrainEnvironment, WeatherEnvironment,
};

/// A reusable implementation family for an ability resolved by static data.
///
/// This is a closed set of native Rust behaviors, not an interpreted command
/// language. New common families add one descriptor and one callback mapping;
/// exceptional abilities may still use narrowly scoped callbacks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbilityBehavior {
    DoubleAttack,
    StatusedAttack,
    DoubleDefense,
    StatusedDefense,
    LowPowerBoost,
    FullHpDamageReduction,
    StatusedSpeed,
    IgnoreParalysisSpeedPenalty,
    IncreaseCriticalRatio,
    PreventCriticalHit,
    BoostCriticalDamage,
    BreaksTargetAbilities,
    SunSpeed,
    RainSpeed,
    SnowSpeed,
    ElectricTerrainSpeed,
    BulletImmunity,
    SoundImmunity,
    AllyDamageImmunity,
}

/// Whether Mold Breaker-style move suppression can omit this handler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbilitySuppression {
    Breakable,
    Unbreakable,
}

/// Joins a stable Dex ability identity to one native behavior family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbilityHandler {
    pub ability: AbilityId,
    pub behavior: AbilityBehavior,
    pub suppression: AbilitySuppression,
}

impl AbilityHandler {
    #[must_use]
    pub const fn new(
        ability: AbilityId,
        behavior: AbilityBehavior,
        suppression: AbilitySuppression,
    ) -> Self {
        Self {
            ability,
            behavior,
            suppression,
        }
    }
}

pub(crate) fn register(
    registry: &mut EffectRegistry,
    handlers: impl IntoIterator<Item = AbilityHandler>,
) {
    for handler in handlers {
        register_handler(registry, handler);
    }
}

fn register_handler(registry: &mut EffectRegistry, handler: AbilityHandler) {
    for registration in BEHAVIOR_REGISTRATIONS
        .iter()
        .filter(|registration| registration.behavior == handler.behavior)
    {
        register_callback(registry, handler, *registration);
    }
}

#[derive(Clone, Copy)]
struct BehaviorRegistration {
    behavior: AbilityBehavior,
    event: BattleEvent,
    order: EffectOrder,
    callback: crate::EffectCallback,
}

impl BehaviorRegistration {
    const fn new(
        behavior: AbilityBehavior,
        event: BattleEvent,
        order: EffectOrder,
        callback: crate::EffectCallback,
    ) -> Self {
        Self {
            behavior,
            event,
            order,
            callback,
        }
    }
}

const DEFAULT_ORDER: EffectOrder = EffectOrder::new(0, 0, 0);
const BEHAVIOR_REGISTRATIONS: &[BehaviorRegistration] = &[
    BehaviorRegistration::new(
        AbilityBehavior::DoubleAttack,
        BattleEvent::ModifyAttack,
        EffectOrder::new(0, 5, 0),
        double_source_stat,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::StatusedAttack,
        BattleEvent::ModifyAttack,
        EffectOrder::new(0, 5, 0),
        guts_attack,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::DoubleDefense,
        BattleEvent::ModifyDefense,
        EffectOrder::new(0, 6, 0),
        double_holder_defense,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::DoubleDefense,
        BattleEvent::ModifySourceDefense,
        EffectOrder::new(0, 6, 0),
        double_holder_defense,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::StatusedDefense,
        BattleEvent::ModifyDefense,
        EffectOrder::new(0, 6, 0),
        marvel_scale_defense,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::StatusedDefense,
        BattleEvent::ModifySourceDefense,
        EffectOrder::new(0, 6, 0),
        marvel_scale_defense,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::LowPowerBoost,
        BattleEvent::ModifyBasePower,
        EffectOrder::new(0, 30, 0),
        technician_base_power,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::FullHpDamageReduction,
        BattleEvent::ModifyDamage,
        DEFAULT_ORDER,
        full_hp_damage_reduction,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::StatusedSpeed,
        BattleEvent::ModifySpeed,
        DEFAULT_ORDER,
        statused_speed,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::IgnoreParalysisSpeedPenalty,
        BattleEvent::ModifySpeed,
        EffectOrder::new(0, -100, 0),
        compensate_paralysis_speed,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::IncreaseCriticalRatio,
        BattleEvent::ModifyCritRatio,
        DEFAULT_ORDER,
        increase_critical_ratio,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::PreventCriticalHit,
        BattleEvent::CriticalHit,
        DEFAULT_ORDER,
        prevent_critical_hit,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::BoostCriticalDamage,
        BattleEvent::ModifyDamage,
        DEFAULT_ORDER,
        boost_critical_damage,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::SunSpeed,
        BattleEvent::ModifySpeed,
        DEFAULT_ORDER,
        sun_speed,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::RainSpeed,
        BattleEvent::ModifySpeed,
        DEFAULT_ORDER,
        rain_speed,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::SnowSpeed,
        BattleEvent::ModifySpeed,
        DEFAULT_ORDER,
        snow_speed,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::ElectricTerrainSpeed,
        BattleEvent::ModifySpeed,
        DEFAULT_ORDER,
        electric_terrain_speed,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::BulletImmunity,
        BattleEvent::TryHit,
        DEFAULT_ORDER,
        bullet_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::SoundImmunity,
        BattleEvent::TryHit,
        DEFAULT_ORDER,
        sound_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::AllyDamageImmunity,
        BattleEvent::TryHit,
        DEFAULT_ORDER,
        ally_damage_immunity,
    ),
];

fn register_callback(
    registry: &mut EffectRegistry,
    handler: AbilityHandler,
    registration: BehaviorRegistration,
) {
    let source = EffectSource::Ability(handler.ability);
    match handler.suppression {
        AbilitySuppression::Breakable => {
            registry.register_suppressible(
                registration.event,
                source,
                registration.order,
                registration.callback,
            );
        }
        AbilitySuppression::Unbreakable => registry.register(
            registration.event,
            source,
            registration.order,
            registration.callback,
        ),
    }
}

fn double_source_stat(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if owner_pokemon(invocation) == context.source {
        modify_integer(dispatch, 2, 1)?;
    }
    Ok(())
}

fn double_holder_defense(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if matching_defense_holder(invocation, context, dispatch).is_some() {
        modify_integer(dispatch, 2, 1)?;
    }
    Ok(())
}

fn guts_attack(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = matching_source(invocation, context) else {
        return Ok(());
    };
    if pokemon(state, owner)?.status.is_some() {
        modify_integer(dispatch, 3, 2)?;
    }
    Ok(())
}

fn marvel_scale_defense(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = matching_defense_holder(invocation, context, dispatch) else {
        return Ok(());
    };
    if pokemon(state, owner)?.status.is_some() {
        modify_integer(dispatch, 3, 2)?;
    }
    Ok(())
}

fn technician_base_power(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if owner_pokemon(invocation) == context.source && integer(dispatch)? <= 60 {
        modify_integer(dispatch, 3, 2)?;
    }
    Ok(())
}

fn full_hp_damage_reduction(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = matching_target(invocation, context) else {
        return Ok(());
    };
    let pokemon = pokemon(state, owner)?;
    if pokemon.current_hp == pokemon.stats.hp {
        modify_integer(dispatch, 1, 2)?;
    }
    Ok(())
}

fn statused_speed(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = matching_source(invocation, context) else {
        return Ok(());
    };
    if pokemon(state, owner)?.status.is_some() {
        modify_integer(dispatch, 3, 2)?;
    }
    Ok(())
}

fn sun_speed(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    environment_speed(
        invocation,
        context,
        dispatch,
        context.weather == Some(WeatherEnvironment::Sun),
    )
}

fn rain_speed(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    environment_speed(
        invocation,
        context,
        dispatch,
        context.weather == Some(WeatherEnvironment::Rain),
    )
}

fn snow_speed(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    environment_speed(
        invocation,
        context,
        dispatch,
        context.weather == Some(WeatherEnvironment::Snow),
    )
}

fn electric_terrain_speed(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    environment_speed(
        invocation,
        context,
        dispatch,
        context.terrain == Some(TerrainEnvironment::Electric),
    )
}

fn environment_speed(
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    environment_matches: bool,
) -> Result<(), BattleError> {
    if environment_matches && matching_source(invocation, context).is_some() {
        modify_integer(dispatch, 2, 1)?;
    }
    Ok(())
}

fn compensate_paralysis_speed(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = matching_source(invocation, context) else {
        return Ok(());
    };
    if pokemon(state, owner)?.status == Some(crate::MajorStatus::Paralysis) {
        modify_integer(dispatch, 2, 1)?;
    }
    Ok(())
}

fn increase_critical_ratio(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if matching_source(invocation, context).is_some() {
        dispatch.value = EventValue::Integer(integer(dispatch)?.saturating_add(1));
    }
    Ok(())
}

fn prevent_critical_hit(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if matching_target(invocation, context).is_some() {
        boolean(dispatch)?;
        dispatch.value = EventValue::Boolean(false);
    }
    Ok(())
}

fn boost_critical_damage(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if context.critical && matching_source(invocation, context).is_some() {
        modify_integer(dispatch, 3, 2)?;
    }
    Ok(())
}

fn bullet_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    prevent_move_hit(invocation, context, dispatch, context.move_flags.bullet)
}

fn sound_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    prevent_move_hit(invocation, context, dispatch, context.move_flags.sound)
}

fn ally_damage_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let is_ally_damage = context
        .source
        .zip(context.target)
        .is_some_and(|(source, target)| source != target && source.side == target.side)
        && context.move_category != Some(MoveCategory::Status);
    prevent_move_hit(invocation, context, dispatch, is_ally_damage)
}

fn prevent_move_hit(
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    blocked: bool,
) -> Result<(), BattleError> {
    let external_target = context.source != context.target;
    if blocked && external_target && matching_target(invocation, context).is_some() {
        boolean(dispatch)?;
        dispatch.value = EventValue::Boolean(false);
        dispatch.mark_immune();
    }
    Ok(())
}

fn matching_source(invocation: &EffectInvocation, context: &EventContext) -> Option<PokemonRef> {
    let owner = owner_pokemon(invocation)?;
    (Some(owner) == context.source).then_some(owner)
}

fn matching_target(invocation: &EffectInvocation, context: &EventContext) -> Option<PokemonRef> {
    let owner = owner_pokemon(invocation)?;
    (Some(owner) == context.target).then_some(owner)
}

fn matching_defense_holder(
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &EventDispatch,
) -> Option<PokemonRef> {
    let owner = owner_pokemon(invocation)?;
    let holder = match dispatch.event {
        BattleEvent::ModifySourceDefense | BattleEvent::ModifySourceSpecialDefense => {
            context.source
        }
        BattleEvent::ModifyDefense | BattleEvent::ModifySpecialDefense => context.target,
        _ => None,
    };
    (Some(owner) == holder).then_some(owner)
}

const fn owner_pokemon(invocation: &EffectInvocation) -> Option<PokemonRef> {
    match invocation.owner {
        EffectOwner::Pokemon { pokemon } | EffectOwner::Move { pokemon } => Some(pokemon),
        EffectOwner::Battle | EffectOwner::Field | EffectOwner::Side { .. } => None,
    }
}

fn pokemon<'a>(
    state: &'a EffectState<'_>,
    target: PokemonRef,
) -> Result<&'a crate::PokemonState, BattleError> {
    state
        .battle()
        .side(target.side)
        .pokemon(target.pokemon)
        .ok_or_else(|| BattleError::InvalidState("ability owner is out of bounds".to_owned()))
}

fn integer(dispatch: &EventDispatch) -> Result<i64, BattleError> {
    match dispatch.value {
        EventValue::Integer(value) => Ok(value),
        EventValue::None | EventValue::Boolean(_) => Err(BattleError::InvalidState(
            "ability modifier received a non-integer relay".to_owned(),
        )),
    }
}

fn boolean(dispatch: &EventDispatch) -> Result<bool, BattleError> {
    match dispatch.value {
        EventValue::Boolean(value) => Ok(value),
        EventValue::None | EventValue::Integer(_) => Err(BattleError::InvalidState(
            "ability modifier received a non-boolean relay".to_owned(),
        )),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BattleState, BattleStats, MajorStatus, MoveId, MoveSlot, MoveTarget, PokemonIndex,
        PokemonPosition, PokemonState, SideId, SideState, SpeciesId, StatStages, StatusState,
        TeamState,
    };

    fn pokemon(ability: AbilityId) -> PokemonState {
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
            ability,
            item: None,
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

    fn battle(source_ability: AbilityId, target_ability: AbilityId) -> BattleState {
        let side =
            |ability| SideState::new(TeamState::new((0..4).map(|_| pokemon(ability)).collect()));
        let mut battle = BattleState::new(side(source_ability), side(target_ability), 4).unwrap();
        let selected = (0..4).map(PokemonIndex::new).collect::<Vec<_>>();
        battle.select_team(SideId::P1, selected.clone()).unwrap();
        battle.select_team(SideId::P2, selected).unwrap();
        battle
    }

    fn context() -> EventContext {
        EventContext {
            source: Some(PokemonRef::new(SideId::P1, PokemonIndex::new(0))),
            target: Some(PokemonRef::new(SideId::P2, PokemonIndex::new(0))),
            move_id: Some(MoveId::new(1)),
            ..EventContext::default()
        }
    }

    const fn unbreakable(ability: AbilityId, behavior: AbilityBehavior) -> AbilityHandler {
        AbilityHandler::new(ability, behavior, AbilitySuppression::Unbreakable)
    }

    const fn breakable(ability: AbilityId, behavior: AbilityBehavior) -> AbilityHandler {
        AbilityHandler::new(ability, behavior, AbilitySuppression::Breakable)
    }

    fn dispatch_integer(
        registry: &EffectRegistry,
        battle: &mut BattleState,
        event: BattleEvent,
        value: i64,
    ) -> EventValue {
        registry
            .dispatch(battle, event, context(), EventValue::Integer(value))
            .unwrap()
            .value
    }

    #[test]
    fn source_stat_and_base_power_handlers_are_scoped_to_the_holder() {
        let huge_power = AbilityId::new(10);
        let technician = AbilityId::new(11);
        let pure_power = AbilityId::new(12);
        let registry = crate::conditions::composed_registry(
            &EffectRegistry::new(),
            [
                unbreakable(huge_power, AbilityBehavior::DoubleAttack),
                unbreakable(pure_power, AbilityBehavior::DoubleAttack),
                unbreakable(technician, AbilityBehavior::LowPowerBoost),
            ],
        );
        let mut huge_power_battle = battle(huge_power, AbilityId::new(1));
        assert_eq!(
            dispatch_integer(
                &registry,
                &mut huge_power_battle,
                BattleEvent::ModifyAttack,
                101,
            ),
            EventValue::Integer(202)
        );
        assert_eq!(
            dispatch_integer(
                &registry,
                &mut huge_power_battle,
                BattleEvent::ModifySpecialAttack,
                101,
            ),
            EventValue::Integer(101)
        );

        let mut pure_power_battle = battle(pure_power, AbilityId::new(1));
        assert_eq!(
            dispatch_integer(
                &registry,
                &mut pure_power_battle,
                BattleEvent::ModifyAttack,
                101,
            ),
            EventValue::Integer(202)
        );

        let mut technician_battle = battle(technician, AbilityId::new(1));
        for (power, expected) in [(60, 90), (61, 61)] {
            assert_eq!(
                dispatch_integer(
                    &registry,
                    &mut technician_battle,
                    BattleEvent::ModifyBasePower,
                    power,
                ),
                EventValue::Integer(expected)
            );
        }
    }

    #[test]
    fn conditional_and_defensive_handlers_use_authoritative_state() {
        let guts = AbilityId::new(20);
        let marvel_scale = AbilityId::new(21);
        let multiscale = AbilityId::new(22);
        let fur_coat = AbilityId::new(24);
        let registry = crate::conditions::composed_registry(
            &EffectRegistry::new(),
            [
                unbreakable(guts, AbilityBehavior::StatusedAttack),
                breakable(marvel_scale, AbilityBehavior::StatusedDefense),
                breakable(multiscale, AbilityBehavior::FullHpDamageReduction),
                breakable(fur_coat, AbilityBehavior::DoubleDefense),
            ],
        );

        let mut guts_battle = battle(guts, AbilityId::new(1));
        guts_battle
            .side_mut(SideId::P1)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .status = Some(MajorStatus::Burn);
        assert_eq!(
            dispatch_integer(&registry, &mut guts_battle, BattleEvent::ModifyAttack, 101,),
            EventValue::Integer(151)
        );

        let mut scale_battle = battle(AbilityId::new(1), marvel_scale);
        scale_battle
            .side_mut(SideId::P2)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .status = Some(MajorStatus::Poison);
        assert_eq!(
            dispatch_integer(
                &registry,
                &mut scale_battle,
                BattleEvent::ModifyDefense,
                101,
            ),
            EventValue::Integer(151)
        );

        let mut fur_coat_battle = battle(AbilityId::new(1), fur_coat);
        assert_eq!(
            dispatch_integer(
                &registry,
                &mut fur_coat_battle,
                BattleEvent::ModifyDefense,
                101,
            ),
            EventValue::Integer(202)
        );
        let target_does_not_modify_source = dispatch_integer(
            &registry,
            &mut fur_coat_battle,
            BattleEvent::ModifySourceDefense,
            101,
        );
        assert_eq!(target_does_not_modify_source, EventValue::Integer(101));
        let mut source_fur_coat = battle(fur_coat, AbilityId::new(1));
        assert_eq!(
            dispatch_integer(
                &registry,
                &mut source_fur_coat,
                BattleEvent::ModifySourceDefense,
                101,
            ),
            EventValue::Integer(202)
        );

        let mut multiscale_battle = battle(AbilityId::new(1), multiscale);
        assert_eq!(
            dispatch_integer(
                &registry,
                &mut multiscale_battle,
                BattleEvent::ModifyDamage,
                51,
            ),
            EventValue::Integer(25)
        );
        multiscale_battle
            .side_mut(SideId::P2)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .current_hp = 99;
        assert_eq!(
            dispatch_integer(
                &registry,
                &mut multiscale_battle,
                BattleEvent::ModifyDamage,
                51,
            ),
            EventValue::Integer(51)
        );
    }

    #[test]
    fn multiple_descriptors_and_critical_handlers_compose_without_identity_branches() {
        let quick_feet = AbilityId::new(30);
        let super_luck = AbilityId::new(31);
        let battle_armor = AbilityId::new(32);
        let sniper = AbilityId::new(33);
        let registry = crate::conditions::composed_registry(
            &EffectRegistry::new(),
            [
                unbreakable(quick_feet, AbilityBehavior::StatusedSpeed),
                unbreakable(quick_feet, AbilityBehavior::IgnoreParalysisSpeedPenalty),
                unbreakable(super_luck, AbilityBehavior::IncreaseCriticalRatio),
                breakable(battle_armor, AbilityBehavior::PreventCriticalHit),
                unbreakable(sniper, AbilityBehavior::BoostCriticalDamage),
            ],
        );

        let mut quick_feet_battle = battle(quick_feet, AbilityId::new(1));
        quick_feet_battle
            .side_mut(SideId::P1)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .status = Some(MajorStatus::Paralysis);
        assert_eq!(
            dispatch_integer(
                &registry,
                &mut quick_feet_battle,
                BattleEvent::ModifySpeed,
                100,
            ),
            EventValue::Integer(150)
        );

        let mut super_luck_battle = battle(super_luck, AbilityId::new(1));
        assert_eq!(
            dispatch_integer(
                &registry,
                &mut super_luck_battle,
                BattleEvent::ModifyCritRatio,
                1,
            ),
            EventValue::Integer(2)
        );

        let mut armor_battle = battle(AbilityId::new(1), battle_armor);
        let normal = registry
            .dispatch(
                &mut armor_battle,
                BattleEvent::CriticalHit,
                context(),
                EventValue::Boolean(true),
            )
            .unwrap();
        assert_eq!(normal.value, EventValue::Boolean(false));
        let suppressed = registry
            .dispatch(
                &mut armor_battle,
                BattleEvent::CriticalHit,
                EventContext {
                    suppress_target_ability: true,
                    ..context()
                },
                EventValue::Boolean(true),
            )
            .unwrap();
        assert_eq!(suppressed.value, EventValue::Boolean(true));

        let mut sniper_battle = battle(sniper, AbilityId::new(1));
        let critical_context = EventContext {
            critical: true,
            ..context()
        };
        let boosted = registry
            .dispatch(
                &mut sniper_battle,
                BattleEvent::ModifyDamage,
                critical_context,
                EventValue::Integer(100),
            )
            .unwrap();
        assert_eq!(boosted.value, EventValue::Integer(150));
    }

    #[test]
    fn environment_immunity_and_suppression_metadata_compose() {
        let chlorophyll = AbilityId::new(40);
        let bulletproof = AbilityId::new(41);
        let source_combo = AbilityId::new(42);
        let multiscale = AbilityId::new(43);
        let registry = crate::conditions::composed_registry(
            &EffectRegistry::new(),
            [
                unbreakable(chlorophyll, AbilityBehavior::SunSpeed),
                breakable(bulletproof, AbilityBehavior::BulletImmunity),
                unbreakable(source_combo, AbilityBehavior::BreaksTargetAbilities),
                unbreakable(source_combo, AbilityBehavior::BoostCriticalDamage),
                breakable(multiscale, AbilityBehavior::FullHpDamageReduction),
            ],
        );

        let mut speed_battle = battle(chlorophyll, AbilityId::new(1));
        let speed = registry
            .dispatch(
                &mut speed_battle,
                BattleEvent::ModifySpeed,
                EventContext {
                    weather: Some(WeatherEnvironment::Sun),
                    ..context()
                },
                EventValue::Integer(100),
            )
            .unwrap();
        assert_eq!(speed.value, EventValue::Integer(200));

        let mut immunity_battle = battle(AbilityId::new(1), bulletproof);
        let bullet_context = EventContext {
            move_flags: crate::MoveInteractionFlags {
                bullet: true,
                sound: false,
            },
            ..context()
        };
        let blocked = registry
            .dispatch(
                &mut immunity_battle,
                BattleEvent::TryHit,
                bullet_context,
                EventValue::Boolean(true),
            )
            .unwrap();
        assert_eq!(blocked.value, EventValue::Boolean(false));
        let bypassed = registry
            .dispatch(
                &mut immunity_battle,
                BattleEvent::TryHit,
                EventContext {
                    suppress_target_ability: true,
                    ..bullet_context
                },
                EventValue::Boolean(true),
            )
            .unwrap();
        assert_eq!(bypassed.value, EventValue::Boolean(true));

        let mut source_battle = battle(source_combo, multiscale);
        let source_preserved = registry
            .dispatch(
                &mut source_battle,
                BattleEvent::ModifyDamage,
                EventContext {
                    suppress_target_ability: true,
                    critical: true,
                    ..context()
                },
                EventValue::Integer(100),
            )
            .unwrap();
        assert_eq!(source_preserved.value, EventValue::Integer(150));
        assert!(source_preserved
            .invocations
            .iter()
            .any(|invocation| invocation.source == EffectSource::Ability(source_combo)));
        assert!(!source_preserved.invocations.iter().any(|invocation| {
            invocation.owner
                == EffectOwner::Pokemon {
                    pokemon: context().target.unwrap(),
                }
        }));
    }
}
