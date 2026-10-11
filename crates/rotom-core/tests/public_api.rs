use rotom_core::{
    AbilityId, ActionOrder, ActionQueue, BattleEvent, BattlePhase, BattleRng, BattleState,
    CombatMove, ConditionId, DamageResult, EffectOrder, EffectRegistry, EffectSource, EventContext,
    EventValue, FieldState, FormId, Gender, ItemId, MoveId, NatureId, PokemonSet, PokemonType,
    QueuedAction, RngReason, SideId, SideState, SpeciesId, StatAllocation, TeamDefinition,
    TeamState, TurnResolution,
};

#[test]
fn foundational_types_are_available_from_the_crate_root() {
    assert_eq!(AbilityId::new(1).get(), 1);
    assert_eq!(ConditionId::new(7).get(), 7);
    assert_eq!(FormId::new(2).get(), 2);
    assert_eq!(ItemId::new(3).get(), 3);
    assert_eq!(MoveId::new(4).get(), 4);
    assert_eq!(NatureId::new(5).get(), 5);
    assert_eq!(SpeciesId::new(6).get(), 6);
    assert_eq!(SideId::P1.opponent(), SideId::P2);
    let _: Option<Gender> = None;
    let _: StatAllocation = StatAllocation::ZERO;
    let _: Option<PokemonSet> = None;
    let _: TeamDefinition = TeamDefinition {
        members: Vec::new(),
    };
    let _: Option<BattleState> = None;
    let _: Option<SideState> = None;
    let _: TeamState = TeamState::default();
    let _: BattlePhase = BattlePhase::TeamPreview;
    let _: FieldState = FieldState::default();
    let mut rng = BattleRng::new(123);
    assert!(rng.range(RngReason::SpeedTie, 0..2).is_ok());
    let _: ActionQueue = ActionQueue::default();
    let _: ActionOrder = ActionOrder::Move;
    let _: Option<QueuedAction> = None;
    let _: Option<CombatMove> = None;
    let _: Option<DamageResult> = None;
    let _: Option<PokemonType> = None;
    let _: TurnResolution = TurnResolution::default();
    let _: BattleEvent = BattleEvent::BeforeTurn;
    let _: EffectSource = EffectSource::Battle;
    let _: EffectOrder = EffectOrder::default();
    let _: EffectRegistry = EffectRegistry::new();
    let _: EventContext = EventContext::default();
    let _: EventValue = EventValue::None;
}
