use crate::{
    ActiveSlot, BattleError, BattlePhase, BattleState, CombatData, EffectRegistry, EventDispatch,
    PokemonAction, PokemonIndex, SideId, TurnChoice, TurnResolution,
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
}

impl<'a, D: CombatData> Mechanics<'a, D> {
    #[must_use]
    pub fn new(data: &'a D, effects: &EffectRegistry) -> Self {
        Self {
            data,
            effects: crate::conditions::composed_registry(effects),
        }
    }

    #[must_use]
    pub const fn data(&self) -> &'a D {
        self.data
    }

    #[must_use]
    pub const fn effects(&self) -> &EffectRegistry {
        &self.effects
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
            next.queue_turn_inner(*p1, *p2, &mechanics.effects)?;
            let queue_events = next.queue.events.clone();
            let turn = next.resolve_turn_inner(mechanics.data, &mechanics.effects)?;
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
