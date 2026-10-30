use crate::{
    BattleError, BattleEvent, EffectInvocation, EffectOrder, EffectOwner, EffectRegistry,
    EffectSource, EffectState, EventContext, EventDispatch, EventValue, ItemId, MoveCategory,
    PokemonRef, PokemonType,
};

/// A reusable implementation family for a held item resolved by static data.
///
/// Item metadata and stable identities remain in the Dex. These native
/// behaviors only describe mechanics already supported by the shared event
/// and move-execution pipeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemBehavior {
    PhysicalPower,
    SpecialPower,
    SuperEffectiveDamage,
    TypePower(PokemonType),
    ResistBerry(PokemonType),
}

/// Joins one stable item identity to a native behavior family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ItemHandler {
    pub item: ItemId,
    pub behavior: ItemBehavior,
}

impl ItemHandler {
    #[must_use]
    pub const fn new(item: ItemId, behavior: ItemBehavior) -> Self {
        Self { item, behavior }
    }
}

pub(crate) fn register(
    registry: &mut EffectRegistry,
    handlers: impl IntoIterator<Item = ItemHandler>,
) {
    for handler in handlers {
        let source = EffectSource::Item(handler.item);
        match handler.behavior {
            ItemBehavior::PhysicalPower => registry.register(
                BattleEvent::ModifyBasePower,
                source,
                EffectOrder::new(0, 16, 0),
                physical_power,
            ),
            ItemBehavior::SpecialPower => registry.register(
                BattleEvent::ModifyBasePower,
                source,
                EffectOrder::new(0, 16, 0),
                special_power,
            ),
            ItemBehavior::TypePower(move_type) => registry.register(
                BattleEvent::ModifyBasePower,
                source,
                EffectOrder::new(0, 15, 0),
                type_power_callback(move_type),
            ),
            ItemBehavior::SuperEffectiveDamage => registry.register(
                BattleEvent::ModifyDamage,
                source,
                EffectOrder::default(),
                super_effective_damage,
            ),
            ItemBehavior::ResistBerry(move_type) => registry.register(
                BattleEvent::ModifyDamage,
                source,
                EffectOrder::default(),
                resist_berry_callback(move_type),
            ),
        }
    }
}

fn physical_power(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    modify_source(
        invocation,
        context,
        dispatch,
        context.move_category == Some(MoveCategory::Physical),
        4505,
    )
}

fn special_power(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    modify_source(
        invocation,
        context,
        dispatch,
        context.move_category == Some(MoveCategory::Special),
        4505,
    )
}

fn type_power(
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    move_type: PokemonType,
) -> Result<(), BattleError> {
    modify_source(
        invocation,
        context,
        dispatch,
        context.move_type == Some(move_type),
        4915,
    )
}

fn resist_berry(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    move_type: PokemonType,
) -> Result<(), BattleError> {
    let Some(owner) = owner_pokemon(invocation) else {
        return Ok(());
    };
    if Some(owner) != context.target
        || context.move_type != Some(move_type)
        || !context.type_effectiveness.is_some_and(|value| value > 0)
        || context.suppress_target_item
        || context.target_berry_blocked
    {
        return Ok(());
    }
    if context.target_berry_interaction_unsupported {
        return Err(BattleError::UnsupportedMechanic(
            "resistance Berry consumption requires a deferred ability interaction".to_owned(),
        ));
    }
    let EffectSource::Item(item) = invocation.source else {
        return Err(BattleError::InvalidState(
            "resistance Berry handler has a non-item source".to_owned(),
        ));
    };
    if state
        .battle()
        .side(owner.side)
        .pokemon(owner.pokemon)
        .and_then(|pokemon| pokemon.item)
        != Some(item)
    {
        return Ok(());
    }
    state.set_item(owner, None)?;
    modify_integer(dispatch, 1, 2)?;
    if context.target_ripen {
        modify_integer(dispatch, 1, 2)?;
    }
    Ok(())
}

macro_rules! type_callbacks {
    ($selector:ident, $worker:ident; $($variant:ident => $callback:ident),+ $(,)?) => {
        const fn $selector(move_type: PokemonType) -> crate::EffectCallback {
            match move_type {
                $(PokemonType::$variant => $callback,)+
            }
        }

        $(
            fn $callback(
                state: &mut EffectState<'_>,
                invocation: &EffectInvocation,
                context: &EventContext,
                dispatch: &mut EventDispatch,
            ) -> Result<(), BattleError> {
                $worker(state, invocation, context, dispatch, PokemonType::$variant)
            }
        )+
    };
}

fn type_power_worker(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    move_type: PokemonType,
) -> Result<(), BattleError> {
    type_power(invocation, context, dispatch, move_type)
}

type_callbacks!(type_power_callback, type_power_worker;
    Bug => bug_power,
    Dark => dark_power,
    Dragon => dragon_power,
    Electric => electric_power,
    Fairy => fairy_power,
    Fighting => fighting_power,
    Fire => fire_power,
    Flying => flying_power,
    Ghost => ghost_power,
    Grass => grass_power,
    Ground => ground_power,
    Ice => ice_power,
    Normal => normal_power,
    Poison => poison_power,
    Psychic => psychic_power,
    Rock => rock_power,
    Steel => steel_power,
    Stellar => stellar_power,
    Water => water_power,
);

type_callbacks!(resist_berry_callback, resist_berry;
    Bug => bug_resist_berry,
    Dark => dark_resist_berry,
    Dragon => dragon_resist_berry,
    Electric => electric_resist_berry,
    Fairy => fairy_resist_berry,
    Fighting => fighting_resist_berry,
    Fire => fire_resist_berry,
    Flying => flying_resist_berry,
    Ghost => ghost_resist_berry,
    Grass => grass_resist_berry,
    Ground => ground_resist_berry,
    Ice => ice_resist_berry,
    Normal => normal_resist_berry,
    Poison => poison_resist_berry,
    Psychic => psychic_resist_berry,
    Rock => rock_resist_berry,
    Steel => steel_resist_berry,
    Stellar => stellar_resist_berry,
    Water => water_resist_berry,
);

fn super_effective_damage(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    modify_source(
        invocation,
        context,
        dispatch,
        context.type_effectiveness.is_some_and(|value| value > 0),
        4915,
    )
}

fn modify_source(
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
    applies: bool,
    modifier: i64,
) -> Result<(), BattleError> {
    if applies && !context.suppress_source_item && owner_pokemon(invocation) == context.source {
        let value = integer(dispatch)?;
        dispatch.value =
            EventValue::Integer(value.saturating_mul(modifier).saturating_add(2047) / 4096);
    }
    Ok(())
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

const fn owner_pokemon(invocation: &EffectInvocation) -> Option<PokemonRef> {
    match invocation.owner {
        EffectOwner::Pokemon { pokemon } | EffectOwner::Move { pokemon } => Some(pokemon),
        EffectOwner::Battle | EffectOwner::Field | EffectOwner::Side { .. } => None,
    }
}

fn integer(dispatch: &EventDispatch) -> Result<i64, BattleError> {
    match dispatch.value {
        EventValue::Integer(value) => Ok(value),
        EventValue::None | EventValue::Boolean(_) => Err(BattleError::InvalidState(
            "item modifier received a non-integer relay".to_owned(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AbilityId, BattleState, BattleStats, MoveId, MoveSlot, MoveTarget, PokemonIndex,
        PokemonPosition, PokemonState, SideId, SideState, SpeciesId, StatStages, StatusState,
        TeamState,
    };

    fn pokemon(item: Option<ItemId>) -> PokemonState {
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
            ability: AbilityId::new(1),
            item,
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

    fn battle(item: ItemId) -> BattleState {
        battle_with_items(Some(item), None)
    }

    fn battle_with_items(source_item: Option<ItemId>, target_item: Option<ItemId>) -> BattleState {
        let source = SideState::new(TeamState::new(
            (0..4).map(|_| pokemon(source_item)).collect(),
        ));
        let target = SideState::new(TeamState::new(
            (0..4).map(|_| pokemon(target_item)).collect(),
        ));
        let mut battle = BattleState::new(source, target, 4).unwrap();
        let selected = (0..4).map(PokemonIndex::new).collect::<Vec<_>>();
        battle.select_team(SideId::P1, selected.clone()).unwrap();
        battle.select_team(SideId::P2, selected).unwrap();
        battle
    }

    fn context(category: MoveCategory, move_type: PokemonType) -> EventContext {
        EventContext {
            source: Some(PokemonRef::new(SideId::P1, PokemonIndex::new(0))),
            target: Some(PokemonRef::new(SideId::P2, PokemonIndex::new(0))),
            move_id: Some(MoveId::new(1)),
            move_category: Some(category),
            move_type: Some(move_type),
            ..EventContext::default()
        }
    }

    fn dispatch(
        registry: &EffectRegistry,
        battle: &mut BattleState,
        event: BattleEvent,
        context: EventContext,
    ) -> i64 {
        let EventValue::Integer(value) = registry
            .dispatch(battle, event, context, EventValue::Integer(100))
            .unwrap()
            .value
        else {
            panic!("item modifier must retain an integer relay");
        };
        value
    }

    #[test]
    fn first_five_item_behaviors_use_their_pinned_modifiers_and_conditions() {
        let muscle_band = ItemId::new(10);
        let wise_glasses = ItemId::new(11);
        let expert_belt = ItemId::new(12);
        let charcoal = ItemId::new(13);
        let mystic_water = ItemId::new(14);
        let mut registry = EffectRegistry::new();
        register(
            &mut registry,
            [
                ItemHandler::new(muscle_band, ItemBehavior::PhysicalPower),
                ItemHandler::new(wise_glasses, ItemBehavior::SpecialPower),
                ItemHandler::new(expert_belt, ItemBehavior::SuperEffectiveDamage),
                ItemHandler::new(charcoal, ItemBehavior::TypePower(PokemonType::Fire)),
                ItemHandler::new(mystic_water, ItemBehavior::TypePower(PokemonType::Water)),
            ],
        );

        let mut state = battle(muscle_band);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyBasePower,
                context(MoveCategory::Physical, PokemonType::Normal),
            ),
            110
        );
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyBasePower,
                context(MoveCategory::Special, PokemonType::Normal),
            ),
            100
        );

        let mut state = battle(wise_glasses);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyBasePower,
                context(MoveCategory::Special, PokemonType::Normal),
            ),
            110
        );

        let mut expert_context = context(MoveCategory::Special, PokemonType::Water);
        expert_context.type_effectiveness = Some(1);
        let mut state = battle(expert_belt);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyDamage,
                expert_context,
            ),
            120
        );
        expert_context.type_effectiveness = Some(0);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyDamage,
                expert_context,
            ),
            100
        );

        let mut state = battle(charcoal);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyBasePower,
                context(MoveCategory::Special, PokemonType::Fire),
            ),
            120
        );
        let mut state = battle(mystic_water);
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyBasePower,
                context(MoveCategory::Special, PokemonType::Water),
            ),
            120
        );
    }

    #[test]
    fn item_modifiers_are_holder_scoped_and_respect_shared_suppression() {
        let muscle_band = ItemId::new(10);
        let mut registry = EffectRegistry::new();
        register(
            &mut registry,
            [ItemHandler::new(muscle_band, ItemBehavior::PhysicalPower)],
        );
        let mut state = battle(muscle_band);
        let mut suppressed = context(MoveCategory::Physical, PokemonType::Normal);
        suppressed.suppress_source_item = true;
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyBasePower,
                suppressed,
            ),
            100
        );

        let mut other_source = context(MoveCategory::Physical, PokemonType::Normal);
        other_source.source = Some(PokemonRef::new(SideId::P2, PokemonIndex::new(0)));
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyBasePower,
                other_source,
            ),
            100
        );
    }

    #[test]
    fn all_type_power_handlers_apply_the_showdown_modifier_only_to_the_matching_type() {
        let types = [
            PokemonType::Bug,
            PokemonType::Dark,
            PokemonType::Dragon,
            PokemonType::Electric,
            PokemonType::Fairy,
            PokemonType::Fighting,
            PokemonType::Flying,
            PokemonType::Ghost,
            PokemonType::Grass,
            PokemonType::Ground,
            PokemonType::Ice,
            PokemonType::Normal,
            PokemonType::Poison,
            PokemonType::Psychic,
            PokemonType::Rock,
            PokemonType::Steel,
        ];
        for (offset, move_type) in types.into_iter().enumerate() {
            let item = ItemId::new(u32::try_from(100 + offset).unwrap());
            let mut registry = EffectRegistry::new();
            register(
                &mut registry,
                [ItemHandler::new(item, ItemBehavior::TypePower(move_type))],
            );
            let mut state = battle(item);
            assert_eq!(
                dispatch(
                    &registry,
                    &mut state,
                    BattleEvent::ModifyBasePower,
                    context(MoveCategory::Special, move_type),
                ),
                120
            );
            assert_eq!(
                dispatch(
                    &registry,
                    &mut state,
                    BattleEvent::ModifyBasePower,
                    context(MoveCategory::Special, PokemonType::Water),
                ),
                if move_type == PokemonType::Water {
                    120
                } else {
                    100
                }
            );
        }
    }

    #[test]
    fn resistance_berry_family_consumes_once_for_matching_super_effective_types() {
        let types = [
            PokemonType::Dark,
            PokemonType::Dragon,
            PokemonType::Fairy,
            PokemonType::Fighting,
            PokemonType::Fire,
            PokemonType::Flying,
            PokemonType::Ghost,
            PokemonType::Grass,
            PokemonType::Ground,
            PokemonType::Poison,
            PokemonType::Psychic,
            PokemonType::Rock,
            PokemonType::Steel,
            PokemonType::Water,
        ];
        for (offset, move_type) in types.into_iter().enumerate() {
            let item = ItemId::new(u32::try_from(200 + offset).unwrap());
            let mut registry = EffectRegistry::new();
            register(
                &mut registry,
                [ItemHandler::new(item, ItemBehavior::ResistBerry(move_type))],
            );
            let mut state = battle_with_items(None, Some(item));
            let mut berry_context = context(MoveCategory::Special, move_type);
            berry_context.type_effectiveness = Some(1);
            assert_eq!(
                dispatch(
                    &registry,
                    &mut state,
                    BattleEvent::ModifyDamage,
                    berry_context,
                ),
                50
            );
            assert_eq!(
                state
                    .side(SideId::P2)
                    .pokemon(PokemonIndex::new(0))
                    .unwrap()
                    .item,
                None
            );
        }
    }

    #[test]
    fn resistance_berry_interactions_are_suppressed_doubled_or_rejected_explicitly() {
        let item = ItemId::new(250);
        let mut registry = EffectRegistry::new();
        register(
            &mut registry,
            [ItemHandler::new(
                item,
                ItemBehavior::ResistBerry(PokemonType::Fire),
            )],
        );
        for blocked_context in [
            EventContext {
                suppress_target_item: true,
                ..context(MoveCategory::Special, PokemonType::Fire)
            },
            EventContext {
                target_berry_blocked: true,
                ..context(MoveCategory::Special, PokemonType::Fire)
            },
        ] {
            let mut state = battle_with_items(None, Some(item));
            let blocked_context = EventContext {
                type_effectiveness: Some(1),
                ..blocked_context
            };
            assert_eq!(
                dispatch(
                    &registry,
                    &mut state,
                    BattleEvent::ModifyDamage,
                    blocked_context,
                ),
                100
            );
            assert_eq!(
                state
                    .side(SideId::P2)
                    .pokemon(PokemonIndex::new(0))
                    .unwrap()
                    .item,
                Some(item)
            );
        }

        let mut state = battle_with_items(None, Some(item));
        let ripen_context = EventContext {
            type_effectiveness: Some(1),
            target_ripen: true,
            ..context(MoveCategory::Special, PokemonType::Fire)
        };
        assert_eq!(
            dispatch(
                &registry,
                &mut state,
                BattleEvent::ModifyDamage,
                ripen_context,
            ),
            25
        );

        let mut state = battle_with_items(None, Some(item));
        let deferred_context = EventContext {
            type_effectiveness: Some(1),
            target_berry_interaction_unsupported: true,
            ..context(MoveCategory::Special, PokemonType::Fire)
        };
        assert!(registry
            .dispatch(
                &mut state,
                BattleEvent::ModifyDamage,
                deferred_context,
                EventValue::Integer(100),
            )
            .is_err());
        assert_eq!(
            state
                .side(SideId::P2)
                .pokemon(PokemonIndex::new(0))
                .unwrap()
                .item,
            Some(item)
        );
    }
}
