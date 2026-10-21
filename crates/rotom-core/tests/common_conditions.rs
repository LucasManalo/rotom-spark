use rotom_core::{
    calculate_damage, step, AbilityId, ActionTarget, BattlePhase, BattleStat, BattleState,
    BattleStateBuilder, BattleStats, CombatData, CombatMove, ConditionId, ConditionState,
    EffectRegistry, FormId, JointAction, MajorStatus, Mechanics, Modifier, MoveAccuracy,
    MoveCapabilities, MoveCategory, MoveId, MoveOperation, MoveSlot, MoveSlotIndex, MoveTarget,
    PokemonAction, PokemonIndex, PokemonPosition, PokemonRef, PokemonState, PokemonType,
    PokemonTypes, RngOperation, RngReason, SideId, SideState, StatStages, StatusState, TeamState,
    TurnChoice,
};
use serde::Deserialize;

const MOVE_ID: MoveId = MoveId::new(1);

struct TestData;

#[derive(Deserialize)]
struct ConditionFixture {
    showdown_commit: String,
    format: String,
    paralysis: ParalysisFixture,
    sleep: SleepFixture,
    freeze: FreezeFixture,
    residual: ResidualFixture,
}

#[derive(Deserialize)]
struct ParalysisFixture {
    speed_from_101: u16,
    before_move_chance: [u64; 2],
}

#[derive(Deserialize)]
struct SleepFixture {
    durations: Vec<u8>,
    initialized_turns: u8,
}

#[derive(Deserialize)]
struct FreezeFixture {
    initialized_turns: u8,
    early_thaw_chance: [u64; 2],
    automatic_thaw_attempt: u8,
}

#[derive(Deserialize)]
struct ResidualFixture {
    max_hp: u16,
    poison: ResidualStatusFixture,
    toxic: ResidualStatusFixture,
    burn: ResidualStatusFixture,
}

#[derive(Deserialize)]
struct ResidualStatusFixture {
    order: i16,
    damage: Vec<u16>,
}

fn reference_fixture() -> ConditionFixture {
    serde_json::from_str(include_str!(
        "../../../data/fixtures/differential/common-conditions.json"
    ))
    .expect("common-condition fixture should deserialize")
}

impl CombatData for TestData {
    fn move_data(&self, move_id: MoveId) -> Option<CombatMove> {
        (move_id == MOVE_ID).then_some(CombatMove {
            move_type: PokemonType::Normal,
            category: MoveCategory::Physical,
            base_power: 10,
            accuracy: MoveAccuracy::Always,
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
            interaction_flags: rotom_core::MoveInteractionFlags::default(),
        })
    }

    fn pokemon_types(&self, _: rotom_core::SpeciesId, _: Option<FormId>) -> Option<PokemonTypes> {
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

fn pokemon(species: u32, speed: u16) -> PokemonState {
    PokemonState {
        species: rotom_core::SpeciesId::new(species),
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
        ability: AbilityId::new(1),
        item: None,
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
        pokemon(1, 140),
        pokemon(2, 120),
        pokemon(3, 80),
        pokemon(4, 70),
    ];
    let p2 = vec![
        pokemon(101, 100),
        pokemon(102, 90),
        pokemon(103, 60),
        pokemon(104, 50),
    ];
    let mut builder = BattleStateBuilder::new_with_seed(
        SideState::new(TeamState::new(p1)),
        SideState::new(TeamState::new(p2)),
        4,
        seed,
    )
    .expect("fixture battle should be valid");
    let selected = vec![index(0), index(1), index(2), index(3)];
    builder
        .select_teams(selected.clone(), selected)
        .expect("selections should be valid");
    builder.build().expect("fixture should be stable")
}

fn move_at(target: ActionTarget) -> PokemonAction {
    PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target,
        mega: false,
    }
}

fn choices() -> (TurnChoice, TurnChoice) {
    (
        TurnChoice {
            actions: [
                move_at(ActionTarget::Foe(rotom_core::ActiveSlot::Left)),
                move_at(ActionTarget::Foe(rotom_core::ActiveSlot::Left)),
            ],
        },
        TurnChoice {
            actions: [
                move_at(ActionTarget::Foe(rotom_core::ActiveSlot::Right)),
                move_at(ActionTarget::Foe(rotom_core::ActiveSlot::Right)),
            ],
        },
    )
}

fn run_turn(battle: &BattleState, p1: TurnChoice, p2: TurnChoice) -> rotom_core::StepResult {
    step(
        battle,
        &JointAction::Turn { p1, p2 },
        &Mechanics::new(&TestData, &EffectRegistry::new()),
    )
    .expect("turn should resolve")
}

#[test]
fn stat_stage_changes_clamp_and_report_the_applied_delta() {
    let mut stages = StatStages::default();
    let raised = stages.change(BattleStat::Attack, 10);
    let lowered = stages.change(BattleStat::Attack, -20);

    assert_eq!(raised.current, 6);
    assert_eq!(raised.applied_delta, 6);
    assert_eq!(lowered.previous, 6);
    assert_eq!(lowered.current, -6);
    assert_eq!(lowered.applied_delta, -12);
    assert_eq!(stages.modified_stat(BattleStat::Attack, 100), 25);
    stages.change(BattleStat::Accuracy, 1);
    stages.change(BattleStat::Evasion, -1);
    assert_eq!(stages.modified_stat(BattleStat::Accuracy, 100), 133);
    assert_eq!(stages.modified_stat(BattleStat::Evasion, 100), 75);
}

#[test]
fn pinned_showdown_fixture_matches_common_condition_rules() {
    let fixture = reference_fixture();
    assert_eq!(
        fixture.showdown_commit,
        "b79ce06d06b8f0dc8de8bb348bc9cab5d931553f"
    );
    assert_eq!(fixture.format, "gen9championsvgc2026regmc");

    let mut paralyzed = pokemon(1, 101);
    paralyzed
        .set_major_status(MajorStatus::Paralysis, StatusState::default())
        .unwrap();
    assert_eq!(
        paralyzed.effective_speed(),
        fixture.paralysis.speed_from_101
    );
    assert_eq!(fixture.paralysis.before_move_chance, [1, 8]);

    let mut builder = BattleStateBuilder::from_state(battle(31)).unwrap();
    builder.enable_rng_trace();
    let target = PokemonRef::new(SideId::P1, index(0));
    builder
        .apply_major_status(target, MajorStatus::Sleep)
        .unwrap();
    let battle = builder.build().unwrap();
    let sample_index = usize::try_from(battle.rng().trace().unwrap()[0].result).unwrap();
    assert_eq!(
        battle
            .side(SideId::P1)
            .pokemon(index(0))
            .unwrap()
            .status_state
            .sleep_turns,
        fixture.sleep.durations[sample_index]
    );
    assert_eq!(fixture.sleep.initialized_turns, fixture.sleep.durations[0]);
    assert_eq!(fixture.freeze.initialized_turns, 3);
    assert_eq!(fixture.freeze.early_thaw_chance, [1, 4]);
    assert_eq!(fixture.freeze.automatic_thaw_attempt, 3);

    assert_eq!(fixture.residual.max_hp, 160);
    assert_eq!(fixture.residual.poison.order, 9);
    assert_eq!(fixture.residual.poison.damage, [20]);
    assert_eq!(fixture.residual.toxic.order, 9);
    assert_eq!(fixture.residual.toxic.damage, [10, 20]);
    assert_eq!(fixture.residual.burn.order, 10);
    assert_eq!(fixture.residual.burn.damage, [10]);
}

#[test]
fn status_application_initializes_champions_counters_and_is_exclusive() {
    let mut builder = BattleStateBuilder::from_state(battle(7)).unwrap();
    builder.enable_rng_trace();
    let target = PokemonRef::new(SideId::P1, index(0));

    builder
        .apply_major_status(target, MajorStatus::Sleep)
        .expect("sleep should apply");
    let state = builder.state().side(SideId::P1).pokemon(index(0)).unwrap();
    assert!(matches!(state.status_state.sleep_turns, 2 | 3));
    assert_eq!(state.status, Some(MajorStatus::Sleep));
    assert!(builder
        .apply_major_status(target, MajorStatus::Burn)
        .is_err());
    assert_eq!(
        builder.state().rng().draws(),
        1,
        "failed replacement must not draw RNG"
    );
    assert_eq!(
        builder.state().rng().trace().unwrap()[0].reason,
        RngReason::SleepDuration
    );

    assert_eq!(
        builder.cure_major_status(target).unwrap(),
        Some(MajorStatus::Sleep)
    );
    builder
        .apply_major_status(target, MajorStatus::Freeze)
        .expect("freeze should apply after curing sleep");
    let battle = builder.build().unwrap();
    assert_eq!(
        battle
            .side(SideId::P1)
            .pokemon(index(0))
            .unwrap()
            .status_state
            .freeze_turns,
        3
    );
}

#[test]
fn speed_stages_and_paralysis_feed_the_action_queue() {
    let mut builder = BattleStateBuilder::from_state(battle(3)).unwrap();
    builder
        .edit_pokemon(PokemonRef::new(SideId::P1, index(0)), |state| {
            state.stat_stages.change(BattleStat::Speed, 2);
        })
        .unwrap();
    builder
        .set_major_status(
            PokemonRef::new(SideId::P1, index(0)),
            MajorStatus::Paralysis,
            StatusState::default(),
        )
        .unwrap();
    let battle = builder.build().unwrap();
    let (p1, p2) = choices();
    let queue = battle
        .inspect_turn_queue(p1, p2, &Mechanics::new(&TestData, &EffectRegistry::new()))
        .expect("turn should queue");
    let action = queue
        .actions
        .iter()
        .find(|action| action.side == SideId::P1 && action.pokemon == index(0))
        .unwrap();

    assert_eq!(action.speed, 140);
    assert!(queue.events.iter().any(|event| {
        event.event == rotom_core::BattleEvent::ModifySpeed
            && event.context.source == Some(PokemonRef::new(SideId::P1, index(0)))
            && !event.invocations.is_empty()
    }));
}

#[test]
fn sleep_wake_and_freeze_cap_run_before_pp_consumption() {
    let mut builder = BattleStateBuilder::from_state(battle(11)).unwrap();
    builder
        .set_major_status(
            PokemonRef::new(SideId::P1, index(0)),
            MajorStatus::Sleep,
            StatusState {
                sleep_turns: 2,
                ..StatusState::default()
            },
        )
        .unwrap();
    builder
        .set_major_status(
            PokemonRef::new(SideId::P1, index(1)),
            MajorStatus::Freeze,
            StatusState {
                freeze_turns: 1,
                ..StatusState::default()
            },
        )
        .unwrap();
    let battle = builder.build().unwrap();
    let (p1, p2) = choices();
    let battle = run_turn(&battle, p1, p2).state;

    let sleeping = battle.side(SideId::P1).pokemon(index(0)).unwrap();
    let thawed = battle.side(SideId::P1).pokemon(index(1)).unwrap();
    assert_eq!(sleeping.status, Some(MajorStatus::Sleep));
    assert_eq!(sleeping.status_state.sleep_turns, 1);
    assert_eq!(sleeping.moves[0].pp, 16);
    assert_eq!(thawed.status, None);
    assert_eq!(thawed.moves[0].pp, 15);
}

#[test]
fn champions_paralysis_uses_one_in_eight_rng() {
    let mut builder = BattleStateBuilder::from_state(battle(19)).unwrap();
    builder.enable_rng_trace();
    builder
        .set_major_status(
            PokemonRef::new(SideId::P1, index(0)),
            MajorStatus::Paralysis,
            StatusState::default(),
        )
        .unwrap();
    let battle = builder.build().unwrap();
    let (p1, p2) = choices();
    let battle = run_turn(&battle, p1, p2).state;

    let paralysis = battle
        .rng()
        .trace()
        .unwrap()
        .iter()
        .find(|entry| entry.reason == RngReason::Paralysis)
        .expect("paralysis should consume the battle RNG");
    assert_eq!(
        paralysis.operation,
        RngOperation::Chance {
            numerator: 1,
            denominator: 8
        }
    );
}

#[test]
fn burn_halves_physical_damage_after_the_normal_formula() {
    let source = pokemon(1, 100);
    let target = pokemon(2, 100);
    let move_data = TestData.move_data(MOVE_ID).unwrap();
    let types = TestData.pokemon_types(source.species, None).unwrap();
    let normal = calculate_damage(&source, &target, move_data, types, Some(0), false, 100).unwrap();
    let mut burned = source;
    burned
        .set_major_status(MajorStatus::Burn, StatusState::default())
        .unwrap();
    let burned = calculate_damage(&burned, &target, move_data, types, Some(0), false, 100).unwrap();

    assert_eq!(burned.final_damage, normal.final_damage / 2);
    assert_eq!(
        normal.random_modifier,
        Modifier {
            numerator: 100,
            denominator: 100
        }
    );
}

#[test]
fn residual_statuses_and_volatile_durations_are_ordered_and_reported() {
    let mut builder = BattleStateBuilder::from_state(battle(23)).unwrap();
    for (side, pokemon, status) in [
        (SideId::P1, index(0), MajorStatus::Poison),
        (SideId::P1, index(1), MajorStatus::Burn),
        (SideId::P2, index(0), MajorStatus::BadlyPoisoned),
    ] {
        builder
            .set_major_status(
                PokemonRef::new(side, pokemon),
                status,
                StatusState::default(),
            )
            .unwrap();
    }
    builder
        .edit_pokemon(PokemonRef::new(SideId::P1, index(0)), |pokemon| {
            pokemon
                .add_volatile(ConditionState::new(ConditionId::new(9), Some(1)))
                .unwrap();
        })
        .unwrap();
    builder
        .edit_pokemon(PokemonRef::new(SideId::P1, index(1)), |pokemon| {
            pokemon
                .add_volatile(ConditionState::new(ConditionId::new(10), Some(2)))
                .unwrap();
        })
        .unwrap();
    let battle = builder.build().unwrap();
    let (p1, p2) = choices();
    let result = run_turn(&battle, p1, p2);
    let resolution = result.diagnostics.turn.as_ref().unwrap();
    let battle = result.state;

    assert_eq!(
        resolution
            .residual_damage
            .iter()
            .map(|entry| (entry.status, entry.damage))
            .collect::<Vec<_>>(),
        vec![
            (MajorStatus::Poison, 25),
            (MajorStatus::BadlyPoisoned, 12),
            (MajorStatus::Burn, 12),
        ]
    );
    assert_eq!(resolution.expired_volatiles.len(), 1);
    assert_eq!(
        resolution.expired_volatiles[0].condition,
        ConditionId::new(9)
    );
    assert_eq!(
        battle.side(SideId::P1).pokemon(index(1)).unwrap().volatiles[0].turns_remaining,
        Some(1)
    );
    assert_eq!(
        battle
            .side(SideId::P2)
            .pokemon(index(0))
            .unwrap()
            .status_state
            .toxic_counter,
        1
    );
}

#[test]
fn residual_knockout_enters_forced_switch_and_clears_status() {
    let mut builder = BattleStateBuilder::from_state(battle(29)).unwrap();
    builder
        .edit_pokemon(PokemonRef::new(SideId::P1, index(0)), |pokemon| {
            pokemon.current_hp = 1;
        })
        .unwrap();
    builder
        .set_major_status(
            PokemonRef::new(SideId::P1, index(0)),
            MajorStatus::Poison,
            StatusState::default(),
        )
        .unwrap();
    let battle = builder.build().unwrap();
    let (p1, p2) = choices();
    let result = run_turn(&battle, p1, p2);
    let resolution = result.diagnostics.turn.as_ref().unwrap();
    let battle = result.state;

    assert_eq!(resolution.residual_damage[0].damage, 1);
    assert_eq!(battle.phase(), BattlePhase::ForcedSwitch);
    assert_eq!(battle.side(SideId::P1).active[0], None);
    let fainted = battle.side(SideId::P1).pokemon(index(0)).unwrap();
    assert!(fainted.is_fainted());
    assert_eq!(fainted.status, None);
}

#[test]
fn toxic_counter_is_preserved_on_exit_and_reset_on_reentry() {
    let mut builder = BattleStateBuilder::from_state(battle(37)).unwrap();
    builder
        .set_major_status(
            PokemonRef::new(SideId::P1, index(0)),
            MajorStatus::BadlyPoisoned,
            StatusState {
                toxic_counter: 3,
                ..StatusState::default()
            },
        )
        .unwrap();
    let battle = builder.build().unwrap();
    let p1_switch_out = TurnChoice {
        actions: [
            PokemonAction::Switch { pokemon: index(2) },
            move_at(ActionTarget::Foe(rotom_core::ActiveSlot::Left)),
        ],
    };
    let (_, p2) = choices();
    let battle = run_turn(&battle, p1_switch_out, p2).state;
    assert_eq!(
        battle
            .side(SideId::P1)
            .pokemon(index(0))
            .unwrap()
            .status_state
            .toxic_counter,
        3
    );

    let p1_switch_in = TurnChoice {
        actions: [
            PokemonAction::Switch { pokemon: index(0) },
            move_at(ActionTarget::Foe(rotom_core::ActiveSlot::Left)),
        ],
    };
    let (_, p2) = choices();
    let battle = run_turn(&battle, p1_switch_in, p2).state;
    assert_eq!(
        battle
            .side(SideId::P1)
            .pokemon(index(0))
            .unwrap()
            .status_state
            .toxic_counter,
        1,
        "switch-in resets toxic before that turn's residual increment"
    );
}
