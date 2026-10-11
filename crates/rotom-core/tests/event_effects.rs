use rotom_core::{
    AbilityId, ActionTarget, ActiveSlot, BattleError, BattleEvent, BattleState, BattleStats,
    CombatData, CombatMove, ConditionId, ConditionState, EffectInvocation, EffectOrder,
    EffectRegistry, EffectSource, EventContext, EventDispatch, EventValue, FormId, ItemId, MoveId,
    MoveSlot, MoveSlotIndex, MoveSupport, MoveTarget, PokemonAction, PokemonIndex, PokemonPosition,
    PokemonRef, PokemonState, PokemonType, PokemonTypes, SideId, SideState, SpeciesId, StatStages,
    StatusState, TeamState, TurnChoice,
};

const MOVE_ID: MoveId = MoveId::new(1);

struct TestData;

impl CombatData for TestData {
    fn move_data(&self, move_id: MoveId) -> Option<CombatMove> {
        (move_id == MOVE_ID).then_some(CombatMove {
            move_type: PokemonType::Normal,
            category: rotom_core::MoveCategory::Physical,
            base_power: 10,
            accuracy: rotom_core::MoveAccuracy::Always,
            crit_ratio: 0,
            target: MoveTarget::Normal,
            support: MoveSupport::BasicDamage,
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
    let mut battle = BattleState::new_with_seed(
        SideState::new(TeamState::new(p1)),
        SideState::new(TeamState::new(p2)),
        4,
        seed,
    )
    .expect("fixture should create a valid battle");
    let selection = vec![index(0), index(1), index(2), index(3)];
    battle
        .select_team(SideId::P1, selection.clone())
        .expect("P1 preview selection should be valid");
    battle
        .select_team(SideId::P2, selection)
        .expect("P2 preview selection should be valid");
    battle
}

#[allow(clippy::unnecessary_wraps)]
fn no_op(
    _: &mut BattleState,
    _: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn cancel(
    _: &mut BattleState,
    _: &EffectInvocation,
    _: &EventContext,
    dispatch: &mut EventDispatch,
) -> Result<(), BattleError> {
    dispatch.cancel();
    Ok(())
}

fn add_one_damage(
    _: &mut BattleState,
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
    _: &mut BattleState,
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
    battle: &mut BattleState,
    _: &EffectInvocation,
    _: &EventContext,
    _: &mut EventDispatch,
) -> Result<(), BattleError> {
    battle.turn += 100;
    Err(BattleError::UnsupportedMechanic(
        "test handler failure".to_owned(),
    ))
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

#[test]
fn dispatch_collects_active_sources_in_showdown_order() {
    let mut battle = battle(1);
    battle
        .field
        .conditions
        .push(ConditionState::new(ConditionId::new(30), Some(2)));
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::Residual,
        EffectSource::Battle,
        EffectOrder::new(0, 0, 0),
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

    let dispatch = registry
        .dispatch(
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

    let dispatch = registry
        .dispatch(
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
    let mut baseline = battle(7);
    baseline
        .queue_turn(both_attack_left(), both_attack_left())
        .expect("baseline choices should queue");
    let mut modified = baseline.clone();

    let baseline_resolution = baseline
        .resolve_turn(&TestData)
        .expect("baseline turn should resolve");
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::ModifyDamage,
        EffectSource::Move(MOVE_ID),
        EffectOrder::default(),
        add_one_damage,
    );
    let modified_resolution = modified
        .resolve_turn_with_effects(&TestData, &registry)
        .expect("effectful turn should resolve");

    for side in [SideId::P1, SideId::P2] {
        let baseline_hp = baseline
            .side(side)
            .pokemon(index(0))
            .expect("fixture target should exist")
            .current_hp;
        let modified_hp = modified
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
    let mut battle = battle(8);
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
        .queue_turn_with_effects(both_attack_left(), both_attack_left(), &registry)
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
    let mut battle = battle(8);
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
        .queue_turn_with_effects(p1, both_attack_left(), &registry)
        .expect("mixed switch and move choices should queue");
    let switching = PokemonRef::new(SideId::P1, index(0));

    assert!(queue.events.iter().all(|dispatch| {
        dispatch.event != BattleEvent::ModifyPriority || dispatch.context.source != Some(switching)
    }));
}

#[test]
fn invalid_active_indexes_fail_effect_collection_explicitly() {
    let mut battle = battle(8);
    battle.side_mut(SideId::P1).active[0] = Some(index(99));
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::Residual,
        EffectSource::Ability(AbilityId::new(1)),
        EffectOrder::default(),
        no_op,
    );

    assert!(matches!(
        registry.dispatch(
            &mut battle,
            BattleEvent::Residual,
            EventContext::default(),
            EventValue::None,
        ),
        Err(BattleError::InvalidState(_))
    ));
}

#[test]
fn handler_errors_roll_back_battle_and_rng_state() {
    let mut battle = battle(9);
    battle
        .queue_turn(both_attack_left(), both_attack_left())
        .expect("choices should queue");
    let before = battle.clone();
    let mut registry = EffectRegistry::new();
    registry.register(
        BattleEvent::BeforeTurn,
        EffectSource::Battle,
        EffectOrder::default(),
        fail_after_mutation,
    );

    assert!(matches!(
        battle.resolve_turn_with_effects(&TestData, &registry),
        Err(BattleError::UnsupportedMechanic(_))
    ));
    assert_eq!(battle, before);
}
