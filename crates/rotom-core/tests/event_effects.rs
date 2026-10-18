use rotom_core::{
    step, AbilityId, ActionTarget, ActiveSlot, BattleError, BattleEvent, BattleState,
    BattleStateBuilder, BattleStats, CombatData, CombatMove, ConditionId, ConditionState,
    EffectInvocation, EffectOrder, EffectRegistry, EffectSource, EffectState, EventContext,
    EventDispatch, EventValue, FormId, ItemId, JointAction, Mechanics, MoveCapabilities, MoveId,
    MoveOperation, MoveSlot, MoveSlotIndex, MoveTarget, PokemonAction, PokemonIndex,
    PokemonPosition, PokemonRef, PokemonState, PokemonType, PokemonTypes, SideId, SideState,
    SpeciesId, StatStages, StatusState, StepResult, TeamState, TurnChoice,
};
use serde::Deserialize;

const MOVE_ID: MoveId = MoveId::new(1);

struct TestData;

#[derive(Deserialize)]
struct EventOrderFixture {
    showdown_commit: String,
    format: String,
    cases: Vec<EventOrderCase>,
}

#[derive(Deserialize)]
struct EventOrderCase {
    name: String,
    handlers: Vec<ReferenceHandler>,
    expected: Vec<u32>,
}

#[derive(Deserialize)]
struct ReferenceHandler {
    label: u32,
    order: Option<i16>,
    priority: i16,
    speed: u16,
    sub_order: i16,
}

impl CombatData for TestData {
    fn move_data(&self, move_id: MoveId) -> Option<CombatMove> {
        (move_id == MOVE_ID).then_some(CombatMove {
            move_type: PokemonType::Normal,
            category: rotom_core::MoveCategory::Physical,
            base_power: 10,
            accuracy: rotom_core::MoveAccuracy::Always,
            crit_ratio: 0,
            target: MoveTarget::Normal,
            operation: MoveOperation::StandardDamage,
            required_capabilities: MoveCapabilities::PR_13_1,
            hit_sequence: rotom_core::HitSequence::Single,
            multi_accuracy: false,
            fixed_damage: None,
            drain: None,
            recoil: None,
            self_damage: None,
            prior_effect: rotom_core::PriorMoveEffect::None,
            offensive_stat: None,
            defensive_stat: None,
            ignore_negative_offensive: false,
            ignore_positive_defensive: false,
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

fn index(value: u8) -> PokemonIndex {
    PokemonIndex::new(value)
}

fn pokemon(species: u32, speed: u16, ability: u32, item: Option<u32>) -> PokemonState {
    PokemonState {
        species: SpeciesId::new(species),
        form: None,
        level: 50,
        stats: BattleStats {
            hp: 200,
            atk: 100,
            def: 100,
            spa: 100,
            spd: 100,
            spe: speed,
        },
        current_hp: 200,
        ability: AbilityId::new(ability),
        item: item.map(ItemId::new),
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

fn battle(seed: u64) -> BattleState {
    let p1 = vec![
        pokemon(1, 100, 1, Some(11)),
        pokemon(2, 200, 2, Some(12)),
        pokemon(3, 80, 3, None),
        pokemon(4, 70, 4, None),
    ];
    let p2 = vec![
        pokemon(101, 90, 5, None),
        pokemon(102, 60, 6, None),
        pokemon(103, 50, 7, None),
        pokemon(104, 40, 8, None),
    ];
    let mut builder = BattleStateBuilder::new_with_seed(
        SideState::new(TeamState::new(p1)),
        SideState::new(TeamState::new(p2)),
        4,
        seed,
    )
    .expect("fixture should create a valid battle");
    let selection = vec![index(0), index(1), index(2), index(3)];
    builder
        .select_teams(selection.clone(), selection)
        .expect("preview selection should be valid");
    builder.build().expect("fixture should be stable")
}

#[allow(clippy::unnecessary_wraps)]
fn no_op(
    _: &mut EffectState<'_>,
    _: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn cancel(
    _: &mut EffectState<'_>,
    _: &EffectInvocation,
    _: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    dispatch.cancel();
    Ok(())
}

fn add_one_damage(
    _: &mut EffectState<'_>,
    _: &EffectInvocation,
    _: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let EventValue::Integer(value) = &mut dispatch.value else {
        return Err(BattleError::InvalidState(
            "damage hook received a non-integer relay".to_owned(),
        ));
    };
    *value += 1;
    Ok(())
}

fn boost_own_action(
    _: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let rotom_core::EffectOwner::Pokemon { pokemon } = invocation.owner else {
        return Ok(());
    };
    if Some(pokemon) != context.source {
        return Ok(());
    }
    let EventValue::Integer(value) = &mut dispatch.value else {
        return Err(BattleError::InvalidState(
            "queue hook received a non-integer relay".to_owned(),
        ));
    };
    match dispatch.event {
        BattleEvent::ModifyPriority => *value += 1,
        BattleEvent::ModifySpeed => *value += 200,
        _ => {}
    }
    Ok(())
}

fn fail_after_mutation(
    state: &mut EffectState<'_>,
    _: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    state.add_field_condition(ConditionState::new(ConditionId::new(99), None))?;
    Err(BattleError::UnsupportedMechanic(
        "test handler failure".to_owned(),
    ))
}

#[allow(clippy::unnecessary_wraps)]
fn set_boolean_relay(
    _: &mut EffectState<'_>,
    _: &EffectInvocation,
    _: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    dispatch.value = EventValue::Boolean(true);
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn set_negative_integer(
    _: &mut EffectState<'_>,
    _: &EffectInvocation,
    _: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    dispatch.value = EventValue::Integer(-1);
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn set_large_integer(
    _: &mut EffectState<'_>,
    _: &EffectInvocation,
    _: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    dispatch.value = EventValue::Integer(i64::from(u16::MAX) + 1);
    Ok(())
}

fn add_two(
    _: &mut EffectState<'_>,
    _: &EffectInvocation,
    _: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let EventValue::Integer(value) = &mut dispatch.value else {
        return Err(BattleError::InvalidState(
            "expected integer relay".to_owned(),
        ));
    };
    *value += 2;
    Ok(())
}

fn triple(
    _: &mut EffectState<'_>,
    _: &EffectInvocation,
    _: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let EventValue::Integer(value) = &mut dispatch.value else {
        return Err(BattleError::InvalidState(
            "expected integer relay".to_owned(),
        ));
    };
    *value *= 3;
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn record_41(
    state: &mut EffectState<'_>,
    _: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    state.add_field_condition(ConditionState::new(ConditionId::new(41), None))
}

#[allow(clippy::unnecessary_wraps)]
fn record_42(
    state: &mut EffectState<'_>,
    _: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    state.add_field_condition(ConditionState::new(ConditionId::new(42), None))
}

fn record_42_and_cancel(
    state: &mut EffectState<'_>,
    invocation: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    record_42(state, invocation, context, dispatch)?;
    dispatch.cancel();
    Ok(())
}

fn observe_lifecycle(
    state: &mut EffectState<'_>,
    _: &EffectInvocation,
    context: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    let source = context
        .source
        .ok_or_else(|| BattleError::InvalidState("lifecycle source is missing".to_owned()))?;
    let target = context
        .target
        .ok_or_else(|| BattleError::InvalidState("lifecycle target is missing".to_owned()))?;
    let source_state = state
        .battle()
        .side(source.side)
        .pokemon(source.pokemon)
        .ok_or_else(|| BattleError::InvalidState("lifecycle source is invalid".to_owned()))?;
    let target_state = state
        .battle()
        .side(target.side)
        .pokemon(target.pokemon)
        .ok_or_else(|| BattleError::InvalidState("lifecycle target is invalid".to_owned()))?;
    let valid = match dispatch.event {
        BattleEvent::SwitchOut => {
            matches!(source_state.position, PokemonPosition::Active { .. })
                && target_state.position == PokemonPosition::Team
        }
        BattleEvent::SwitchIn => {
            matches!(source_state.position, PokemonPosition::Active { .. })
                && target_state.position == PokemonPosition::Team
        }
        BattleEvent::AfterDamage => {
            target != PokemonRef::new(SideId::P2, index(0)) || target_state.current_hp == 0
        }
        BattleEvent::Faint => {
            target_state.current_hp == 0
                && matches!(target_state.position, PokemonPosition::Active { .. })
        }
        _ => false,
    };
    if !valid {
        return Err(BattleError::InvalidState(format!(
            "unexpected state during {:?}",
            dispatch.event
        )));
    }
    Ok(())
}

fn observe_turn_boundary(
    state: &mut EffectState<'_>,
    _: &EffectInvocation,
    _: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    if state.battle().phase() != rotom_core::BattlePhase::ResolvingTurn {
        return Err(BattleError::InvalidState(format!(
            "{:?} ran outside turn resolution",
            dispatch.event
        )));
    }
    let marker = match dispatch.event {
        BattleEvent::BeforeTurn => 51,
        BattleEvent::Residual => 52,
        _ => return Ok(()),
    };
    state.add_field_condition(ConditionState::new(ConditionId::new(marker), None))
}

fn remove_slower_item(
    state: &mut EffectState<'_>,
    _: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    state.set_item(PokemonRef::new(SideId::P1, index(0)), None)?;
    Ok(())
}

fn both_attack_left() -> TurnChoice {
    let action = PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Foe(ActiveSlot::Left),
        mega: false,
    };
    TurnChoice {
        actions: [action, action],
    }
}

fn resolve_turn(
    battle: &BattleState,
    p1: TurnChoice,
    p2: TurnChoice,
    effects: &EffectRegistry,
) -> Result<StepResult, BattleError> {
    step(
        battle,
        &JointAction::Turn { p1, p2 },
        &Mechanics::new(&TestData, effects),
    )
}

fn dispatch_effect(
    registry: &EffectRegistry,
    battle: &mut BattleState,
    event: BattleEvent,
    context: EventContext,
    value: EventValue,
) -> Result<EventDispatch, BattleError> {
    let mut builder = BattleStateBuilder::from_state(battle.clone())?;
    let dispatch = builder.dispatch_effect_for_fixture(registry, event, context, value)?;
    *battle = builder.build()?;
    Ok(dispatch)
}

#[test]
fn dispatch_collects_active_sources_in_showdown_order() {
    let mut builder = BattleStateBuilder::from_state(battle(1)).expect("fixture should be stable");
    builder.edit_field(|field| {
        field
            .conditions
            .push(ConditionState::new(ConditionId::new(30), Some(2)));
    });
    let mut battle = builder.build().expect("fixture edit should be stable");
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::Residual,
        EffectSource::Battle,
        EffectOrder::new(1, 0, 0),
        no_op,
    );
    registry.register(
        BattleEvent::Residual,
        EffectSource::Ability(AbilityId::new(1)),
        EffectOrder::new(i16::MAX, 1, 0),
        no_op,
    );
    registry.register(
        BattleEvent::Residual,
        EffectSource::Ability(AbilityId::new(2)),
        EffectOrder::default(),
        no_op,
    );
    registry.register(
        BattleEvent::Residual,
        EffectSource::Item(ItemId::new(12)),
        EffectOrder::default(),
        no_op,
    );
    registry.register(
        BattleEvent::Residual,
        EffectSource::Condition(ConditionId::new(30)),
        EffectOrder::default(),
        no_op,
    );

    let dispatch = dispatch_effect(
        &registry,
        &mut battle,
        BattleEvent::Residual,
        EventContext::default(),
        EventValue::None,
    )
    .expect("registered effects should dispatch");
    let sources = dispatch
        .invocations
        .iter()
        .map(|invocation| invocation.source)
        .collect::<Vec<_>>();

    assert_eq!(
        sources,
        vec![
            EffectSource::Battle,
            EffectSource::Ability(AbilityId::new(1)),
            EffectSource::Ability(AbilityId::new(2)),
            EffectSource::Item(ItemId::new(12)),
            EffectSource::Condition(ConditionId::new(30)),
        ]
    );
    assert_eq!(dispatch.invocations[1].speed, 100);
    assert_eq!(dispatch.invocations[1].order.sub_order, 7);
    assert_eq!(dispatch.invocations[2].speed, 200);
    assert_eq!(dispatch.invocations[2].order.sub_order, 7);
    assert_eq!(dispatch.invocations[3].order.sub_order, 8);
    assert_eq!(dispatch.invocations[4].order.sub_order, 5);
}

#[test]
fn ordering_keys_match_the_pinned_showdown_comparator_fixture() {
    let fixture: EventOrderFixture = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/fixtures/differential/event-order.json"
    )))
    .expect("event-order fixture should deserialize");
    assert_eq!(
        fixture.showdown_commit,
        "b79ce06d06b8f0dc8de8bb348bc9cab5d931553f"
    );
    assert_eq!(fixture.format, "gen9championsvgc2026regmc");
    let holders = [
        PokemonRef::new(SideId::P1, index(0)),
        PokemonRef::new(SideId::P1, index(1)),
        PokemonRef::new(SideId::P2, index(0)),
        PokemonRef::new(SideId::P2, index(1)),
    ];

    for test_case in fixture.cases {
        let mut builder =
            BattleStateBuilder::from_state(battle(1)).expect("fixture should be stable");
        for (offset, holder) in holders.into_iter().enumerate() {
            builder
                .edit_pokemon(holder, |state| {
                    state.ability = AbilityId::new(
                        1_000 + u32::try_from(offset).expect("fixture offset should fit in u32"),
                    );
                })
                .expect("fixture holder should exist");
        }
        let mut registry = EffectRegistry::new();
        for (handler, holder) in test_case.handlers.iter().zip(holders) {
            builder
                .edit_pokemon(holder, |state| {
                    state.ability = AbilityId::new(handler.label);
                    state.stats.spe = handler.speed;
                })
                .expect("fixture holder should exist");
            registry.register(
                BattleEvent::Residual,
                EffectSource::Ability(AbilityId::new(handler.label)),
                EffectOrder::new(
                    handler.order.unwrap_or(i16::MAX),
                    handler.priority,
                    handler.sub_order,
                ),
                no_op,
            );
        }
        let mut battle = builder.build().expect("fixture edits should be stable");

        let dispatch = dispatch_effect(
            &registry,
            &mut battle,
            BattleEvent::Residual,
            EventContext::default(),
            EventValue::None,
        )
        .unwrap_or_else(|error| panic!("{} failed: {error}", test_case.name));
        let actual = dispatch
            .invocations
            .iter()
            .filter_map(|invocation| match invocation.source {
                EffectSource::Ability(id) => Some(id.get()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, test_case.expected, "{}", test_case.name);
    }
}

#[test]
fn cancellation_stops_later_handlers() {
    let mut battle = battle(2);
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::TryHit,
        EffectSource::Battle,
        EffectOrder::new(0, 1, 0),
        cancel,
    );
    registry.register(
        BattleEvent::TryHit,
        EffectSource::Battle,
        EffectOrder::new(0, 0, 0),
        no_op,
    );

    let dispatch = dispatch_effect(
        &registry,
        &mut battle,
        BattleEvent::TryHit,
        EventContext::default(),
        EventValue::Boolean(true),
    )
    .expect("cancellation is a successful dispatch");

    assert!(dispatch.cancelled);
    assert_eq!(dispatch.invocations.len(), 1);
}

#[test]
fn move_damage_hook_changes_combat_and_emits_inspectable_events() {
    let baseline = battle(7);
    let modified = baseline.clone();
    let baseline_result = resolve_turn(
        &baseline,
        both_attack_left(),
        both_attack_left(),
        &EffectRegistry::new(),
    )
    .expect("baseline turn should resolve");
    let baseline_resolution = baseline_result
        .diagnostics
        .turn
        .expect("turn diagnostics should be present");
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::ModifyDamage,
        EffectSource::Move(MOVE_ID),
        EffectOrder::default(),
        add_one_damage,
    );
    let modified_result =
        resolve_turn(&modified, both_attack_left(), both_attack_left(), &registry)
            .expect("effectful turn should resolve");
    let modified_resolution = modified_result
        .diagnostics
        .turn
        .expect("turn diagnostics should be present");

    for side in [SideId::P1, SideId::P2] {
        let baseline_hp = baseline_result
            .state
            .side(side)
            .pokemon(index(0))
            .expect("fixture target should exist")
            .current_hp;
        let modified_hp = modified_result
            .state
            .side(side)
            .pokemon(index(0))
            .expect("fixture target should exist")
            .current_hp;
        assert_eq!(modified_hp + 2, baseline_hp);
    }
    assert_eq!(baseline_resolution.actions.len(), 4);
    assert_eq!(modified_resolution.actions.len(), 4);
    let damage_dispatches = modified_resolution
        .events
        .iter()
        .filter(|dispatch| dispatch.event == BattleEvent::ModifyDamage)
        .collect::<Vec<_>>();
    assert_eq!(damage_dispatches.len(), 4);
    assert!(damage_dispatches.iter().all(|dispatch| {
        dispatch.invocations.len() == 1
            && dispatch.invocations[0].source == EffectSource::Move(MOVE_ID)
    }));
    assert_eq!(modified_resolution.events[0].event, BattleEvent::BeforeTurn);
}

#[test]
fn queue_hooks_modify_priority_and_speed_before_sorting() {
    let battle = battle(8);
    let mut registry = EffectRegistry::new();
    for event in [BattleEvent::ModifyPriority, BattleEvent::ModifySpeed] {
        registry.register(
            event,
            EffectSource::Ability(AbilityId::new(1)),
            EffectOrder::default(),
            boost_own_action,
        );
    }

    let queue = battle
        .inspect_turn_queue(
            both_attack_left(),
            both_attack_left(),
            &Mechanics::new(&TestData, &registry),
        )
        .expect("queue modifiers should produce valid ordering values");

    assert_eq!(queue.actions[0].side, SideId::P1);
    assert_eq!(queue.actions[0].pokemon, index(0));
    assert_eq!(queue.actions[0].priority, 1);
    assert_eq!(queue.actions[0].speed, 300);
    assert_eq!(queue.events.len(), 8);
    assert_eq!(queue.events[0].event, BattleEvent::ModifyPriority);
    assert_eq!(queue.events[1].event, BattleEvent::ModifySpeed);
}

#[test]
fn switch_actions_skip_move_priority_hooks() {
    let battle = battle(8);
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::ModifyPriority,
        EffectSource::Battle,
        EffectOrder::default(),
        no_op,
    );
    let p1 = TurnChoice {
        actions: [
            PokemonAction::Switch { pokemon: index(2) },
            both_attack_left().actions[1],
        ],
    };

    let queue = battle
        .inspect_turn_queue(
            p1,
            both_attack_left(),
            &Mechanics::new(&TestData, &registry),
        )
        .expect("mixed switch and move choices should queue");
    let switching = PokemonRef::new(SideId::P1, index(0));

    assert!(queue.events.iter().all(|dispatch| {
        dispatch.event != BattleEvent::ModifyPriority || dispatch.context.source != Some(switching)
    }));
}

#[test]
fn invalid_active_indexes_fail_effect_collection_explicitly() {
    let mut value = serde_json::to_value(battle(8)).expect("fixture should serialize");
    value["sides"][0]["active"][0] = serde_json::json!(99);
    let mut battle: BattleState =
        serde_json::from_value(value).expect("negative fixture should deserialize unchecked");
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::Residual,
        EffectSource::Ability(AbilityId::new(1)),
        EffectOrder::default(),
        no_op,
    );

    assert!(matches!(
        dispatch_effect(
            &registry,
            &mut battle,
            BattleEvent::Residual,
            EventContext::default(),
            EventValue::None,
        ),
        Err(BattleError::InvalidState(_))
    ));
}

#[test]
fn equal_keys_use_stable_registration_order_and_sub_order_precedes_it() {
    let mut tied = battle(10);
    let mut tied_registry = EffectRegistry::new();
    tied_registry.register(
        BattleEvent::Residual,
        EffectSource::Battle,
        EffectOrder::default(),
        record_41,
    );
    tied_registry.register(
        BattleEvent::Residual,
        EffectSource::Battle,
        EffectOrder::default(),
        record_42,
    );
    dispatch_effect(
        &tied_registry,
        &mut tied,
        BattleEvent::Residual,
        EventContext::default(),
        EventValue::None,
    )
    .expect("tied handlers should dispatch");
    assert_eq!(
        tied.field()
            .conditions
            .iter()
            .map(|condition| condition.id)
            .collect::<Vec<_>>(),
        vec![ConditionId::new(41), ConditionId::new(42)]
    );

    let mut sub_ordered = battle(10);
    let mut sub_order_registry = EffectRegistry::new();
    sub_order_registry.register(
        BattleEvent::Residual,
        EffectSource::Battle,
        EffectOrder::new(i16::MAX, 0, 2),
        record_42,
    );
    sub_order_registry.register(
        BattleEvent::Residual,
        EffectSource::Battle,
        EffectOrder::new(i16::MAX, 0, 1),
        record_41,
    );
    dispatch_effect(
        &sub_order_registry,
        &mut sub_ordered,
        BattleEvent::Residual,
        EventContext::default(),
        EventValue::None,
    )
    .expect("sub-ordered handlers should dispatch");
    assert_eq!(
        sub_ordered
            .field()
            .conditions
            .iter()
            .map(|condition| condition.id)
            .collect::<Vec<_>>(),
        vec![ConditionId::new(41), ConditionId::new(42)]
    );
}

#[test]
fn multiple_holders_of_one_effect_are_ordered_by_speed() {
    let mut builder = BattleStateBuilder::from_state(battle(11)).expect("fixture should be stable");
    builder
        .edit_pokemon(PokemonRef::new(SideId::P1, index(1)), |pokemon| {
            pokemon.ability = AbilityId::new(1);
        })
        .expect("fixture holder should exist");
    let mut battle = builder.build().expect("fixture edit should be stable");
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::Residual,
        EffectSource::Ability(AbilityId::new(1)),
        EffectOrder::default(),
        no_op,
    );

    let dispatch = dispatch_effect(
        &registry,
        &mut battle,
        BattleEvent::Residual,
        EventContext::default(),
        EventValue::None,
    )
    .expect("both holders should dispatch");

    assert_eq!(dispatch.invocations.len(), 2);
    assert_eq!(dispatch.invocations[0].speed, 200);
    assert_eq!(dispatch.invocations[1].speed, 100);
}

#[test]
fn discovery_distinguishes_holders_and_ignores_benched_effects() {
    let mut builder = BattleStateBuilder::from_state(battle(12)).expect("fixture should be stable");
    let condition = ConditionId::new(60);
    builder.edit_field(|field| {
        field.conditions.push(ConditionState::new(condition, None));
    });
    builder.edit_side(SideId::P1, |side| {
        side.side_conditions
            .push(ConditionState::new(condition, None));
    });
    builder
        .edit_pokemon(PokemonRef::new(SideId::P1, index(0)), |pokemon| {
            pokemon.volatiles.push(ConditionState::new(condition, None));
        })
        .expect("active fixture should exist");
    let mut battle = builder.build().expect("fixture edits should be stable");
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::Residual,
        EffectSource::Condition(condition),
        EffectOrder::default(),
        no_op,
    );
    registry.register(
        BattleEvent::Residual,
        EffectSource::Ability(AbilityId::new(3)),
        EffectOrder::default(),
        no_op,
    );

    let dispatch = dispatch_effect(
        &registry,
        &mut battle,
        BattleEvent::Residual,
        EventContext::default(),
        EventValue::None,
    )
    .expect("condition holders should dispatch");
    let owners = dispatch
        .invocations
        .iter()
        .map(|invocation| invocation.owner)
        .collect::<Vec<_>>();
    assert_eq!(owners.len(), 3);
    assert!(owners.contains(&rotom_core::EffectOwner::Field));
    assert!(owners.contains(&rotom_core::EffectOwner::Side { side: SideId::P1 }));
    assert!(owners.contains(&rotom_core::EffectOwner::Pokemon {
        pokemon: PokemonRef::new(SideId::P1, index(0)),
    }));
    assert_eq!(dispatch.invocations[0].order.sub_order, 2);
    assert_eq!(dispatch.invocations[1].order.sub_order, 4);
    assert_eq!(dispatch.invocations[2].order.sub_order, 5);
    assert!(dispatch
        .invocations
        .iter()
        .all(|invocation| invocation.source != EffectSource::Ability(AbilityId::new(3))));

    let mut builder = BattleStateBuilder::from_state(battle).expect("fixture should be stable");
    builder.edit_field(|field| field.conditions.clear());
    builder.edit_side(SideId::P1, |side| side.side_conditions.clear());
    builder
        .edit_pokemon(PokemonRef::new(SideId::P1, index(0)), |pokemon| {
            pokemon.volatiles.clear();
        })
        .expect("active fixture should exist");
    let mut battle = builder.build().expect("fixture edits should be stable");
    let removed = dispatch_effect(
        &registry,
        &mut battle,
        BattleEvent::Residual,
        EventContext::default(),
        EventValue::None,
    )
    .expect("removed effects should not fail dispatch");
    assert_eq!(removed.invocations.len(), 0);
}

#[test]
fn collected_handlers_remain_eligible_for_the_current_dispatch() {
    let mut builder = BattleStateBuilder::from_state(battle(13)).expect("fixture should be stable");
    builder
        .edit_pokemon(PokemonRef::new(SideId::P1, index(1)), |pokemon| {
            pokemon.item = Some(ItemId::new(11));
        })
        .expect("fixture holder should exist");
    let mut battle = builder.build().expect("fixture edit should be stable");
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::Residual,
        EffectSource::Item(ItemId::new(11)),
        EffectOrder::default(),
        remove_slower_item,
    );

    let dispatch = dispatch_effect(
        &registry,
        &mut battle,
        BattleEvent::Residual,
        EventContext::default(),
        EventValue::None,
    )
    .expect("snapshotted handlers should finish dispatch");

    assert_eq!(dispatch.invocations.len(), 2);
    assert_eq!(dispatch.invocations[0].speed, 200);
    assert_eq!(dispatch.invocations[1].speed, 100);
    assert_eq!(
        battle
            .side(SideId::P1)
            .pokemon(index(0))
            .expect("fixture holder should exist")
            .item,
        None
    );
}

#[test]
fn relay_contract_rejects_wrong_types_illegal_cancellation_and_read_only_changes() {
    let cases = [
        (
            BattleEvent::ModifyDamage,
            EventValue::Integer(10),
            set_boolean_relay as rotom_core::EffectCallback,
        ),
        (
            BattleEvent::ModifyDamage,
            EventValue::Integer(10),
            cancel as rotom_core::EffectCallback,
        ),
        (
            BattleEvent::AfterDamage,
            EventValue::Integer(10),
            set_negative_integer as rotom_core::EffectCallback,
        ),
    ];
    for (event, value, callback) in cases {
        let mut battle = battle(14);
        let mut registry = EffectRegistry::new();
        registry.register(
            event,
            EffectSource::Battle,
            EffectOrder::default(),
            callback,
        );
        assert!(matches!(
            dispatch_effect(
                &registry,
                &mut battle,
                event,
                EventContext::default(),
                value
            ),
            Err(BattleError::InvalidState(_))
        ));
    }

    let mut battle = battle(14);
    assert!(matches!(
        dispatch_effect(
            &EffectRegistry::new(),
            &mut battle,
            BattleEvent::ModifyDamage,
            EventContext::default(),
            EventValue::Boolean(true),
        ),
        Err(BattleError::InvalidState(_))
    ));
}

#[test]
fn cancellation_in_the_middle_preserves_prior_mutation_and_stops_later_handlers() {
    let mut battle = battle(14);
    let mut registry = EffectRegistry::new();
    for callback in [
        record_41 as rotom_core::EffectCallback,
        record_42_and_cancel as rotom_core::EffectCallback,
        record_41 as rotom_core::EffectCallback,
    ] {
        registry.register(
            BattleEvent::BeforeMove,
            EffectSource::Battle,
            EffectOrder::default(),
            callback,
        );
    }

    let dispatch = dispatch_effect(
        &registry,
        &mut battle,
        BattleEvent::BeforeMove,
        EventContext::default(),
        EventValue::None,
    )
    .expect("middle cancellation should complete dispatch");

    assert!(dispatch.cancelled);
    assert_eq!(dispatch.invocations.len(), 2);
    assert_eq!(
        battle
            .field()
            .conditions
            .iter()
            .map(|condition| condition.id)
            .collect::<Vec<_>>(),
        vec![ConditionId::new(41), ConditionId::new(42)]
    );
}

#[test]
fn every_event_contract_accepts_its_declared_initial_relay() {
    let events = [
        BattleEvent::BeforeTurn,
        BattleEvent::BeforeMove,
        BattleEvent::ModifyPriority,
        BattleEvent::ModifySpeed,
        BattleEvent::ModifyAttack,
        BattleEvent::ModifyDefense,
        BattleEvent::ModifyBasePower,
        BattleEvent::ModifyDamage,
        BattleEvent::TryHit,
        BattleEvent::Hit,
        BattleEvent::AfterDamage,
        BattleEvent::AfterMove,
        BattleEvent::SwitchIn,
        BattleEvent::SwitchOut,
        BattleEvent::Faint,
        BattleEvent::Residual,
    ];
    for event in events {
        let value = match event.contract().relay {
            rotom_core::EventRelay::None => EventValue::None,
            rotom_core::EventRelay::Boolean => EventValue::Boolean(true),
            rotom_core::EventRelay::Integer => EventValue::Integer(1),
        };
        dispatch_effect(
            &EffectRegistry::new(),
            &mut battle(14),
            event,
            EventContext::default(),
            value,
        )
        .expect("declared relay should satisfy its event contract");
    }
}

#[test]
fn event_dispatch_trace_round_trips_through_json() {
    let mut battle = battle(14);
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::Residual,
        EffectSource::Ability(AbilityId::new(1)),
        EffectOrder::default(),
        no_op,
    );
    let dispatch = dispatch_effect(
        &registry,
        &mut battle,
        BattleEvent::Residual,
        EventContext::default(),
        EventValue::None,
    )
    .expect("trace should dispatch");

    let json = serde_json::to_string(&dispatch).expect("trace should serialize");
    let decoded: EventDispatch = serde_json::from_str(&json).expect("trace should deserialize");
    assert_eq!(decoded, dispatch);
}

#[test]
fn sequential_integer_modifiers_use_ordered_relay_results() {
    let baseline = battle(15);
    let modified = baseline.clone();
    let baseline_resolution = resolve_turn(
        &baseline,
        both_attack_left(),
        both_attack_left(),
        &EffectRegistry::new(),
    )
    .expect("baseline should resolve")
    .diagnostics
    .turn
    .expect("turn diagnostics should be present");
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::ModifyDamage,
        EffectSource::Move(MOVE_ID),
        EffectOrder::default(),
        add_two,
    );
    registry.register(
        BattleEvent::ModifyDamage,
        EffectSource::Move(MOVE_ID),
        EffectOrder::default(),
        triple,
    );
    let modified_resolution =
        resolve_turn(&modified, both_attack_left(), both_attack_left(), &registry)
            .expect("ordered modifiers should resolve")
            .diagnostics
            .turn
            .expect("turn diagnostics should be present");
    let baseline_damage = baseline_resolution.actions.iter().filter_map(|action| {
        if let rotom_core::ResolvedAction::Damaged { damage, .. } = action {
            Some(damage.final_damage)
        } else {
            None
        }
    });
    let modified_damage = modified_resolution.actions.iter().filter_map(|action| {
        if let rotom_core::ResolvedAction::Damaged { damage, .. } = action {
            Some(damage.final_damage)
        } else {
            None
        }
    });
    for (baseline, modified) in baseline_damage.zip(modified_damage) {
        assert_eq!(modified, (baseline + 2) * 3);
    }
}

#[test]
fn invalid_numeric_outputs_roll_back_queue_and_resolution() {
    let queue_cases = [
        (
            BattleEvent::ModifyPriority,
            set_large_integer as rotom_core::EffectCallback,
        ),
        (
            BattleEvent::ModifySpeed,
            set_negative_integer as rotom_core::EffectCallback,
        ),
    ];
    for (event, callback) in queue_cases {
        let battle = battle(16);
        let before = battle.clone();
        let mut registry = EffectRegistry::new();
        registry.register(
            event,
            EffectSource::Battle,
            EffectOrder::default(),
            callback,
        );
        assert!(matches!(
            battle.inspect_turn_queue(
                both_attack_left(),
                both_attack_left(),
                &Mechanics::new(&TestData, &registry),
            ),
            Err(BattleError::InvalidState(_))
        ));
        assert_eq!(battle, before);
    }

    for callback in [
        set_negative_integer as rotom_core::EffectCallback,
        set_large_integer as rotom_core::EffectCallback,
    ] {
        let battle = battle(16);
        let before = battle.clone();
        let mut registry = EffectRegistry::new();
        registry.register(
            BattleEvent::ModifyDamage,
            EffectSource::Move(MOVE_ID),
            EffectOrder::default(),
            callback,
        );
        assert!(matches!(
            resolve_turn(&battle, both_attack_left(), both_attack_left(), &registry,),
            Err(BattleError::InvalidState(_))
        ));
        assert_eq!(battle, before);
    }
}

#[test]
fn switch_hooks_see_pre_exit_and_post_entry_state_in_order() {
    let battle = battle(17);
    let p1 = TurnChoice {
        actions: [
            PokemonAction::Switch { pokemon: index(2) },
            both_attack_left().actions[1],
        ],
    };
    let mut registry = EffectRegistry::new();
    for event in [BattleEvent::SwitchOut, BattleEvent::SwitchIn] {
        registry.register(
            event,
            EffectSource::Battle,
            EffectOrder::default(),
            observe_lifecycle,
        );
    }

    let result = resolve_turn(&battle, p1, both_attack_left(), &registry)
        .expect("switch lifecycle should resolve");
    let resolution = result
        .diagnostics
        .turn
        .expect("turn diagnostics should be present");
    let switch_events = resolution
        .events
        .iter()
        .filter(|dispatch| {
            matches!(
                dispatch.event,
                BattleEvent::SwitchOut | BattleEvent::SwitchIn
            )
        })
        .map(|dispatch| dispatch.event)
        .collect::<Vec<_>>();

    assert_eq!(
        switch_events,
        vec![BattleEvent::SwitchOut, BattleEvent::SwitchIn]
    );
    assert_eq!(result.state.side(SideId::P1).active[0], Some(index(2)));
}

#[test]
fn cancelling_switch_out_preserves_positions_and_skips_switch_in() {
    let battle = battle(18);
    let p1 = TurnChoice {
        actions: [
            PokemonAction::Switch { pokemon: index(2) },
            both_attack_left().actions[1],
        ],
    };
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::SwitchOut,
        EffectSource::Battle,
        EffectOrder::default(),
        cancel,
    );

    let result = resolve_turn(&battle, p1, both_attack_left(), &registry)
        .expect("cancelling a switch is a valid resolution");
    let resolution = result
        .diagnostics
        .turn
        .expect("turn diagnostics should be present");

    assert_eq!(result.state.side(SideId::P1).active[0], Some(index(0)));
    assert_eq!(
        result
            .state
            .side(SideId::P1)
            .pokemon(index(2))
            .expect("switch target should exist")
            .position,
        PokemonPosition::Team
    );
    assert!(resolution
        .events
        .iter()
        .all(|dispatch| dispatch.event != BattleEvent::SwitchIn));
    assert!(resolution.actions.iter().any(|action| {
        matches!(
            action,
            rotom_core::ResolvedAction::Skipped {
                side: SideId::P1,
                slot: ActiveSlot::Left
            }
        )
    }));
}

#[test]
fn before_move_cancellation_preserves_pp_hp_and_rng() {
    let battle = battle(18);
    let before_rng = battle.rng().clone();
    let before_hp = battle
        .side(SideId::P2)
        .pokemon(index(0))
        .expect("target should exist")
        .current_hp;
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::BeforeMove,
        EffectSource::Move(MOVE_ID),
        EffectOrder::default(),
        cancel,
    );

    let result = resolve_turn(&battle, both_attack_left(), both_attack_left(), &registry)
        .expect("before-move cancellation should resolve");
    let resolution = result
        .diagnostics
        .turn
        .expect("turn diagnostics should be present");

    assert_eq!(result.state.rng(), &before_rng);
    assert_eq!(
        result
            .state
            .side(SideId::P2)
            .pokemon(index(0))
            .expect("target should exist")
            .current_hp,
        before_hp
    );
    assert!(result
        .state
        .sides()
        .iter()
        .all(|side| side.active.iter().flatten().all(|index| side
            .pokemon(*index)
            .is_some_and(|pokemon| pokemon.moves[0].pp == 16))));
    assert!(resolution
        .actions
        .iter()
        .all(|action| matches!(action, rotom_core::ResolvedAction::Skipped { .. })));
}

#[test]
fn try_hit_cancellation_occurs_after_pp_but_before_damage_rng() {
    let battle = battle(18);
    let before_rng = battle.rng().clone();
    let before_hp = battle
        .side(SideId::P2)
        .pokemon(index(0))
        .expect("target should exist")
        .current_hp;
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::TryHit,
        EffectSource::Move(MOVE_ID),
        EffectOrder::default(),
        cancel,
    );

    let result = resolve_turn(&battle, both_attack_left(), both_attack_left(), &registry)
        .expect("try-hit cancellation should resolve");

    assert_eq!(result.state.rng(), &before_rng);
    assert_eq!(
        result
            .state
            .side(SideId::P2)
            .pokemon(index(0))
            .expect("target should exist")
            .current_hp,
        before_hp
    );
    assert!(result
        .state
        .sides()
        .iter()
        .all(|side| side.active.iter().flatten().all(|index| side
            .pokemon(*index)
            .is_some_and(|pokemon| pokemon.moves[0].pp == 15))));
}

#[test]
fn faint_and_after_damage_hooks_run_before_the_target_leaves_the_field() {
    let mut builder = BattleStateBuilder::from_state(battle(19)).expect("fixture should be stable");
    builder
        .edit_pokemon(PokemonRef::new(SideId::P2, index(0)), |pokemon| {
            pokemon.current_hp = 1;
        })
        .expect("faint target should exist");
    let battle = builder.build().expect("fixture edit should be stable");
    let p1 = TurnChoice {
        actions: [
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(ActiveSlot::Right),
                mega: false,
            },
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(ActiveSlot::Left),
                mega: false,
            },
        ],
    };
    let p2 = TurnChoice {
        actions: [
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(ActiveSlot::Left),
                mega: false,
            },
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(ActiveSlot::Right),
                mega: false,
            },
        ],
    };
    let mut registry = EffectRegistry::new();
    for event in [BattleEvent::AfterDamage, BattleEvent::Faint] {
        registry.register(
            event,
            EffectSource::Battle,
            EffectOrder::default(),
            observe_lifecycle,
        );
    }

    let result = resolve_turn(&battle, p1, p2, &registry).expect("fainting turn should resolve");
    let resolution = result
        .diagnostics
        .turn
        .expect("turn diagnostics should be present");
    let faint = resolution
        .events
        .iter()
        .position(|dispatch| dispatch.event == BattleEvent::Faint)
        .expect("faint event should be emitted");
    assert_eq!(resolution.events[faint - 1].event, BattleEvent::AfterDamage);
    assert_eq!(
        resolution.events[faint - 1].value,
        EventValue::Integer(1),
        "AfterDamage must relay actual clamped HP loss"
    );
    assert_eq!(result.state.side(SideId::P2).active[0], None);
    assert_eq!(
        result
            .state
            .side(SideId::P2)
            .pokemon(index(0))
            .expect("fainted target should exist")
            .position,
        PokemonPosition::Fainted
    );
}

#[test]
fn turn_boundary_hooks_run_in_resolution_before_actions_and_after_them() {
    let battle = battle(20);
    let mut registry = EffectRegistry::new();
    for event in [BattleEvent::BeforeTurn, BattleEvent::Residual] {
        registry.register(
            event,
            EffectSource::Battle,
            EffectOrder::default(),
            observe_turn_boundary,
        );
    }

    let result = resolve_turn(&battle, both_attack_left(), both_attack_left(), &registry)
        .expect("turn boundary hooks should resolve");
    let resolution = result
        .diagnostics
        .turn
        .expect("turn diagnostics should be present");
    assert_eq!(resolution.events[0].event, BattleEvent::BeforeTurn);
    assert_eq!(
        resolution.events.last().map(|dispatch| dispatch.event),
        Some(BattleEvent::Residual)
    );
    assert_eq!(
        result
            .state
            .field()
            .conditions
            .iter()
            .map(|condition| condition.id)
            .collect::<Vec<_>>(),
        vec![ConditionId::new(51), ConditionId::new(52)]
    );
}

#[test]
fn cloned_battles_with_one_registry_reproduce_state_rng_and_event_trace() {
    let left = battle(21);
    let right = left.clone();
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::ModifyDamage,
        EffectSource::Move(MOVE_ID),
        EffectOrder::default(),
        add_one_damage,
    );

    let left_result = resolve_turn(&left, both_attack_left(), both_attack_left(), &registry)
        .expect("left clone should resolve");
    let right_result = resolve_turn(&right, both_attack_left(), both_attack_left(), &registry)
        .expect("right clone should resolve");

    assert_eq!(left_result.diagnostics, right_result.diagnostics);
    assert_eq!(left_result.state, right_result.state);
    assert_eq!(left_result.state.rng(), right_result.state.rng());
}

#[test]
fn handler_errors_roll_back_battle_and_rng_state() {
    let battle = battle(9);
    let before = battle.clone();
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::BeforeTurn,
        EffectSource::Battle,
        EffectOrder::default(),
        fail_after_mutation,
    );

    assert!(matches!(
        resolve_turn(&battle, both_attack_left(), both_attack_left(), &registry,),
        Err(BattleError::UnsupportedMechanic(_))
    ));
    assert_eq!(battle, before);
}
