use super::{ActiveSlot, BattleState, FieldState, PokemonIndex, PokemonState, SideState};
use crate::{
    BattleError, BattleEvent, EffectRegistry, EventContext, EventDispatch, EventValue, FaintTarget,
    MajorStatus, PokemonRef, RngReason, SideId, StatusState,
};
use std::ops::Range;

/// Validating builder for artificial battle scenarios and differential fixtures.
///
/// Edits occur on a detached state owned by the builder. [`Self::build`]
/// rejects any result that violates stable public-boundary invariants, so this
/// type is not a simulation advancement API and cannot commit partial states.
pub struct BattleStateBuilder {
    state: BattleState,
}

impl BattleStateBuilder {
    /// Starts a zero-seeded team-preview fixture.
    ///
    /// # Errors
    ///
    /// Returns an error when the initial sides or picked-team size are invalid.
    pub fn new(p1: SideState, p2: SideState, picked_team_size: usize) -> Result<Self, BattleError> {
        Self::new_with_seed(p1, p2, picked_team_size, 0)
    }

    /// Starts a team-preview fixture with an explicit deterministic seed.
    ///
    /// # Errors
    ///
    /// Returns an error when the initial sides or picked-team size are invalid.
    pub fn new_with_seed(
        p1: SideState,
        p2: SideState,
        picked_team_size: usize,
        seed: u64,
    ) -> Result<Self, BattleError> {
        Ok(Self {
            state: BattleState::new_with_seed(p1, p2, picked_team_size, seed)?,
        })
    }

    /// Starts from an existing stable state.
    ///
    /// # Errors
    ///
    /// Returns an error when the supplied state violates a stable invariant.
    pub fn from_state(state: BattleState) -> Result<Self, BattleError> {
        state.validate_stable_state()?;
        Ok(Self { state })
    }

    /// Applies both complete preview selections to the detached fixture.
    ///
    /// # Errors
    ///
    /// Returns an error when either selection is illegal.
    pub fn select_teams(
        &mut self,
        p1: Vec<PokemonIndex>,
        p2: Vec<PokemonIndex>,
    ) -> Result<&mut Self, BattleError> {
        self.state.select_team(SideId::P1, p1)?;
        self.state.select_team(SideId::P2, p2)?;
        Ok(self)
    }

    /// Edits one detached Pokémon. The complete scenario is validated by
    /// [`Self::build`].
    ///
    /// # Errors
    ///
    /// Returns an error when the target index is out of bounds.
    pub fn edit_pokemon(
        &mut self,
        target: PokemonRef,
        edit: impl FnOnce(&mut PokemonState),
    ) -> Result<&mut Self, BattleError> {
        let pokemon = self
            .state
            .side_mut(target.side)
            .pokemon_mut(target.pokemon)
            .ok_or_else(|| {
                BattleError::InvalidState("fixture Pokémon is out of bounds".to_owned())
            })?;
        edit(pokemon);
        Ok(self)
    }

    /// Edits detached side metadata. Coupled references are checked at build.
    pub fn edit_side(&mut self, side: SideId, edit: impl FnOnce(&mut SideState)) -> &mut Self {
        edit(self.state.side_mut(side));
        self
    }

    /// Edits detached field state. Condition invariants are checked at build.
    pub fn edit_field(&mut self, edit: impl FnOnce(&mut FieldState)) -> &mut Self {
        edit(&mut self.state.field);
        self
    }

    /// Applies one structurally valid voluntary switch to the fixture.
    ///
    /// # Errors
    ///
    /// Returns an error when the switch is illegal for the fixture state.
    pub fn switch_active(
        &mut self,
        side: SideId,
        slot: ActiveSlot,
        incoming: PokemonIndex,
    ) -> Result<&mut Self, BattleError> {
        self.state.switch_active(side, slot, incoming)?;
        Ok(self)
    }

    /// Settles an ordered faint batch in the fixture.
    ///
    /// # Errors
    ///
    /// Returns an error when the batch or current phase is invalid.
    pub fn process_faints(&mut self, targets: &[FaintTarget]) -> Result<&mut Self, BattleError> {
        self.state.process_faints(targets)?;
        Ok(self)
    }

    /// Applies a forced replacement in the fixture.
    ///
    /// # Errors
    ///
    /// Returns an error when the replacement is illegal.
    pub fn replace_fainted(
        &mut self,
        side: SideId,
        slot: ActiveSlot,
        incoming: PokemonIndex,
    ) -> Result<&mut Self, BattleError> {
        self.state.replace_fainted(side, slot, incoming)?;
        Ok(self)
    }

    /// Applies a supported major status through normal status mechanics.
    ///
    /// # Errors
    ///
    /// Returns an error when the target or status application is invalid.
    pub fn apply_major_status(
        &mut self,
        target: PokemonRef,
        status: MajorStatus,
    ) -> Result<&mut Self, BattleError> {
        self.state.apply_major_status(target, status)?;
        Ok(self)
    }

    /// Installs an explicit coherent status/counter pair for a fixture.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid target or incoherent counter state.
    pub fn set_major_status(
        &mut self,
        target: PokemonRef,
        status: MajorStatus,
        status_state: StatusState,
    ) -> Result<&mut Self, BattleError> {
        let pokemon = self
            .state
            .side_mut(target.side)
            .pokemon_mut(target.pokemon)
            .ok_or_else(|| {
                BattleError::InvalidState("fixture Pokémon is out of bounds".to_owned())
            })?;
        pokemon.set_major_status(status, status_state)?;
        Ok(self)
    }

    /// Cures a fixture Pokémon through the coherent status operation.
    ///
    /// # Errors
    ///
    /// Returns an error when the target is invalid.
    pub fn cure_major_status(
        &mut self,
        target: PokemonRef,
    ) -> Result<Option<MajorStatus>, BattleError> {
        self.state.cure_major_status(target)
    }

    /// Enables diagnostic RNG tracing for the fixture.
    pub fn enable_rng_trace(&mut self) -> &mut Self {
        self.state.enable_rng_trace();
        self
    }

    /// Returns the detached scenario's deterministic RNG for replay fixture setup.
    pub const fn rng_mut(&mut self) -> &mut crate::BattleRng {
        &mut self.state.rng
    }

    /// Advances the fixture RNG explicitly. This is intended only for replay
    /// and snapshot scenarios that need a known pre-advanced stream.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested range is empty or invalid.
    pub fn draw_rng(&mut self, reason: RngReason, range: Range<u64>) -> Result<u64, BattleError> {
        self.state.rng.range(reason, range)
    }

    /// Dispatches one event against the detached fixture.
    ///
    /// This exists for focused effect tests and scenario construction. The
    /// resulting state is still rejected by [`Self::build`] unless every
    /// stable invariant holds; production advancement must use [`crate::step`].
    ///
    /// # Errors
    ///
    /// Propagates event collection, relay validation, or callback errors.
    pub fn dispatch_effect_for_fixture(
        &mut self,
        registry: &EffectRegistry,
        event: BattleEvent,
        context: EventContext,
        value: EventValue,
    ) -> Result<EventDispatch, BattleError> {
        registry.dispatch(&mut self.state, event, context, value)
    }

    #[must_use]
    pub const fn state(&self) -> &BattleState {
        &self.state
    }

    /// Returns the fixture only if it satisfies every stable-state invariant.
    ///
    /// # Errors
    ///
    /// Returns an error rather than normalizing any invalid state.
    pub fn build(self) -> Result<BattleState, BattleError> {
        self.state.validate_stable_state()?;
        Ok(self.state)
    }
}
