use crate::{
    ActiveSlot, BattleError, BattlePhase, BattleState, CombatData, EffectRegistry, EventDispatch,
    FieldEffectIds, MajorStatus, PokemonAction, PokemonIndex, PokemonRef, SideId, TurnChoice,
    TurnResolution,
};
use serde::{Deserialize, Serialize};

/// One side's complete ordered team-preview selection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeamSelection {
    pub order: Vec<PokemonIndex>,
}

impl TeamSelection {
    #[must_use]
    pub fn new(order: Vec<PokemonIndex>) -> Self {
        Self { order }
    }
}

/// Both players' complete decisions for one public battle phase.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum JointAction {
    TeamPreview {
        p1: TeamSelection,
        p2: TeamSelection,
    },
    Turn {
        p1: TurnChoice,
        p2: TurnChoice,
    },
    ForcedSwitch {
        p1: TurnChoice,
        p2: TurnChoice,
    },
}

/// Immutable mechanics inputs shared across transitions.
pub struct Mechanics<'a, D: CombatData> {
    data: &'a D,
    effects: EffectRegistry,
    field_effects: FieldEffectIds,
}

impl<'a, D: CombatData> Mechanics<'a, D> {
    #[must_use]
    pub fn new(data: &'a D, effects: &EffectRegistry) -> Self {
        Self {
            data,
            effects: crate::conditions::composed_registry(effects, data.ability_handlers()),
            field_effects: data.field_effect_ids(),
        }
    }

    /// Replaces the data provider's field identity mapping.
    #[must_use]
    pub fn with_field_effects(mut self, field_effects: FieldEffectIds) -> Self {
        self.field_effects = field_effects;
        self
    }

    #[must_use]
    pub const fn data(&self) -> &'a D {
        self.data
    }

    #[must_use]
    pub const fn effects(&self) -> &EffectRegistry {
        &self.effects
    }

    #[must_use]
    pub const fn field_effects(&self) -> &FieldEffectIds {
        &self.field_effects
    }

    /// Returns whether a target currently participates in grounded mechanics.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid target or missing type data.
    pub fn is_grounded(
        &self,
        battle: &BattleState,
        target: PokemonRef,
    ) -> Result<bool, BattleError> {
        crate::field_effects::grounded(battle, self.data, &self.field_effects, target)
    }

    /// Checks the common Electric/Misty Terrain major-status prevention rule.
    /// Other immunities remain owned by the status source and later mechanics.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid target or missing type data.
    pub fn terrain_allows_status(
        &self,
        battle: &BattleState,
        target: PokemonRef,
        status: MajorStatus,
    ) -> Result<bool, BattleError> {
        crate::field_effects::terrain_allows_status(
            battle,
            self.data,
            &self.field_effects,
            target,
            status,
        )
    }
}

/// Non-authoritative observations produced by one successful transition.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StepDiagnostics {
    /// Events emitted while constructing a normal-turn action queue.
    pub queue_events: Vec<EventDispatch>,
    /// Resolution details for a normal turn; absent for preview/replacement steps.
    pub turn: Option<TurnResolution>,
}

/// The committed result of one complete decision phase.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StepResult {
    pub state: BattleState,
    pub diagnostics: StepDiagnostics,
}

/// Advances one complete, phase-appropriate joint decision atomically.
///
/// The input battle owns the sole authoritative RNG stream. This function
/// validates the stable input, clones it once, advances only that clone, and
/// validates the stable output before returning it. Any error therefore leaves
/// the caller's state and RNG unchanged.
///
/// # Errors
///
/// Returns an error for an unstable input, an action that does not match the
/// current phase, an illegal choice, unsupported mechanics, inconsistent
/// static data, callback failure, or an unstable result.
pub fn step<D: CombatData>(
    current: &BattleState,
    action: &JointAction,
    mechanics: &Mechanics<'_, D>,
) -> Result<StepResult, BattleError> {
    current.validate_stable_state()?;
    validate_phase(current.phase, action)?;

    if let JointAction::ForcedSwitch { p1, p2 } = action {
        current.validate_choice(SideId::P1, p1)?;
        current.validate_choice(SideId::P2, p2)?;
    }

    let mut next = current.clone();
    let diagnostics = match action {
        JointAction::TeamPreview { p1, p2 } => {
            next.select_team(SideId::P1, p1.order.clone())?;
            next.select_team(SideId::P2, p2.order.clone())?;
            StepDiagnostics::default()
        }
        JointAction::Turn { p1, p2 } => {
            next.queue_turn_inner(*p1, *p2, mechanics)?;
            let queue_events = next.queue.events.clone();
            let turn = next.resolve_turn_inner(
                mechanics.data,
                &mechanics.effects,
                &mechanics.field_effects,
            )?;
            StepDiagnostics {
                queue_events,
                turn: Some(turn),
            }
        }
        JointAction::ForcedSwitch { p1, p2 } => {
            apply_replacements(&mut next, SideId::P1, *p1)?;
            apply_replacements(&mut next, SideId::P2, *p2)?;
            StepDiagnostics::default()
        }
    };

    next.validate_stable_state()?;
    Ok(StepResult {
        state: next,
        diagnostics,
    })
}

fn validate_phase(phase: BattlePhase, action: &JointAction) -> Result<(), BattleError> {
    let matches = matches!(
        (phase, action),
        (BattlePhase::TeamPreview, JointAction::TeamPreview { .. })
            | (BattlePhase::TurnSelection, JointAction::Turn { .. })
            | (BattlePhase::ForcedSwitch, JointAction::ForcedSwitch { .. })
    );
    if matches {
        Ok(())
    } else {
        Err(BattleError::IllegalChoice(format!(
            "joint action does not match battle phase {phase:?}"
        )))
    }
}

fn apply_replacements(
    state: &mut BattleState,
    side: SideId,
    choice: TurnChoice,
) -> Result<(), BattleError> {
    for (slot, action) in ActiveSlot::ALL.into_iter().zip(choice.actions) {
        match action {
            PokemonAction::Switch { pokemon } => state.replace_fainted(side, slot, pokemon)?,
            PokemonAction::Pass => {}
            PokemonAction::Move { .. } | PokemonAction::Struggle => {
                return Err(BattleError::InvalidState(
                    "validated forced-switch choice contained a non-replacement action".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AbilityId, ActionTarget, BattleStats, CombatMove, FormId, MoveAccuracy, MoveCapabilities,
        MoveCategory, MoveId, MoveOperation, MoveSlot, MoveSlotIndex, MoveTarget, PokemonPosition,
        PokemonState, PokemonType, PokemonTypes, SideState, SpeciesId, StatStages, StatusState,
        TeamState,
    };

    const MOVE_ID: MoveId = MoveId::new(1);

    struct TestData;

    impl CombatData for TestData {
        fn move_data(&self, move_id: MoveId) -> Option<CombatMove> {
            (move_id == MOVE_ID).then_some(CombatMove {
                move_type: PokemonType::Normal,
                category: MoveCategory::Physical,
                base_power: 20,
                accuracy: MoveAccuracy::Always,
                crit_ratio: 0,
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
                interaction_flags: crate::MoveInteractionFlags::default(),
            })
        }

        fn pokemon_types(&self, _: SpeciesId, _: Option<FormId>) -> Option<PokemonTypes> {
            Some(PokemonTypes {
                primary: PokemonType::Normal,
                secondary: None,
            })
        }

        fn type_modifier(&self, _: PokemonType, _: PokemonType) -> Option<i8> {
            Some(0)
        }
    }

    fn pokemon(species: u32, speed: u16) -> PokemonState {
        PokemonState {
            species: SpeciesId::new(species),
            form: None,
            level: 50,
            stats: BattleStats {
                hp: 120,
                atk: 100,
                def: 100,
                spa: 100,
                spd: 100,
                spe: speed,
            },
            current_hp: 120,
            ability: AbilityId::new(1),
            item: None,
            moves: vec![MoveSlot {
                move_id: MOVE_ID,
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

    fn move_at(target: ActiveSlot) -> PokemonAction {
        PokemonAction::Move {
            move_slot: MoveSlotIndex::new(0),
            target: ActionTarget::Foe(target),
            mega: false,
        }
    }

    #[test]
    fn canonical_turn_matches_internal_queue_then_resolve_sequence() {
        let p1 = vec![pokemon(1, 140), pokemon(2, 130)];
        let p2 = vec![pokemon(101, 100), pokemon(102, 90)];
        let mut current = BattleState::new_with_seed(
            SideState::new(TeamState::new(p1)),
            SideState::new(TeamState::new(p2)),
            2,
            0x5eed,
        )
        .unwrap();
        current
            .select_team(SideId::P1, vec![PokemonIndex::new(0), PokemonIndex::new(1)])
            .unwrap();
        current
            .select_team(SideId::P2, vec![PokemonIndex::new(0), PokemonIndex::new(1)])
            .unwrap();

        let p1_choice = TurnChoice {
            actions: [move_at(ActiveSlot::Left), move_at(ActiveSlot::Right)],
        };
        let p2_choice = TurnChoice {
            actions: [move_at(ActiveSlot::Left), move_at(ActiveSlot::Right)],
        };
        let effects = EffectRegistry::new();
        let mechanics = Mechanics::new(&TestData, &effects);

        let mut sequential = current.clone();
        sequential
            .queue_turn_inner(p1_choice, p2_choice, &mechanics)
            .unwrap();
        let queue_events = sequential.queue.events.clone();
        let turn = sequential
            .resolve_turn_inner(
                mechanics.data(),
                mechanics.effects(),
                mechanics.field_effects(),
            )
            .unwrap();
        sequential.validate_stable_state().unwrap();

        let canonical = step(
            &current,
            &JointAction::Turn {
                p1: p1_choice,
                p2: p2_choice,
            },
            &mechanics,
        )
        .unwrap();

        assert_eq!(canonical.state, sequential);
        assert_eq!(canonical.diagnostics.queue_events, queue_events);
        assert_eq!(canonical.diagnostics.turn, Some(turn));
        assert_eq!(current.rng.draws(), 0);
        assert_eq!(current.queue.actions, Vec::new());
    }
}
