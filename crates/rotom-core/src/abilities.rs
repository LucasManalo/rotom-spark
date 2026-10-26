use crate::{
    AbilityId, BattleError, BattleEvent, BattleStat, EffectInvocation, EffectOrder, EffectOwner,
    EffectRegistry, EffectSource, EffectState, EventContext, EventDispatch, EventValue,
    MajorStatus, MoveCategory, PokemonRef, PokemonType, RngReason, TerrainEnvironment,
    WeatherEnvironment,
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
    LowHpFireBoost,
    LowHpGrassBoost,
    LowHpWaterBoost,
    LowHpBugBoost,
    PunchBoost,
    BiteBoost,
    SlicingBoost,
    ContactBoost,
    RecoilBoost,
    SoundPower,
    SandForce,
    AllySteelBoost,
    SuperEffectiveReduction,
    Fluffy,
    ThickFat,
    GrassyDefense,
    GaleWings,
    OpposingStatusImmunity,
    PoisonImmunity,
    SleepImmunity,
    ParalysisImmunity,
    FreezeImmunity,
    AllySleepImmunity,
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
    SandRush,
    Prankster,
    BulletImmunity,
    SoundImmunity,
    AllyDamageImmunity,
    WaterAbsorb,
    VoltAbsorb,
    EarthEater,
    MotorDrive,
    GrassImmunityAttackBoost,
    GroundImmunity,
    WeatherEvasion(WeatherEnvironment),
    ContactDamageReduction,
    PowderWeatherImmunity,
    BurnImmunity,
    WaterBubble,
    NaturalCure,
    Regenerator,
    Hospitality,
    Hydration,
    ShedSkin,
    Healer,
    Stamina,
    WeakArmor,
    Justified,
    ThermalExchange,
    AngerPoint,
    FriendGuard,
    MegaLauncher,
    Merciless,
    CompoundEyes,
    Hustle,
    IceBody,
    RainDish,
    Moxie,
    Gooey,
    /// The pinned ability has no authoritative battle-state effect.
    NoAuthoritativeEffect,
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
        AbilityBehavior::LowHpFireBoost,
        BattleEvent::ModifyAttack,
        EffectOrder::new(0, 5, 0),
        low_hp_fire_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::LowHpFireBoost,
        BattleEvent::ModifySpecialAttack,
        EffectOrder::new(0, 5, 0),
        low_hp_fire_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::LowHpGrassBoost,
        BattleEvent::ModifyAttack,
        EffectOrder::new(0, 5, 0),
        low_hp_grass_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::LowHpGrassBoost,
        BattleEvent::ModifySpecialAttack,
        EffectOrder::new(0, 5, 0),
        low_hp_grass_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::LowHpWaterBoost,
        BattleEvent::ModifyAttack,
        EffectOrder::new(0, 5, 0),
        low_hp_water_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::LowHpWaterBoost,
        BattleEvent::ModifySpecialAttack,
        EffectOrder::new(0, 5, 0),
        low_hp_water_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::LowHpBugBoost,
        BattleEvent::ModifyAttack,
        EffectOrder::new(0, 5, 0),
        low_hp_bug_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::LowHpBugBoost,
        BattleEvent::ModifySpecialAttack,
        EffectOrder::new(0, 5, 0),
        low_hp_bug_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::PunchBoost,
        BattleEvent::ModifyBasePower,
        EffectOrder::new(0, 23, 0),
        punch_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::BiteBoost,
        BattleEvent::ModifyBasePower,
        EffectOrder::new(0, 19, 0),
        bite_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::SlicingBoost,
        BattleEvent::ModifyBasePower,
        EffectOrder::new(0, 19, 0),
        slicing_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::ContactBoost,
        BattleEvent::ModifyBasePower,
        EffectOrder::new(0, 21, 0),
        contact_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::RecoilBoost,
        BattleEvent::ModifyBasePower,
        EffectOrder::new(0, 23, 0),
        recoil_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::SoundPower,
        BattleEvent::ModifyBasePower,
        EffectOrder::new(0, 7, 0),
        sound_power_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::SoundPower,
        BattleEvent::ModifyDamage,
        DEFAULT_ORDER,
        sound_damage_reduction,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::SandForce,
        BattleEvent::ModifyBasePower,
        EffectOrder::new(0, 21, 0),
        sand_force_power,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::SandForce,
        BattleEvent::WeatherImmunity,
        DEFAULT_ORDER,
        sandstorm_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::AllySteelBoost,
        BattleEvent::ModifyBasePower,
        EffectOrder::new(0, 22, 0),
        ally_steel_boost,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::SuperEffectiveReduction,
        BattleEvent::ModifyDamage,
        DEFAULT_ORDER,
        super_effective_reduction,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::Fluffy,
        BattleEvent::ModifyDamage,
        DEFAULT_ORDER,
        fluffy_damage,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::ThickFat,
        BattleEvent::ModifyAttack,
        EffectOrder::new(0, 6, 0),
        thick_fat_stat,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::ThickFat,
        BattleEvent::ModifySpecialAttack,
        EffectOrder::new(0, 5, 0),
        thick_fat_stat,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::GrassyDefense,
        BattleEvent::ModifyDefense,
        EffectOrder::new(0, 6, 0),
        grassy_defense,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::GaleWings,
        BattleEvent::ModifyPriority,
        DEFAULT_ORDER,
        gale_wings_priority,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::OpposingStatusImmunity,
        BattleEvent::TryHit,
        DEFAULT_ORDER,
        opposing_status_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::PoisonImmunity,
        BattleEvent::TrySetStatus,
        DEFAULT_ORDER,
        poison_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::PoisonImmunity,
        BattleEvent::BeforeTurn,
        DEFAULT_ORDER,
        cure_poison,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::PoisonImmunity,
        BattleEvent::SwitchIn,
        DEFAULT_ORDER,
        cure_poison,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::PoisonImmunity,
        BattleEvent::AfterSetStatus,
        DEFAULT_ORDER,
        cure_poison,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::SleepImmunity,
        BattleEvent::TrySetStatus,
        DEFAULT_ORDER,
        sleep_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::SleepImmunity,
        BattleEvent::BeforeTurn,
        DEFAULT_ORDER,
        cure_sleep,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::SleepImmunity,
        BattleEvent::SwitchIn,
        DEFAULT_ORDER,
        cure_sleep,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::SleepImmunity,
        BattleEvent::AfterSetStatus,
        DEFAULT_ORDER,
        cure_sleep,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::ParalysisImmunity,
        BattleEvent::TrySetStatus,
        DEFAULT_ORDER,
        paralysis_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::ParalysisImmunity,
        BattleEvent::BeforeTurn,
        DEFAULT_ORDER,
        cure_paralysis,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::ParalysisImmunity,
        BattleEvent::SwitchIn,
        DEFAULT_ORDER,
        cure_paralysis,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::ParalysisImmunity,
        BattleEvent::AfterSetStatus,
        DEFAULT_ORDER,
        cure_paralysis,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::FreezeImmunity,
        BattleEvent::TrySetStatus,
        DEFAULT_ORDER,
        freeze_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::FreezeImmunity,
        BattleEvent::BeforeTurn,
        DEFAULT_ORDER,
        cure_freeze,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::FreezeImmunity,
        BattleEvent::SwitchIn,
        DEFAULT_ORDER,
        cure_freeze,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::FreezeImmunity,
        BattleEvent::AfterSetStatus,
        DEFAULT_ORDER,
        cure_freeze,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::AllySleepImmunity,
        BattleEvent::TrySetStatus,
        DEFAULT_ORDER,
        ally_sleep_immunity,
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
        AbilityBehavior::SandRush,
        BattleEvent::ModifySpeed,
        DEFAULT_ORDER,
        sand_rush_speed,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::SandRush,
        BattleEvent::WeatherImmunity,
        DEFAULT_ORDER,
        sandstorm_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::Prankster,
        BattleEvent::ModifyPriority,
        DEFAULT_ORDER,
        prankster_priority,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::Prankster,
        BattleEvent::TryImmunity,
        DEFAULT_ORDER,
        prankster_dark_immunity,
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
    BehaviorRegistration::new(
        AbilityBehavior::WaterAbsorb,
        BattleEvent::TryHit,
        DEFAULT_ORDER,
        water_absorb,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::VoltAbsorb,
        BattleEvent::TryHit,
        DEFAULT_ORDER,
        volt_absorb,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::EarthEater,
        BattleEvent::TryHit,
        DEFAULT_ORDER,
        earth_eater,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::MotorDrive,
        BattleEvent::TryHit,
        DEFAULT_ORDER,
        motor_drive,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::GrassImmunityAttackBoost,
        BattleEvent::TryHit,
        EffectOrder::new(0, 1, 0),
        sap_sipper,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::GroundImmunity,
        BattleEvent::TryHit,
        DEFAULT_ORDER,
        ground_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::WeatherEvasion(WeatherEnvironment::Sandstorm),
        BattleEvent::ModifyAccuracy,
        EffectOrder::new(0, -1, 0),
        sand_veil_accuracy,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::WeatherEvasion(WeatherEnvironment::Sandstorm),
        BattleEvent::WeatherImmunity,
        DEFAULT_ORDER,
        sandstorm_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::WeatherEvasion(WeatherEnvironment::Snow),
        BattleEvent::ModifyAccuracy,
        EffectOrder::new(0, -1, 0),
        snow_cloak_accuracy,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::WeatherEvasion(WeatherEnvironment::Snow),
        BattleEvent::WeatherImmunity,
        DEFAULT_ORDER,
        snow_weather_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::ContactDamageReduction,
        BattleEvent::ModifyDamage,
        DEFAULT_ORDER,
        contact_damage_reduction,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::PowderWeatherImmunity,
        BattleEvent::TryHit,
        EffectOrder::new(0, 1, 0),
        powder_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::PowderWeatherImmunity,
        BattleEvent::WeatherImmunity,
        DEFAULT_ORDER,
        sandstorm_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::BurnImmunity,
        BattleEvent::TrySetStatus,
        DEFAULT_ORDER,
        burn_immunity,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::BurnImmunity,
        BattleEvent::BeforeTurn,
        DEFAULT_ORDER,
        cure_burn,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::BurnImmunity,
        BattleEvent::SwitchIn,
        DEFAULT_ORDER,
        cure_burn,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::BurnImmunity,
        BattleEvent::AfterSetStatus,
        DEFAULT_ORDER,
        cure_burn,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::WaterBubble,
        BattleEvent::ModifyAttack,
        EffectOrder::new(0, 5, 0),
        water_bubble_stat,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::WaterBubble,
        BattleEvent::ModifySpecialAttack,
        EffectOrder::new(0, 5, 0),
        water_bubble_stat,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::NaturalCure,
        BattleEvent::SwitchOut,
        DEFAULT_ORDER,
        cure_any_status,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::Regenerator,
        BattleEvent::SwitchOut,
        DEFAULT_ORDER,
        regenerator,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::Hospitality,
        BattleEvent::SwitchIn,
        EffectOrder::new(0, -2, 0),
        hospitality,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::Hydration,
        BattleEvent::Residual,
        EffectOrder::new(5, 0, 3),
        hydration,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::ShedSkin,
        BattleEvent::Residual,
        EffectOrder::new(5, 0, 3),
        shed_skin,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::Healer,
        BattleEvent::Residual,
        EffectOrder::new(5, 0, 3),
        healer,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::Stamina,
        BattleEvent::AfterDamage,
        DEFAULT_ORDER,
        stamina,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::WeakArmor,
        BattleEvent::AfterDamage,
        DEFAULT_ORDER,
        weak_armor,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::Justified,
        BattleEvent::AfterDamage,
        DEFAULT_ORDER,
        justified,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::ThermalExchange,
        BattleEvent::AfterDamage,
        DEFAULT_ORDER,
        thermal_exchange,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::AngerPoint,
        BattleEvent::AfterDamage,
        DEFAULT_ORDER,
        anger_point,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::FriendGuard,
        BattleEvent::ModifyDamage,
        DEFAULT_ORDER,
        friend_guard,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::MegaLauncher,
        BattleEvent::ModifyBasePower,
        EffectOrder::new(0, 19, 0),
        mega_launcher,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::Merciless,
        BattleEvent::ModifyCritRatio,
        DEFAULT_ORDER,
        merciless,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::CompoundEyes,
        BattleEvent::ModifyAccuracy,
        EffectOrder::new(0, -1, 0),
        compound_eyes,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::Hustle,
        BattleEvent::ModifyAttack,
        EffectOrder::new(0, 5, 0),
        hustle_attack,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::Hustle,
        BattleEvent::ModifyAccuracy,
        EffectOrder::new(0, -1, 0),
        hustle_accuracy,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::IceBody,
        BattleEvent::Residual,
        EffectOrder::new(1, 0, 0),
        ice_body,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::RainDish,
        BattleEvent::Residual,
        EffectOrder::new(1, 0, 0),
        rain_dish,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::Moxie,
        BattleEvent::Faint,
        DEFAULT_ORDER,
        moxie,
    ),
    BehaviorRegistration::new(
        AbilityBehavior::Gooey,
        BattleEvent::AfterDamage,
        DEFAULT_ORDER,
        gooey,
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

fn low_hp_fire_boost(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    low_hp_type_boost(state, invocation, context, dispatch, PokemonType::Fire)
}

fn low_hp_grass_boost(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    low_hp_type_boost(state, invocation, context, dispatch, PokemonType::Grass)
}

fn low_hp_water_boost(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    low_hp_type_boost(state, invocation, context, dispatch, PokemonType::Water)
}

fn low_hp_bug_boost(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    low_hp_type_boost(state, invocation, context, dispatch, PokemonType::Bug)
}

fn low_hp_type_boost(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    move_type: PokemonType,
) -> Result<(), BattleError> {
    let Some(owner) = matching_source(invocation, context) else {
        return Ok(());
    };
    let holder = pokemon(state, owner)?;
    if context.move_type == Some(move_type)
        && u32::from(holder.current_hp).saturating_mul(3) <= u32::from(holder.stats.hp)
    {
        modify_integer(dispatch, 3, 2)?;
    }
    Ok(())
}

fn punch_boost(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    source_flag_boost(
        invocation,
        context,
        dispatch,
        context.move_flags.punch(),
        4915,
        4096,
    )
}

fn bite_boost(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    source_flag_boost(
        invocation,
        context,
        dispatch,
        context.move_flags.bite(),
        3,
        2,
    )
}

fn slicing_boost(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    source_flag_boost(
        invocation,
        context,
        dispatch,
        context.move_flags.slicing(),
        3,
        2,
    )
}

fn contact_boost(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    source_flag_boost(
        invocation,
        context,
        dispatch,
        context.move_flags.contact(),
        5325,
        4096,
    )
}

fn recoil_boost(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    source_flag_boost(
        invocation,
        context,
        dispatch,
        context.move_flags.recoil(),
        4915,
        4096,
    )
}

fn sound_power_boost(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    source_flag_boost(
        invocation,
        context,
        dispatch,
        context.move_flags.sound(),
        5325,
        4096,
    )
}

fn source_flag_boost(
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    applies: bool,
    numerator: i64,
    denominator: i64,
) -> Result<(), BattleError> {
    if applies && matching_source(invocation, context).is_some() {
        modify_integer(dispatch, numerator, denominator)?;
    }
    Ok(())
}

fn sound_damage_reduction(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if context.move_flags.sound() && matching_target(invocation, context).is_some() {
        modify_integer(dispatch, 1, 2)?;
    }
    Ok(())
}

fn sand_force_power(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let boosted_type = matches!(
        context.move_type,
        Some(PokemonType::Rock | PokemonType::Ground | PokemonType::Steel)
    );
    if context.weather == Some(WeatherEnvironment::Sandstorm)
        && boosted_type
        && matching_source(invocation, context).is_some()
    {
        modify_integer(dispatch, 5325, 4096)?;
    }
    Ok(())
}

fn ally_steel_boost(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let applies = owner_pokemon(invocation)
        .zip(context.source)
        .is_some_and(|(owner, source)| owner.side == source.side)
        && context.move_type == Some(PokemonType::Steel);
    if applies {
        modify_integer(dispatch, 3, 2)?;
    }
    Ok(())
}

fn super_effective_reduction(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if context.type_effectiveness.is_some_and(|value| value > 0)
        && matching_target(invocation, context).is_some()
    {
        modify_integer(dispatch, 3, 4)?;
    }
    Ok(())
}

fn fluffy_damage(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if matching_target(invocation, context).is_none() {
        return Ok(());
    }
    match (
        context.move_type == Some(PokemonType::Fire),
        context.move_flags.contact(),
    ) {
        (true, false) => modify_integer(dispatch, 2, 1)?,
        (false, true) => modify_integer(dispatch, 1, 2)?,
        (true, true) | (false, false) => {}
    }
    Ok(())
}

fn thick_fat_stat(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if matches!(
        context.move_type,
        Some(PokemonType::Fire | PokemonType::Ice)
    ) && matching_target(invocation, context).is_some()
    {
        modify_integer(dispatch, 1, 2)?;
    }
    Ok(())
}

fn grassy_defense(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if context.terrain == Some(TerrainEnvironment::Grassy)
        && matching_target(invocation, context).is_some()
    {
        modify_integer(dispatch, 3, 2)?;
    }
    Ok(())
}

fn gale_wings_priority(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = matching_source(invocation, context) else {
        return Ok(());
    };
    let holder = pokemon(state, owner)?;
    if context.move_type == Some(PokemonType::Flying) && holder.current_hp == holder.stats.hp {
        dispatch.value = EventValue::Integer(integer(dispatch)?.saturating_add(1));
    }
    Ok(())
}

fn opposing_status_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if context.move_category == Some(MoveCategory::Status)
        && context.source != context.target
        && matching_target(invocation, context).is_some()
    {
        boolean(dispatch)?;
        dispatch.value = EventValue::Boolean(false);
        dispatch.mark_immune();
    }
    Ok(())
}

fn poison_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    prevent_status(
        invocation,
        context,
        dispatch,
        matches!(
            context.requested_status,
            Some(MajorStatus::Poison | MajorStatus::BadlyPoisoned)
        ),
        false,
    )
}

fn sleep_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    prevent_status(
        invocation,
        context,
        dispatch,
        context.requested_status == Some(MajorStatus::Sleep),
        false,
    )
}

fn paralysis_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    prevent_status(
        invocation,
        context,
        dispatch,
        context.requested_status == Some(MajorStatus::Paralysis),
        false,
    )
}

fn freeze_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    prevent_status(
        invocation,
        context,
        dispatch,
        context.requested_status == Some(MajorStatus::Freeze),
        false,
    )
}

fn ally_sleep_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    prevent_status(
        invocation,
        context,
        dispatch,
        context.requested_status == Some(MajorStatus::Sleep),
        true,
    )
}

fn prevent_status(
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    status_matches: bool,
    protects_allies: bool,
) -> Result<(), BattleError> {
    let applies = if protects_allies {
        owner_pokemon(invocation)
            .zip(context.target)
            .is_some_and(|(owner, target)| owner.side == target.side)
    } else {
        matching_target(invocation, context).is_some()
    };
    if status_matches && applies {
        boolean(dispatch)?;
        dispatch.value = EventValue::Boolean(false);
    }
    Ok(())
}

fn cure_poison(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    cure_matching_status(state, invocation, |status| {
        matches!(status, MajorStatus::Poison | MajorStatus::BadlyPoisoned)
    })
}

fn cure_sleep(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    cure_matching_status(state, invocation, |status| status == MajorStatus::Sleep)
}

fn cure_paralysis(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    cure_matching_status(state, invocation, |status| status == MajorStatus::Paralysis)
}

fn cure_freeze(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    cure_matching_status(state, invocation, |status| status == MajorStatus::Freeze)
}

fn cure_matching_status(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    blocked: impl FnOnce(MajorStatus) -> bool,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    if pokemon(state, owner)?.status.is_some_and(blocked) {
        state.cure_major_status(owner)?;
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

fn sand_rush_speed(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    environment_speed(
        invocation,
        context,
        dispatch,
        context.weather == Some(WeatherEnvironment::Sandstorm),
    )
}

fn sandstorm_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if context.weather == Some(WeatherEnvironment::Sandstorm)
        && matching_target(invocation, context).is_some()
    {
        boolean(dispatch)?;
        dispatch.value = EventValue::Boolean(false);
    }
    Ok(())
}

fn prankster_priority(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if context.move_category == Some(MoveCategory::Status)
        && matching_source(invocation, context).is_some()
    {
        dispatch.value = EventValue::Integer(integer(dispatch)?.saturating_add(1));
        dispatch.mark_prankster_boosted();
    }
    Ok(())
}

fn prankster_dark_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let opposing_dark_target = context
        .source
        .zip(context.target)
        .zip(context.target_types)
        .is_some_and(|((source, target), types)| {
            source.side != target.side
                && (types.primary == crate::PokemonType::Dark
                    || types.secondary == Some(crate::PokemonType::Dark))
        });
    if context.prankster_boosted
        && opposing_dark_target
        && matching_source(invocation, context).is_some()
    {
        boolean(dispatch)?;
        dispatch.value = EventValue::Boolean(false);
        dispatch.mark_immune();
    }
    Ok(())
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
    prevent_move_hit(invocation, context, dispatch, context.move_flags.bullet())
}

fn sound_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    prevent_move_hit(invocation, context, dispatch, context.move_flags.sound())
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

fn water_absorb(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    absorb_type(state, invocation, context, dispatch, PokemonType::Water)
}

fn volt_absorb(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    absorb_type(state, invocation, context, dispatch, PokemonType::Electric)
}

fn earth_eater(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    absorb_type(state, invocation, context, dispatch, PokemonType::Ground)
}

fn absorb_type(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    move_type: PokemonType,
) -> Result<(), BattleError> {
    let Some(owner) = matching_target(invocation, context) else {
        return Ok(());
    };
    if context.source == context.target || context.move_type != Some(move_type) {
        return Ok(());
    }
    let healing = pokemon(state, owner)?.stats.hp / 4;
    state.heal_hp(owner, healing)?;
    boolean(dispatch)?;
    dispatch.value = EventValue::Boolean(false);
    dispatch.mark_immune();
    Ok(())
}

fn motor_drive(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = matching_target(invocation, context) else {
        return Ok(());
    };
    if context.source != context.target && context.move_type == Some(PokemonType::Electric) {
        state.change_stat_stage(owner, BattleStat::Speed, 1)?;
        boolean(dispatch)?;
        dispatch.value = EventValue::Boolean(false);
        dispatch.mark_immune();
    }
    Ok(())
}

fn sap_sipper(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = matching_target(invocation, context) else {
        return Ok(());
    };
    if context.source != context.target && context.move_type == Some(PokemonType::Grass) {
        state.change_stat_stage(owner, BattleStat::Attack, 1)?;
        boolean(dispatch)?;
        dispatch.value = EventValue::Boolean(false);
        dispatch.mark_immune();
    }
    Ok(())
}

fn ground_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    prevent_move_hit(
        invocation,
        context,
        dispatch,
        context.move_type == Some(PokemonType::Ground) && context.target_grounded == Some(false),
    )
}

fn sand_veil_accuracy(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    weather_evasion(invocation, context, dispatch, WeatherEnvironment::Sandstorm)
}

fn snow_cloak_accuracy(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    weather_evasion(invocation, context, dispatch, WeatherEnvironment::Snow)
}

fn weather_evasion(
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    weather: WeatherEnvironment,
) -> Result<(), BattleError> {
    if context.weather == Some(weather) && matching_target(invocation, context).is_some() {
        modify_integer(dispatch, 3277, 4096)?;
    }
    Ok(())
}

fn snow_weather_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if context.weather == Some(WeatherEnvironment::Snow)
        && matching_target(invocation, context).is_some()
    {
        boolean(dispatch)?;
        dispatch.value = EventValue::Boolean(false);
    }
    Ok(())
}

fn contact_damage_reduction(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if context.source != context.target
        && context.move_flags.contact()
        && matching_target(invocation, context).is_some()
    {
        modify_integer(dispatch, 1, 2)?;
    }
    Ok(())
}

fn powder_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    prevent_move_hit(invocation, context, dispatch, context.move_flags.powder())
}

fn burn_immunity(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    prevent_status(
        invocation,
        context,
        dispatch,
        context.requested_status == Some(MajorStatus::Burn),
        false,
    )
}

fn cure_burn(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    cure_matching_status(state, invocation, |status| status == MajorStatus::Burn)
}

fn water_bubble_stat(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if context.move_type == Some(PokemonType::Water)
        && matching_source(invocation, context).is_some()
    {
        modify_integer(dispatch, 2, 1)?;
    } else if context.move_type == Some(PokemonType::Fire)
        && matching_target(invocation, context).is_some()
    {
        modify_integer(dispatch, 1, 2)?;
    }
    Ok(())
}

fn cure_any_status(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    if let Some(owner) = matching_source(invocation, context) {
        state.cure_major_status(owner)?;
    }
    Ok(())
}

fn regenerator(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = matching_source(invocation, context) else {
        return Ok(());
    };
    let healing = pokemon(state, owner)?.stats.hp / 3;
    state.heal_hp(owner, healing)?;
    Ok(())
}

fn hospitality(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = matching_source(invocation, context) else {
        return Ok(());
    };
    let active = state.battle().side(owner.side).active;
    for ally in active
        .into_iter()
        .flatten()
        .map(|pokemon| PokemonRef::new(owner.side, pokemon))
        .filter(|ally| *ally != owner)
    {
        let healing = pokemon(state, ally)?.stats.hp / 4;
        state.heal_hp(ally, healing)?;
    }
    Ok(())
}

fn hydration(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    if context.weather == Some(WeatherEnvironment::Rain) && pokemon(state, owner)?.status.is_some()
    {
        state.cure_major_status(owner)?;
    }
    Ok(())
}

fn shed_skin(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    let holder = pokemon(state, owner)?;
    if holder.current_hp > 0
        && holder.status.is_some()
        && state.chance(RngReason::Ability, 33, 100)?
    {
        state.cure_major_status(owner)?;
    }
    Ok(())
}

fn healer(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    let active = state.battle().side(owner.side).active;
    for ally in active
        .into_iter()
        .flatten()
        .map(|pokemon| PokemonRef::new(owner.side, pokemon))
        .filter(|ally| *ally != owner)
    {
        if pokemon(state, ally)?.status.is_some() && state.chance(RngReason::Ability, 3, 10)? {
            state.cure_major_status(ally)?;
        }
    }
    Ok(())
}

fn stamina(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    boost_damaged_holder(
        state,
        invocation,
        context,
        dispatch,
        BattleStat::Defense,
        1,
        true,
    )
}

fn weak_armor(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if context.move_category == Some(MoveCategory::Physical) {
        if let Some(owner) = damaged_holder(invocation, context, dispatch) {
            if pokemon(state, owner)?.current_hp > 0 {
                state.change_stat_stage(owner, BattleStat::Defense, -1)?;
                state.change_stat_stage(owner, BattleStat::Speed, 2)?;
            }
        }
    }
    Ok(())
}

fn justified(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    boost_damaged_holder(
        state,
        invocation,
        context,
        dispatch,
        BattleStat::Attack,
        1,
        context.move_type == Some(PokemonType::Dark),
    )
}

fn thermal_exchange(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    boost_damaged_holder(
        state,
        invocation,
        context,
        dispatch,
        BattleStat::Attack,
        1,
        context.move_type == Some(PokemonType::Fire),
    )
}

fn anger_point(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = damaged_holder(invocation, context, dispatch) else {
        return Ok(());
    };
    if context.critical && pokemon(state, owner)?.current_hp > 0 {
        state.change_stat_stage(owner, BattleStat::Attack, 12)?;
    }
    Ok(())
}

fn damaged_holder(
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &EventDispatch,
) -> Option<PokemonRef> {
    let owner = matching_target(invocation, context)?;
    (integer(dispatch).ok()? > 0).then_some(owner)
}

fn boost_damaged_holder(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &EventDispatch,
    stat: BattleStat,
    delta: i8,
    condition: bool,
) -> Result<(), BattleError> {
    if condition {
        if let Some(owner) = damaged_holder(invocation, context, dispatch) {
            if pokemon(state, owner)?.current_hp > 0 {
                state.change_stat_stage(owner, stat, delta)?;
            }
        }
    }
    Ok(())
}

fn friend_guard(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let protects_ally = owner_pokemon(invocation)
        .zip(context.target)
        .is_some_and(|(owner, target)| owner != target && owner.side == target.side);
    if protects_ally {
        modify_integer(dispatch, 3, 4)?;
    }
    Ok(())
}

fn mega_launcher(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    source_flag_boost(
        invocation,
        context,
        dispatch,
        context.move_flags.pulse(),
        3,
        2,
    )
}

#[allow(clippy::unnecessary_wraps)]
fn merciless(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if matching_source(invocation, context).is_some()
        && context.target.is_some_and(|target| {
            pokemon(state, target).is_ok_and(|target| {
                matches!(
                    target.status,
                    Some(MajorStatus::Poison | MajorStatus::BadlyPoisoned)
                )
            })
        })
    {
        dispatch.value = EventValue::Integer(5);
    }
    Ok(())
}

fn compound_eyes(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if matching_source(invocation, context).is_some() {
        modify_integer(dispatch, 5325, 4096)?;
    }
    Ok(())
}

fn hustle_attack(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if matching_source(invocation, context).is_some() {
        modify_integer(dispatch, 3, 2)?;
    }
    Ok(())
}

fn hustle_accuracy(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if context.move_category == Some(MoveCategory::Physical)
        && matching_source(invocation, context).is_some()
    {
        modify_integer(dispatch, 3277, 4096)?;
    }
    Ok(())
}

fn ice_body(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    if context.weather == Some(WeatherEnvironment::Snow) && pokemon(state, owner)?.current_hp > 0 {
        let healing = pokemon(state, owner)?.stats.hp / 16;
        state.heal_hp(owner, healing)?;
    }
    Ok(())
}

fn rain_dish(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    if context.weather == Some(WeatherEnvironment::Rain) && pokemon(state, owner)?.current_hp > 0 {
        let healing = pokemon(state, owner)?.stats.hp / 16;
        state.heal_hp(owner, healing)?;
    }
    Ok(())
}

fn moxie(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(owner) = matching_source(invocation, context) else {
        return Ok(());
    };
    if context.move_id.is_some()
        && context
            .target
            .is_some_and(|target| target.side != owner.side)
        && pokemon(state, owner)?.current_hp > 0
    {
        state.change_stat_stage(owner, BattleStat::Attack, 1)?;
    }
    Ok(())
}

fn gooey(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if context.move_flags.contact() && damaged_holder(invocation, context, dispatch).is_some() {
        if let Some(source) = context.source {
            if source != context.target.unwrap_or(source) && pokemon(state, source)?.current_hp > 0
            {
                state.change_stat_stage(source, BattleStat::Speed, -1)?;
            }
        }
    }
    Ok(())
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
            move_flags: crate::MoveInteractionFlags::default().with_bullet(true),
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

    #[test]
    fn compound_sand_rush_and_prankster_handlers_share_one_descriptor_each() {
        let sand_rush = AbilityId::new(44);
        let prankster = AbilityId::new(45);
        let registry = crate::conditions::composed_registry(
            &EffectRegistry::new(),
            [
                unbreakable(sand_rush, AbilityBehavior::SandRush),
                unbreakable(prankster, AbilityBehavior::Prankster),
            ],
        );

        let mut sand_battle = battle(sand_rush, AbilityId::new(1));
        let sand_speed = registry
            .dispatch(
                &mut sand_battle,
                BattleEvent::ModifySpeed,
                EventContext {
                    weather: Some(WeatherEnvironment::Sandstorm),
                    ..context()
                },
                EventValue::Integer(100),
            )
            .unwrap();
        assert_eq!(sand_speed.value, EventValue::Integer(200));
        let sand_immunity = registry
            .dispatch(
                &mut sand_battle,
                BattleEvent::WeatherImmunity,
                EventContext {
                    source: None,
                    target: context().source,
                    weather: Some(WeatherEnvironment::Sandstorm),
                    ..EventContext::default()
                },
                EventValue::Boolean(true),
            )
            .unwrap();
        assert_eq!(sand_immunity.value, EventValue::Boolean(false));

        let mut prankster_battle = battle(prankster, AbilityId::new(1));
        let priority = registry
            .dispatch(
                &mut prankster_battle,
                BattleEvent::ModifyPriority,
                EventContext {
                    move_category: Some(MoveCategory::Status),
                    ..context()
                },
                EventValue::Integer(0),
            )
            .unwrap();
        assert_eq!(priority.value, EventValue::Integer(1));
        assert!(priority.prankster_boosted);
        let damaging_priority = registry
            .dispatch(
                &mut prankster_battle,
                BattleEvent::ModifyPriority,
                EventContext {
                    move_category: Some(MoveCategory::Physical),
                    ..context()
                },
                EventValue::Integer(0),
            )
            .unwrap();
        assert_eq!(damaging_priority.value, EventValue::Integer(0));
        assert!(!damaging_priority.prankster_boosted);

        let dark_context = EventContext {
            prankster_boosted: true,
            target_types: Some(crate::PokemonTypes {
                primary: crate::PokemonType::Dark,
                secondary: None,
            }),
            ..context()
        };
        let dark_immunity = registry
            .dispatch(
                &mut prankster_battle,
                BattleEvent::TryImmunity,
                dark_context,
                EventValue::Boolean(true),
            )
            .unwrap();
        assert_eq!(dark_immunity.value, EventValue::Boolean(false));
        assert!(dark_immunity.immune);
        let ally_dark = registry
            .dispatch(
                &mut prankster_battle,
                BattleEvent::TryImmunity,
                EventContext {
                    target: dark_context.source,
                    ..dark_context
                },
                EventValue::Boolean(true),
            )
            .unwrap();
        assert_eq!(ally_dark.value, EventValue::Boolean(true));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn absorption_powder_and_burn_immunities_mutate_through_typed_effect_state() {
        for (behavior, move_type) in [
            (AbilityBehavior::WaterAbsorb, PokemonType::Water),
            (AbilityBehavior::VoltAbsorb, PokemonType::Electric),
            (AbilityBehavior::EarthEater, PokemonType::Ground),
        ] {
            let ability = AbilityId::new(60);
            let registry = crate::conditions::composed_registry(
                &EffectRegistry::new(),
                [breakable(ability, behavior)],
            );
            let mut state = battle(AbilityId::new(1), ability);
            state
                .side_mut(SideId::P2)
                .pokemon_mut(PokemonIndex::new(0))
                .unwrap()
                .current_hp = 40;
            let event = registry
                .dispatch(
                    &mut state,
                    BattleEvent::TryHit,
                    EventContext {
                        move_type: Some(move_type),
                        ..context()
                    },
                    EventValue::Boolean(true),
                )
                .unwrap();
            assert_eq!(event.value, EventValue::Boolean(false));
            assert!(event.immune);
            assert_eq!(
                state
                    .side(SideId::P2)
                    .pokemon(PokemonIndex::new(0))
                    .unwrap()
                    .current_hp,
                65
            );

            let mut suppressed = battle(AbilityId::new(1), ability);
            suppressed
                .side_mut(SideId::P2)
                .pokemon_mut(PokemonIndex::new(0))
                .unwrap()
                .current_hp = 40;
            let event = registry
                .dispatch(
                    &mut suppressed,
                    BattleEvent::TryHit,
                    EventContext {
                        move_type: Some(move_type),
                        suppress_target_ability: true,
                        ..context()
                    },
                    EventValue::Boolean(true),
                )
                .unwrap();
            assert_eq!(event.value, EventValue::Boolean(true));
            assert!(!event.immune);
            assert_eq!(
                suppressed
                    .side(SideId::P2)
                    .pokemon(PokemonIndex::new(0))
                    .unwrap()
                    .current_hp,
                40
            );
        }

        let motor_drive = AbilityId::new(61);
        let overcoat = AbilityId::new(62);
        let water_veil = AbilityId::new(63);
        let registry = crate::conditions::composed_registry(
            &EffectRegistry::new(),
            [
                breakable(motor_drive, AbilityBehavior::MotorDrive),
                breakable(overcoat, AbilityBehavior::PowderWeatherImmunity),
                breakable(water_veil, AbilityBehavior::BurnImmunity),
            ],
        );
        let mut drive = battle(AbilityId::new(1), motor_drive);
        let event = registry
            .dispatch(
                &mut drive,
                BattleEvent::TryHit,
                EventContext {
                    move_type: Some(PokemonType::Electric),
                    ..context()
                },
                EventValue::Boolean(true),
            )
            .unwrap();
        assert_eq!(event.value, EventValue::Boolean(false));
        assert_eq!(
            drive
                .side(SideId::P2)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .stat_stages
                .spe,
            1
        );

        let mut coat = battle(AbilityId::new(1), overcoat);
        let powder = registry
            .dispatch(
                &mut coat,
                BattleEvent::TryHit,
                EventContext {
                    move_flags: crate::MoveInteractionFlags::default().with_powder(true),
                    ..context()
                },
                EventValue::Boolean(true),
            )
            .unwrap();
        assert_eq!(powder.value, EventValue::Boolean(false));
        assert!(powder.immune);

        let mut veil = battle(AbilityId::new(1), water_veil);
        let target = context().target.unwrap();
        veil.side_mut(target.side)
            .pokemon_mut(target.pokemon)
            .unwrap()
            .status = Some(MajorStatus::Burn);
        registry
            .dispatch(
                &mut veil,
                BattleEvent::AfterSetStatus,
                EventContext {
                    source: context().target,
                    target: None,
                    ..EventContext::default()
                },
                EventValue::None,
            )
            .unwrap();
        assert_eq!(
            veil.side(target.side)
                .pokemon(target.pokemon)
                .unwrap()
                .status,
            None
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn grass_ground_weather_and_contact_defenses_are_typed_and_suppressible() {
        let sap_sipper = AbilityId::new(64);
        let levitate = AbilityId::new(65);
        let sand_veil = AbilityId::new(66);
        let snow_cloak = AbilityId::new(67);
        let aura_guard = AbilityId::new(68);
        let registry = crate::conditions::composed_registry(
            &EffectRegistry::new(),
            [
                breakable(sap_sipper, AbilityBehavior::GrassImmunityAttackBoost),
                breakable(levitate, AbilityBehavior::GroundImmunity),
                breakable(
                    sand_veil,
                    AbilityBehavior::WeatherEvasion(WeatherEnvironment::Sandstorm),
                ),
                breakable(
                    snow_cloak,
                    AbilityBehavior::WeatherEvasion(WeatherEnvironment::Snow),
                ),
                breakable(aura_guard, AbilityBehavior::ContactDamageReduction),
            ],
        );

        let mut sap = battle(AbilityId::new(1), sap_sipper);
        let grass = registry
            .dispatch(
                &mut sap,
                BattleEvent::TryHit,
                EventContext {
                    move_type: Some(PokemonType::Grass),
                    ..context()
                },
                EventValue::Boolean(true),
            )
            .unwrap();
        assert_eq!(grass.value, EventValue::Boolean(false));
        assert!(grass.immune);
        assert_eq!(
            sap.side(SideId::P2)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .stat_stages
                .atk,
            1
        );

        let mut floating = battle(AbilityId::new(1), levitate);
        let ground = registry
            .dispatch(
                &mut floating,
                BattleEvent::TryHit,
                EventContext {
                    move_type: Some(PokemonType::Ground),
                    target_grounded: Some(false),
                    ..context()
                },
                EventValue::Boolean(true),
            )
            .unwrap();
        assert_eq!(ground.value, EventValue::Boolean(false));
        assert!(ground.immune);
        let grounded = registry
            .dispatch(
                &mut floating,
                BattleEvent::TryHit,
                EventContext {
                    move_type: Some(PokemonType::Ground),
                    target_grounded: Some(true),
                    ..context()
                },
                EventValue::Boolean(true),
            )
            .unwrap();
        assert_eq!(grounded.value, EventValue::Boolean(true));

        for (ability, weather) in [
            (sand_veil, WeatherEnvironment::Sandstorm),
            (snow_cloak, WeatherEnvironment::Snow),
        ] {
            let mut state = battle(AbilityId::new(1), ability);
            let accuracy = registry
                .dispatch(
                    &mut state,
                    BattleEvent::ModifyAccuracy,
                    EventContext {
                        weather: Some(weather),
                        ..context()
                    },
                    EventValue::Integer(100),
                )
                .unwrap();
            assert_eq!(accuracy.value, EventValue::Integer(80));
        }

        let mut guarded = battle(AbilityId::new(1), aura_guard);
        let contact = registry
            .dispatch(
                &mut guarded,
                BattleEvent::ModifyDamage,
                EventContext {
                    move_flags: crate::MoveInteractionFlags::default().with_contact(true),
                    ..context()
                },
                EventValue::Integer(101),
            )
            .unwrap();
        assert_eq!(contact.value, EventValue::Integer(50));

        for (ability, event, event_context, value) in [
            (
                sap_sipper,
                BattleEvent::TryHit,
                EventContext {
                    move_type: Some(PokemonType::Grass),
                    suppress_target_ability: true,
                    ..context()
                },
                EventValue::Boolean(true),
            ),
            (
                levitate,
                BattleEvent::TryHit,
                EventContext {
                    move_type: Some(PokemonType::Ground),
                    target_grounded: Some(false),
                    suppress_target_ability: true,
                    ..context()
                },
                EventValue::Boolean(true),
            ),
            (
                aura_guard,
                BattleEvent::ModifyDamage,
                EventContext {
                    move_flags: crate::MoveInteractionFlags::default().with_contact(true),
                    suppress_target_ability: true,
                    ..context()
                },
                EventValue::Integer(101),
            ),
        ] {
            let mut state = battle(AbilityId::new(1), ability);
            let suppressed = registry
                .dispatch(&mut state, event, event_context, value)
                .unwrap();
            assert_eq!(suppressed.value, value);
            assert_eq!(suppressed.invocations.len(), 0);
            assert_eq!(
                state
                    .side(SideId::P2)
                    .pokemon(PokemonIndex::new(0))
                    .unwrap()
                    .stat_stages,
                StatStages::default()
            );
        }
    }

    #[test]
    fn offensive_defensive_and_accuracy_modifiers_compose_in_registry_order() {
        let water_bubble = AbilityId::new(70);
        let friend_guard = AbilityId::new(71);
        let mega_launcher = AbilityId::new(72);
        let merciless = AbilityId::new(73);
        let compound_eyes = AbilityId::new(74);
        let hustle = AbilityId::new(75);
        let registry = crate::conditions::composed_registry(
            &EffectRegistry::new(),
            [
                breakable(water_bubble, AbilityBehavior::WaterBubble),
                breakable(friend_guard, AbilityBehavior::FriendGuard),
                unbreakable(mega_launcher, AbilityBehavior::MegaLauncher),
                unbreakable(merciless, AbilityBehavior::Merciless),
                unbreakable(compound_eyes, AbilityBehavior::CompoundEyes),
                unbreakable(hustle, AbilityBehavior::Hustle),
            ],
        );

        let mut bubble_source = battle(water_bubble, AbilityId::new(1));
        let water = EventContext {
            move_type: Some(PokemonType::Water),
            ..context()
        };
        assert_eq!(
            registry
                .dispatch(
                    &mut bubble_source,
                    BattleEvent::ModifySpecialAttack,
                    water,
                    EventValue::Integer(100),
                )
                .unwrap()
                .value,
            EventValue::Integer(200)
        );
        let mut bubble_target = battle(AbilityId::new(1), water_bubble);
        assert_eq!(
            registry
                .dispatch(
                    &mut bubble_target,
                    BattleEvent::ModifyAttack,
                    EventContext {
                        move_type: Some(PokemonType::Fire),
                        ..context()
                    },
                    EventValue::Integer(100),
                )
                .unwrap()
                .value,
            EventValue::Integer(50)
        );

        let mut guarded = battle(AbilityId::new(1), friend_guard);
        assert_eq!(
            dispatch_integer(&registry, &mut guarded, BattleEvent::ModifyDamage, 100,),
            EventValue::Integer(75)
        );

        let mut launcher = battle(mega_launcher, AbilityId::new(1));
        let pulse = registry
            .dispatch(
                &mut launcher,
                BattleEvent::ModifyBasePower,
                EventContext {
                    move_flags: crate::MoveInteractionFlags::default().with_pulse(true),
                    ..context()
                },
                EventValue::Integer(100),
            )
            .unwrap();
        assert_eq!(pulse.value, EventValue::Integer(150));

        let mut poisoned = battle(merciless, AbilityId::new(1));
        poisoned
            .side_mut(SideId::P2)
            .pokemon_mut(PokemonIndex::new(0))
            .unwrap()
            .status = Some(MajorStatus::Poison);
        assert_eq!(
            dispatch_integer(&registry, &mut poisoned, BattleEvent::ModifyCritRatio, 0,),
            EventValue::Integer(5)
        );

        let mut eyes = battle(compound_eyes, AbilityId::new(1));
        assert_eq!(
            dispatch_integer(&registry, &mut eyes, BattleEvent::ModifyAccuracy, 100,),
            EventValue::Integer(130)
        );
        let mut hustler = battle(hustle, AbilityId::new(1));
        assert_eq!(
            dispatch_integer(&registry, &mut hustler, BattleEvent::ModifyAttack, 100),
            EventValue::Integer(150)
        );
        let hustle_accuracy = registry
            .dispatch(
                &mut hustler,
                BattleEvent::ModifyAccuracy,
                EventContext {
                    move_category: Some(MoveCategory::Physical),
                    ..context()
                },
                EventValue::Integer(100),
            )
            .unwrap();
        assert_eq!(hustle_accuracy.value, EventValue::Integer(80));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn damaging_hit_reactions_are_typed_survive_only_and_suppression_aware() {
        let cases = [
            (
                AbilityBehavior::Stamina,
                PokemonType::Normal,
                MoveCategory::Special,
                0,
                1,
                0,
            ),
            (
                AbilityBehavior::WeakArmor,
                PokemonType::Normal,
                MoveCategory::Physical,
                0,
                -1,
                2,
            ),
            (
                AbilityBehavior::Justified,
                PokemonType::Dark,
                MoveCategory::Physical,
                1,
                0,
                0,
            ),
            (
                AbilityBehavior::ThermalExchange,
                PokemonType::Fire,
                MoveCategory::Special,
                1,
                0,
                0,
            ),
        ];
        for (behavior, move_type, category, attack, defense, speed) in cases {
            let ability = AbilityId::new(80);
            let suppression = if behavior == AbilityBehavior::ThermalExchange {
                AbilitySuppression::Breakable
            } else {
                AbilitySuppression::Unbreakable
            };
            let registry = crate::conditions::composed_registry(
                &EffectRegistry::new(),
                [AbilityHandler::new(ability, behavior, suppression)],
            );
            let mut state = battle(AbilityId::new(1), ability);
            registry
                .dispatch(
                    &mut state,
                    BattleEvent::AfterDamage,
                    EventContext {
                        move_type: Some(move_type),
                        move_category: Some(category),
                        ..context()
                    },
                    EventValue::Integer(1),
                )
                .unwrap();
            let stages = state
                .side(SideId::P2)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .stat_stages;
            assert_eq!(
                (stages.atk, stages.def, stages.spe),
                (attack, defense, speed)
            );

            if behavior == AbilityBehavior::ThermalExchange {
                let mut suppressed = battle(AbilityId::new(1), ability);
                registry
                    .dispatch(
                        &mut suppressed,
                        BattleEvent::AfterDamage,
                        EventContext {
                            move_type: Some(move_type),
                            move_category: Some(category),
                            suppress_target_ability: true,
                            ..context()
                        },
                        EventValue::Integer(1),
                    )
                    .unwrap();
                assert_eq!(
                    suppressed
                        .side(SideId::P2)
                        .pokemon(PokemonIndex::new(0))
                        .unwrap()
                        .stat_stages,
                    StatStages::default()
                );
            }
        }

        let anger_point = AbilityId::new(81);
        let registry = crate::conditions::composed_registry(
            &EffectRegistry::new(),
            [unbreakable(anger_point, AbilityBehavior::AngerPoint)],
        );
        let mut state = battle(AbilityId::new(1), anger_point);
        registry
            .dispatch(
                &mut state,
                BattleEvent::AfterDamage,
                EventContext {
                    critical: true,
                    ..context()
                },
                EventValue::Integer(1),
            )
            .unwrap();
        assert_eq!(
            state
                .side(SideId::P2)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .stat_stages
                .atk,
            6
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn switch_and_residual_abilities_use_kernel_mutations_and_ability_rng() {
        let natural_cure = AbilityId::new(90);
        let regenerator = AbilityId::new(91);
        let hospitality = AbilityId::new(92);
        let hydration = AbilityId::new(93);
        let shed_skin = AbilityId::new(94);
        let healer = AbilityId::new(95);
        let registry = crate::conditions::composed_registry(
            &EffectRegistry::new(),
            [
                unbreakable(natural_cure, AbilityBehavior::NaturalCure),
                unbreakable(regenerator, AbilityBehavior::Regenerator),
                unbreakable(hospitality, AbilityBehavior::Hospitality),
                unbreakable(hydration, AbilityBehavior::Hydration),
                unbreakable(shed_skin, AbilityBehavior::ShedSkin),
                unbreakable(healer, AbilityBehavior::Healer),
            ],
        );

        let source = context().source.unwrap();
        let mut cure = battle(natural_cure, AbilityId::new(1));
        cure.side_mut(source.side)
            .pokemon_mut(source.pokemon)
            .unwrap()
            .status = Some(MajorStatus::BadlyPoisoned);
        registry
            .dispatch(
                &mut cure,
                BattleEvent::SwitchOut,
                context(),
                EventValue::None,
            )
            .unwrap();
        assert_eq!(
            cure.side(source.side)
                .pokemon(source.pokemon)
                .unwrap()
                .status,
            None
        );

        let mut regen = battle(regenerator, AbilityId::new(1));
        regen
            .side_mut(source.side)
            .pokemon_mut(source.pokemon)
            .unwrap()
            .current_hp = 40;
        registry
            .dispatch(
                &mut regen,
                BattleEvent::SwitchOut,
                context(),
                EventValue::None,
            )
            .unwrap();
        assert_eq!(
            regen
                .side(source.side)
                .pokemon(source.pokemon)
                .unwrap()
                .current_hp,
            73
        );

        let ally = PokemonRef::new(SideId::P1, PokemonIndex::new(1));
        let mut host = battle(hospitality, AbilityId::new(1));
        host.side_mut(ally.side)
            .pokemon_mut(ally.pokemon)
            .unwrap()
            .current_hp = 40;
        registry
            .dispatch(
                &mut host,
                BattleEvent::SwitchIn,
                EventContext {
                    source: Some(source),
                    ..EventContext::default()
                },
                EventValue::None,
            )
            .unwrap();
        assert_eq!(
            host.side(ally.side)
                .pokemon(ally.pokemon)
                .unwrap()
                .current_hp,
            65
        );

        let mut rain = battle(hydration, AbilityId::new(1));
        rain.side_mut(source.side)
            .pokemon_mut(source.pokemon)
            .unwrap()
            .status = Some(MajorStatus::Paralysis);
        registry
            .dispatch(
                &mut rain,
                BattleEvent::Residual,
                EventContext {
                    weather: Some(WeatherEnvironment::Rain),
                    ..EventContext::default()
                },
                EventValue::None,
            )
            .unwrap();
        assert_eq!(
            rain.side(source.side)
                .pokemon(source.pokemon)
                .unwrap()
                .status,
            None
        );

        let mut skin = battle(shed_skin, AbilityId::new(1));
        skin.enable_rng_trace();
        skin.side_mut(source.side)
            .pokemon_mut(source.pokemon)
            .unwrap()
            .status = Some(MajorStatus::Poison);
        let mut replay = skin.clone();
        registry
            .dispatch(
                &mut skin,
                BattleEvent::Residual,
                EventContext::default(),
                EventValue::None,
            )
            .unwrap();
        registry
            .dispatch(
                &mut replay,
                BattleEvent::Residual,
                EventContext::default(),
                EventValue::None,
            )
            .unwrap();
        assert_eq!(skin, replay);
        assert!(skin
            .rng()
            .trace()
            .unwrap()
            .iter()
            .any(|entry| entry.reason == RngReason::Ability));

        let mut healing = battle(healer, AbilityId::new(1));
        healing.enable_rng_trace();
        healing
            .side_mut(ally.side)
            .pokemon_mut(ally.pokemon)
            .unwrap()
            .status = Some(MajorStatus::Burn);
        registry
            .dispatch(
                &mut healing,
                BattleEvent::Residual,
                EventContext::default(),
                EventValue::None,
            )
            .unwrap();
        assert!(healing
            .rng()
            .trace()
            .unwrap()
            .iter()
            .any(|entry| entry.reason == RngReason::Ability));
    }

    #[test]
    fn weather_healing_moxie_and_gooey_use_shared_lifecycle_events() {
        let ice_body = AbilityId::new(96);
        let rain_dish = AbilityId::new(97);
        let moxie = AbilityId::new(98);
        let gooey = AbilityId::new(99);
        let registry = crate::conditions::composed_registry(
            &EffectRegistry::new(),
            [
                unbreakable(ice_body, AbilityBehavior::IceBody),
                unbreakable(rain_dish, AbilityBehavior::RainDish),
                unbreakable(moxie, AbilityBehavior::Moxie),
                unbreakable(gooey, AbilityBehavior::Gooey),
            ],
        );
        let source = context().source.unwrap();
        for (ability, weather) in [
            (ice_body, WeatherEnvironment::Snow),
            (rain_dish, WeatherEnvironment::Rain),
        ] {
            let mut state = battle(ability, AbilityId::new(1));
            state
                .side_mut(source.side)
                .pokemon_mut(source.pokemon)
                .unwrap()
                .current_hp = 40;
            registry
                .dispatch(
                    &mut state,
                    BattleEvent::Residual,
                    EventContext {
                        weather: Some(weather),
                        ..EventContext::default()
                    },
                    EventValue::None,
                )
                .unwrap();
            assert_eq!(
                state
                    .side(source.side)
                    .pokemon(source.pokemon)
                    .unwrap()
                    .current_hp,
                46
            );
        }

        let mut attacker = battle(moxie, AbilityId::new(1));
        registry
            .dispatch(
                &mut attacker,
                BattleEvent::Faint,
                context(),
                EventValue::None,
            )
            .unwrap();
        assert_eq!(
            attacker
                .side(source.side)
                .pokemon(source.pokemon)
                .unwrap()
                .stat_stages
                .atk,
            1
        );

        let mut sticky = battle(AbilityId::new(1), gooey);
        registry
            .dispatch(
                &mut sticky,
                BattleEvent::AfterDamage,
                EventContext {
                    move_flags: crate::MoveInteractionFlags::default().with_contact(true),
                    ..context()
                },
                EventValue::Integer(1),
            )
            .unwrap();
        assert_eq!(
            sticky
                .side(source.side)
                .pokemon(source.pokemon)
                .unwrap()
                .stat_stages
                .spe,
            -1
        );
    }

    #[test]
    fn informational_only_abilities_register_no_authoritative_callback() {
        let ability = AbilityId::new(100);
        let registry = crate::conditions::composed_registry(
            &EffectRegistry::new(),
            [unbreakable(ability, AbilityBehavior::NoAuthoritativeEffect)],
        );
        let mut state = battle(ability, AbilityId::new(1));
        let event = registry
            .dispatch(
                &mut state,
                BattleEvent::SwitchIn,
                EventContext {
                    source: context().source,
                    ..EventContext::default()
                },
                EventValue::None,
            )
            .unwrap();
        assert_eq!(event.invocations.len(), 0);
    }
}
