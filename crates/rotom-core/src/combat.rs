use crate::{
    kernel::StateKernel, ActionTarget, ActiveSlot, BattleError, BattleEvent, BattlePhase,
    BattleState, ConditionId, ConditionState, EffectRegistry, EventContext, EventDispatch,
    EventValue, FaintTarget, FieldEffectIds, FormId, MajorStatus, MoveId, MoveSlotIndex,
    MoveTarget, PokemonAction, PokemonIndex, PokemonRef, PokemonState, RngReason, SideId,
    SpeciesId,
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
}

impl MoveCapability {
    const ALL: [Self; 26] = [
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

    /// Whether the move is stopped by Protect-like guards.
    fn move_is_protectable(&self, _move_id: MoveId) -> bool {
        false
    }

    /// Whether the move removes and bypasses Protect-like guards.
    fn move_breaks_protect(&self, _move_id: MoveId) -> bool {
        false
    }

    /// Resolves format-specific identities for built-in field mechanics.
    /// Partial data providers may leave the default empty mapping.
    fn field_effect_ids(&self) -> crate::FieldEffectIds {
        crate::FieldEffectIds::default()
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
}

#[derive(Clone, Copy)]
struct TargetExecutionContext {
    target: PokemonRef,
    slot: ActiveSlot,
    redirected: bool,
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
    defense: Option<(u32, u32)>,
    spread: bool,
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
        for action in queued {
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
                        effects,
                        action.side,
                        action.slot,
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
                    self.execute_move(
                        data,
                        QueuedMoveAction {
                            side: action.side,
                            slot: action.slot,
                            move_slot,
                            target,
                        },
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
        self.tick_volatile_durations(resolution)?;
        self.process_weather_residuals(data, field_effects, resolution)?;
        self.tick_weather_duration(resolution);
        self.process_terrain_residuals(data, field_effects, resolution)?;
        let before = self.active_status_hp()?;
        let residual = effects.dispatch(
            self,
            BattleEvent::Residual,
            EventContext::default(),
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
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let move_context = self.prepare_move_execution(data, queued, field_effects)?;
        let source_index = move_context.source.pokemon;
        let context = EventContext {
            source: Some(move_context.source),
            target: move_context.original_target.map(|target| target.target),
            move_id: Some(move_context.move_id),
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

        let remaining_pp = self
            .side(queued.side)
            .pokemon(source_index)
            .and_then(|source| source.moves.get(usize::from(queued.move_slot.get())))
            .ok_or_else(|| BattleError::InvalidState("move slot became invalid".to_owned()))?
            .pp
            .checked_sub(1)
            .ok_or_else(|| {
                BattleError::InvalidState("queued move has no remaining PP".to_owned())
            })?;
        StateKernel::new(self).set_pp(move_context.source, queued.move_slot, remaining_pp)?;

        self.execute_primary_operation(data, &move_context, effects, field_effects, resolution)
    }

    fn prepare_move_execution<D: CombatData>(
        &self,
        data: &D,
        queued: QueuedMoveAction,
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
        let move_data = data.move_data(move_id).ok_or_else(|| {
            BattleError::DataMismatch(format!("missing combat data for move {move_id}"))
        })?;
        let intrinsic = data.move_intrinsic(move_id);
        preflight_move(move_id, move_data, intrinsic)?;
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
                Err(BattleError::UnsupportedMechanic(format!(
                    "move {} reached an unsupported primary operation after preflight",
                    move_context.move_id
                )))
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
        let mut outcomes = vec![None; move_context.targets.len()];
        let mut eligible = move_context
            .targets
            .iter()
            .copied()
            .enumerate()
            .collect::<Vec<_>>();

        let mut after_try_hit = Vec::with_capacity(eligible.len());
        for (index, target_context) in eligible {
            let target = target_context.target;
            let context = EventContext {
                source: Some(move_context.source),
                target: Some(target),
                move_id: Some(move_context.move_id),
            };
            let try_hit = effects.dispatch(
                self,
                BattleEvent::TryHit,
                context,
                EventValue::Boolean(true),
            )?;
            let cancelled = try_hit.cancelled || try_hit.value == EventValue::Boolean(false);
            resolution.events.push(try_hit);
            // Pinned Showdown resolves Psychic Terrain's priority-4 TryHit
            // guard before Protect's priority-3 TryHit handler, per target.
            let priority_blocked =
                self.priority_move_blocked_target(data, field_effects, move_context, target)?;
            let protected = if cancelled || priority_blocked {
                false
            } else {
                self.target_is_protected(data, field_effects, move_context, target)?
            };
            if cancelled || priority_blocked || protected {
                outcomes[index] = Some(TargetOutcome {
                    source: move_context.source,
                    move_id: move_context.move_id,
                    target,
                    redirected: target_context.redirected,
                    result: if priority_blocked {
                        TargetResult::PriorityBlocked
                    } else if protected {
                        TargetResult::Protected
                    } else {
                        TargetResult::Cancelled
                    },
                    calculated_damage: 0,
                    applied_hp_loss: 0,
                    fainted: false,
                });
                continue;
            }
            after_try_hit.push((index, target_context));
        }

        let mut after_immunity = Vec::with_capacity(after_try_hit.len());
        for (index, target_context) in after_try_hit {
            let target = target_context.target;
            if self.target_is_type_immune(data, move_context.move_data, target)? {
                outcomes[index] = Some(TargetOutcome {
                    source: move_context.source,
                    move_id: move_context.move_id,
                    target,
                    redirected: target_context.redirected,
                    result: TargetResult::Immune,
                    calculated_damage: 0,
                    applied_hp_loss: 0,
                    fainted: false,
                });
                continue;
            }
            after_immunity.push((index, target_context));
        }

        eligible = Vec::with_capacity(after_immunity.len());
        for (index, target_context) in after_immunity {
            let target = target_context.target;
            if !self.check_accuracy(
                move_context.move_data.accuracy,
                move_context.move_id,
                field_effects,
                move_context.source,
                target,
            )? {
                resolution.actions.push(ResolvedAction::Missed {
                    side: move_context.queued.side,
                    slot: move_context.queued.slot,
                    move_slot: move_context.queued.move_slot,
                });
                outcomes[index] = Some(TargetOutcome {
                    source: move_context.source,
                    move_id: move_context.move_id,
                    target,
                    redirected: target_context.redirected,
                    result: TargetResult::Missed,
                    calculated_damage: 0,
                    applied_hp_loss: 0,
                    fainted: false,
                });
                continue;
            }
            eligible.push((index, target_context));
        }

        let mut any_hit = false;
        for (index, target_context) in eligible {
            let target = target_context.target;
            let context = EventContext {
                source: Some(move_context.source),
                target: Some(target),
                move_id: Some(move_context.move_id),
            };
            let mut damage = self.calculate_standard_damage(
                data,
                field_effects,
                move_context,
                target_context,
                spread,
            )?;
            let modify_damage = effects.dispatch(
                self,
                BattleEvent::ModifyDamage,
                context,
                EventValue::Integer(i64::from(damage.final_damage)),
            )?;
            damage.final_damage = match modify_damage.value {
                EventValue::Integer(value) => u16::try_from(value).map_err(|_| {
                    BattleError::InvalidState(
                        "ModifyDamage produced a value outside the u16 damage range".to_owned(),
                    )
                })?,
                EventValue::None | EventValue::Boolean(_) => {
                    return Err(BattleError::InvalidState(
                        "ModifyDamage replaced its integer relay with an incompatible value"
                            .to_owned(),
                    ));
                }
            };
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
            let (applied_hp_loss, fainted) = self.apply_damage(
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
            outcomes[index] = Some(TargetOutcome {
                source: move_context.source,
                move_id: move_context.move_id,
                target,
                redirected: target_context.redirected,
                result: TargetResult::Hit,
                calculated_damage: damage.final_damage,
                applied_hp_loss,
                fainted,
            });
            any_hit = true;
        }
        resolution
            .target_outcomes
            .extend(outcomes.into_iter().flatten());
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
        };
        resolution.events.push(effects.dispatch(
            self,
            BattleEvent::AfterMove,
            after_context,
            EventValue::None,
        )?);
        Ok(())
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
        let context = EventContext {
            source: Some(move_context.source),
            target: Some(move_context.source),
            move_id: Some(move_context.move_id),
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
            MoveTarget::AllySide
            | MoveTarget::AllyTeam
            | MoveTarget::FoeSide
            | MoveTarget::RandomNormal
            | MoveTarget::Scripted => {
                return Err(BattleError::UnsupportedMechanic(format!(
                    "target category {move_target:?} requires a deferred non-Pokémon or random target operation"
                )));
            }
        };
        if targets.len() == 1 && targets[0].target.side != source.side {
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
            || (ids.item_is_active(self)
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
        &mut self,
        data: &D,
        ids: &FieldEffectIds,
        context: &MoveExecutionContext,
        target: PokemonRef,
    ) -> Result<bool, BattleError> {
        let Some(protect) = ids.protect else {
            return Ok(false);
        };
        let guarded = self
            .side(target.side)
            .pokemon(target.pokemon)
            .is_some_and(|pokemon| pokemon.has_volatile(protect));
        if !guarded {
            return Ok(false);
        }
        if data.move_breaks_protect(context.move_id) {
            StateKernel::new(self).remove_volatile(target, protect)?;
            return Ok(false);
        }
        Ok(data.move_is_protectable(context.move_id))
    }

    fn target_is_type_immune<D: CombatData>(
        &self,
        data: &D,
        move_data: CombatMove,
        target: PokemonRef,
    ) -> Result<bool, BattleError> {
        let target = self
            .side(target.side)
            .pokemon(target.pokemon)
            .ok_or_else(|| BattleError::InvalidState("immunity target is invalid".to_owned()))?;
        let types = data
            .pokemon_types(target.species, target.form)
            .ok_or_else(|| BattleError::DataMismatch("missing immunity target types".to_owned()))?;
        Ok(combined_type_modifier(data, move_data.move_type, types)?.is_none())
    }

    fn calculate_standard_damage<D: CombatData>(
        &mut self,
        data: &D,
        field_effects: &FieldEffectIds,
        move_context: &MoveExecutionContext,
        target_context: TargetExecutionContext,
        spread: bool,
    ) -> Result<DamageResult, BattleError> {
        let source = move_context.source;
        let target = target_context.target;
        let move_id = move_context.move_id;
        let move_data = move_context.move_data;
        let source_species = move_context.source_species;
        let source_form = move_context.source_form;
        let critical = roll_critical(&mut self.rng, move_data.crit_ratio)?;
        let random_percent = u8::try_from(self.rng.range(RngReason::DamageRoll, 85..101)?)
            .map_err(|_| BattleError::InvalidState("damage roll does not fit in u8".to_owned()))?;
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
        let type_exponent = combined_type_modifier(data, move_data.move_type, target_types)?;
        let source_state = self
            .side(source.side)
            .pokemon(source.pokemon)
            .ok_or_else(|| BattleError::InvalidState("damage source is out of bounds".to_owned()))?
            .clone();
        let mut adjusted_move = move_data;
        self.apply_field_base_power(
            data,
            field_effects,
            source,
            target,
            move_id,
            &mut adjusted_move,
        )?;
        let defense = self.apply_field_defense(
            data,
            field_effects,
            adjusted_move.category,
            &mut target_state,
        )?;
        let weather =
            crate::field_effects::weather_damage_modifier(self, field_effects, move_data.move_type);
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
                defense,
                spread,
            },
        )?;
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
        Ok(damage)
    }

    fn apply_damage(
        &mut self,
        effects: &EffectRegistry,
        application: DamageApplication,
        resolution: &mut TurnResolution,
    ) -> Result<(u16, bool), BattleError> {
        let DamageApplication {
            context,
            side,
            slot,
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
        if fainted {
            resolution.events.push(effects.dispatch(
                self,
                BattleEvent::Faint,
                context,
                EventValue::None,
            )?);
            self.process_faints(&[FaintTarget::new(side, slot)])?;
        }
        Ok((applied, fainted))
    }

    fn check_accuracy(
        &mut self,
        accuracy: MoveAccuracy,
        move_id: MoveId,
        field_effects: &FieldEffectIds,
        source: PokemonRef,
        target: PokemonRef,
    ) -> Result<bool, BattleError> {
        let MoveAccuracy::Percent { value } = accuracy else {
            return Ok(true);
        };
        let value = match field_effects.weather_accuracy(self, move_id, u16::from(value)) {
            crate::field_effects::WeatherAccuracy::Always => return Ok(true),
            crate::field_effects::WeatherAccuracy::Percent(value) => value,
        };
        let source_stage = self
            .side(source.side)
            .pokemon(source.pokemon)
            .ok_or_else(|| BattleError::InvalidState("accuracy source is invalid".to_owned()))?
            .stat_stages
            .accuracy;
        let target_stage = self
            .side(target.side)
            .pokemon(target.pokemon)
            .ok_or_else(|| BattleError::InvalidState("accuracy target is invalid".to_owned()))?
            .stat_stages
            .evasion;
        let stage = (source_stage - target_stage).clamp(-6, 6);
        let adjusted = match stage.cmp(&0) {
            std::cmp::Ordering::Greater => value * u16::try_from(3 + stage).unwrap_or(9) / 3,
            std::cmp::Ordering::Less => value * 3 / u16::try_from(3 - stage).unwrap_or(9),
            std::cmp::Ordering::Equal => value,
        };
        let adjusted = field_effects.gravity_accuracy(self, adjusted);
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
        if !field_effects.field_has(self, field_effects.psychic_terrain)
            || context.source.side == target.side
        {
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
        Ok(priority > 0 && crate::field_effects::grounded(self, data, field_effects, target)?)
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

    fn resolve_queued_switch(
        &mut self,
        effects: &EffectRegistry,
        side: SideId,
        slot: ActiveSlot,
        incoming: PokemonIndex,
        resolution: &mut TurnResolution,
    ) -> Result<bool, BattleError> {
        let outgoing = self.side(side).active[slot.index()].ok_or_else(|| {
            BattleError::InvalidState("queued switch has no outgoing Pokémon".to_owned())
        })?;
        let context = EventContext {
            source: Some(PokemonRef::new(side, outgoing)),
            target: Some(PokemonRef::new(side, incoming)),
            move_id: None,
        };
        let switch_out =
            effects.dispatch(self, BattleEvent::SwitchOut, context, EventValue::None)?;
        let cancelled = switch_out.cancelled;
        resolution.events.push(switch_out);
        if cancelled {
            return Ok(false);
        }
        StateKernel::new(self).switch_active_position(side, slot, incoming)?;
        resolution.events.push(effects.dispatch(
            self,
            BattleEvent::SwitchIn,
            EventContext {
                source: Some(PokemonRef::new(side, incoming)),
                target: Some(PokemonRef::new(side, outgoing)),
                move_id: None,
            },
            EventValue::None,
        )?);
        Ok(true)
    }

    fn process_weather_residuals<D: CombatData>(
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
                    let requested = (pokemon.stats.hp / 16).max(1);
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
    let attack = staged_stat(raw_attack, attack_stage);
    let mut defense = staged_stat(raw_defense, defense_stage).max(1);
    if let Some((numerator, denominator)) = environment.defense {
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
        final_damage = showdown_modify(final_damage, 3, 2);
        Modifier {
            numerator: 3,
            denominator: 2,
        }
    } else {
        Modifier::NEUTRAL
    };
    let (typed_damage, type_modifier) = apply_type_modifier(final_damage, type_exponent);
    final_damage = typed_damage;
    if source.status == Some(MajorStatus::Burn) && move_data.category == MoveCategory::Physical {
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
/// outside the PR 13.1 executor slice, or damage inputs are invalid.
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
    preflight_move(move_id, move_data, MoveIntrinsic::Ordinary)?;
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

fn preflight_move(
    move_id: MoveId,
    move_data: CombatMove,
    intrinsic: MoveIntrinsic,
) -> Result<(), BattleError> {
    if let Some(missing) = move_data
        .required_capabilities
        .first_missing_from(MoveCapabilities::PR_13_2)
    {
        return Err(BattleError::UnsupportedMechanic(format!(
            "move {move_id} requires unsupported capability {missing}"
        )));
    }
    let valid_damage = move_data.operation == MoveOperation::StandardDamage
        && move_data.category != MoveCategory::Status
        && move_data.base_power > 0;
    let valid_intrinsic = move_data.operation == MoveOperation::NonDamage
        && move_data.category == MoveCategory::Status
        && move_data.base_power == 0
        && intrinsic != MoveIntrinsic::Ordinary;
    if !valid_damage && !valid_intrinsic {
        return Err(BattleError::DataMismatch(format!(
            "move {move_id} declares inconsistent standard-damage metadata"
        )));
    }
    if matches!(
        move_data.target,
        MoveTarget::AllySide
            | MoveTarget::AllyTeam
            | MoveTarget::FoeSide
            | MoveTarget::RandomNormal
            | MoveTarget::Scripted
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
    let mut modifier = 0_i8;
    for defending_type in [Some(defending.primary), defending.secondary]
        .into_iter()
        .flatten()
    {
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
            | MoveTarget::Allies,
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
        }
    }

    #[test]
    fn move_preflight_accepts_the_foundation_and_names_missing_capabilities() {
        let move_id = MoveId::new(1);
        assert_eq!(
            preflight_move(move_id, standard_move(), MoveIntrinsic::Ordinary),
            Ok(())
        );

        let mut secondary = standard_move();
        secondary.required_capabilities = secondary
            .required_capabilities
            .with(MoveCapability::SecondaryEffect);
        let error = preflight_move(move_id, secondary, MoveIntrinsic::Ordinary).unwrap_err();
        assert_eq!(
            error,
            BattleError::UnsupportedMechanic(
                "move 1 requires unsupported capability secondary_effect".to_owned()
            )
        );
    }

    #[test]
    fn move_preflight_rejects_inconsistent_definition_data() {
        let mut inconsistent = standard_move();
        inconsistent.category = MoveCategory::Status;
        assert!(matches!(
            preflight_move(MoveId::new(1), inconsistent, MoveIntrinsic::Ordinary),
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
