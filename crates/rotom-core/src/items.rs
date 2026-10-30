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
    FirePower,
    WaterPower,
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
            ItemBehavior::FirePower => registry.register(
                BattleEvent::ModifyBasePower,
                source,
                EffectOrder::new(0, 15, 0),
                fire_power,
            ),
            ItemBehavior::WaterPower => registry.register(
                BattleEvent::ModifyBasePower,
                source,
                EffectOrder::new(0, 15, 0),
                water_power,
            ),
            ItemBehavior::SuperEffectiveDamage => registry.register(
                BattleEvent::ModifyDamage,
                source,
                EffectOrder::default(),
                super_effective_damage,
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

fn fire_power(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    modify_source(
        invocation,
        context,
        dispatch,
        context.move_type == Some(PokemonType::Fire),
        4915,
    )
}

fn water_power(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    modify_source(
        invocation,
        context,
        dispatch,
        context.move_type == Some(PokemonType::Water),
        4915,
    )
}

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
        let source = SideState::new(TeamState::new(
            (0..4).map(|_| pokemon(Some(item))).collect(),
        ));
        let target = SideState::new(TeamState::new((0..4).map(|_| pokemon(None)).collect()));
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
                ItemHandler::new(charcoal, ItemBehavior::FirePower),
                ItemHandler::new(mystic_water, ItemBehavior::WaterPower),
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
}
