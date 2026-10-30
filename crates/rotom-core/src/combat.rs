use crate::{
    kernel::StateKernel, AbilityId, ActionTarget, ActiveSlot, BattleError, BattleEvent,
    BattlePhase, BattleStat, BattleState, ConditionId, ConditionState, EffectRegistry,
    EventContext, EventDispatch, EventValue, FaintTarget, FieldEffectIds, FormId, MajorStatus,
    MoveId, MoveSlotIndex, MoveTarget, PokemonAction, PokemonIndex, PokemonRef, PokemonState,
    QueuedAction, RngReason, SideId, SpeciesId, TerrainEnvironment, WeatherEnvironment,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PokemonType {
    Bug,
    Dark,
    Dragon,
    Electric,
    Fairy,
    Fighting,
    Fire,
    Flying,
    Ghost,
    Grass,
    Ground,
    Ice,
    Normal,
    Poison,
    Psychic,
    Rock,
    Steel,
    Stellar,
    Water,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveCategory {
    Physical,
    Special,
    Status,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MoveAccuracy {
    Always,
    Percent { value: u8 },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveOperation {
    /// The shared level/power/Attack-or-Sp. Atk damage formula.
    StandardDamage,
    /// A numeric fixed-damage operation. Execution is deferred past PR 13.1.
    FixedDamage,
    /// A level-based damage operation. Execution is deferred past PR 13.1.
    LevelDamage,
    /// A move whose primary behavior does not use the standard damage formula.
    NonDamage,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HitSequence {
    #[default]
    Single,
    Fixed {
        hits: u8,
    },
    Range {
        min: u8,
        max: u8,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EffectFraction {
    pub numerator: u16,
    pub denominator: u16,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorMoveEffect {
    #[default]
    None,
    Facade,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecondaryMoveEffect {
    pub chance: u8,
    pub effects: Vec<PrimaryMoveEffect>,
    pub self_effects: Vec<PrimaryMoveEffect>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelfMoveEffect {
    pub chance: Option<u8>,
    pub effects: Vec<PrimaryMoveEffect>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PrimaryMoveEffect {
    InflictStatus {
        status: MajorStatus,
    },
    CureStatus,
    ChangeStatStage {
        stat: BattleStat,
        delta: i8,
    },
    Heal {
        numerator: u16,
        denominator: u16,
    },
    AddVolatile {
        condition: ConditionId,
        duration: Option<u8>,
    },
    SetWeather {
        condition: ConditionId,
        duration: u8,
    },
    SetTerrain {
        condition: ConditionId,
        duration: u8,
    },
    AddFieldCondition {
        condition: ConditionId,
        duration: u8,
    },
    ToggleFieldCondition {
        condition: ConditionId,
        duration: u8,
    },
    AddSideCondition {
        condition: ConditionId,
        duration: u8,
    },
}

/// A move-execution facility required by a resolved move definition.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[repr(u32)]
#[serde(rename_all = "snake_case")]
pub enum MoveCapability {
    StandardDamage = 1 << 0,
    SelectedSingleTarget = 1 << 1,
    Accuracy = 1 << 2,
    FixedDamage = 1 << 3,
    NonDamagePrimary = 1 << 4,
    AdvancedTargeting = 1 << 5,
    MultiHit = 1 << 6,
    SecondaryEffect = 1 << 7,
    SelfEffect = 1 << 8,
    Drain = 1 << 9,
    Recoil = 1 << 10,
    MajorStatus = 1 << 11,
    Volatile = 1 << 12,
    StatStage = 1 << 13,
    Healing = 1 << 14,
    FieldOrSideCondition = 1 << 15,
    SwitchControl = 1 << 16,
    CallsMove = 1 << 17,
    CustomCallback = 1 << 18,
    SpecialBehavior = 1 << 19,
    TargetSet = 1 << 20,
    SpreadDamage = 1 << 21,
    Protection = 1 << 22,
    Redirection = 1 << 23,
    Retargeting = 1 << 24,
    PerTargetHit = 1 << 25,
    ConditionalPower = 1 << 26,
    EffectiveStatOverride = 1 << 27,
}

impl MoveCapability {
    const ALL: [Self; 28] = [
        Self::StandardDamage,
        Self::SelectedSingleTarget,
        Self::Accuracy,
        Self::FixedDamage,
        Self::NonDamagePrimary,
        Self::AdvancedTargeting,
        Self::MultiHit,
        Self::SecondaryEffect,
        Self::SelfEffect,
        Self::Drain,
        Self::Recoil,
        Self::MajorStatus,
        Self::Volatile,
        Self::StatStage,
        Self::Healing,
        Self::FieldOrSideCondition,
        Self::SwitchControl,
        Self::CallsMove,
        Self::CustomCallback,
        Self::SpecialBehavior,
        Self::TargetSet,
        Self::SpreadDamage,
        Self::Protection,
        Self::Redirection,
        Self::Retargeting,
        Self::PerTargetHit,
        Self::ConditionalPower,
        Self::EffectiveStatOverride,
    ];
}

impl fmt::Display for MoveCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::StandardDamage => "standard_damage",
            Self::SelectedSingleTarget => "selected_single_target",
            Self::Accuracy => "accuracy",
            Self::FixedDamage => "fixed_damage",
            Self::NonDamagePrimary => "non_damage_primary",
            Self::AdvancedTargeting => "advanced_targeting",
            Self::MultiHit => "multi_hit",
            Self::SecondaryEffect => "secondary_effect",
            Self::SelfEffect => "self_effect",
            Self::Drain => "drain",
            Self::Recoil => "recoil",
            Self::MajorStatus => "major_status",
            Self::Volatile => "volatile",
            Self::StatStage => "stat_stage",
            Self::Healing => "healing",
            Self::FieldOrSideCondition => "field_or_side_condition",
            Self::SwitchControl => "switch_control",
            Self::CallsMove => "calls_move",
            Self::CustomCallback => "custom_callback",
            Self::SpecialBehavior => "special_behavior",
            Self::TargetSet => "target_set",
            Self::SpreadDamage => "spread_damage",
            Self::Protection => "protection",
            Self::Redirection => "redirection",
            Self::Retargeting => "retargeting",
            Self::PerTargetHit => "per_target_hit",
            Self::ConditionalPower => "conditional_power",
            Self::EffectiveStatOverride => "effective_stat_override",
        };
        formatter.write_str(name)
    }
}

/// Compact set of required move-executor capabilities.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MoveCapabilities(u32);

impl MoveCapabilities {
    pub const NONE: Self = Self(0);
    pub const PR_13_1: Self = Self(
        MoveCapability::StandardDamage as u32
            | MoveCapability::SelectedSingleTarget as u32
            | MoveCapability::Accuracy as u32,
    );
    pub const PR_13_2: Self = Self(
        Self::PR_13_1.0
            | MoveCapability::TargetSet as u32
            | MoveCapability::SpreadDamage as u32
            | MoveCapability::Protection as u32
            | MoveCapability::Redirection as u32
            | MoveCapability::Retargeting as u32
            | MoveCapability::PerTargetHit as u32
            | MoveCapability::AdvancedTargeting as u32
            | MoveCapability::NonDamagePrimary as u32
            | MoveCapability::Volatile as u32,
    );
    pub const PR_13_3: Self = Self(
        Self::PR_13_2.0
            | MoveCapability::MajorStatus as u32
            | MoveCapability::StatStage as u32
            | MoveCapability::Healing as u32
            | MoveCapability::FieldOrSideCondition as u32,
    );
    pub const PR_13_4: Self = Self(
        Self::PR_13_3.0
            | MoveCapability::FixedDamage as u32
            | MoveCapability::MultiHit as u32
            | MoveCapability::SecondaryEffect as u32
            | MoveCapability::SelfEffect as u32
            | MoveCapability::Drain as u32
            | MoveCapability::Recoil as u32
            | MoveCapability::ConditionalPower as u32
            | MoveCapability::EffectiveStatOverride as u32,
    );

    #[must_use]
    pub const fn with(self, capability: MoveCapability) -> Self {
        Self(self.0 | capability as u32)
    }

    #[must_use]
    pub const fn contains(self, capability: MoveCapability) -> bool {
        self.0 & capability as u32 != 0
    }

    #[must_use]
    pub fn first_missing_from(self, supported: Self) -> Option<MoveCapability> {
        MoveCapability::ALL
            .into_iter()
            .find(|capability| self.contains(*capability) && !supported.contains(*capability))
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MoveInteractionFlags(u16);

impl MoveInteractionFlags {
    const BULLET: u16 = 1 << 0;
    const SOUND: u16 = 1 << 1;
    const CONTACT: u16 = 1 << 2;
    const BITE: u16 = 1 << 3;
    const PUNCH: u16 = 1 << 4;
    const SLICING: u16 = 1 << 5;
    const RECOIL: u16 = 1 << 6;
    const PULSE: u16 = 1 << 7;
    const POWDER: u16 = 1 << 8;
    const EXPLOSION: u16 = 1 << 9;

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub const fn with_bullet(self, enabled: bool) -> Self {
        self.with(Self::BULLET, enabled)
    }

    #[must_use]
    pub const fn with_sound(self, enabled: bool) -> Self {
        self.with(Self::SOUND, enabled)
    }

    #[must_use]
    pub const fn with_contact(self, enabled: bool) -> Self {
        self.with(Self::CONTACT, enabled)
    }

    #[must_use]
    pub const fn with_bite(self, enabled: bool) -> Self {
        self.with(Self::BITE, enabled)
    }

    #[must_use]
    pub const fn with_punch(self, enabled: bool) -> Self {
        self.with(Self::PUNCH, enabled)
    }

    #[must_use]
    pub const fn with_slicing(self, enabled: bool) -> Self {
        self.with(Self::SLICING, enabled)
    }

    #[must_use]
    pub const fn with_recoil(self, enabled: bool) -> Self {
        self.with(Self::RECOIL, enabled)
    }

    #[must_use]
    pub const fn with_pulse(self, enabled: bool) -> Self {
        self.with(Self::PULSE, enabled)
    }

    #[must_use]
    pub const fn with_powder(self, enabled: bool) -> Self {
        self.with(Self::POWDER, enabled)
    }

    #[must_use]
    pub const fn with_explosion(self, enabled: bool) -> Self {
        self.with(Self::EXPLOSION, enabled)
    }

    #[must_use]
    pub const fn bullet(self) -> bool {
        self.contains(Self::BULLET)
    }

    #[must_use]
    pub const fn sound(self) -> bool {
        self.contains(Self::SOUND)
    }

    #[must_use]
    pub const fn contact(self) -> bool {
        self.contains(Self::CONTACT)
    }

    #[must_use]
    pub const fn bite(self) -> bool {
        self.contains(Self::BITE)
    }

    #[must_use]
    pub const fn punch(self) -> bool {
        self.contains(Self::PUNCH)
    }

    #[must_use]
    pub const fn slicing(self) -> bool {
        self.contains(Self::SLICING)
    }

    #[must_use]
    pub const fn recoil(self) -> bool {
        self.contains(Self::RECOIL)
    }

    #[must_use]
    pub const fn pulse(self) -> bool {
        self.contains(Self::PULSE)
    }

    #[must_use]
    pub const fn powder(self) -> bool {
        self.contains(Self::POWDER)
    }

    #[must_use]
    pub const fn explosion(self) -> bool {
        self.contains(Self::EXPLOSION)
    }

    const fn with(self, flag: u16, enabled: bool) -> Self {
        if enabled {
            Self(self.0 | flag)
        } else {
            Self(self.0 & !flag)
        }
    }

    const fn contains(self, flag: u16) -> bool {
        self.0 & flag != 0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatMove {
    pub move_type: PokemonType,
    pub category: MoveCategory,
    pub base_power: u16,
    pub accuracy: MoveAccuracy,
    pub crit_ratio: u8,
    pub target: MoveTarget,
    pub operation: MoveOperation,
    pub required_capabilities: MoveCapabilities,
    pub hit_sequence: HitSequence,
    pub multi_accuracy: bool,
    pub fixed_damage: Option<u16>,
    pub drain: Option<EffectFraction>,
    pub recoil: Option<EffectFraction>,
    pub self_damage: Option<EffectFraction>,
    pub prior_effect: PriorMoveEffect,
    pub offensive_stat: Option<BattleStat>,
    pub defensive_stat: Option<BattleStat>,
    pub ignore_negative_offensive: bool,
    pub ignore_positive_defensive: bool,
    pub interaction_flags: MoveInteractionFlags,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PokemonTypes {
    pub primary: PokemonType,
    pub secondary: Option<PokemonType>,
}

/// Static battle data consumed by mechanics without coupling `rotom-core` to a Dex implementation.
pub trait CombatData {
    fn move_data(&self, move_id: MoveId) -> Option<CombatMove>;
    fn pokemon_types(&self, species: SpeciesId, form: Option<FormId>) -> Option<PokemonTypes>;
    /// Returns Showdown's type modifier: `1` weak, `-1` resist, and `None` immune.
    fn type_modifier(&self, attacking: PokemonType, defending: PokemonType) -> Option<i8>;

    /// Narrow intrinsic behavior implemented by the unified move executor.
    fn move_intrinsic(&self, _move_id: MoveId) -> MoveIntrinsic {
        MoveIntrinsic::Ordinary
    }

    fn primary_move_effects(&self, _move_id: MoveId) -> Vec<PrimaryMoveEffect> {
        Vec::new()
    }

    fn secondary_move_effects(&self, _move_id: MoveId) -> Vec<SecondaryMoveEffect> {
        Vec::new()
    }

    fn self_move_effect(&self, _move_id: MoveId) -> Option<SelfMoveEffect> {
        None
    }

    /// Whether the move is stopped by Protect-like guards.
    fn move_is_protectable(&self, _move_id: MoveId) -> bool {
        false
    }

    /// Whether the move removes and bypasses Protect-like guards.
    fn move_breaks_protect(&self, _move_id: MoveId) -> bool {
        false
    }

    /// Whether this move is Struggle, whose recoil bypasses Rock Head.
    fn move_is_struggle(&self, _move_id: MoveId) -> bool {
        false
    }

    /// Resolves format-specific identities for built-in field mechanics.
    /// Partial data providers may leave the default empty mapping.
    fn field_effect_ids(&self) -> crate::FieldEffectIds {
        crate::FieldEffectIds::default()
    }

    /// Resolves the native behavior family for one stable ability identity.
    fn ability_has_behavior(&self, _ability: AbilityId, _behavior: crate::AbilityBehavior) -> bool {
        false
    }

    /// Enumerates stable ability identities with implemented native handlers.
    ///
    /// This is consumed once while building immutable mechanics, outside the
    /// event-dispatch hot path.
    fn ability_handlers(&self) -> Vec<crate::AbilityHandler> {
        Vec::new()
    }

    /// Enumerates stable item identities with implemented native handlers.
    ///
    /// This is consumed once while building immutable mechanics, outside the
    /// event-dispatch hot path.
    fn item_handlers(&self) -> Vec<crate::ItemHandler> {
        Vec::new()
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveIntrinsic {
    #[default]
    Ordinary,
    Protect,
    FollowMe,
    RagePowder,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Modifier {
    pub numerator: u16,
    pub denominator: u16,
}

impl Modifier {
    pub const NEUTRAL: Self = Self {
        numerator: 1,
        denominator: 1,
    };
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DamageResult {
    pub base_damage: u32,
    pub attack_stat: u32,
    pub defense_stat: u32,
    pub stab_modifier: Modifier,
    /// Combined dual-type modifier. A zero numerator denotes immunity.
    pub type_modifier: Modifier,
    pub critical_modifier: Modifier,
    pub random_modifier: Modifier,
    pub final_damage: u16,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResolvedAction {
    Used {
        side: SideId,
        slot: ActiveSlot,
        move_slot: MoveSlotIndex,
    },
    Switched {
        side: SideId,
        slot: ActiveSlot,
        incoming: PokemonIndex,
    },
    Missed {
        side: SideId,
        slot: ActiveSlot,
        move_slot: MoveSlotIndex,
    },
    Damaged {
        side: SideId,
        slot: ActiveSlot,
        target_side: SideId,
        target_slot: ActiveSlot,
        move_slot: MoveSlotIndex,
        damage: DamageResult,
    },
    Skipped {
        side: SideId,
        slot: ActiveSlot,
    },
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TurnResolution {
    pub actions: Vec<ResolvedAction>,
    /// HP loss caused by common major statuses during residual processing.
    #[serde(default)]
    pub residual_damage: Vec<ResidualDamage>,
    /// Timed volatile conditions removed before residual callbacks.
    #[serde(default)]
    pub expired_volatiles: Vec<ExpiredVolatile>,
    /// Timed weather, terrain, field, and side conditions removed at upkeep.
    #[serde(default)]
    pub expired_conditions: Vec<ExpiredCondition>,
    /// HP changes produced by weather and terrain residual effects.
    #[serde(default)]
    pub field_hp_changes: Vec<FieldHpChange>,
    /// Ordered event dispatches emitted while resolving this turn.
    #[serde(default)]
    pub events: Vec<EventDispatch>,
    /// Ordered per-target outcomes from the unified move executor.
    #[serde(default)]
    pub target_outcomes: Vec<TargetOutcome>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetOutcome {
    pub source: PokemonRef,
    pub move_id: MoveId,
    pub target: PokemonRef,
    pub redirected: bool,
    pub result: TargetResult,
    pub calculated_damage: u16,
    pub applied_hp_loss: u16,
    pub fainted: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetResult {
    Hit,
    Missed,
    Protected,
    PriorityBlocked,
    Immune,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResidualDamage {
    pub pokemon: PokemonRef,
    pub status: MajorStatus,
    pub damage: u16,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpiredVolatile {
    pub pokemon: PokemonRef,
    pub condition: crate::ConditionId,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "location", rename_all = "snake_case")]
pub enum ExpiredCondition {
    Weather {
        condition: ConditionId,
    },
    Terrain {
        condition: ConditionId,
    },
    Field {
        condition: ConditionId,
    },
    Side {
        side: SideId,
        condition: ConditionId,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldHpChange {
    pub pokemon: PokemonRef,
    pub condition: ConditionId,
    /// Positive values heal and negative values damage.
    pub amount: i32,
}

#[derive(Clone, Copy)]
struct QueuedMoveAction {
    side: SideId,
    slot: ActiveSlot,
    move_slot: MoveSlotIndex,
    target: ActionTarget,
    prankster_boosted: bool,
}

#[derive(Clone)]
struct MoveExecutionContext {
    queued: QueuedMoveAction,
    source: PokemonRef,
    original_target: Option<TargetExecutionContext>,
    targets: Vec<TargetExecutionContext>,
    move_id: MoveId,
    move_data: CombatMove,
    source_species: SpeciesId,
    source_form: Option<FormId>,
    primary_effects: Vec<PrimaryMoveEffect>,
    secondary_effects: Vec<SecondaryMoveEffect>,
    self_effect: Option<SelfMoveEffect>,
    suppress_target_ability: bool,
    move_type_converted: bool,
    sheer_force_boosted: bool,
    later_actors: Vec<PokemonRef>,
}

#[derive(Clone, Copy)]
struct TargetExecutionContext {
    target: PokemonRef,
    slot: ActiveSlot,
    redirected: bool,
}

struct HitLifecycle {
    eligible: Vec<(usize, TargetExecutionContext)>,
    outcomes: Vec<Option<TargetOutcome>>,
}

#[derive(Clone, Copy)]
struct DamageApplication {
    context: EventContext,
    side: SideId,
    slot: ActiveSlot,
    pokemon: PokemonIndex,
    damage: u16,
}

#[derive(Clone, Copy, Default)]
struct DamageEnvironment {
    weather: Option<(u32, u32)>,
    defense_modifier: Option<(u32, u32)>,
    attack_override: Option<u32>,
    defense_override: Option<u32>,
    spread: bool,
    ignore_burn: bool,
    adaptability: bool,
}

impl BattleState {
    pub(crate) fn resolve_turn_inner<D: CombatData>(
        &mut self,
        data: &D,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
    ) -> Result<TurnResolution, BattleError> {
        let queued = self.queue.actions.clone();
        let mut resolution = TurnResolution::default();
        resolution.events.push(effects.dispatch(
            self,
            BattleEvent::BeforeTurn,
            EventContext::default(),
            EventValue::None,
        )?);
        for (action_index, action) in queued.iter().copied().enumerate() {
            if self.phase == BattlePhase::BattleEnd {
                break;
            }
            if self.side(action.side).active[action.slot.index()] != Some(action.pokemon) {
                resolution.actions.push(ResolvedAction::Skipped {
                    side: action.side,
                    slot: action.slot,
                });
                continue;
            }
            match action.choice {
                PokemonAction::Switch { pokemon } => {
                    let switched = self.resolve_queued_switch(
                        data,
                        effects,
                        field_effects,
                        action,
                        pokemon,
                        &mut resolution,
                    )?;
                    resolution.actions.push(if switched {
                        ResolvedAction::Switched {
                            side: action.side,
                            slot: action.slot,
                            incoming: pokemon,
                        }
                    } else {
                        ResolvedAction::Skipped {
                            side: action.side,
                            slot: action.slot,
                        }
                    });
                }
                PokemonAction::Move {
                    move_slot,
                    target,
                    mega: false,
                } => {
                    let later_actors = queued[action_index + 1..]
                        .iter()
                        .map(|later| PokemonRef::new(later.side, later.pokemon))
                        .collect::<Vec<_>>();
                    self.execute_move(
                        data,
                        QueuedMoveAction {
                            side: action.side,
                            slot: action.slot,
                            move_slot,
                            target,
                            prankster_boosted: action.prankster_boosted,
                        },
                        later_actors,
                        effects,
                        field_effects,
                        &mut resolution,
                    )?;
                }
                PokemonAction::Move { mega: true, .. } => {
                    return Err(BattleError::UnsupportedMechanic(
                        "Mega Evolution resolution is deferred".to_owned(),
                    ));
                }
                PokemonAction::Struggle => {
                    return Err(BattleError::UnsupportedMechanic(
                        "Struggle resolution is deferred".to_owned(),
                    ));
                }
                PokemonAction::Pass => {}
            }
        }

        if self.phase != BattlePhase::BattleEnd {
            self.finish_residual_phase(data, effects, field_effects, &mut resolution)?;
        }
        self.queue.actions.clear();
        self.queue.events.clear();
        if self.phase != BattlePhase::BattleEnd {
            self.turn = self.turn.checked_add(1).ok_or_else(|| {
                BattleError::InvalidState("battle turn counter overflowed".to_owned())
            })?;
            if self.phase == BattlePhase::ResolvingTurn {
                self.phase = BattlePhase::TurnSelection;
            }
        }
        Ok(resolution)
    }

    fn finish_residual_phase<D: CombatData>(
        &mut self,
        data: &D,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let residual_weather = field_effects.effective_weather_environment(self, data);
        let residual_terrain = field_effects.terrain_environment(self);
        self.tick_volatile_durations(resolution)?;
        self.process_weather_residuals(data, effects, field_effects, resolution)?;
        self.tick_weather_duration(resolution);
        self.process_terrain_residuals(data, field_effects, resolution)?;
        let before = self.active_status_hp()?;
        let residual = effects.dispatch(
            self,
            BattleEvent::Residual,
            EventContext {
                weather: residual_weather,
                terrain: residual_terrain,
                ..EventContext::default()
            },
            EventValue::None,
        )?;
        self.record_residual_damage(&before, &residual, resolution)?;
        let residual_invocations = residual.invocations.clone();
        resolution.events.push(residual);
        self.process_residual_faints(effects, &residual_invocations, resolution)?;
        if self.phase != BattlePhase::BattleEnd {
            self.tick_side_durations(resolution);
            self.tick_late_field_durations(field_effects, resolution);
        }
        Ok(())
    }

    /// Executes every supported move action through the shared move lifecycle.
    fn execute_move<D: CombatData>(
        &mut self,
        data: &D,
        queued: QueuedMoveAction,
        later_actors: Vec<PokemonRef>,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let move_context =
            self.prepare_move_execution(data, queued, later_actors, field_effects)?;
        let source_index = move_context.source.pokemon;
        let context = EventContext {
            source: Some(move_context.source),
            target: move_context.original_target.map(|target| target.target),
            move_id: Some(move_context.move_id),
            suppress_target_ability: move_context.suppress_target_ability,
            critical: false,
            ..EventContext::default()
        };
        let before_move =
            effects.dispatch(self, BattleEvent::BeforeMove, context, EventValue::None)?;
        let cancelled = before_move.cancelled;
        resolution.events.push(before_move);
        if cancelled {
            resolution.actions.push(ResolvedAction::Skipped {
                side: queued.side,
                slot: queued.slot,
            });
            return Ok(());
        }

        let pressure = move_context
            .targets
            .iter()
            .filter(|target| {
                target.target.side != move_context.source.side
                    && self
                        .side(target.target.side)
                        .pokemon(target.target.pokemon)
                        .is_some_and(|pokemon| {
                            data.ability_has_behavior(
                                pokemon.ability,
                                crate::AbilityBehavior::Pressure,
                            )
                        })
            })
            .count();
        let pp_cost = u8::try_from(1_usize.saturating_add(pressure)).unwrap_or(u8::MAX);
        let remaining_pp = self
            .side(queued.side)
            .pokemon(source_index)
            .and_then(|source| source.moves.get(usize::from(queued.move_slot.get())))
            .ok_or_else(|| BattleError::InvalidState("move slot became invalid".to_owned()))?
            .pp
            .saturating_sub(pp_cost);
        StateKernel::new(self).set_pp(move_context.source, queued.move_slot, remaining_pp)?;

        self.execute_primary_operation(data, &move_context, effects, field_effects, resolution)
    }

    fn prepare_move_execution<D: CombatData>(
        &self,
        data: &D,
        queued: QueuedMoveAction,
        later_actors: Vec<PokemonRef>,
        field_effects: &FieldEffectIds,
    ) -> Result<MoveExecutionContext, BattleError> {
        let source_index = self.side(queued.side).active[queued.slot.index()].ok_or_else(|| {
            BattleError::InvalidState("queued move source is no longer active".to_owned())
        })?;
        let source = self
            .side(queued.side)
            .pokemon(source_index)
            .ok_or_else(|| {
                BattleError::InvalidState("queued move source is out of bounds".to_owned())
            })?;
        let move_state = source
            .moves
            .get(usize::from(queued.move_slot.get()))
            .ok_or_else(|| BattleError::InvalidState("move slot is out of bounds".to_owned()))?;
        if move_state.pp == 0 {
            return Err(BattleError::InvalidState(
                "queued move has no remaining PP".to_owned(),
            ));
        }
        let move_id = move_state.move_id;
        let mut move_data = data.move_data(move_id).ok_or_else(|| {
            BattleError::DataMismatch(format!("missing combat data for move {move_id}"))
        })?;
        let source_ability = source.ability;
        let move_type_converted = apply_type_conversion(data, source_ability, &mut move_data);
        if data.ability_has_behavior(source_ability, crate::AbilityBehavior::LiquidVoice)
            && move_data.interaction_flags.sound()
        {
            move_data.move_type = PokemonType::Water;
        }
        if data.ability_has_behavior(source.ability, crate::AbilityBehavior::LongReach) {
            move_data.interaction_flags = move_data.interaction_flags.with_contact(false);
        }
        if data.ability_has_behavior(source.ability, crate::AbilityBehavior::SkillLink) {
            if let HitSequence::Range { max, .. } = move_data.hit_sequence {
                move_data.hit_sequence = HitSequence::Fixed { hits: max };
                move_data.multi_accuracy = false;
            }
        }
        let intrinsic = data.move_intrinsic(move_id);
        let primary_effects = data.primary_move_effects(move_id);
        let mut secondary_effects = data.secondary_move_effects(move_id);
        let mut self_effect = data.self_move_effect(move_id);
        let sheer_force_boosted = data
            .ability_has_behavior(source_ability, crate::AbilityBehavior::SheerForce)
            && !secondary_effects.is_empty();
        if sheer_force_boosted {
            secondary_effects.clear();
            self_effect = None;
        }
        preflight_move(
            move_id,
            move_data,
            intrinsic,
            &primary_effects,
            &secondary_effects,
            self_effect.as_ref(),
        )?;
        let original_target = self.resolve_original_target(queued, move_data.target)?;
        let targets = self.resolve_target_set(
            data,
            field_effects,
            queued,
            move_data.target,
            original_target,
        )?;
        Ok(MoveExecutionContext {
            queued,
            source: PokemonRef::new(queued.side, source_index),
            original_target,
            targets,
            move_id,
            move_data,
            source_species: source.species,
            source_form: source.form,
            primary_effects,
            secondary_effects,
            self_effect,
            suppress_target_ability: data.ability_has_behavior(
                source.ability,
                crate::AbilityBehavior::BreaksTargetAbilities,
            ),
            move_type_converted,
            sheer_force_boosted,
            later_actors,
        })
    }

    fn execute_primary_operation<D: CombatData>(
        &mut self,
        data: &D,
        move_context: &MoveExecutionContext,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        match move_context.move_data.operation {
            MoveOperation::StandardDamage => {
                self.execute_standard_damage(data, move_context, effects, field_effects, resolution)
            }
            MoveOperation::NonDamage => {
                self.execute_non_damage(data, move_context, effects, field_effects, resolution)
            }
            MoveOperation::FixedDamage | MoveOperation::LevelDamage => {
                self.execute_fixed_damage(data, move_context, effects, field_effects, resolution)
            }
        }
    }

    // Showdown advances the complete target set through each hit stage before
    // entering the next one. Keep original indexes so diagnostics remain in
    // resolved-target order even when targets fail at different stages.
    #[allow(clippy::too_many_lines)]
    fn execute_standard_damage<D: CombatData>(
        &mut self,
        data: &D,
        move_context: &MoveExecutionContext,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let spread = move_context.targets.len() > 1
            && matches!(
                move_context.move_data.target,
                MoveTarget::AllAdjacent | MoveTarget::AllAdjacentFoes
            );
        let HitLifecycle {
            eligible,
            mut outcomes,
        } = self.resolve_hit_lifecycle(
            data,
            move_context,
            effects,
            field_effects,
            true,
            resolution,
        )?;

        let hit_count = self.resolve_hit_count(move_context.move_data.hit_sequence)?;
        let mut any_hit = false;
        let mut total_applied_damage = 0_u32;
        let mut hit_outcomes = Vec::new();
        let mut self_effect_applied = false;
        for (index, target_context) in eligible {
            let target = target_context.target;
            let target_ability = self
                .side(target.side)
                .pokemon(target.pokemon)
                .ok_or_else(|| BattleError::InvalidState("move target became invalid".to_owned()))?
                .ability;
            let target_berry_blocked = self
                .side(target.side.opponent())
                .active
                .into_iter()
                .flatten()
                .any(|pokemon| {
                    self.side(target.side.opponent())
                        .pokemon(pokemon)
                        .is_some_and(|state| {
                            data.ability_has_behavior(
                                state.ability,
                                crate::AbilityBehavior::BerryPrevention,
                            )
                        })
                });
            let target_berry_interaction_unsupported = [
                crate::AbilityBehavior::BerryHeal,
                crate::AbilityBehavior::BerryReuse,
                crate::AbilityBehavior::BerryRestore,
            ]
            .into_iter()
            .any(|behavior| data.ability_has_behavior(target_ability, behavior))
                || self
                    .side(target.side)
                    .active
                    .into_iter()
                    .flatten()
                    .filter(|pokemon| *pokemon != target.pokemon)
                    .any(|pokemon| {
                        self.side(target.side)
                            .pokemon(pokemon)
                            .is_some_and(|state| {
                                data.ability_has_behavior(
                                    state.ability,
                                    crate::AbilityBehavior::BerrySymbiosis,
                                )
                            })
                    });
            let mut context = EventContext {
                source: Some(move_context.source),
                target: Some(target),
                move_id: Some(move_context.move_id),
                suppress_target_ability: move_context.suppress_target_ability,
                suppress_source_item: !field_effects.item_is_active_for(
                    self,
                    data,
                    move_context.source,
                ),
                suppress_target_item: !field_effects.item_is_active_for(self, data, target),
                target_berry_blocked,
                target_ripen: data
                    .ability_has_behavior(target_ability, crate::AbilityBehavior::Ripen),
                target_berry_interaction_unsupported,
                critical: false,
                move_category: Some(move_context.move_data.category),
                move_type: Some(move_context.move_data.move_type),
                move_flags: move_context.move_data.interaction_flags,
                move_type_converted: move_context.move_type_converted,
                sheer_force_boosted: move_context.sheer_force_boosted,
                type_effectiveness: self.target_type_effectiveness(
                    data,
                    move_context.move_data,
                    move_context.source,
                    target,
                )?,
                weather: field_effects.effective_weather_environment(self, data),
                terrain: field_effects.terrain_environment(self),
                ..EventContext::default()
            };
            let mut target_hit = false;
            for hit_index in 0..hit_count {
                if self
                    .side(target.side)
                    .pokemon(target.pokemon)
                    .map_or(true, PokemonState::is_fainted)
                {
                    break;
                }
                if hit_index > 0
                    && move_context.move_data.multi_accuracy
                    && !self.check_accuracy(
                        data,
                        effects,
                        field_effects,
                        move_context,
                        target,
                        resolution,
                    )?
                {
                    break;
                }
                let mut damage = self.calculate_standard_damage(
                    data,
                    effects,
                    field_effects,
                    move_context,
                    target_context,
                    spread,
                    resolution,
                )?;
                context.critical = damage.critical_modifier != Modifier::NEUTRAL;
                let modify_damage = effects.dispatch(
                    self,
                    BattleEvent::ModifyDamage,
                    context,
                    EventValue::Integer(i64::from(damage.final_damage)),
                )?;
                damage.final_damage = integer_relay_u16(&modify_damage, "ModifyDamage")?;
                resolution.events.push(modify_damage);
                resolution.events.push(effects.dispatch(
                    self,
                    BattleEvent::Hit,
                    context,
                    EventValue::None,
                )?);
                resolution.actions.push(ResolvedAction::Damaged {
                    side: move_context.queued.side,
                    slot: move_context.queued.slot,
                    target_side: target.side,
                    target_slot: target_context.slot,
                    move_slot: move_context.queued.move_slot,
                    damage,
                });
                let (applied_hp_loss, fainted) = self.apply_hp_damage(
                    effects,
                    DamageApplication {
                        context,
                        side: target.side,
                        slot: target_context.slot,
                        pokemon: target.pokemon,
                        damage: damage.final_damage,
                    },
                    resolution,
                )?;
                self.apply_reactive_field_abilities(data, field_effects, target, applied_hp_loss)?;
                total_applied_damage =
                    total_applied_damage.saturating_add(u32::from(applied_hp_loss));
                let outcome = TargetOutcome {
                    source: move_context.source,
                    move_id: move_context.move_id,
                    target,
                    redirected: target_context.redirected,
                    result: TargetResult::Hit,
                    calculated_damage: damage.final_damage,
                    applied_hp_loss,
                    fainted,
                };
                if target_hit {
                    hit_outcomes.push(outcome);
                } else {
                    outcomes[index] = Some(outcome);
                }
                target_hit = true;
                any_hit = true;
                self.apply_drain(data, move_context, target, applied_hp_loss)?;
                if !self_effect_applied {
                    self.apply_self_move_effect(
                        data,
                        effects,
                        field_effects,
                        move_context,
                        resolution,
                    )?;
                    self_effect_applied = true;
                }
                self.apply_secondary_effects(
                    data,
                    effects,
                    field_effects,
                    move_context,
                    target,
                    resolution,
                )?;
                if fainted {
                    self.settle_faint(
                        effects,
                        DamageApplication {
                            context,
                            side: target.side,
                            slot: target_context.slot,
                            pokemon: target.pokemon,
                            damage: 0,
                        },
                        resolution,
                    )?;
                    break;
                }
            }
        }
        resolution
            .target_outcomes
            .extend(outcomes.into_iter().flatten());
        resolution.target_outcomes.extend(hit_outcomes);
        self.apply_recoil(
            data,
            move_context,
            total_applied_damage,
            effects,
            resolution,
        )?;
        if !any_hit {
            resolution.actions.push(ResolvedAction::Skipped {
                side: move_context.queued.side,
                slot: move_context.queued.slot,
            });
        }
        let after_context = EventContext {
            source: Some(move_context.source),
            target: move_context.targets.last().map(|target| target.target),
            move_id: Some(move_context.move_id),
            suppress_target_ability: move_context.suppress_target_ability,
            critical: false,
            ..EventContext::default()
        };
        resolution.events.push(effects.dispatch(
            self,
            BattleEvent::AfterMove,
            after_context,
            EventValue::None,
        )?);
        Ok(())
    }

    fn resolve_hit_count(&mut self, sequence: HitSequence) -> Result<u8, BattleError> {
        match sequence {
            HitSequence::Single => Ok(1),
            HitSequence::Fixed { hits } if hits > 0 => Ok(hits),
            HitSequence::Range { min: 2, max: 5 } => {
                const DISTRIBUTION: [u8; 20] =
                    [2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 5, 5, 5];
                self.rng.sample(RngReason::MultiHit, &DISTRIBUTION).copied()
            }
            HitSequence::Range { min, max } if min > 0 && min <= max => self
                .rng
                .range(RngReason::MultiHit, u64::from(min)..u64::from(max) + 1)
                .and_then(|value| {
                    u8::try_from(value).map_err(|_| {
                        BattleError::InvalidState("multi-hit count does not fit in u8".to_owned())
                    })
                }),
            HitSequence::Fixed { .. } | HitSequence::Range { .. } => Err(
                BattleError::DataMismatch("move declares an invalid hit sequence".to_owned()),
            ),
        }
    }

    fn apply_drain<D: CombatData>(
        &mut self,
        data: &D,
        context: &MoveExecutionContext,
        target: PokemonRef,
        applied_damage: u16,
    ) -> Result<(), BattleError> {
        let Some(fraction) = context.move_data.drain else {
            return Ok(());
        };
        let amount = rounded_fraction(u32::from(applied_damage), fraction)?;
        if amount > 0 {
            let target_has_liquid_ooze = self
                .side(target.side)
                .pokemon(target.pokemon)
                .is_some_and(|pokemon| {
                    data.ability_has_behavior(pokemon.ability, crate::AbilityBehavior::LiquidOoze)
                });
            let amount = u16::try_from(amount).unwrap_or(u16::MAX);
            if target_has_liquid_ooze {
                StateKernel::new(self).damage_hp(context.source, amount)?;
            } else {
                StateKernel::new(self).heal_hp(context.source, amount)?;
            }
        }
        Ok(())
    }

    fn apply_recoil<D: CombatData>(
        &mut self,
        data: &D,
        context: &MoveExecutionContext,
        applied_damage: u32,
        effects: &EffectRegistry,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let fraction_and_basis = context
            .move_data
            .recoil
            .map(|fraction| (fraction, applied_damage))
            .or_else(|| {
                context.move_data.self_damage.map(|fraction| {
                    let max_hp = self
                        .side(context.source.side)
                        .pokemon(context.source.pokemon)
                        .map_or(0, |source| u32::from(source.stats.hp));
                    (fraction, max_hp)
                })
            });
        let Some((fraction, basis)) = fraction_and_basis else {
            return Ok(());
        };
        let source_ability = self
            .side(context.source.side)
            .pokemon(context.source.pokemon)
            .ok_or_else(|| BattleError::InvalidState("recoil source is invalid".to_owned()))?
            .ability;
        if data.ability_has_behavior(source_ability, crate::AbilityBehavior::RockHead)
            && !data.move_is_struggle(context.move_id)
        {
            return Ok(());
        }
        if applied_damage == 0 {
            return Ok(());
        }
        let requested = rounded_fraction(basis, fraction)?.max(1);
        let source_state = self
            .side(context.source.side)
            .pokemon(context.source.pokemon)
            .ok_or_else(|| BattleError::InvalidState("recoil source is invalid".to_owned()))?;
        if source_state.is_fainted() {
            return Ok(());
        }
        let slot = match source_state.position {
            crate::PokemonPosition::Active { slot } => slot,
            crate::PokemonPosition::Team | crate::PokemonPosition::Fainted => {
                return Err(BattleError::InvalidState(
                    "recoil source is not active".to_owned(),
                ));
            }
        };
        let event_context = EventContext {
            source: Some(context.source),
            target: Some(context.source),
            move_id: Some(context.move_id),
            suppress_target_ability: context.suppress_target_ability,
            critical: false,
            ..EventContext::default()
        };
        let _ = self.apply_damage(
            effects,
            DamageApplication {
                context: event_context,
                side: context.source.side,
                slot,
                pokemon: context.source.pokemon,
                damage: u16::try_from(requested).unwrap_or(u16::MAX),
            },
            resolution,
        )?;
        Ok(())
    }

    fn apply_self_move_effect<D: CombatData>(
        &mut self,
        data: &D,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        context: &MoveExecutionContext,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let Some(effect) = &context.self_effect else {
            return Ok(());
        };
        // Showdown samples the self-effect roll even when `chance` is omitted.
        let roll = self.rng.range(RngReason::SecondaryEffect, 0..100)?;
        if effect
            .chance
            .is_some_and(|chance| roll >= u64::from(chance))
        {
            return Ok(());
        }
        for operation in &effect.effects {
            let _ = self.apply_primary_effect(
                data,
                effects,
                field_effects,
                context,
                Some(context.source),
                *operation,
                resolution,
            )?;
        }
        Ok(())
    }

    fn apply_secondary_effects<D: CombatData>(
        &mut self,
        data: &D,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        context: &MoveExecutionContext,
        target: PokemonRef,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let shield_dust = !context.suppress_target_ability
            && self
                .side(target.side)
                .pokemon(target.pokemon)
                .is_some_and(|pokemon| {
                    data.ability_has_behavior(pokemon.ability, crate::AbilityBehavior::ShieldDust)
                });
        for secondary in &context.secondary_effects {
            if shield_dust && secondary.self_effects.is_empty() {
                continue;
            }
            // Showdown samples `random(100)` for every secondary, then compares
            // the sampled value with the declared chance.
            let roll = self.rng.range(RngReason::SecondaryEffect, 0..100)?;
            if roll >= u64::from(secondary.chance) {
                continue;
            }
            if !shield_dust {
                for operation in &secondary.effects {
                    let _ = self.apply_primary_effect(
                        data,
                        effects,
                        field_effects,
                        context,
                        Some(target),
                        *operation,
                        resolution,
                    )?;
                }
            }
            for operation in &secondary.self_effects {
                let _ = self.apply_primary_effect(
                    data,
                    effects,
                    field_effects,
                    context,
                    Some(context.source),
                    *operation,
                    resolution,
                )?;
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn execute_fixed_damage<D: CombatData>(
        &mut self,
        data: &D,
        context: &MoveExecutionContext,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let HitLifecycle {
            eligible,
            mut outcomes,
        } = self.resolve_hit_lifecycle(data, context, effects, field_effects, true, resolution)?;
        let requested = match context.move_data.operation {
            MoveOperation::FixedDamage => context.move_data.fixed_damage.ok_or_else(|| {
                BattleError::DataMismatch("fixed-damage move has no damage value".to_owned())
            })?,
            MoveOperation::LevelDamage => self
                .side(context.source.side)
                .pokemon(context.source.pokemon)
                .ok_or_else(|| {
                    BattleError::InvalidState("level-damage source is invalid".to_owned())
                })?
                .level
                .into(),
            MoveOperation::StandardDamage | MoveOperation::NonDamage => {
                return Err(BattleError::InvalidState(
                    "non-fixed move entered fixed-damage execution".to_owned(),
                ));
            }
        };
        let mut any_hit = false;
        let mut total_applied = 0_u32;
        for (index, target_context) in eligible {
            let target_state = self
                .side(target_context.target.side)
                .pokemon(target_context.target.pokemon)
                .ok_or_else(|| {
                    BattleError::InvalidState("fixed-damage target is invalid".to_owned())
                })?;
            let sturdy = !context.suppress_target_ability
                && data.ability_has_behavior(target_state.ability, crate::AbilityBehavior::Sturdy)
                && target_state.current_hp == target_state.stats.hp
                && requested >= target_state.current_hp;
            let applied_request = if sturdy {
                target_state.current_hp.saturating_sub(1)
            } else {
                requested
            };
            let event_context = EventContext {
                source: Some(context.source),
                target: Some(target_context.target),
                move_id: Some(context.move_id),
                suppress_target_ability: context.suppress_target_ability,
                critical: false,
                move_category: Some(context.move_data.category),
                move_type: Some(context.move_data.move_type),
                move_flags: context.move_data.interaction_flags,
                type_effectiveness: self.target_type_effectiveness(
                    data,
                    context.move_data,
                    context.source,
                    target_context.target,
                )?,
                weather: field_effects.effective_weather_environment(self, data),
                terrain: field_effects.terrain_environment(self),
                ..EventContext::default()
            };
            resolution.events.push(effects.dispatch(
                self,
                BattleEvent::Hit,
                event_context,
                EventValue::None,
            )?);
            let damage = DamageResult {
                base_damage: u32::from(applied_request),
                attack_stat: 0,
                defense_stat: 0,
                stab_modifier: Modifier::NEUTRAL,
                type_modifier: Modifier::NEUTRAL,
                critical_modifier: Modifier::NEUTRAL,
                random_modifier: Modifier::NEUTRAL,
                final_damage: applied_request,
            };
            resolution.actions.push(ResolvedAction::Damaged {
                side: context.queued.side,
                slot: context.queued.slot,
                target_side: target_context.target.side,
                target_slot: target_context.slot,
                move_slot: context.queued.move_slot,
                damage,
            });
            let (applied_hp_loss, fainted) = self.apply_hp_damage(
                effects,
                DamageApplication {
                    context: event_context,
                    side: target_context.target.side,
                    slot: target_context.slot,
                    pokemon: target_context.target.pokemon,
                    damage: applied_request,
                },
                resolution,
            )?;
            self.apply_reactive_field_abilities(
                data,
                field_effects,
                target_context.target,
                applied_hp_loss,
            )?;
            total_applied = total_applied.saturating_add(u32::from(applied_hp_loss));
            outcomes[index] = Some(TargetOutcome {
                source: context.source,
                move_id: context.move_id,
                target: target_context.target,
                redirected: target_context.redirected,
                result: TargetResult::Hit,
                calculated_damage: applied_request,
                applied_hp_loss,
                fainted,
            });
            self.apply_drain(data, context, target_context.target, applied_hp_loss)?;
            self.apply_self_move_effect(data, effects, field_effects, context, resolution)?;
            self.apply_secondary_effects(
                data,
                effects,
                field_effects,
                context,
                target_context.target,
                resolution,
            )?;
            if fainted {
                self.settle_faint(
                    effects,
                    DamageApplication {
                        context: event_context,
                        side: target_context.target.side,
                        slot: target_context.slot,
                        pokemon: target_context.target.pokemon,
                        damage: 0,
                    },
                    resolution,
                )?;
            }
            any_hit = true;
        }
        resolution
            .target_outcomes
            .extend(outcomes.into_iter().flatten());
        self.apply_recoil(data, context, total_applied, effects, resolution)?;
        resolution.actions.push(if any_hit {
            ResolvedAction::Used {
                side: context.queued.side,
                slot: context.queued.slot,
                move_slot: context.queued.move_slot,
            }
        } else {
            ResolvedAction::Skipped {
                side: context.queued.side,
                slot: context.queued.slot,
            }
        });
        resolution.events.push(effects.dispatch(
            self,
            BattleEvent::AfterMove,
            EventContext {
                source: Some(context.source),
                target: context.targets.last().map(|target| target.target),
                move_id: Some(context.move_id),
                suppress_target_ability: context.suppress_target_ability,
                critical: false,
                ..EventContext::default()
            },
            EventValue::None,
        )?);
        Ok(())
    }

    /// Advances a resolved target set through the shared `TryHit`, priority,
    /// protection, optional type-immunity, accuracy, and break-protection stages.
    /// Targets move through each stage as a group to preserve Showdown ordering.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn resolve_hit_lifecycle<D: CombatData>(
        &mut self,
        data: &D,
        move_context: &MoveExecutionContext,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        check_type_immunity: bool,
        resolution: &mut TurnResolution,
    ) -> Result<HitLifecycle, BattleError> {
        let mut outcomes = vec![None; move_context.targets.len()];
        let mut after_try_hit = Vec::with_capacity(move_context.targets.len());
        for (index, target_context) in move_context.targets.iter().copied().enumerate() {
            let target = target_context.target;
            let context = EventContext {
                source: Some(move_context.source),
                target: Some(target),
                move_id: Some(move_context.move_id),
                suppress_target_ability: move_context.suppress_target_ability,
                critical: false,
                move_category: Some(move_context.move_data.category),
                move_type: Some(move_context.move_data.move_type),
                move_flags: move_context.move_data.interaction_flags,
                prankster_boosted: move_context.queued.prankster_boosted,
                target_grounded: Some(crate::field_effects::grounded(
                    self,
                    data,
                    field_effects,
                    target,
                )?),
                weather: field_effects.effective_weather_environment(self, data),
                terrain: field_effects.terrain_environment(self),
                ..EventContext::default()
            };
            // Psychic Terrain resolves at priority 4 and Protect at priority 3
            // in Showdown's TryHit event, before ordinary ability handlers.
            let priority_blocked =
                self.priority_move_blocked_target(data, field_effects, move_context, target)?;
            let protected = !priority_blocked
                && self.target_is_protected(data, field_effects, move_context, target);
            if priority_blocked || protected {
                outcomes[index] = Some(Self::empty_target_outcome(
                    move_context,
                    target_context,
                    if priority_blocked {
                        TargetResult::PriorityBlocked
                    } else {
                        TargetResult::Protected
                    },
                ));
                continue;
            }
            let try_hit = effects.dispatch(
                self,
                BattleEvent::TryHit,
                context,
                EventValue::Boolean(true),
            )?;
            let cancelled = try_hit.cancelled || try_hit.value == EventValue::Boolean(false);
            let ability_immune = try_hit.immune;
            resolution.events.push(try_hit);
            if cancelled {
                outcomes[index] = Some(Self::empty_target_outcome(
                    move_context,
                    target_context,
                    if ability_immune {
                        TargetResult::Immune
                    } else {
                        TargetResult::Cancelled
                    },
                ));
            } else {
                after_try_hit.push((index, target_context));
            }
        }

        let mut after_immunity = Vec::with_capacity(after_try_hit.len());
        for (index, target_context) in after_try_hit {
            if check_type_immunity
                && self.target_is_type_immune(
                    data,
                    move_context.move_data,
                    move_context.source,
                    target_context.target,
                )?
            {
                outcomes[index] = Some(Self::empty_target_outcome(
                    move_context,
                    target_context,
                    TargetResult::Immune,
                ));
            } else {
                after_immunity.push((index, target_context));
            }
        }

        let mut after_try_immunity = Vec::with_capacity(after_immunity.len());
        for (index, target_context) in after_immunity {
            if !move_context.queued.prankster_boosted {
                after_try_immunity.push((index, target_context));
                continue;
            }
            let target = target_context.target;
            let target_state = self
                .side(target.side)
                .pokemon(target.pokemon)
                .ok_or_else(|| BattleError::InvalidState("move target is invalid".to_owned()))?;
            let try_immunity = effects.dispatch(
                self,
                BattleEvent::TryImmunity,
                EventContext {
                    source: Some(move_context.source),
                    target: Some(target),
                    move_id: Some(move_context.move_id),
                    suppress_target_ability: move_context.suppress_target_ability,
                    move_category: Some(move_context.move_data.category),
                    move_type: Some(move_context.move_data.move_type),
                    move_flags: move_context.move_data.interaction_flags,
                    prankster_boosted: move_context.queued.prankster_boosted,
                    target_types: Some(
                        data.pokemon_types(target_state.species, target_state.form)
                            .ok_or_else(|| {
                                BattleError::DataMismatch(
                                    "missing move target type data".to_owned(),
                                )
                            })?,
                    ),
                    ..EventContext::default()
                },
                EventValue::Boolean(true),
            )?;
            let immune = try_immunity.immune
                || try_immunity.cancelled
                || try_immunity.value == EventValue::Boolean(false);
            resolution.events.push(try_immunity);
            if immune {
                outcomes[index] = Some(Self::empty_target_outcome(
                    move_context,
                    target_context,
                    TargetResult::Immune,
                ));
            } else {
                after_try_immunity.push((index, target_context));
            }
        }

        let mut eligible = Vec::with_capacity(after_try_immunity.len());
        for (index, target_context) in after_try_immunity {
            if self.check_accuracy(
                data,
                effects,
                field_effects,
                move_context,
                target_context.target,
                resolution,
            )? {
                eligible.push((index, target_context));
            } else {
                resolution.actions.push(ResolvedAction::Missed {
                    side: move_context.queued.side,
                    slot: move_context.queued.slot,
                    move_slot: move_context.queued.move_slot,
                });
                outcomes[index] = Some(Self::empty_target_outcome(
                    move_context,
                    target_context,
                    TargetResult::Missed,
                ));
            }
        }

        for (_, target_context) in &eligible {
            self.break_target_protection_if_needed(
                data,
                field_effects,
                move_context,
                target_context.target,
            )?;
        }
        Ok(HitLifecycle { eligible, outcomes })
    }

    fn empty_target_outcome(
        context: &MoveExecutionContext,
        target: TargetExecutionContext,
        result: TargetResult,
    ) -> TargetOutcome {
        TargetOutcome {
            source: context.source,
            move_id: context.move_id,
            target: target.target,
            redirected: target.redirected,
            result,
            calculated_damage: 0,
            applied_hp_loss: 0,
            fainted: false,
        }
    }

    // These three narrow condition intrinsics share the normal move lifecycle;
    // splitting them would create parallel executors for tiny branches.
    #[allow(clippy::too_many_lines)]
    fn execute_non_damage<D: CombatData>(
        &mut self,
        data: &D,
        move_context: &MoveExecutionContext,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        if !move_context.primary_effects.is_empty() {
            return self.execute_declarative_non_damage(
                data,
                move_context,
                effects,
                field_effects,
                resolution,
            );
        }
        let context = EventContext {
            source: Some(move_context.source),
            target: Some(move_context.source),
            move_id: Some(move_context.move_id),
            suppress_target_ability: move_context.suppress_target_ability,
            critical: false,
            ..EventContext::default()
        };
        let applied = match data.move_intrinsic(move_context.move_id) {
            MoveIntrinsic::Protect => {
                let protect = field_effects.protect.ok_or_else(|| {
                    BattleError::DataMismatch(
                        "Protect condition identity is unavailable".to_owned(),
                    )
                })?;
                let stall = field_effects.stall.ok_or_else(|| {
                    BattleError::DataMismatch("stall condition identity is unavailable".to_owned())
                })?;
                let existing = self
                    .side(move_context.source.side)
                    .pokemon(move_context.source.pokemon)
                    .and_then(|pokemon| {
                        pokemon
                            .volatiles
                            .iter()
                            .find(|condition| condition.id == stall)
                            .copied()
                    });
                let success = match existing {
                    None => true,
                    Some(condition) => self.rng.chance(
                        RngReason::Protect,
                        1,
                        u64::from(condition.counter.unwrap_or(1).max(1)),
                    )?,
                };
                if let Some(condition) = existing {
                    StateKernel::new(self).remove_volatile(move_context.source, condition.id)?;
                }
                if success {
                    StateKernel::new(self)
                        .add_volatile(move_context.source, ConditionState::new(protect, Some(1)))?;
                    let next_counter = existing
                        .and_then(|condition| condition.counter)
                        .map_or(3, |counter| counter.saturating_mul(3).min(729));
                    StateKernel::new(self).add_volatile(
                        move_context.source,
                        ConditionState::with_counter(stall, Some(2), next_counter),
                    )?;
                }
                success
            }
            MoveIntrinsic::FollowMe | MoveIntrinsic::RagePowder => {
                let condition = match data.move_intrinsic(move_context.move_id) {
                    MoveIntrinsic::FollowMe => field_effects.follow_me,
                    MoveIntrinsic::RagePowder => field_effects.rage_powder,
                    MoveIntrinsic::Ordinary | MoveIntrinsic::Protect => None,
                }
                .ok_or_else(|| {
                    BattleError::DataMismatch(
                        "redirection condition identity is unavailable".to_owned(),
                    )
                })?;
                let doubles_active = self
                    .side(move_context.source.side)
                    .active
                    .into_iter()
                    .flatten()
                    .count()
                    > 1;
                if doubles_active {
                    StateKernel::new(self).add_volatile(
                        move_context.source,
                        ConditionState::new(condition, Some(1)),
                    )?;
                }
                doubles_active
            }
            MoveIntrinsic::Ordinary => {
                return Err(BattleError::UnsupportedMechanic(format!(
                    "move {} has no supported non-damage intrinsic",
                    move_context.move_id
                )));
            }
        };
        resolution.actions.push(if applied {
            ResolvedAction::Used {
                side: move_context.queued.side,
                slot: move_context.queued.slot,
                move_slot: move_context.queued.move_slot,
            }
        } else {
            ResolvedAction::Skipped {
                side: move_context.queued.side,
                slot: move_context.queued.slot,
            }
        });
        if applied {
            resolution.events.push(effects.dispatch(
                self,
                BattleEvent::Hit,
                context,
                EventValue::None,
            )?);
        }
        resolution.target_outcomes.push(TargetOutcome {
            source: move_context.source,
            move_id: move_context.move_id,
            target: move_context.source,
            redirected: false,
            result: if applied {
                TargetResult::Hit
            } else {
                TargetResult::Cancelled
            },
            calculated_damage: 0,
            applied_hp_loss: 0,
            fainted: false,
        });
        resolution.events.push(effects.dispatch(
            self,
            BattleEvent::AfterMove,
            context,
            EventValue::None,
        )?);
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn execute_declarative_non_damage<D: CombatData>(
        &mut self,
        data: &D,
        move_context: &MoveExecutionContext,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let global = move_context.primary_effects.iter().all(|effect| {
            matches!(
                effect,
                PrimaryMoveEffect::SetWeather { .. }
                    | PrimaryMoveEffect::SetTerrain { .. }
                    | PrimaryMoveEffect::AddFieldCondition { .. }
                    | PrimaryMoveEffect::ToggleFieldCondition { .. }
                    | PrimaryMoveEffect::AddSideCondition { .. }
            )
        });
        let (targets, mut outcomes) = if global {
            (vec![(0, None)], Vec::new())
        } else {
            let HitLifecycle { eligible, outcomes } = self.resolve_hit_lifecycle(
                data,
                move_context,
                effects,
                field_effects,
                false,
                resolution,
            )?;
            (
                eligible
                    .into_iter()
                    .map(|(index, target)| (index, Some(target)))
                    .collect(),
                outcomes,
            )
        };
        let mut any_applied = false;
        for (index, target_context) in targets {
            let target = target_context.map(|target| target.target);
            let context = EventContext {
                source: Some(move_context.source),
                target,
                move_id: Some(move_context.move_id),
                suppress_target_ability: move_context.suppress_target_ability,
                critical: false,
                ..EventContext::default()
            };
            let mut applied = false;
            for effect in &move_context.primary_effects {
                applied |= self.apply_primary_effect(
                    data,
                    effects,
                    field_effects,
                    move_context,
                    target,
                    *effect,
                    resolution,
                )?;
            }
            if applied {
                resolution.events.push(effects.dispatch(
                    self,
                    BattleEvent::Hit,
                    context,
                    EventValue::None,
                )?);
            }
            if let Some(target_context) = target_context {
                outcomes[index] = Some(Self::empty_target_outcome(
                    move_context,
                    target_context,
                    if applied {
                        TargetResult::Hit
                    } else {
                        TargetResult::Cancelled
                    },
                ));
            }
            any_applied |= applied;
        }
        resolution
            .target_outcomes
            .extend(outcomes.into_iter().flatten());
        resolution.actions.push(if any_applied {
            ResolvedAction::Used {
                side: move_context.queued.side,
                slot: move_context.queued.slot,
                move_slot: move_context.queued.move_slot,
            }
        } else {
            ResolvedAction::Skipped {
                side: move_context.queued.side,
                slot: move_context.queued.slot,
            }
        });
        resolution.events.push(effects.dispatch(
            self,
            BattleEvent::AfterMove,
            EventContext {
                source: Some(move_context.source),
                target: move_context.targets.last().map(|target| target.target),
                move_id: Some(move_context.move_id),
                suppress_target_ability: move_context.suppress_target_ability,
                critical: false,
                ..EventContext::default()
            },
            EventValue::None,
        )?);
        Ok(())
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn apply_primary_effect<D: CombatData>(
        &mut self,
        data: &D,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        context: &MoveExecutionContext,
        target: Option<PokemonRef>,
        effect: PrimaryMoveEffect,
        resolution: &mut TurnResolution,
    ) -> Result<bool, BattleError> {
        let pokemon = || {
            target.ok_or_else(|| {
                BattleError::DataMismatch(
                    "Pokémon-targeted primary effect has no target".to_owned(),
                )
            })
        };
        match effect {
            PrimaryMoveEffect::InflictStatus { status } => {
                let target = pokemon()?;
                if !self.status_is_eligible(
                    data,
                    field_effects,
                    target,
                    status,
                    context.move_data.move_type,
                    context.source,
                )? {
                    return Ok(false);
                }
                let status_context = EventContext {
                    source: Some(context.source),
                    target: Some(target),
                    move_id: Some(context.move_id),
                    suppress_target_ability: context.suppress_target_ability,
                    critical: false,
                    move_category: Some(context.move_data.category),
                    move_type: Some(context.move_data.move_type),
                    move_flags: context.move_data.interaction_flags,
                    requested_status: Some(status),
                    target_types: data.pokemon_types(
                        self.side(target.side)
                            .pokemon(target.pokemon)
                            .ok_or_else(|| {
                                BattleError::InvalidState("status target is invalid".to_owned())
                            })?
                            .species,
                        self.side(target.side)
                            .pokemon(target.pokemon)
                            .and_then(|pokemon| pokemon.form),
                    ),
                    weather: field_effects.effective_weather_environment(self, data),
                    terrain: field_effects.terrain_environment(self),
                    ..EventContext::default()
                };
                let event = effects.dispatch(
                    self,
                    BattleEvent::TrySetStatus,
                    status_context,
                    EventValue::Boolean(true),
                )?;
                let allowed = !event.cancelled && event.value == EventValue::Boolean(true);
                resolution.events.push(event);
                if allowed {
                    self.apply_major_status(target, status)?;
                    resolution.events.push(effects.dispatch(
                        self,
                        BattleEvent::AfterSetStatus,
                        EventContext {
                            suppress_target_ability: false,
                            ..status_context
                        },
                        EventValue::None,
                    )?);
                }
                Ok(allowed)
            }
            PrimaryMoveEffect::CureStatus => Ok(self.cure_major_status(pokemon()?)?.is_some()),
            PrimaryMoveEffect::ChangeStatStage { stat, delta } => {
                let target = pokemon()?;
                if self
                    .side(target.side)
                    .pokemon(target.pokemon)
                    .map_or(true, PokemonState::is_fainted)
                {
                    return Ok(false);
                }
                let stat_context = EventContext {
                    source: Some(context.source),
                    target: Some(target),
                    move_id: Some(context.move_id),
                    suppress_target_ability: context.suppress_target_ability,
                    requested_stat: Some(stat),
                    target_types: data.pokemon_types(
                        self.side(target.side)
                            .pokemon(target.pokemon)
                            .ok_or_else(|| {
                                BattleError::InvalidState("stat target is invalid".to_owned())
                            })?
                            .species,
                        self.side(target.side)
                            .pokemon(target.pokemon)
                            .and_then(|pokemon| pokemon.form),
                    ),
                    ..EventContext::default()
                };
                let event = effects.dispatch(
                    self,
                    BattleEvent::TryStatStageChange,
                    stat_context,
                    EventValue::Integer(i64::from(delta)),
                )?;
                let EventValue::Integer(relayed_delta) = event.value else {
                    return Err(BattleError::InvalidState(
                        "TryStatStageChange returned a non-integer relay".to_owned(),
                    ));
                };
                let delta = i8::try_from(relayed_delta).map_err(|_| {
                    BattleError::InvalidState("stat-stage relay does not fit in i8".to_owned())
                })?;
                resolution.events.push(event);
                let applied = StateKernel::new(self)
                    .change_stat_stage(target, stat, delta)?
                    .applied_delta;
                if applied != 0 {
                    resolution.events.push(effects.dispatch(
                        self,
                        BattleEvent::AfterStatStageChange,
                        stat_context,
                        EventValue::Integer(i64::from(applied)),
                    )?);
                }
                Ok(applied != 0)
            }
            PrimaryMoveEffect::Heal {
                numerator,
                denominator,
            } => {
                if denominator == 0 {
                    return Err(BattleError::DataMismatch(
                        "healing denominator is zero".to_owned(),
                    ));
                }
                let target = pokemon()?;
                let max_hp = self
                    .side(target.side)
                    .pokemon(target.pokemon)
                    .ok_or_else(|| {
                        BattleError::InvalidState("healing target is invalid".to_owned())
                    })?
                    .stats
                    .hp;
                // Gen 5+ Showdown move healing uses Math.round before the
                // bounded heal (for example, 155 / 2 = 78).
                let amount = (u32::from(max_hp) * u32::from(numerator)
                    + u32::from(denominator) / 2)
                    / u32::from(denominator);
                Ok(StateKernel::new(self)
                    .heal_hp(target, u16::try_from(amount).unwrap_or(u16::MAX))?
                    .applied
                    != 0)
            }
            PrimaryMoveEffect::AddVolatile {
                condition,
                duration,
            } => {
                let target = pokemon()?;
                let target_state =
                    self.side(target.side)
                        .pokemon(target.pokemon)
                        .ok_or_else(|| {
                            BattleError::InvalidState("volatile target is invalid".to_owned())
                        })?;
                let magnet_rise_blocked = field_effects.magnet_rise == Some(condition)
                    && (field_effects.field_has(self, field_effects.gravity)
                        || field_effects
                            .ingrain
                            .is_some_and(|id| target_state.has_volatile(id))
                        || field_effects
                            .smack_down
                            .is_some_and(|id| target_state.has_volatile(id)));
                if target_state.has_volatile(condition) || magnet_rise_blocked {
                    return Ok(false);
                }
                StateKernel::new(self)
                    .add_volatile(target, ConditionState::new(condition, duration))?;
                Ok(true)
            }
            PrimaryMoveEffect::SetWeather {
                condition,
                duration,
            } => {
                if self
                    .field
                    .weather
                    .is_some_and(|state| state.id == condition)
                {
                    return Ok(false);
                }
                StateKernel::new(self)
                    .set_weather(Some(ConditionState::new(condition, Some(duration))))?;
                Ok(true)
            }
            PrimaryMoveEffect::SetTerrain {
                condition,
                duration,
            } => {
                if self
                    .field
                    .terrain
                    .is_some_and(|state| state.id == condition)
                {
                    return Ok(false);
                }
                StateKernel::new(self)
                    .set_terrain(Some(ConditionState::new(condition, Some(duration))))?;
                Ok(true)
            }
            PrimaryMoveEffect::AddFieldCondition {
                condition,
                duration,
            } => {
                if self
                    .field
                    .conditions
                    .iter()
                    .any(|state| state.id == condition)
                {
                    return Ok(false);
                }
                StateKernel::new(self)
                    .add_field_condition(ConditionState::new(condition, Some(duration)))?;
                Ok(true)
            }
            PrimaryMoveEffect::ToggleFieldCondition {
                condition,
                duration,
            } => {
                if self
                    .field
                    .conditions
                    .iter()
                    .any(|state| state.id == condition)
                {
                    let _ = StateKernel::new(self).remove_field_condition(condition);
                } else {
                    StateKernel::new(self)
                        .add_field_condition(ConditionState::new(condition, Some(duration)))?;
                }
                Ok(true)
            }
            PrimaryMoveEffect::AddSideCondition {
                condition,
                duration,
            } => {
                let side = match context.move_data.target {
                    MoveTarget::FoeSide => context.source.side.opponent(),
                    _ => context.source.side,
                };
                if self
                    .side(side)
                    .side_conditions
                    .iter()
                    .any(|state| state.id == condition)
                {
                    return Ok(false);
                }
                StateKernel::new(self)
                    .add_side_condition(side, ConditionState::new(condition, Some(duration)))?;
                Ok(true)
            }
        }
    }

    fn status_is_eligible<D: CombatData>(
        &self,
        data: &D,
        ids: &FieldEffectIds,
        target: PokemonRef,
        status: MajorStatus,
        move_type: PokemonType,
        source: PokemonRef,
    ) -> Result<bool, BattleError> {
        let pokemon = self
            .side(target.side)
            .pokemon(target.pokemon)
            .ok_or_else(|| BattleError::InvalidState("status target is invalid".to_owned()))?;
        if pokemon.status.is_some() || pokemon.is_fainted() {
            return Ok(false);
        }
        let types = data
            .pokemon_types(pokemon.species, pokemon.form)
            .ok_or_else(|| BattleError::DataMismatch("missing status target types".to_owned()))?;
        let immune = match status {
            MajorStatus::Burn => {
                types.primary == PokemonType::Fire || types.secondary == Some(PokemonType::Fire)
            }
            MajorStatus::Freeze => {
                types.primary == PokemonType::Ice || types.secondary == Some(PokemonType::Ice)
            }
            MajorStatus::Paralysis => {
                types.primary == PokemonType::Electric
                    || types.secondary == Some(PokemonType::Electric)
                    || (move_type == PokemonType::Electric
                        && (types.primary == PokemonType::Ground
                            || types.secondary == Some(PokemonType::Ground)))
            }
            MajorStatus::Poison | MajorStatus::BadlyPoisoned
                if data.ability_has_behavior(
                    self.side(source.side)
                        .pokemon(source.pokemon)
                        .ok_or_else(|| {
                            BattleError::InvalidState("status source is invalid".to_owned())
                        })?
                        .ability,
                    crate::AbilityBehavior::Corrosion,
                ) =>
            {
                false
            }
            MajorStatus::Poison | MajorStatus::BadlyPoisoned => {
                [types.primary, types.secondary.unwrap_or(types.primary)]
                    .into_iter()
                    .any(|kind| matches!(kind, PokemonType::Poison | PokemonType::Steel))
            }
            MajorStatus::Sleep => false,
        };
        Ok(
            !immune
                && crate::field_effects::terrain_allows_status(self, data, ids, target, status)?,
        )
    }

    fn resolve_original_target(
        &self,
        queued: QueuedMoveAction,
        move_target: MoveTarget,
    ) -> Result<Option<TargetExecutionContext>, BattleError> {
        let location = resolve_target(queued.side, queued.slot, move_target, queued.target)?;
        let Some((side, slot)) = location else {
            return Ok(None);
        };
        Ok(
            self.side(side).active[slot.index()].map(|pokemon| TargetExecutionContext {
                target: PokemonRef::new(side, pokemon),
                slot,
                redirected: false,
            }),
        )
    }

    fn resolve_target_set<D: CombatData>(
        &self,
        data: &D,
        ids: &FieldEffectIds,
        queued: QueuedMoveAction,
        move_target: MoveTarget,
        original: Option<TargetExecutionContext>,
    ) -> Result<Vec<TargetExecutionContext>, BattleError> {
        let source_index = self.side(queued.side).active[queued.slot.index()].ok_or_else(|| {
            BattleError::InvalidState("queued move source is no longer active".to_owned())
        })?;
        let source = PokemonRef::new(queued.side, source_index);
        let active = |side: SideId, slot: ActiveSlot| {
            self.side(side).active[slot.index()].map(|pokemon| TargetExecutionContext {
                target: PokemonRef::new(side, pokemon),
                slot,
                redirected: false,
            })
        };
        let mut targets = match move_target {
            MoveTarget::Self_ => vec![TargetExecutionContext {
                target: source,
                slot: queued.slot,
                redirected: false,
            }],
            MoveTarget::AdjacentAlly => original.into_iter().collect(),
            MoveTarget::AdjacentAllyOrSelf => original
                .into_iter()
                .collect::<Vec<_>>()
                .into_iter()
                .chain((original.is_none()).then_some(TargetExecutionContext {
                    target: source,
                    slot: queued.slot,
                    redirected: false,
                }))
                .collect(),
            MoveTarget::Normal | MoveTarget::Any | MoveTarget::AdjacentFoe => {
                if let Some(target) = original {
                    vec![target]
                } else if matches!(queued.target, ActionTarget::Foe(_)) {
                    [ActiveSlot::Left, ActiveSlot::Right]
                        .into_iter()
                        .filter_map(|slot| active(queued.side.opponent(), slot))
                        .take(1)
                        .collect()
                } else if move_target == MoveTarget::AdjacentAllyOrSelf {
                    vec![TargetExecutionContext {
                        target: source,
                        slot: queued.slot,
                        redirected: false,
                    }]
                } else {
                    Vec::new()
                }
            }
            MoveTarget::AllAdjacent => [active(
                queued.side,
                match queued.slot {
                    ActiveSlot::Left => ActiveSlot::Right,
                    ActiveSlot::Right => ActiveSlot::Left,
                },
            )]
            .into_iter()
            .flatten()
            .chain(
                [ActiveSlot::Left, ActiveSlot::Right]
                    .into_iter()
                    .filter_map(|slot| active(queued.side.opponent(), slot)),
            )
            .collect(),
            MoveTarget::AllAdjacentFoes => [ActiveSlot::Left, ActiveSlot::Right]
                .into_iter()
                .filter_map(|slot| active(queued.side.opponent(), slot))
                .collect(),
            MoveTarget::All => [ActiveSlot::Left, ActiveSlot::Right]
                .into_iter()
                .filter_map(|slot| active(queued.side, slot))
                .chain(
                    [ActiveSlot::Left, ActiveSlot::Right]
                        .into_iter()
                        .filter_map(|slot| active(queued.side.opponent(), slot)),
                )
                .collect(),
            MoveTarget::Allies => [ActiveSlot::Left, ActiveSlot::Right]
                .into_iter()
                .filter_map(|slot| active(queued.side, slot))
                .collect(),
            MoveTarget::AllySide | MoveTarget::AllyTeam | MoveTarget::FoeSide => Vec::new(),
            MoveTarget::RandomNormal | MoveTarget::Scripted => {
                return Err(BattleError::UnsupportedMechanic(format!(
                    "target category {move_target:?} requires a deferred non-Pokémon or random target operation"
                )));
            }
        };
        let ignores_redirection =
            self.side(source.side)
                .pokemon(source.pokemon)
                .is_some_and(|pokemon| {
                    data.ability_has_behavior(pokemon.ability, crate::AbilityBehavior::Stalwart)
                });
        if !ignores_redirection && targets.len() == 1 && targets[0].target.side != source.side {
            if let Some(redirected) = self.redirect_target(data, ids, source, move_target)? {
                targets[0] = redirected;
            }
        }
        Ok(targets)
    }

    fn redirect_target<D: CombatData>(
        &self,
        data: &D,
        ids: &FieldEffectIds,
        source: PokemonRef,
        move_target: MoveTarget,
    ) -> Result<Option<TargetExecutionContext>, BattleError> {
        if !matches!(
            move_target,
            MoveTarget::Normal | MoveTarget::AdjacentFoe | MoveTarget::Any
        ) {
            return Ok(None);
        }
        let source_state = self
            .side(source.side)
            .pokemon(source.pokemon)
            .ok_or_else(|| BattleError::InvalidState("redirection source is invalid".to_owned()))?;
        let source_types = data
            .pokemon_types(source_state.species, source_state.form)
            .ok_or_else(|| {
                BattleError::DataMismatch("missing redirection source types".to_owned())
            })?;
        let powder_immune = source_types.primary == PokemonType::Grass
            || source_types.secondary == Some(PokemonType::Grass)
            || ids
                .overcoat
                .is_some_and(|ability| source_state.ability == ability)
            || (ids.item_is_active_for(self, data, source)
                && ids
                    .safety_goggles
                    .is_some_and(|item| source_state.item == Some(item)));
        let mut candidates = [ActiveSlot::Left, ActiveSlot::Right]
            .into_iter()
            .filter_map(|slot| {
                let pokemon = self.side(source.side.opponent()).active[slot.index()]?;
                let target = self.side(source.side.opponent()).pokemon(pokemon)?;
                let follow = ids
                    .follow_me
                    .and_then(|id| target.volatiles.iter().find(|condition| condition.id == id));
                let rage = ids
                    .rage_powder
                    .and_then(|id| target.volatiles.iter().find(|condition| condition.id == id));
                let redirect = follow.or_else(|| (!powder_immune).then_some(rage).flatten());
                if redirect.is_some() {
                    Some((
                        ids.action_speed(self, source.side.opponent(), target.effective_speed()),
                        self.active_effect_order(source.side.opponent(), slot)
                            .unwrap_or(u32::MAX),
                        slot,
                        pokemon,
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(speed, effect_order, slot, _)| {
            (std::cmp::Reverse(*speed), *effect_order, slot.index())
        });
        Ok(candidates
            .first()
            .map(|(_, _, slot, pokemon)| TargetExecutionContext {
                target: PokemonRef::new(source.side.opponent(), *pokemon),
                slot: *slot,
                redirected: true,
            }))
    }

    fn target_is_protected<D: CombatData>(
        &self,
        data: &D,
        ids: &FieldEffectIds,
        context: &MoveExecutionContext,
        target: PokemonRef,
    ) -> bool {
        let Some(protect) = ids.protect else {
            return false;
        };
        let guarded = self
            .side(target.side)
            .pokemon(target.pokemon)
            .is_some_and(|pokemon| pokemon.has_volatile(protect));
        if !guarded {
            return false;
        }
        if data.move_breaks_protect(context.move_id) {
            return false;
        }
        let contact_bypass = context.move_data.interaction_flags.contact()
            && self
                .side(context.source.side)
                .pokemon(context.source.pokemon)
                .is_some_and(|pokemon| {
                    data.ability_has_behavior(
                        pokemon.ability,
                        crate::AbilityBehavior::ContactProtectBypass,
                    )
                });
        if contact_bypass {
            return false;
        }
        data.move_is_protectable(context.move_id)
    }

    fn break_target_protection_if_needed<D: CombatData>(
        &mut self,
        data: &D,
        ids: &FieldEffectIds,
        context: &MoveExecutionContext,
        target: PokemonRef,
    ) -> Result<(), BattleError> {
        if !data.move_breaks_protect(context.move_id) {
            return Ok(());
        }
        if let Some(protect) = ids.protect {
            StateKernel::new(self).remove_volatile(target, protect)?;
        }
        Ok(())
    }

    fn target_is_type_immune<D: CombatData>(
        &self,
        data: &D,
        move_data: CombatMove,
        source: PokemonRef,
        target: PokemonRef,
    ) -> Result<bool, BattleError> {
        let target = self
            .side(target.side)
            .pokemon(target.pokemon)
            .ok_or_else(|| BattleError::InvalidState("immunity target is invalid".to_owned()))?;
        let types = data
            .pokemon_types(target.species, target.form)
            .ok_or_else(|| BattleError::DataMismatch("missing immunity target types".to_owned()))?;
        Ok(self
            .type_modifier_for_source(data, move_data.move_type, source, types)?
            .is_none())
    }

    fn target_type_effectiveness<D: CombatData>(
        &self,
        data: &D,
        move_data: CombatMove,
        source: PokemonRef,
        target: PokemonRef,
    ) -> Result<Option<i8>, BattleError> {
        let target = self
            .side(target.side)
            .pokemon(target.pokemon)
            .ok_or_else(|| BattleError::InvalidState("damage target is invalid".to_owned()))?;
        let types = data
            .pokemon_types(target.species, target.form)
            .ok_or_else(|| BattleError::DataMismatch("missing damage target types".to_owned()))?;
        self.type_modifier_for_source(data, move_data.move_type, source, types)
    }

    fn type_modifier_for_source<D: CombatData>(
        &self,
        data: &D,
        attacking: PokemonType,
        source: PokemonRef,
        defending: PokemonTypes,
    ) -> Result<Option<i8>, BattleError> {
        let scrappy = self
            .side(source.side)
            .pokemon(source.pokemon)
            .is_some_and(|pokemon| {
                data.ability_has_behavior(pokemon.ability, crate::AbilityBehavior::Scrappy)
            });
        combined_type_modifier_with_scrappy(data, attacking, defending, scrappy)
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn calculate_standard_damage<D: CombatData>(
        &mut self,
        data: &D,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        move_context: &MoveExecutionContext,
        target_context: TargetExecutionContext,
        spread: bool,
        resolution: &mut TurnResolution,
    ) -> Result<DamageResult, BattleError> {
        let source = move_context.source;
        let target = target_context.target;
        let move_id = move_context.move_id;
        let move_data = move_context.move_data;
        let source_species = move_context.source_species;
        let source_form = move_context.source_form;
        let source_types = data
            .pokemon_types(source_species, source_form)
            .ok_or_else(|| BattleError::DataMismatch("missing source type data".to_owned()))?;
        let mut target_state = self
            .side(target.side)
            .pokemon(target.pokemon)
            .ok_or_else(|| BattleError::InvalidState("damage target is out of bounds".to_owned()))?
            .clone();
        let target_types = data
            .pokemon_types(target_state.species, target_state.form)
            .ok_or_else(|| BattleError::DataMismatch("missing target type data".to_owned()))?;
        let type_exponent =
            self.type_modifier_for_source(data, move_data.move_type, source, target_types)?;
        let mut source_state = self
            .side(source.side)
            .pokemon(source.pokemon)
            .ok_or_else(|| BattleError::InvalidState("damage source is out of bounds".to_owned()))?
            .clone();
        if data.ability_has_behavior(source_state.ability, crate::AbilityBehavior::Unaware) {
            target_state.stat_stages.def = 0;
            target_state.stat_stages.spd = 0;
            target_state.stat_stages.evasion = 0;
        }
        if !move_context.suppress_target_ability
            && data.ability_has_behavior(target_state.ability, crate::AbilityBehavior::Unaware)
        {
            source_state.stat_stages.atk = 0;
            source_state.stat_stages.spa = 0;
            source_state.stat_stages.accuracy = 0;
        }
        let plus_minus_active = data
            .ability_has_behavior(source_state.ability, crate::AbilityBehavior::PlusMinus)
            && self
                .side(source.side)
                .active
                .iter()
                .flatten()
                .copied()
                .any(|index| {
                    index != source.pokemon
                        && self.side(source.side).pokemon(index).is_some_and(|ally| {
                            data.ability_has_behavior(
                                ally.ability,
                                crate::AbilityBehavior::PlusMinus,
                            )
                        })
                });
        let mut event_context = EventContext {
            source: Some(source),
            target: Some(target),
            move_id: Some(move_id),
            suppress_target_ability: move_context.suppress_target_ability,
            suppress_source_item: !field_effects.item_is_active_for(self, data, source),
            critical: false,
            move_category: Some(move_data.category),
            move_type: Some(move_data.move_type),
            move_flags: move_data.interaction_flags,
            move_type_converted: move_context.move_type_converted,
            sheer_force_boosted: move_context.sheer_force_boosted,
            plus_minus_active,
            analytic_active: move_context.later_actors.is_empty(),
            type_effectiveness: type_exponent,
            weather: field_effects.effective_weather_environment(self, data),
            terrain: field_effects.terrain_environment(self),
            ..EventContext::default()
        };
        let crit_ratio_event = effects.dispatch(
            self,
            BattleEvent::ModifyCritRatio,
            event_context,
            EventValue::Integer(i64::from(move_data.crit_ratio)),
        )?;
        let crit_ratio = integer_relay_u8(&crit_ratio_event, "ModifyCritRatio")?.min(4);
        resolution.events.push(crit_ratio_event);
        let mut critical = roll_critical(&mut self.rng, crit_ratio)?;
        if critical {
            let critical_hit = effects.dispatch(
                self,
                BattleEvent::CriticalHit,
                event_context,
                EventValue::Boolean(true),
            )?;
            critical = boolean_relay(&critical_hit, "CriticalHit")?;
            resolution.events.push(critical_hit);
        }
        event_context.critical = critical;
        let mut adjusted_move = move_data;
        self.apply_field_base_power(
            data,
            field_effects,
            source,
            target,
            move_id,
            &mut adjusted_move,
        )?;
        let facade_active = facade_activates(source_state.status);
        if adjusted_move.prior_effect == PriorMoveEffect::Facade && facade_active {
            adjusted_move.base_power = adjusted_move.base_power.saturating_mul(2);
        }
        let base_power = effects.dispatch(
            self,
            BattleEvent::ModifyBasePower,
            event_context,
            EventValue::Integer(i64::from(adjusted_move.base_power)),
        )?;
        adjusted_move.base_power = integer_relay_u16(&base_power, "ModifyBasePower")?.max(1);
        resolution.events.push(base_power);
        let defense_modifier = self.apply_field_defense(
            data,
            field_effects,
            adjusted_move.category,
            &mut target_state,
        )?;
        let (mut attack, mut defense) =
            effective_damage_stats(&source_state, &target_state, adjusted_move, critical)?;
        let modify_attack = effects.dispatch(
            self,
            offensive_stat_event(adjusted_move)?,
            event_context,
            EventValue::Integer(i64::from(attack)),
        )?;
        attack = integer_relay_u32(&modify_attack, "ModifyAttack")?;
        resolution.events.push(modify_attack);
        let modify_defense = effects.dispatch(
            self,
            defensive_stat_event(adjusted_move)?,
            event_context,
            EventValue::Integer(i64::from(defense)),
        )?;
        defense = integer_relay_u32(&modify_defense, "ModifyDefense")?.max(1);
        resolution.events.push(modify_defense);
        let random_percent = u8::try_from(self.rng.range(RngReason::DamageRoll, 85..101)?)
            .map_err(|_| BattleError::InvalidState("damage roll does not fit in u8".to_owned()))?;
        let personal_sun =
            data.ability_has_behavior(source_state.ability, crate::AbilityBehavior::PersonalSun);
        let weather = if personal_sun {
            match move_data.move_type {
                PokemonType::Fire => Some((3, 2)),
                PokemonType::Water => Some((1, 2)),
                _ => None,
            }
        } else if event_context.weather.is_some() {
            crate::field_effects::weather_damage_modifier(self, field_effects, move_data.move_type)
        } else {
            None
        };
        let mut damage = calculate_damage_inner(
            &source_state,
            &target_state,
            adjusted_move,
            source_types,
            type_exponent,
            critical,
            random_percent,
            DamageEnvironment {
                weather,
                defense_modifier,
                attack_override: Some(attack),
                defense_override: Some(defense),
                spread,
                ignore_burn: (adjusted_move.prior_effect == PriorMoveEffect::Facade
                    && facade_active)
                    || data.ability_has_behavior(
                        source_state.ability,
                        crate::AbilityBehavior::StatusedAttack,
                    ),
                adaptability: data.ability_has_behavior(
                    source_state.ability,
                    crate::AbilityBehavior::Adaptability,
                ),
            },
        )?;
        let infiltrates =
            data.ability_has_behavior(source_state.ability, crate::AbilityBehavior::Infiltrator);
        if !infiltrates {
            if let Some((numerator, denominator)) = crate::field_effects::screen_modifier(
                self,
                field_effects,
                target.side,
                move_data.category,
                critical,
            ) {
                damage.final_damage =
                    apply_damage_modifier(damage.final_damage, numerator, denominator);
            }
        }
        Ok(damage)
    }

    fn apply_hp_damage(
        &mut self,
        effects: &EffectRegistry,
        application: DamageApplication,
        resolution: &mut TurnResolution,
    ) -> Result<(u16, bool), BattleError> {
        let DamageApplication {
            context,
            side,
            slot: _,
            pokemon,
            damage,
        } = application;
        let applied = StateKernel::new(self)
            .damage_hp(PokemonRef::new(side, pokemon), damage)?
            .applied;
        resolution.events.push(effects.dispatch(
            self,
            BattleEvent::AfterDamage,
            context,
            EventValue::Integer(i64::from(applied)),
        )?);
        let fainted = self
            .side(side)
            .pokemon(pokemon)
            .is_some_and(|target| target.current_hp == 0);
        Ok((applied, fainted))
    }

    fn settle_faint(
        &mut self,
        effects: &EffectRegistry,
        application: DamageApplication,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let DamageApplication {
            context,
            side,
            slot,
            pokemon,
            ..
        } = application;
        if self
            .side(side)
            .pokemon(pokemon)
            .is_some_and(|target| target.current_hp == 0)
        {
            resolution.events.push(effects.dispatch(
                self,
                BattleEvent::Faint,
                context,
                EventValue::None,
            )?);
            self.process_faints(&[FaintTarget::new(side, slot)])?;
        }
        Ok(())
    }

    fn apply_damage(
        &mut self,
        effects: &EffectRegistry,
        application: DamageApplication,
        resolution: &mut TurnResolution,
    ) -> Result<(u16, bool), BattleError> {
        let (applied, fainted) = self.apply_hp_damage(effects, application, resolution)?;
        if fainted {
            self.settle_faint(effects, application, resolution)?;
        }
        Ok((applied, fainted))
    }

    fn check_accuracy<D: CombatData>(
        &mut self,
        data: &D,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        move_context: &MoveExecutionContext,
        target: PokemonRef,
        resolution: &mut TurnResolution,
    ) -> Result<bool, BattleError> {
        let MoveAccuracy::Percent { value } = move_context.move_data.accuracy else {
            return Ok(true);
        };
        let source_ability = self
            .side(move_context.source.side)
            .pokemon(move_context.source.pokemon)
            .ok_or_else(|| BattleError::InvalidState("accuracy source is invalid".to_owned()))?
            .ability;
        let target_ability = self
            .side(target.side)
            .pokemon(target.pokemon)
            .ok_or_else(|| BattleError::InvalidState("accuracy target is invalid".to_owned()))?
            .ability;
        if data.ability_has_behavior(source_ability, crate::AbilityBehavior::NoGuard)
            || (!move_context.suppress_target_ability
                && data.ability_has_behavior(target_ability, crate::AbilityBehavior::NoGuard))
        {
            return Ok(true);
        }
        let value = match if field_effects
            .effective_weather_environment(self, data)
            .is_some()
        {
            field_effects.weather_accuracy(self, move_context.move_id, u16::from(value))
        } else {
            crate::field_effects::WeatherAccuracy::Percent(u16::from(value))
        } {
            crate::field_effects::WeatherAccuracy::Always => return Ok(true),
            crate::field_effects::WeatherAccuracy::Percent(value) => value,
        };
        let mut source_stage = self
            .side(move_context.source.side)
            .pokemon(move_context.source.pokemon)
            .ok_or_else(|| BattleError::InvalidState("accuracy source is invalid".to_owned()))?
            .stat_stages
            .accuracy;
        let mut target_stage = self
            .side(target.side)
            .pokemon(target.pokemon)
            .ok_or_else(|| BattleError::InvalidState("accuracy target is invalid".to_owned()))?
            .stat_stages
            .evasion;
        if data.ability_has_behavior(source_ability, crate::AbilityBehavior::Unaware) {
            target_stage = 0;
        }
        if !move_context.suppress_target_ability
            && data.ability_has_behavior(target_ability, crate::AbilityBehavior::Unaware)
        {
            source_stage = 0;
        }
        let ignores_evasion = self
            .side(move_context.source.side)
            .pokemon(move_context.source.pokemon)
            .is_some_and(|pokemon| {
                data.ability_has_behavior(
                    pokemon.ability,
                    crate::AbilityBehavior::IgnoreTargetEvasion,
                )
            });
        if ignores_evasion && target_stage > 0 {
            target_stage = 0;
        }
        let stage = (source_stage - target_stage).clamp(-6, 6);
        let adjusted = match stage.cmp(&0) {
            std::cmp::Ordering::Greater => value * u16::try_from(3 + stage).unwrap_or(9) / 3,
            std::cmp::Ordering::Less => value * 3 / u16::try_from(3 - stage).unwrap_or(9),
            std::cmp::Ordering::Equal => value,
        };
        let adjusted = field_effects.gravity_accuracy(self, adjusted);
        let accuracy = effects.dispatch(
            self,
            BattleEvent::ModifyAccuracy,
            EventContext {
                source: Some(move_context.source),
                target: Some(target),
                move_id: Some(move_context.move_id),
                suppress_target_ability: move_context.suppress_target_ability,
                move_category: Some(move_context.move_data.category),
                move_type: Some(move_context.move_data.move_type),
                move_flags: move_context.move_data.interaction_flags,
                weather: field_effects.effective_weather_environment(self, data),
                terrain: field_effects.terrain_environment(self),
                ..EventContext::default()
            },
            EventValue::Integer(i64::from(adjusted)),
        )?;
        let adjusted = integer_relay_u16(&accuracy, "ModifyAccuracy")?;
        resolution.events.push(accuracy);
        self.rng
            .chance(RngReason::Accuracy, u64::from(adjusted.min(100)), 100)
    }

    fn priority_move_blocked_target<D: CombatData>(
        &self,
        data: &D,
        field_effects: &FieldEffectIds,
        context: &MoveExecutionContext,
        target: PokemonRef,
    ) -> Result<bool, BattleError> {
        if context.source.side == target.side {
            return Ok(false);
        }
        let priority = self
            .side(context.source.side)
            .pokemon(context.source.pokemon)
            .and_then(|pokemon| {
                pokemon
                    .moves
                    .get(usize::from(context.queued.move_slot.get()))
            })
            .ok_or_else(|| BattleError::InvalidState("priority source became invalid".to_owned()))?
            .priority;
        let positive_priority = priority > 0 || context.queued.prankster_boosted;
        if !positive_priority {
            return Ok(false);
        }
        let ability_blocked = !context.suppress_target_ability
            && self
                .side(target.side)
                .active
                .into_iter()
                .flatten()
                .any(|index| {
                    self.side(target.side)
                        .pokemon(index)
                        .is_some_and(|pokemon| {
                            data.ability_has_behavior(
                                pokemon.ability,
                                crate::AbilityBehavior::PriorityProtection,
                            )
                        })
                });
        if ability_blocked {
            return Ok(true);
        }
        Ok(field_effects.field_has(self, field_effects.psychic_terrain)
            && crate::field_effects::grounded(self, data, field_effects, target)?)
    }

    fn apply_field_base_power<D: CombatData>(
        &self,
        data: &D,
        field_effects: &FieldEffectIds,
        source: PokemonRef,
        target: PokemonRef,
        move_id: MoveId,
        move_data: &mut CombatMove,
    ) -> Result<(), BattleError> {
        let source_grounded = crate::field_effects::grounded(self, data, field_effects, source)?;
        let target_grounded = crate::field_effects::grounded(self, data, field_effects, target)?;
        let mut base_power = u32::from(move_data.base_power);
        if source_grounded
            && ((move_data.move_type == PokemonType::Electric
                && field_effects.field_has(self, field_effects.electric_terrain))
                || (move_data.move_type == PokemonType::Grass
                    && field_effects.field_has(self, field_effects.grassy_terrain))
                || (move_data.move_type == PokemonType::Psychic
                    && field_effects.field_has(self, field_effects.psychic_terrain)))
        {
            base_power = showdown_modify(base_power, 5325, 4096);
        }
        if target_grounded
            && move_data.move_type == PokemonType::Dragon
            && field_effects.field_has(self, field_effects.misty_terrain)
        {
            base_power = showdown_modify(base_power, 1, 2);
        }
        if target_grounded
            && field_effects.field_has(self, field_effects.grassy_terrain)
            && field_effects.grassy_weakens(move_id)
        {
            base_power = showdown_modify(base_power, 1, 2);
        }
        move_data.base_power = u16::try_from(base_power.max(1)).unwrap_or(u16::MAX);
        Ok(())
    }

    fn apply_field_defense<D: CombatData>(
        &self,
        data: &D,
        field_effects: &FieldEffectIds,
        category: MoveCategory,
        target_state: &mut PokemonState,
    ) -> Result<Option<(u32, u32)>, BattleError> {
        if field_effects.field_has(self, field_effects.wonder_room) {
            std::mem::swap(&mut target_state.stats.def, &mut target_state.stats.spd);
        }
        let types = data
            .pokemon_types(target_state.species, target_state.form)
            .ok_or_else(|| {
                BattleError::DataMismatch("missing field-defense type data".to_owned())
            })?;
        let has_type = |kind| types.primary == kind || types.secondary == Some(kind);
        if category == MoveCategory::Special
            && has_type(PokemonType::Rock)
            && field_effects.field_has(self, field_effects.sandstorm)
        {
            return Ok(Some((3, 2)));
        }
        if category == MoveCategory::Physical
            && has_type(PokemonType::Ice)
            && field_effects.field_has(self, field_effects.snow)
        {
            return Ok(Some((3, 2)));
        }
        Ok(None)
    }

    fn resolve_queued_switch<D: CombatData>(
        &mut self,
        data: &D,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        action: QueuedAction,
        incoming: PokemonIndex,
        resolution: &mut TurnResolution,
    ) -> Result<bool, BattleError> {
        let side = action.side;
        let slot = action.slot;
        let outgoing = self.side(side).active[slot.index()].ok_or_else(|| {
            BattleError::InvalidState("queued switch has no outgoing Pokémon".to_owned())
        })?;
        let context = EventContext {
            source: Some(PokemonRef::new(side, outgoing)),
            target: Some(PokemonRef::new(side, incoming)),
            move_id: None,
            ..EventContext::default()
        };
        let switch_out =
            effects.dispatch(self, BattleEvent::SwitchOut, context, EventValue::None)?;
        let cancelled = switch_out.cancelled;
        resolution.events.push(switch_out);
        if cancelled {
            return Ok(false);
        }
        StateKernel::new(self).switch_active_position(side, slot, incoming)?;
        self.apply_direct_switch_in_abilities(
            data,
            effects,
            field_effects,
            PokemonRef::new(side, incoming),
            resolution,
        )?;
        resolution.events.push(effects.dispatch(
            self,
            BattleEvent::SwitchIn,
            EventContext {
                source: Some(PokemonRef::new(side, incoming)),
                target: Some(PokemonRef::new(side, outgoing)),
                move_id: None,
                ..EventContext::default()
            },
            EventValue::None,
        )?);
        Ok(true)
    }

    fn apply_direct_switch_in_abilities<D: CombatData>(
        &mut self,
        data: &D,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        source: PokemonRef,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let ability = self
            .side(source.side)
            .pokemon(source.pokemon)
            .ok_or_else(|| BattleError::InvalidState("switch-in source is invalid".to_owned()))?
            .ability;
        self.apply_switch_in_weather(data, field_effects, ability)?;
        self.apply_switch_in_terrain(data, field_effects, ability)?;
        if data.ability_has_behavior(ability, crate::AbilityBehavior::ScreenCleaner) {
            self.clear_screens(field_effects);
        }
        if data.ability_has_behavior(ability, crate::AbilityBehavior::CuriousMedicine) {
            self.clear_active_ally_stages(source)?;
        }
        if data.ability_has_behavior(ability, crate::AbilityBehavior::Intimidate) {
            for target in self
                .side(source.side.opponent())
                .active
                .into_iter()
                .flatten()
                .map(|pokemon| PokemonRef::new(source.side.opponent(), pokemon))
                .collect::<Vec<_>>()
            {
                let context = EventContext {
                    source: Some(source),
                    target: Some(target),
                    requested_stat: Some(BattleStat::Attack),
                    intimidate: true,
                    ..EventContext::default()
                };
                let event = effects.dispatch(
                    self,
                    BattleEvent::TryStatStageChange,
                    context,
                    EventValue::Integer(-1),
                )?;
                let EventValue::Integer(delta) = event.value else {
                    return Err(BattleError::InvalidState(
                        "Intimidate stage relay is not an integer".to_owned(),
                    ));
                };
                resolution.events.push(event);
                let delta = i8::try_from(delta).map_err(|_| {
                    BattleError::InvalidState("Intimidate stage relay is out of range".to_owned())
                })?;
                let applied = StateKernel::new(self)
                    .change_stat_stage(target, BattleStat::Attack, delta)?
                    .applied_delta;
                if applied != 0 {
                    resolution.events.push(effects.dispatch(
                        self,
                        BattleEvent::AfterStatStageChange,
                        context,
                        EventValue::Integer(i64::from(applied)),
                    )?);
                }
            }
        }
        if data.ability_has_behavior(ability, crate::AbilityBehavior::SupersweetSyrup) {
            for target in self
                .side(source.side.opponent())
                .active
                .into_iter()
                .flatten()
                .map(|pokemon| PokemonRef::new(source.side.opponent(), pokemon))
                .collect::<Vec<_>>()
            {
                StateKernel::new(self).change_stat_stage(target, BattleStat::Evasion, -1)?;
            }
        }
        Ok(())
    }

    fn apply_switch_in_weather<D: CombatData>(
        &mut self,
        data: &D,
        field_effects: &FieldEffectIds,
        ability: AbilityId,
    ) -> Result<(), BattleError> {
        for (behavior, condition) in [
            (
                crate::AbilityBehavior::WeatherSetter(WeatherEnvironment::Rain),
                field_effects.rain,
            ),
            (
                crate::AbilityBehavior::WeatherSetter(WeatherEnvironment::Sun),
                field_effects.sun,
            ),
            (
                crate::AbilityBehavior::WeatherSetter(WeatherEnvironment::Sandstorm),
                field_effects.sandstorm,
            ),
            (
                crate::AbilityBehavior::WeatherSetter(WeatherEnvironment::Snow),
                field_effects.snow,
            ),
        ] {
            if data.ability_has_behavior(ability, behavior) {
                let condition = condition.ok_or_else(|| {
                    BattleError::DataMismatch(
                        "weather-setting ability has no condition identity".to_owned(),
                    )
                })?;
                StateKernel::new(self).set_weather(Some(ConditionState::new(condition, None)))?;
            }
        }
        Ok(())
    }

    fn apply_reactive_field_abilities<D: CombatData>(
        &mut self,
        data: &D,
        field_effects: &FieldEffectIds,
        target: PokemonRef,
        applied_damage: u16,
    ) -> Result<(), BattleError> {
        if applied_damage == 0 {
            return Ok(());
        }
        let ability = self
            .side(target.side)
            .pokemon(target.pokemon)
            .ok_or_else(|| BattleError::InvalidState("damage target is invalid".to_owned()))?
            .ability;
        if data.ability_has_behavior(
            ability,
            crate::AbilityBehavior::ReactiveWeather(WeatherEnvironment::Sandstorm),
        ) {
            let condition = field_effects.sandstorm.ok_or_else(|| {
                BattleError::DataMismatch("Sand Spit has no weather identity".to_owned())
            })?;
            StateKernel::new(self).set_weather(Some(ConditionState::new(condition, Some(5))))?;
        }
        if data.ability_has_behavior(
            ability,
            crate::AbilityBehavior::ReactiveTerrain(TerrainEnvironment::Grassy),
        ) {
            let condition = field_effects.grassy_terrain.ok_or_else(|| {
                BattleError::DataMismatch("Seed Sower has no terrain identity".to_owned())
            })?;
            StateKernel::new(self).set_terrain(Some(ConditionState::new(condition, Some(5))))?;
        }
        Ok(())
    }

    fn apply_switch_in_terrain<D: CombatData>(
        &mut self,
        data: &D,
        field_effects: &FieldEffectIds,
        ability: AbilityId,
    ) -> Result<(), BattleError> {
        for (behavior, condition) in [
            (
                crate::AbilityBehavior::TerrainSetter(TerrainEnvironment::Electric),
                field_effects.electric_terrain,
            ),
            (
                crate::AbilityBehavior::TerrainSetter(TerrainEnvironment::Grassy),
                field_effects.grassy_terrain,
            ),
            (
                crate::AbilityBehavior::TerrainSetter(TerrainEnvironment::Psychic),
                field_effects.psychic_terrain,
            ),
        ] {
            if data.ability_has_behavior(ability, behavior) {
                let condition = condition.ok_or_else(|| {
                    BattleError::DataMismatch(
                        "terrain-setting ability has no condition identity".to_owned(),
                    )
                })?;
                StateKernel::new(self)
                    .set_terrain(Some(ConditionState::new(condition, Some(5))))?;
            }
        }
        Ok(())
    }

    fn clear_screens(&mut self, field_effects: &FieldEffectIds) {
        for side in [SideId::P1, SideId::P2] {
            for condition in [
                field_effects.reflect,
                field_effects.light_screen,
                field_effects.aurora_veil,
            ]
            .into_iter()
            .flatten()
            {
                StateKernel::new(self).remove_side_condition(side, condition);
            }
        }
    }

    fn clear_active_ally_stages(&mut self, source: PokemonRef) -> Result<(), BattleError> {
        let allies = self
            .side(source.side)
            .active
            .into_iter()
            .flatten()
            .filter(|ally| *ally != source.pokemon)
            .collect::<Vec<_>>();
        for ally in allies {
            let ally = PokemonRef::new(source.side, ally);
            let stages = self
                .side(ally.side)
                .pokemon(ally.pokemon)
                .ok_or_else(|| {
                    BattleError::InvalidState("Curious Medicine ally is invalid".to_owned())
                })?
                .stat_stages;
            for stat in [
                BattleStat::Attack,
                BattleStat::Defense,
                BattleStat::SpecialAttack,
                BattleStat::SpecialDefense,
                BattleStat::Speed,
                BattleStat::Accuracy,
                BattleStat::Evasion,
            ] {
                StateKernel::new(self).change_stat_stage(ally, stat, -stages.get(stat))?;
            }
        }
        Ok(())
    }

    fn process_weather_residuals<D: CombatData>(
        &mut self,
        data: &D,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        if field_effects.weather_environment(self).is_some()
            && field_effects
                .effective_weather_environment(self, data)
                .is_none()
        {
            return Ok(());
        }
        let active = [SideId::P1, SideId::P2]
            .into_iter()
            .flat_map(|side| {
                self.side(side)
                    .active
                    .into_iter()
                    .flatten()
                    .map(move |pokemon| PokemonRef::new(side, pokemon))
            })
            .collect::<Vec<_>>();
        if let Some(condition) = field_effects
            .sandstorm
            .filter(|id| field_effects.field_has(self, Some(*id)))
        {
            for target in active.iter().copied() {
                let pokemon = self
                    .side(target.side)
                    .pokemon(target.pokemon)
                    .ok_or_else(|| {
                        BattleError::InvalidState("sandstorm target is invalid".to_owned())
                    })?;
                let types = data
                    .pokemon_types(pokemon.species, pokemon.form)
                    .ok_or_else(|| {
                        BattleError::DataMismatch("missing sandstorm type data".to_owned())
                    })?;
                let immune = [PokemonType::Rock, PokemonType::Ground, PokemonType::Steel]
                    .into_iter()
                    .any(|kind| types.primary == kind || types.secondary == Some(kind));
                if !immune {
                    let max_hp = pokemon.stats.hp;
                    let weather_immunity = effects.dispatch(
                        self,
                        BattleEvent::WeatherImmunity,
                        EventContext {
                            target: Some(target),
                            weather: Some(WeatherEnvironment::Sandstorm),
                            ..EventContext::default()
                        },
                        EventValue::Boolean(true),
                    )?;
                    let ability_immune = weather_immunity.cancelled
                        || weather_immunity.value == EventValue::Boolean(false);
                    resolution.events.push(weather_immunity);
                    if ability_immune {
                        continue;
                    }
                    let requested = (max_hp / 16).max(1);
                    let applied = StateKernel::new(self).damage_hp(target, requested)?.applied;
                    resolution.field_hp_changes.push(FieldHpChange {
                        pokemon: target,
                        condition,
                        amount: -i32::from(applied),
                    });
                }
            }
        }
        Ok(())
    }

    fn process_terrain_residuals<D: CombatData>(
        &mut self,
        data: &D,
        field_effects: &FieldEffectIds,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let active = [SideId::P1, SideId::P2]
            .into_iter()
            .flat_map(|side| {
                self.side(side)
                    .active
                    .into_iter()
                    .flatten()
                    .map(move |pokemon| PokemonRef::new(side, pokemon))
            })
            .collect::<Vec<_>>();
        if let Some(condition) = field_effects
            .grassy_terrain
            .filter(|id| field_effects.field_has(self, Some(*id)))
        {
            for target in active {
                if !crate::field_effects::grounded(self, data, field_effects, target)? {
                    continue;
                }
                let pokemon = self
                    .side(target.side)
                    .pokemon(target.pokemon)
                    .ok_or_else(|| {
                        BattleError::InvalidState("grassy terrain target is invalid".to_owned())
                    })?;
                if pokemon.current_hp == 0 {
                    continue;
                }
                let requested = (pokemon.stats.hp / 16).max(1);
                let applied = StateKernel::new(self).heal_hp(target, requested)?.applied;
                if applied > 0 {
                    resolution.field_hp_changes.push(FieldHpChange {
                        pokemon: target,
                        condition,
                        amount: i32::from(applied),
                    });
                }
            }
        }
        Ok(())
    }

    fn tick_weather_duration(&mut self, resolution: &mut TurnResolution) {
        let mut kernel = StateKernel::new(self);
        if let Some(condition) = kernel.tick_weather_duration() {
            resolution
                .expired_conditions
                .push(ExpiredCondition::Weather { condition });
        }
    }

    fn tick_side_durations(&mut self, resolution: &mut TurnResolution) {
        let mut kernel = StateKernel::new(self);
        for side in [SideId::P1, SideId::P2] {
            resolution.expired_conditions.extend(
                kernel
                    .tick_side_condition_durations(side)
                    .into_iter()
                    .map(|condition| ExpiredCondition::Side { side, condition }),
            );
        }
    }

    fn tick_late_field_durations(
        &mut self,
        field_effects: &FieldEffectIds,
        resolution: &mut TurnResolution,
    ) {
        let mut kernel = StateKernel::new(self);
        let mut expired = kernel
            .tick_field_condition_durations()
            .into_iter()
            .map(|condition| ExpiredCondition::Field { condition })
            .collect::<Vec<_>>();
        if let Some(condition) = kernel.tick_terrain_duration() {
            expired.push(ExpiredCondition::Terrain { condition });
        }
        expired.sort_by_key(|condition| match *condition {
            ExpiredCondition::Field { condition } | ExpiredCondition::Terrain { condition } => {
                field_effects.field_residual_sub_order(condition)
            }
            ExpiredCondition::Weather { .. } | ExpiredCondition::Side { .. } => 0,
        });
        resolution.expired_conditions.extend(expired);
    }

    fn tick_volatile_durations(
        &mut self,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        for side in [SideId::P1, SideId::P2] {
            for pokemon in self.side(side).active.into_iter().flatten() {
                let pokemon_ref = PokemonRef::new(side, pokemon);
                let expired = StateKernel::new(self).tick_volatile_durations(pokemon_ref)?;
                resolution
                    .expired_volatiles
                    .extend(expired.into_iter().map(|condition| ExpiredVolatile {
                        pokemon: pokemon_ref,
                        condition,
                    }));
            }
        }
        Ok(())
    }

    fn active_status_hp(&self) -> Result<Vec<(PokemonRef, MajorStatus, u16)>, BattleError> {
        let mut output = Vec::new();
        for side in [SideId::P1, SideId::P2] {
            for pokemon in self.side(side).active.into_iter().flatten() {
                let state = self.side(side).pokemon(pokemon).ok_or_else(|| {
                    BattleError::InvalidState(
                        "active Pokémon is invalid before residual processing".to_owned(),
                    )
                })?;
                if let Some(status) = state.status {
                    output.push((PokemonRef::new(side, pokemon), status, state.current_hp));
                }
            }
        }
        Ok(output)
    }

    fn record_residual_damage(
        &self,
        before: &[(PokemonRef, MajorStatus, u16)],
        residual: &EventDispatch,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let mut ordered = Vec::new();
        let mut seen = HashSet::new();
        for invocation in &residual.invocations {
            let crate::EffectOwner::Pokemon { pokemon } = invocation.owner else {
                continue;
            };
            if seen.insert(pokemon) {
                ordered.push(pokemon);
            }
        }
        ordered.extend(
            before
                .iter()
                .map(|(pokemon, _, _)| *pokemon)
                .filter(|pokemon| seen.insert(*pokemon)),
        );
        for pokemon in ordered {
            let Some(&(_, status, hp)) = before
                .iter()
                .find(|(candidate, _, _)| *candidate == pokemon)
            else {
                continue;
            };
            let current_hp = self
                .side(pokemon.side)
                .pokemon(pokemon.pokemon)
                .ok_or_else(|| {
                    BattleError::InvalidState(
                        "status holder became invalid during residual processing".to_owned(),
                    )
                })?
                .current_hp;
            if current_hp < hp {
                resolution.residual_damage.push(ResidualDamage {
                    pokemon,
                    status,
                    damage: hp - current_hp,
                });
            }
        }
        Ok(())
    }

    fn process_residual_faints(
        &mut self,
        effects: &EffectRegistry,
        residual_invocations: &[crate::EffectInvocation],
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let mut ordered = Vec::new();
        let mut seen = HashSet::new();
        for invocation in residual_invocations {
            let crate::EffectOwner::Pokemon { pokemon } = invocation.owner else {
                continue;
            };
            if seen.insert(pokemon) && self.pokemon_has_zero_hp(pokemon)? {
                ordered.push(pokemon);
            }
        }
        for side in [SideId::P1, SideId::P2] {
            for pokemon in self.side(side).active.into_iter().flatten() {
                let pokemon = PokemonRef::new(side, pokemon);
                if seen.insert(pokemon) && self.pokemon_has_zero_hp(pokemon)? {
                    ordered.push(pokemon);
                }
            }
        }
        if ordered.is_empty() {
            return Ok(());
        }
        let mut targets = Vec::with_capacity(ordered.len());
        for pokemon in ordered {
            let slot = match self
                .side(pokemon.side)
                .pokemon(pokemon.pokemon)
                .ok_or_else(|| {
                    BattleError::InvalidState("residual faint target is invalid".to_owned())
                })?
                .position
            {
                crate::PokemonPosition::Active { slot } => slot,
                crate::PokemonPosition::Team | crate::PokemonPosition::Fainted => {
                    return Err(BattleError::InvalidState(
                        "residual faint target is not active".to_owned(),
                    ));
                }
            };
            let context = EventContext {
                source: None,
                target: Some(pokemon),
                move_id: None,
                ..EventContext::default()
            };
            resolution.events.push(effects.dispatch(
                self,
                BattleEvent::Faint,
                context,
                EventValue::None,
            )?);
            targets.push(FaintTarget::new(pokemon.side, slot));
        }
        self.process_faints(&targets)
    }

    fn pokemon_has_zero_hp(&self, pokemon: PokemonRef) -> Result<bool, BattleError> {
        Ok(self
            .side(pokemon.side)
            .pokemon(pokemon.pokemon)
            .ok_or_else(|| {
                BattleError::InvalidState("residual effect owner is invalid".to_owned())
            })?
            .current_hp
            == 0)
    }
}

#[allow(clippy::too_many_arguments)]
/// Calculates one normal Gen 9 damage result from already-resolved inputs.
///
/// # Errors
///
/// Returns an error for status moves or a random percentage outside 85–100.
pub fn calculate_damage(
    source: &PokemonState,
    target: &PokemonState,
    move_data: CombatMove,
    source_types: PokemonTypes,
    type_exponent: Option<i8>,
    critical: bool,
    random_percent: u8,
) -> Result<DamageResult, BattleError> {
    calculate_damage_inner(
        source,
        target,
        move_data,
        source_types,
        type_exponent,
        critical,
        random_percent,
        DamageEnvironment::default(),
    )
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
fn calculate_damage_inner(
    source: &PokemonState,
    target: &PokemonState,
    move_data: CombatMove,
    source_types: PokemonTypes,
    type_exponent: Option<i8>,
    critical: bool,
    random_percent: u8,
    environment: DamageEnvironment,
) -> Result<DamageResult, BattleError> {
    if !(85..=100).contains(&random_percent) {
        return Err(BattleError::InvalidState(
            "damage roll must be between 85 and 100".to_owned(),
        ));
    }
    let (raw_attack, raw_defense, attack_stage, defense_stage) = match move_data.category {
        MoveCategory::Physical => (
            source.stats.atk,
            target.stats.def,
            source.stat_stages.atk,
            target.stat_stages.def,
        ),
        MoveCategory::Special => (
            source.stats.spa,
            target.stats.spd,
            source.stat_stages.spa,
            target.stat_stages.spd,
        ),
        MoveCategory::Status => {
            return Err(BattleError::UnsupportedMechanic(
                "status moves do not use the damage formula".to_owned(),
            ));
        }
    };
    let attack_stage = if critical && attack_stage < 0 {
        0
    } else {
        attack_stage
    };
    let defense_stage = if critical && defense_stage > 0 {
        0
    } else {
        defense_stage
    };
    let attack = environment
        .attack_override
        .unwrap_or_else(|| staged_stat(raw_attack, attack_stage));
    let mut defense = environment
        .defense_override
        .unwrap_or_else(|| staged_stat(raw_defense, defense_stage))
        .max(1);
    if let Some((numerator, denominator)) = environment.defense_modifier {
        defense = showdown_modify(defense, numerator, denominator).max(1);
    }
    let level_factor = (2 * u32::from(source.level) / 5) + 2;
    let base_damage = level_factor
        .saturating_mul(u32::from(move_data.base_power))
        .saturating_mul(attack)
        / defense
        / 50;
    let mut final_damage = base_damage + 2;
    if environment.spread {
        final_damage = showdown_modify(final_damage, 3, 4);
    }
    if let Some((numerator, denominator)) = environment.weather {
        final_damage = showdown_modify(final_damage, numerator, denominator);
    }
    let critical_modifier = if critical {
        final_damage = final_damage.saturating_mul(3) / 2;
        Modifier {
            numerator: 3,
            denominator: 2,
        }
    } else {
        Modifier::NEUTRAL
    };
    final_damage = final_damage.saturating_mul(u32::from(random_percent)) / 100;
    let has_stab = source_types.primary == move_data.move_type
        || source_types.secondary == Some(move_data.move_type);
    let stab_modifier = if has_stab {
        let (numerator, denominator) = if environment.adaptability {
            (2, 1)
        } else {
            (3, 2)
        };
        final_damage = showdown_modify(final_damage, numerator, denominator);
        Modifier {
            numerator: u16::try_from(numerator).unwrap_or(u16::MAX),
            denominator: u16::try_from(denominator).unwrap_or(u16::MAX),
        }
    } else {
        Modifier::NEUTRAL
    };
    let (typed_damage, type_modifier) = apply_type_modifier(final_damage, type_exponent);
    final_damage = typed_damage;
    if source.status == Some(MajorStatus::Burn)
        && move_data.category == MoveCategory::Physical
        && !environment.ignore_burn
    {
        final_damage /= 2;
    }
    if type_exponent.is_some() && final_damage == 0 {
        final_damage = 1;
    }
    Ok(DamageResult {
        base_damage,
        attack_stat: attack,
        defense_stat: defense,
        stab_modifier,
        type_modifier,
        critical_modifier,
        random_modifier: Modifier {
            numerator: u16::from(random_percent),
            denominator: 100,
        },
        final_damage: u16::try_from(final_damage.min(u32::from(u16::MAX))).unwrap_or(u16::MAX),
    })
}

fn apply_type_modifier(damage: u32, exponent: Option<i8>) -> (u32, Modifier) {
    match exponent {
        None => (
            0,
            Modifier {
                numerator: 0,
                denominator: 1,
            },
        ),
        Some(value) if value > 0 => {
            let factor = 1_u16
                .checked_shl(u32::from(value.unsigned_abs()))
                .unwrap_or(u16::MAX);
            (
                damage.saturating_mul(u32::from(factor)),
                Modifier {
                    numerator: factor,
                    denominator: 1,
                },
            )
        }
        Some(value) if value < 0 => {
            let factor = 1_u16
                .checked_shl(u32::from(value.unsigned_abs()))
                .unwrap_or(u16::MAX);
            (
                damage / u32::from(factor),
                Modifier {
                    numerator: 1,
                    denominator: factor,
                },
            )
        }
        Some(_) => (damage, Modifier::NEUTRAL),
    }
}

/// Resolves static typing and calculates one forced-outcome damage value.
///
/// This is the deterministic seam used by focused Showdown differential
/// fixtures. Turn resolution supplies its rolled critical and random values.
///
/// # Errors
///
/// Returns an error when static data is missing, the move requires a capability
/// outside the static damage seam, or damage inputs are invalid. Moves with
/// state-dependent power or effective-stat behavior must run through the
/// authoritative transition executor.
pub fn calculate_move_damage<D: CombatData>(
    data: &D,
    source: &PokemonState,
    target: &PokemonState,
    move_id: MoveId,
    critical: bool,
    random_percent: u8,
) -> Result<DamageResult, BattleError> {
    let move_data = data.move_data(move_id).ok_or_else(|| {
        BattleError::DataMismatch(format!("missing combat data for move {move_id}"))
    })?;
    let transition_only = [
        MoveCapability::ConditionalPower,
        MoveCapability::EffectiveStatOverride,
    ]
    .into_iter()
    .find(|capability| move_data.required_capabilities.contains(*capability));
    if let Some(capability) = transition_only {
        return Err(BattleError::UnsupportedMechanic(format!(
            "move {move_id} requires transition-only capability {capability}"
        )));
    }
    let secondaries = data.secondary_move_effects(move_id);
    let self_effect = data.self_move_effect(move_id);
    preflight_move(
        move_id,
        move_data,
        MoveIntrinsic::Ordinary,
        &[],
        &secondaries,
        self_effect.as_ref(),
    )?;
    let source_types = data
        .pokemon_types(source.species, source.form)
        .ok_or_else(|| BattleError::DataMismatch("missing source type data".to_owned()))?;
    let target_types = data
        .pokemon_types(target.species, target.form)
        .ok_or_else(|| BattleError::DataMismatch("missing target type data".to_owned()))?;
    let type_exponent = combined_type_modifier(data, move_data.move_type, target_types)?;
    calculate_damage(
        source,
        target,
        move_data,
        source_types,
        type_exponent,
        critical,
        random_percent,
    )
}

fn staged_stat(stat: u16, stage: i8) -> u32 {
    let stage = stage.clamp(-6, 6);
    if stage >= 0 {
        u32::from(stat) * u32::try_from(2 + stage).unwrap_or(8) / 2
    } else {
        u32::from(stat) * 2 / u32::try_from(2 - stage).unwrap_or(8)
    }
}

fn effective_damage_stats(
    source: &PokemonState,
    target: &PokemonState,
    move_data: CombatMove,
    critical: bool,
) -> Result<(u32, u32), BattleError> {
    let (default_attack, default_defense) = match move_data.category {
        MoveCategory::Physical => (BattleStat::Attack, BattleStat::Defense),
        MoveCategory::Special => (BattleStat::SpecialAttack, BattleStat::SpecialDefense),
        MoveCategory::Status => {
            return Err(BattleError::DataMismatch(
                "status move requested effective damage stats".to_owned(),
            ));
        }
    };
    let attack_stat = move_data.offensive_stat.unwrap_or(default_attack);
    let defense_stat = move_data.defensive_stat.unwrap_or(default_defense);
    let raw = |pokemon: &PokemonState, stat: BattleStat| match stat {
        BattleStat::Attack => Some(pokemon.stats.atk),
        BattleStat::Defense => Some(pokemon.stats.def),
        BattleStat::SpecialAttack => Some(pokemon.stats.spa),
        BattleStat::SpecialDefense => Some(pokemon.stats.spd),
        BattleStat::Speed => Some(pokemon.stats.spe),
        BattleStat::Accuracy | BattleStat::Evasion => None,
    };
    let mut attack_stage = source.stat_stages.get(attack_stat);
    let mut defense_stage = target.stat_stages.get(defense_stat);
    if (critical || move_data.ignore_negative_offensive) && attack_stage < 0 {
        attack_stage = 0;
    }
    if (critical || move_data.ignore_positive_defensive) && defense_stage > 0 {
        defense_stage = 0;
    }
    let attack = raw(source, attack_stat).ok_or_else(|| {
        BattleError::DataMismatch("move declares an invalid offensive stat override".to_owned())
    })?;
    let defense = raw(target, defense_stat).ok_or_else(|| {
        BattleError::DataMismatch("move declares an invalid defensive stat override".to_owned())
    })?;
    Ok((
        staged_stat(attack, attack_stage),
        staged_stat(defense, defense_stage).max(1),
    ))
}

fn offensive_stat_event(move_data: CombatMove) -> Result<BattleEvent, BattleError> {
    let default = match move_data.category {
        MoveCategory::Physical => BattleStat::Attack,
        MoveCategory::Special => BattleStat::SpecialAttack,
        MoveCategory::Status => {
            return Err(BattleError::DataMismatch(
                "status move requested an offensive stat event".to_owned(),
            ));
        }
    };
    match move_data.offensive_stat.unwrap_or(default) {
        BattleStat::Attack => Ok(BattleEvent::ModifyAttack),
        BattleStat::SpecialAttack => Ok(BattleEvent::ModifySpecialAttack),
        BattleStat::Defense => Ok(BattleEvent::ModifySourceDefense),
        BattleStat::SpecialDefense => Ok(BattleEvent::ModifySourceSpecialDefense),
        BattleStat::Speed | BattleStat::Accuracy | BattleStat::Evasion => Err(
            BattleError::DataMismatch("move selected a non-damage calculation stat".to_owned()),
        ),
    }
}

fn defensive_stat_event(move_data: CombatMove) -> Result<BattleEvent, BattleError> {
    let default = match move_data.category {
        MoveCategory::Physical => BattleStat::Defense,
        MoveCategory::Special => BattleStat::SpecialDefense,
        MoveCategory::Status => {
            return Err(BattleError::DataMismatch(
                "status move requested a defensive stat event".to_owned(),
            ));
        }
    };
    match move_data.defensive_stat.unwrap_or(default) {
        BattleStat::Defense => Ok(BattleEvent::ModifyDefense),
        BattleStat::SpecialDefense => Ok(BattleEvent::ModifySpecialDefense),
        BattleStat::Attack
        | BattleStat::SpecialAttack
        | BattleStat::Speed
        | BattleStat::Accuracy
        | BattleStat::Evasion => Err(BattleError::DataMismatch(
            "move selected a non-damage calculation stat".to_owned(),
        )),
    }
}

fn integer_relay_u16(dispatch: &EventDispatch, name: &str) -> Result<u16, BattleError> {
    let EventValue::Integer(value) = dispatch.value else {
        return Err(BattleError::InvalidState(format!(
            "{name} replaced its integer relay with an incompatible value"
        )));
    };
    u16::try_from(value).map_err(|_| {
        BattleError::InvalidState(format!("{name} produced a value outside the u16 range"))
    })
}

fn integer_relay_u8(dispatch: &EventDispatch, name: &str) -> Result<u8, BattleError> {
    let EventValue::Integer(value) = dispatch.value else {
        return Err(BattleError::InvalidState(format!(
            "{name} replaced its integer relay with an incompatible value"
        )));
    };
    u8::try_from(value).map_err(|_| {
        BattleError::InvalidState(format!("{name} produced a value outside the u8 range"))
    })
}

fn boolean_relay(dispatch: &EventDispatch, name: &str) -> Result<bool, BattleError> {
    let EventValue::Boolean(value) = dispatch.value else {
        return Err(BattleError::InvalidState(format!(
            "{name} replaced its boolean relay with an incompatible value"
        )));
    };
    Ok(value)
}

fn integer_relay_u32(dispatch: &EventDispatch, name: &str) -> Result<u32, BattleError> {
    let EventValue::Integer(value) = dispatch.value else {
        return Err(BattleError::InvalidState(format!(
            "{name} replaced its integer relay with an incompatible value"
        )));
    };
    u32::try_from(value).map_err(|_| {
        BattleError::InvalidState(format!("{name} produced a value outside the u32 range"))
    })
}

fn facade_activates(status: Option<MajorStatus>) -> bool {
    matches!(
        status,
        Some(
            MajorStatus::Burn
                | MajorStatus::Paralysis
                | MajorStatus::Poison
                | MajorStatus::BadlyPoisoned
        )
    )
}

fn rounded_fraction(value: u32, fraction: EffectFraction) -> Result<u32, BattleError> {
    if fraction.denominator == 0 {
        return Err(BattleError::DataMismatch(
            "move fraction denominator is zero".to_owned(),
        ));
    }
    Ok(value
        .saturating_mul(u32::from(fraction.numerator))
        .saturating_add(u32::from(fraction.denominator) / 2)
        / u32::from(fraction.denominator))
}

#[allow(clippy::too_many_lines)]
fn apply_type_conversion<D: CombatData>(
    data: &D,
    ability: AbilityId,
    move_data: &mut CombatMove,
) -> bool {
    if move_data.move_type != PokemonType::Normal {
        return false;
    }
    for converted_type in [
        PokemonType::Flying,
        PokemonType::Fairy,
        PokemonType::Ice,
        PokemonType::Dragon,
    ] {
        if data.ability_has_behavior(
            ability,
            crate::AbilityBehavior::TypeConversion(converted_type),
        ) {
            move_data.move_type = converted_type;
            return true;
        }
    }
    false
}

#[allow(clippy::too_many_lines)] // Capability validation is one cohesive, linear boundary check.
fn preflight_move(
    move_id: MoveId,
    move_data: CombatMove,
    intrinsic: MoveIntrinsic,
    primary_effects: &[PrimaryMoveEffect],
    secondary_effects: &[SecondaryMoveEffect],
    self_effect: Option<&SelfMoveEffect>,
) -> Result<(), BattleError> {
    if let Some(missing) = move_data
        .required_capabilities
        .first_missing_from(MoveCapabilities::PR_13_4)
    {
        return Err(BattleError::UnsupportedMechanic(format!(
            "move {move_id} requires unsupported capability {missing}"
        )));
    }
    if move_data
        .required_capabilities
        .contains(MoveCapability::SecondaryEffect)
        && secondary_effects.is_empty()
    {
        return Err(BattleError::UnsupportedMechanic(format!(
            "move {move_id} requires secondary effect data"
        )));
    }
    if move_data
        .required_capabilities
        .contains(MoveCapability::SelfEffect)
        && self_effect.is_none()
    {
        return Err(BattleError::UnsupportedMechanic(format!(
            "move {move_id} requires self-effect data"
        )));
    }
    if move_data
        .required_capabilities
        .contains(MoveCapability::MultiHit)
        && move_data.hit_sequence == HitSequence::Single
    {
        return Err(BattleError::UnsupportedMechanic(format!(
            "move {move_id} requires multi-hit sequence data"
        )));
    }
    if move_data
        .required_capabilities
        .contains(MoveCapability::Drain)
        && move_data.drain.is_none()
    {
        return Err(BattleError::UnsupportedMechanic(format!(
            "move {move_id} requires drain data"
        )));
    }
    if move_data
        .required_capabilities
        .contains(MoveCapability::Recoil)
        && move_data.recoil.is_none()
        && move_data.self_damage.is_none()
    {
        return Err(BattleError::UnsupportedMechanic(format!(
            "move {move_id} requires recoil or self-damage data"
        )));
    }
    if move_data
        .required_capabilities
        .contains(MoveCapability::ConditionalPower)
        && move_data.prior_effect == PriorMoveEffect::None
    {
        return Err(BattleError::UnsupportedMechanic(format!(
            "move {move_id} requires conditional-power data"
        )));
    }
    if move_data
        .required_capabilities
        .contains(MoveCapability::EffectiveStatOverride)
        && move_data.offensive_stat.is_none()
        && move_data.defensive_stat.is_none()
        && !move_data.ignore_negative_offensive
        && !move_data.ignore_positive_defensive
    {
        return Err(BattleError::UnsupportedMechanic(format!(
            "move {move_id} requires effective-stat override data"
        )));
    }
    let valid_damage = move_data.operation == MoveOperation::StandardDamage
        && move_data.category != MoveCategory::Status
        && move_data.base_power > 0;
    let valid_fixed = match move_data.operation {
        MoveOperation::FixedDamage => move_data.base_power == 0 && move_data.fixed_damage.is_some(),
        MoveOperation::LevelDamage => move_data.base_power == 0 && move_data.fixed_damage.is_none(),
        MoveOperation::StandardDamage | MoveOperation::NonDamage => false,
    };
    let valid_intrinsic = move_data.operation == MoveOperation::NonDamage
        && move_data.category == MoveCategory::Status
        && move_data.base_power == 0
        && (intrinsic != MoveIntrinsic::Ordinary || !primary_effects.is_empty());
    if !valid_damage && !valid_fixed && !valid_intrinsic {
        return Err(BattleError::DataMismatch(format!(
            "move {move_id} declares inconsistent operation metadata"
        )));
    }
    if matches!(
        move_data.target,
        MoveTarget::RandomNormal | MoveTarget::Scripted
    ) {
        return Err(BattleError::DataMismatch(format!(
            "move {move_id} declares selected-target support for an incompatible target"
        )));
    }
    Ok(())
}

fn showdown_modify(value: u32, numerator: u32, denominator: u32) -> u32 {
    let modifier = numerator.saturating_mul(4096) / denominator;
    value.saturating_mul(modifier).saturating_add(2047) / 4096
}

fn apply_damage_modifier(value: u16, numerator: u32, denominator: u32) -> u16 {
    let modified = showdown_modify(u32::from(value), numerator, denominator).max(1);
    u16::try_from(modified.min(u32::from(u16::MAX))).unwrap_or(u16::MAX)
}

fn roll_critical(rng: &mut crate::BattleRng, ratio: u8) -> Result<bool, BattleError> {
    let denominator = match ratio.min(4) {
        0 => return Ok(false),
        1 => 24,
        2 => 8,
        3 => 2,
        _ => 1,
    };
    rng.chance(RngReason::CriticalHit, 1, denominator)
}

fn combined_type_modifier<D: CombatData>(
    data: &D,
    attacking: PokemonType,
    defending: PokemonTypes,
) -> Result<Option<i8>, BattleError> {
    combined_type_modifier_with_scrappy(data, attacking, defending, false)
}

fn combined_type_modifier_with_scrappy<D: CombatData>(
    data: &D,
    attacking: PokemonType,
    defending: PokemonTypes,
    scrappy: bool,
) -> Result<Option<i8>, BattleError> {
    let mut modifier = 0_i8;
    for defending_type in [Some(defending.primary), defending.secondary]
        .into_iter()
        .flatten()
    {
        if scrappy
            && defending_type == PokemonType::Ghost
            && matches!(attacking, PokemonType::Normal | PokemonType::Fighting)
        {
            continue;
        }
        let Some(value) = data.type_modifier(attacking, defending_type) else {
            return Ok(None);
        };
        modifier = modifier.checked_add(value).ok_or_else(|| {
            BattleError::InvalidState("combined type modifier overflowed".to_owned())
        })?;
    }
    Ok(Some(modifier))
}

fn resolve_target(
    source_side: SideId,
    source_slot: ActiveSlot,
    move_target: MoveTarget,
    selected_target: ActionTarget,
) -> Result<Option<(SideId, ActiveSlot)>, BattleError> {
    match (move_target, selected_target) {
        (
            MoveTarget::Normal
            | MoveTarget::Any
            | MoveTarget::AdjacentAlly
            | MoveTarget::AdjacentAllyOrSelf,
            ActionTarget::Ally(slot),
        ) if slot != source_slot => Ok(Some((source_side, slot))),
        (
            MoveTarget::Normal | MoveTarget::Any | MoveTarget::AdjacentFoe,
            ActionTarget::Foe(slot),
        ) => Ok(Some((source_side.opponent(), slot))),
        (MoveTarget::AdjacentAllyOrSelf, ActionTarget::Self_)
        | (MoveTarget::Self_, ActionTarget::Automatic | ActionTarget::Self_) => {
            Ok(Some((source_side, source_slot)))
        }
        (
            MoveTarget::All
            | MoveTarget::AllAdjacent
            | MoveTarget::AllAdjacentFoes
            | MoveTarget::Allies
            | MoveTarget::AllySide
            | MoveTarget::AllyTeam
            | MoveTarget::FoeSide,
            ActionTarget::Automatic,
        ) => Ok(None),
        (
            MoveTarget::Normal | MoveTarget::Any | MoveTarget::AdjacentFoe,
            ActionTarget::Automatic | ActionTarget::Self_ | ActionTarget::Ally(_),
        ) => Err(BattleError::IllegalChoice(format!(
            "selected target {selected_target:?} is incompatible with move target {move_target:?}"
        ))),
        (_, ActionTarget::Automatic) => Err(BattleError::IllegalChoice(format!(
            "automatic target is incompatible with move target {move_target:?}"
        ))),
        _ => Err(BattleError::IllegalChoice(format!(
            "selected target {selected_target:?} is incompatible with move target {move_target:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standard_move() -> CombatMove {
        CombatMove {
            move_type: PokemonType::Dragon,
            category: MoveCategory::Physical,
            base_power: 80,
            accuracy: MoveAccuracy::Percent { value: 100 },
            crit_ratio: 1,
            target: MoveTarget::Normal,
            operation: MoveOperation::StandardDamage,
            required_capabilities: MoveCapabilities::PR_13_1,
            hit_sequence: crate::HitSequence::Single,
            multi_accuracy: false,
            fixed_damage: None,
            drain: None,
            recoil: None,
            self_damage: None,
            prior_effect: crate::PriorMoveEffect::None,
            offensive_stat: None,
            defensive_stat: None,
            ignore_negative_offensive: false,
            ignore_positive_defensive: false,
            interaction_flags: MoveInteractionFlags::default(),
        }
    }

    #[test]
    fn facade_only_activates_for_supported_major_statuses() {
        for (status, expected) in [
            (None, false),
            (Some(MajorStatus::Burn), true),
            (Some(MajorStatus::Paralysis), true),
            (Some(MajorStatus::Poison), true),
            (Some(MajorStatus::BadlyPoisoned), true),
            (Some(MajorStatus::Sleep), false),
            (Some(MajorStatus::Freeze), false),
        ] {
            assert_eq!(facade_activates(status), expected, "status: {status:?}");
        }
    }

    #[test]
    fn damage_stat_hooks_distinguish_physical_special_and_overrides() {
        let physical = standard_move();
        assert_eq!(
            offensive_stat_event(physical),
            Ok(BattleEvent::ModifyAttack)
        );
        assert_eq!(
            defensive_stat_event(physical),
            Ok(BattleEvent::ModifyDefense)
        );

        let special = CombatMove {
            category: MoveCategory::Special,
            ..physical
        };
        assert_eq!(
            offensive_stat_event(special),
            Ok(BattleEvent::ModifySpecialAttack)
        );
        assert_eq!(
            defensive_stat_event(special),
            Ok(BattleEvent::ModifySpecialDefense)
        );

        let body_press = CombatMove {
            offensive_stat: Some(BattleStat::Defense),
            ..physical
        };
        assert_eq!(
            offensive_stat_event(body_press),
            Ok(BattleEvent::ModifySourceDefense)
        );
    }

    #[test]
    fn move_preflight_accepts_pr_13_4_and_names_missing_capabilities() {
        let move_id = MoveId::new(1);
        assert_eq!(
            preflight_move(
                move_id,
                standard_move(),
                MoveIntrinsic::Ordinary,
                &[],
                &[],
                None,
            ),
            Ok(())
        );

        let mut unsupported = standard_move();
        unsupported.required_capabilities = unsupported
            .required_capabilities
            .with(MoveCapability::SpecialBehavior);
        let error = preflight_move(
            move_id,
            unsupported,
            MoveIntrinsic::Ordinary,
            &[],
            &[],
            None,
        )
        .unwrap_err();
        assert_eq!(
            error,
            BattleError::UnsupportedMechanic(
                "move 1 requires unsupported capability special_behavior".to_owned()
            )
        );
    }

    #[test]
    fn move_preflight_rejects_inconsistent_definition_data() {
        let mut inconsistent = standard_move();
        inconsistent.category = MoveCategory::Status;
        assert!(matches!(
            preflight_move(
                MoveId::new(1),
                inconsistent,
                MoveIntrinsic::Ordinary,
                &[],
                &[],
                None,
            ),
            Err(BattleError::DataMismatch(_))
        ));
    }

    #[test]
    fn selected_targets_must_match_the_authoritative_move_definition() {
        let left = ActiveSlot::Left;
        let right = ActiveSlot::Right;
        for move_target in [MoveTarget::Normal, MoveTarget::Any] {
            assert_eq!(
                resolve_target(SideId::P1, left, move_target, ActionTarget::Ally(right)),
                Ok(Some((SideId::P1, right)))
            );
            for foe in [left, right] {
                assert_eq!(
                    resolve_target(SideId::P1, left, move_target, ActionTarget::Foe(foe)),
                    Ok(Some((SideId::P2, foe)))
                );
            }
            for selected in [
                ActionTarget::Ally(left),
                ActionTarget::Self_,
                ActionTarget::Automatic,
            ] {
                assert!(matches!(
                    resolve_target(SideId::P1, left, move_target, selected),
                    Err(BattleError::IllegalChoice(_))
                ));
            }
        }

        for foe in [left, right] {
            assert_eq!(
                resolve_target(
                    SideId::P1,
                    left,
                    MoveTarget::AdjacentFoe,
                    ActionTarget::Foe(foe)
                ),
                Ok(Some((SideId::P2, foe)))
            );
        }
        for selected in [
            ActionTarget::Ally(right),
            ActionTarget::Ally(left),
            ActionTarget::Self_,
            ActionTarget::Automatic,
        ] {
            assert!(matches!(
                resolve_target(SideId::P1, left, MoveTarget::AdjacentFoe, selected),
                Err(BattleError::IllegalChoice(_))
            ));
        }
    }
}
