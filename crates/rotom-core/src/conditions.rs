use crate::{
    kernel::StateKernel, BattleError, BattleEvent, BattleState, EffectInvocation, EffectOrder,
    EffectOwner, EffectRegistry, EffectSource, EventContext, EventDispatch, EventValue,
    MajorStatus, PokemonPosition, PokemonRef, RngReason, StatusState,
};

const CHAMPIONS_SLEEP_DURATIONS: [u8; 3] = [2, 3, 3];

pub(crate) fn composed_registry(custom: &EffectRegistry) -> EffectRegistry {
    let mut registry = EffectRegistry::new();
    register_common_conditions(&mut registry);
    registry.append(custom);
    registry
}

fn register_common_conditions(registry: &mut EffectRegistry) {
    registry.register(
        BattleEvent::ModifySpeed,
        EffectSource::MajorStatus(MajorStatus::Paralysis),
        EffectOrder::new(0, -101, 0),
        paralysis_speed,
    );
    registry.register(
        BattleEvent::BeforeMove,
        EffectSource::MajorStatus(MajorStatus::Sleep),
        EffectOrder::new(0, 10, 0),
        sleep_before_move,
    );
    registry.register(
        BattleEvent::BeforeMove,
        EffectSource::MajorStatus(MajorStatus::Freeze),
        EffectOrder::new(0, 10, 0),
        freeze_before_move,
    );
    registry.register(
        BattleEvent::BeforeMove,
        EffectSource::MajorStatus(MajorStatus::Paralysis),
        EffectOrder::new(0, 1, 0),
        paralysis_before_move,
    );
    for status in [MajorStatus::Poison, MajorStatus::BadlyPoisoned] {
        registry.register(
            BattleEvent::Residual,
            EffectSource::MajorStatus(status),
            EffectOrder::new(9, 0, 0),
            status_residual,
        );
    }
    registry.register(
        BattleEvent::Residual,
        EffectSource::MajorStatus(MajorStatus::Burn),
        EffectOrder::new(10, 0, 0),
        status_residual,
    );
}

impl BattleState {
    /// Applies one major status and initializes its Champions-specific state.
    ///
    /// Type, move, ability, and item immunities are enforced by the effect that
    /// requests status application; this operation owns the common status
    /// exclusivity, counters, and deterministic duration roll.
    ///
    /// # Errors
    ///
    /// Returns an error for a missing or fainted target, an existing status,
    /// or an invalid RNG operation.
    pub fn apply_major_status(
        &mut self,
        target: PokemonRef,
        status: MajorStatus,
    ) -> Result<(), BattleError> {
        let position = self
            .side(target.side)
            .pokemon(target.pokemon)
            .ok_or_else(|| BattleError::InvalidState("status target is out of bounds".to_owned()))?
            .position;
        if matches!(position, PokemonPosition::Fainted) {
            return Err(BattleError::IllegalChoice(
                "cannot apply a major status to a fainted Pokémon".to_owned(),
            ));
        }
        if self
            .side(target.side)
            .pokemon(target.pokemon)
            .is_some_and(|pokemon| pokemon.status.is_some())
        {
            return Err(BattleError::IllegalChoice(
                "cannot replace an existing major status".to_owned(),
            ));
        }
        let state = match status {
            MajorStatus::Sleep => StatusState {
                sleep_turns: *self
                    .rng
                    .sample(RngReason::SleepDuration, &CHAMPIONS_SLEEP_DURATIONS)?,
                ..StatusState::default()
            },
            MajorStatus::Freeze => StatusState {
                freeze_turns: 3,
                ..StatusState::default()
            },
            MajorStatus::Burn
            | MajorStatus::Paralysis
            | MajorStatus::Poison
            | MajorStatus::BadlyPoisoned => StatusState::default(),
        };
        StateKernel::new(self).set_major_status(target, status, state)
    }

    /// Cures a target's major status and clears all associated counters.
    ///
    /// # Errors
    ///
    /// Returns an error when the target index is outside its team.
    pub fn cure_major_status(
        &mut self,
        target: PokemonRef,
    ) -> Result<Option<MajorStatus>, BattleError> {
        StateKernel::new(self).cure_major_status(target)
    }
}

fn paralysis_speed(
    _: &mut BattleState,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if owner_pokemon(invocation) != context.source {
        return Ok(());
    }
    let EventValue::Integer(speed) = &mut dispatch.value else {
        return Err(BattleError::InvalidState(
            "paralysis received a non-integer Speed relay".to_owned(),
        ));
    };
    *speed /= 2;
    Ok(())
}

fn sleep_before_move(
    battle: &mut BattleState,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(pokemon) = matching_source(invocation, context) else {
        return Ok(());
    };
    let turns = StateKernel::new(battle).decrement_sleep_turns(pokemon)?;
    if turns > 0 {
        dispatch.cancel();
    }
    Ok(())
}

fn freeze_before_move(
    battle: &mut BattleState,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(pokemon) = matching_source(invocation, context) else {
        return Ok(());
    };
    let turns = StateKernel::new(battle).decrement_freeze_turns(pokemon)?;
    let thawed = turns == 0 || battle.rng.chance(RngReason::Freeze, 1, 4)?;
    if !thawed {
        dispatch.cancel();
    } else if turns > 0 {
        StateKernel::new(battle).cure_major_status(pokemon)?;
    }
    Ok(())
}

fn paralysis_before_move(
    battle: &mut BattleState,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if matching_source(invocation, context).is_some()
        && battle.rng.chance(RngReason::Paralysis, 1, 8)?
    {
        dispatch.cancel();
    }
    Ok(())
}

fn status_residual(
    battle: &mut BattleState,
    invocation: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    let Some(pokemon) = owner_pokemon(invocation) else {
        return Ok(());
    };
    let (max_hp, status) = {
        let state = battle
            .side(pokemon.side)
            .pokemon(pokemon.pokemon)
            .ok_or_else(|| {
                BattleError::InvalidState("condition owner is out of bounds".to_owned())
            })?;
        (state.stats.hp, state.status)
    };
    let unit = (max_hp / 16).max(1);
    let damage = match status {
        Some(MajorStatus::Poison) => (max_hp / 8).max(1),
        Some(MajorStatus::Burn) => unit,
        Some(MajorStatus::BadlyPoisoned) => {
            let counter = StateKernel::new(battle).increment_toxic_counter(pokemon)?;
            unit.saturating_mul(u16::from(counter))
        }
        Some(MajorStatus::Freeze | MajorStatus::Paralysis | MajorStatus::Sleep) | None => 0,
    };
    StateKernel::new(battle).damage_hp(pokemon, damage)?;
    Ok(())
}

const fn owner_pokemon(invocation: &EffectInvocation) -> Option<PokemonRef> {
    match invocation.owner {
        EffectOwner::Pokemon { pokemon } => Some(pokemon),
        EffectOwner::Battle
        | EffectOwner::Field
        | EffectOwner::Side { .. }
        | EffectOwner::Move { .. } => None,
    }
}

fn matching_source(invocation: &EffectInvocation, context: &EventContext) -> Option<PokemonRef> {
    let owner = owner_pokemon(invocation)?;
    (Some(owner) == context.source).then_some(owner)
}
