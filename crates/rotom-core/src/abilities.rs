use crate::{
    AbilityId, BattleError, BattleEvent, EffectInvocation, EffectOrder, EffectOwner,
    EffectRegistry, EffectSource, EffectState, EventContext, EventDispatch, EventValue, PokemonRef,
};

/// Resolved identities for the first PR 13.5 ability-handler batch.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AbilityEffectIds {
    pub huge_power: Option<AbilityId>,
    pub pure_power: Option<AbilityId>,
    pub guts: Option<AbilityId>,
    pub fur_coat: Option<AbilityId>,
    pub marvel_scale: Option<AbilityId>,
    pub technician: Option<AbilityId>,
    pub multiscale: Option<AbilityId>,
}

pub(crate) fn register(registry: &mut EffectRegistry, ids: AbilityEffectIds) {
    for ability in [ids.huge_power, ids.pure_power].into_iter().flatten() {
        registry.register(
            BattleEvent::ModifyAttack,
            EffectSource::Ability(ability),
            EffectOrder::new(0, 5, 0),
            double_source_stat,
        );
    }
    if let Some(ability) = ids.guts {
        registry.register(
            BattleEvent::ModifyAttack,
            EffectSource::Ability(ability),
            EffectOrder::new(0, 5, 0),
            guts_attack,
        );
    }
    if let Some(ability) = ids.fur_coat {
        for event in [BattleEvent::ModifyDefense, BattleEvent::ModifySourceDefense] {
            registry.register(
                event,
                EffectSource::Ability(ability),
                EffectOrder::new(0, 6, 0),
                double_holder_defense,
            );
        }
    }
    if let Some(ability) = ids.marvel_scale {
        for event in [BattleEvent::ModifyDefense, BattleEvent::ModifySourceDefense] {
            registry.register(
                event,
                EffectSource::Ability(ability),
                EffectOrder::new(0, 6, 0),
                marvel_scale_defense,
            );
        }
    }
    if let Some(ability) = ids.technician {
        registry.register(
            BattleEvent::ModifyBasePower,
            EffectSource::Ability(ability),
            EffectOrder::new(0, 30, 0),
            technician_base_power,
        );
    }
    for ability in [ids.multiscale].into_iter().flatten() {
        registry.register(
            BattleEvent::ModifyDamage,
            EffectSource::Ability(ability),
            EffectOrder::default(),
            full_hp_damage_reduction,
        );
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

pub(crate) fn ignores_burn_attack_penalty(ability: AbilityId, ids: AbilityEffectIds) -> bool {
    ids.guts == Some(ability)
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
        }
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
        let mut registry = EffectRegistry::new();
        register(
            &mut registry,
            AbilityEffectIds {
                huge_power: Some(huge_power),
                pure_power: Some(pure_power),
                technician: Some(technician),
                ..AbilityEffectIds::default()
            },
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
        let mut registry = EffectRegistry::new();
        register(
            &mut registry,
            AbilityEffectIds {
                guts: Some(guts),
                marvel_scale: Some(marvel_scale),
                multiscale: Some(multiscale),
                fur_coat: Some(fur_coat),
                ..AbilityEffectIds::default()
            },
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
}
