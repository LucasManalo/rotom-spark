use super::{ActionQueue, FieldState, SideState};
use crate::{BattleError, BattleRng, PokemonPosition, SideId};
use serde::{Deserialize, Serialize};

/// The explicit point in the battle lifecycle at which choices or resolution occur.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BattlePhase {
    TeamPreview,
    TurnSelection,
    ResolvingTurn,
    ForcedSwitch,
    BattleEnd,
}

/// Complete authoritative state for one battle.
///
/// Random mechanics consume the battle-owned deterministic RNG. Gameplay
/// fields are read-only to downstream crates; advance them with [`crate::step`]
/// or construct validated artificial scenarios with [`crate::BattleStateBuilder`].
///
/// ```compile_fail
/// fn bypass_transition(mut battle: rotom_core::BattleState) {
///     battle.turn = 99;
/// }
/// ```
///
/// ```compile_fail
/// fn bypass_nested_state(mut battle: rotom_core::BattleState) {
///     let _ = battle.side_mut(rotom_core::SideId::P1);
/// }
/// ```
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleState {
    pub(crate) turn: u32,
    pub(crate) picked_team_size: u8,
    pub(crate) sides: [SideState; 2],
    pub(crate) field: FieldState,
    pub(crate) rng: BattleRng,
    pub(crate) queue: ActionQueue,
    pub(crate) phase: BattlePhase,
    pub(crate) winner: Option<SideId>,
}

impl BattleState {
    /// Creates a battle at team preview with an empty field.
    ///
    /// # Errors
    ///
    /// Returns an error if the format's picked-team size cannot supply two
    /// doubles leads, exceeds either loaded team, or either side is not in its
    /// initial pre-preview state.
    pub fn new(p1: SideState, p2: SideState, picked_team_size: usize) -> Result<Self, BattleError> {
        Self::new_with_seed(p1, p2, picked_team_size, 0)
    }

    /// Creates a battle with an explicit deterministic RNG seed.
    ///
    /// Use this constructor for simulations and replays whose random stream
    /// must be reproducible. [`Self::new`] remains a zero-seeded convenience
    /// constructor for callers that do not yet consume randomness.
    ///
    /// # Errors
    ///
    /// Returns the same state-validation errors as [`Self::new`].
    pub fn new_with_seed(
        p1: SideState,
        p2: SideState,
        picked_team_size: usize,
        seed: u64,
    ) -> Result<Self, BattleError> {
        let picked_team_size = u8::try_from(picked_team_size).map_err(|_| {
            BattleError::InvalidState("picked-team size does not fit in a u8".to_owned())
        })?;
        if picked_team_size < 2 {
            return Err(BattleError::InvalidState(
                "a doubles battle must pick at least two Pokémon".to_owned(),
            ));
        }
        for (side_id, side) in [(SideId::P1, &p1), (SideId::P2, &p2)] {
            if side.team.members.len() < usize::from(picked_team_size) {
                return Err(BattleError::InvalidState(format!(
                    "{side_id:?} has fewer Pokémon than the picked-team size"
                )));
            }
            if side.active != [None, None] || !side.selected_team.is_empty() {
                return Err(BattleError::InvalidState(format!(
                    "{side_id:?} is not in its initial team-preview state"
                )));
            }
            if side.team.members.iter().any(|pokemon| {
                pokemon.position != PokemonPosition::Team
                    || pokemon.current_hp == 0
                    || pokemon.current_hp > pokemon.stats.hp
            }) {
                return Err(BattleError::InvalidState(format!(
                    "{side_id:?} has an invalid Pokémon state before team preview"
                )));
            }
        }

        let battle = Self {
            turn: 0,
            picked_team_size,
            sides: [p1, p2],
            field: FieldState::default(),
            rng: BattleRng::new(seed),
            queue: ActionQueue::default(),
            phase: BattlePhase::TeamPreview,
            winner: None,
        };
        battle.validate_stable_state()?;
        Ok(battle)
    }

    #[must_use]
    pub const fn turn(&self) -> u32 {
        self.turn
    }

    #[must_use]
    pub const fn picked_team_size(&self) -> u8 {
        self.picked_team_size
    }

    #[must_use]
    pub const fn sides(&self) -> &[SideState; 2] {
        &self.sides
    }

    #[must_use]
    pub const fn field(&self) -> &FieldState {
        &self.field
    }

    #[must_use]
    pub const fn rng(&self) -> &BattleRng {
        &self.rng
    }

    #[must_use]
    pub const fn queue(&self) -> &ActionQueue {
        &self.queue
    }

    #[must_use]
    pub const fn phase(&self) -> BattlePhase {
        self.phase
    }

    #[must_use]
    pub const fn winner(&self) -> Option<SideId> {
        self.winner
    }

    /// Enables diagnostic RNG tracing without exposing mutable RNG state.
    pub fn enable_rng_trace(&mut self) {
        self.rng.enable_trace();
    }

    /// Disables diagnostic RNG tracing without changing the random stream.
    pub fn disable_rng_trace(&mut self) {
        self.rng.disable_trace();
    }

    /// Clears recorded diagnostic RNG entries without changing the stream.
    pub fn clear_rng_trace(&mut self) {
        self.rng.clear_trace();
    }

    /// Returns one side by its stable player identifier.
    #[must_use]
    pub const fn side(&self, side: SideId) -> &SideState {
        match side {
            SideId::P1 => &self.sides[0],
            SideId::P2 => &self.sides[1],
        }
    }

    /// Returns one mutable side by its stable player identifier.
    #[must_use]
    pub(crate) const fn side_mut(&mut self, side: SideId) -> &mut SideState {
        match side {
            SideId::P1 => &mut self.sides[0],
            SideId::P2 => &mut self.sides[1],
        }
    }

    /// Whether the battle ended without a winning side.
    #[must_use]
    pub const fn is_tie(&self) -> bool {
        matches!(self.phase, BattlePhase::BattleEnd) && self.winner.is_none()
    }
}
