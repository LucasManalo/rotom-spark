use crate::{
    kernel::StateKernel, ActionTarget, ActiveSlot, BattleError, BattleEvent, BattlePhase,
    BattleState, ConditionId, EffectRegistry, EventContext, EventDispatch, EventValue, FaintTarget,
    FieldEffectIds, FormId, MajorStatus, MoveId, MoveSlotIndex, MoveTarget, PokemonAction,
    PokemonIndex, PokemonRef, PokemonState, RngReason, SideId, SpeciesId,
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
}

impl MoveCapability {
    const ALL: [Self; 20] = [
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

    /// Resolves format-specific identities for built-in field mechanics.
    /// Partial data providers may leave the default empty mapping.
    fn field_effect_ids(&self) -> crate::FieldEffectIds {
        crate::FieldEffectIds::default()
    }
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

#[derive(Clone, Copy)]
struct MoveExecutionContext {
    queued: QueuedMoveAction,
    source: PokemonRef,
    target: PokemonRef,
    target_slot: ActiveSlot,
    move_id: MoveId,
    move_data: CombatMove,
    source_species: SpeciesId,
    source_form: Option<FormId>,
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
        let move_context = self.prepare_move_execution(data, queued)?;
        let source_index = move_context.source.pokemon;
        let context = EventContext {
            source: Some(move_context.source),
            target: Some(move_context.target),
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

        if !self.check_accuracy(
            move_context.move_data.accuracy,
            move_context.move_id,
            field_effects,
            move_context.source,
            move_context.target,
        )? {
            resolution.actions.push(ResolvedAction::Missed {
                side: queued.side,
                slot: queued.slot,
                move_slot: queued.move_slot,
            });
            resolution.events.push(effects.dispatch(
                self,
                BattleEvent::AfterMove,
                context,
                EventValue::None,
            )?);
            return Ok(());
        }

        if self.priority_move_blocked(data, field_effects, move_context)? {
            resolution.actions.push(ResolvedAction::Skipped {
                side: queued.side,
                slot: queued.slot,
            });
            resolution.events.push(effects.dispatch(
                self,
                BattleEvent::AfterMove,
                context,
                EventValue::None,
            )?);
            return Ok(());
        }

        let try_hit = effects.dispatch(
            self,
            BattleEvent::TryHit,
            context,
            EventValue::Boolean(true),
        )?;
        let cancelled = try_hit.cancelled || try_hit.value == EventValue::Boolean(false);
        resolution.events.push(try_hit);
        if cancelled {
            resolution.actions.push(ResolvedAction::Skipped {
                side: queued.side,
                slot: queued.slot,
            });
            resolution.events.push(effects.dispatch(
                self,
                BattleEvent::AfterMove,
                context,
                EventValue::None,
            )?);
            return Ok(());
        }

        self.execute_primary_operation(data, move_context, effects, field_effects, resolution)
    }

    fn prepare_move_execution<D: CombatData>(
        &self,
        data: &D,
        queued: QueuedMoveAction,
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
        preflight_move(move_id, move_data)?;
        let (target_side, target_slot) = resolve_target(queued.side, queued.slot, queued.target)?;
        let target_index = self.side(target_side).active[target_slot.index()].ok_or_else(|| {
            BattleError::UnsupportedMechanic(
                "retargeting after an empty or fainted target is deferred".to_owned(),
            )
        })?;
        Ok(MoveExecutionContext {
            queued,
            source: PokemonRef::new(queued.side, source_index),
            target: PokemonRef::new(target_side, target_index),
            target_slot,
            move_id,
            move_data,
            source_species: source.species,
            source_form: source.form,
        })
    }

    fn execute_primary_operation<D: CombatData>(
        &mut self,
        data: &D,
        move_context: MoveExecutionContext,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        match move_context.move_data.operation {
            MoveOperation::StandardDamage => {
                self.execute_standard_damage(data, move_context, effects, field_effects, resolution)
            }
            MoveOperation::FixedDamage | MoveOperation::LevelDamage | MoveOperation::NonDamage => {
                Err(BattleError::UnsupportedMechanic(format!(
                    "move {} reached an unsupported primary operation after preflight",
                    move_context.move_id
                )))
            }
        }
    }

    fn execute_standard_damage<D: CombatData>(
        &mut self,
        data: &D,
        move_context: MoveExecutionContext,
        effects: &EffectRegistry,
        field_effects: &FieldEffectIds,
        resolution: &mut TurnResolution,
    ) -> Result<(), BattleError> {
        let MoveExecutionContext {
            queued,
            source,
            target,
            target_slot,
            move_id,
            ..
        } = move_context;
        let context = EventContext {
            source: Some(source),
            target: Some(target),
            move_id: Some(move_id),
        };

        let mut damage = self.calculate_standard_damage(data, field_effects, move_context)?;
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
                    "ModifyDamage replaced its integer relay with an incompatible value".to_owned(),
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
            side: queued.side,
            slot: queued.slot,
            target_side: target.side,
            target_slot,
            move_slot: queued.move_slot,
            damage,
        });
        self.apply_damage(
            effects,
            DamageApplication {
                context,
                side: target.side,
                slot: target_slot,
                pokemon: target.pokemon,
                damage: damage.final_damage,
            },
            resolution,
        )?;
        resolution.events.push(effects.dispatch(
            self,
            BattleEvent::AfterMove,
            context,
            EventValue::None,
        )?);
        Ok(())
    }

    fn calculate_standard_damage<D: CombatData>(
        &mut self,
        data: &D,
        field_effects: &FieldEffectIds,
        move_context: MoveExecutionContext,
    ) -> Result<DamageResult, BattleError> {
        let MoveExecutionContext {
            source,
            target,
            move_id,
            move_data,
            source_species,
            source_form,
            ..
        } = move_context;
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
            DamageEnvironment { weather, defense },
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
    ) -> Result<(), BattleError> {
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
        Ok(())
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

    fn priority_move_blocked<D: CombatData>(
        &self,
        data: &D,
        field_effects: &FieldEffectIds,
        context: MoveExecutionContext,
    ) -> Result<bool, BattleError> {
        if !field_effects.field_has(self, field_effects.psychic_terrain)
            || context.source.side == context.target.side
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
        Ok(priority > 0
            && crate::field_effects::grounded(self, data, field_effects, context.target)?)
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
    preflight_move(move_id, move_data)?;
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

fn preflight_move(move_id: MoveId, move_data: CombatMove) -> Result<(), BattleError> {
    if let Some(missing) = move_data
        .required_capabilities
        .first_missing_from(MoveCapabilities::PR_13_1)
    {
        return Err(BattleError::UnsupportedMechanic(format!(
            "move {move_id} requires unsupported capability {missing}"
        )));
    }
    if move_data.operation != MoveOperation::StandardDamage
        || move_data.category == MoveCategory::Status
        || move_data.base_power == 0
    {
        return Err(BattleError::DataMismatch(format!(
            "move {move_id} declares inconsistent standard-damage metadata"
        )));
    }
    if !matches!(
        move_data.target,
        MoveTarget::Normal | MoveTarget::Any | MoveTarget::AdjacentFoe
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
    target: ActionTarget,
) -> Result<(SideId, ActiveSlot), BattleError> {
    match target {
        ActionTarget::Self_ => Ok((source_side, source_slot)),
        ActionTarget::Ally(slot) => Ok((source_side, slot)),
        ActionTarget::Foe(slot) => Ok((source_side.opponent(), slot)),
        ActionTarget::Automatic => Err(BattleError::UnsupportedMechanic(
            "automatic or spread targeting is deferred".to_owned(),
        )),
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
        assert_eq!(preflight_move(move_id, standard_move()), Ok(()));

        let mut secondary = standard_move();
        secondary.required_capabilities = secondary
            .required_capabilities
            .with(MoveCapability::SecondaryEffect);
        let error = preflight_move(move_id, secondary).unwrap_err();
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
            preflight_move(MoveId::new(1), inconsistent),
            Err(BattleError::DataMismatch(_))
        ));
    }
}
