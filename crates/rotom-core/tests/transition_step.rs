use proptest::prelude::*;
use rotom_core::{
    step, AbilityId, ActionTarget, ActiveSlot, BattleError, BattlePhase, BattleState, BattleStats,
    CombatData, CombatMove, EffectRegistry, FormId, JointAction, Mechanics, MoveAccuracy,
    MoveCapabilities, MoveCapability, MoveCategory, MoveId, MoveOperation, MoveSlot, MoveSlotIndex,
    MoveTarget, PokemonAction, PokemonIndex, PokemonPosition, PokemonState, PokemonType,
    PokemonTypes, SideId, SideState, SpeciesId, StatStages, StatusState, StepDiagnostics,
    TeamSelection, TeamState, TurnChoice,
};

const MOVE_ID: MoveId = MoveId::new(1);

#[derive(Clone, Copy)]
struct TestData {
    base_power: u16,
    required_capabilities: MoveCapabilities,
    target: MoveTarget,
}

impl TestData {
    const fn basic(base_power: u16) -> Self {
        Self {
            base_power,
            required_capabilities: MoveCapabilities::PR_13_1,
            target: MoveTarget::Normal,
        }
    }
}

impl CombatData for TestData {
    fn move_data(&self, move_id: MoveId) -> Option<CombatMove> {
        (move_id == MOVE_ID).then_some(CombatMove {
            move_type: PokemonType::Normal,
            category: MoveCategory::Physical,
            base_power: self.base_power,
            accuracy: MoveAccuracy::Always,
            crit_ratio: 0,
            target: self.target,
            operation: MoveOperation::StandardDamage,
            required_capabilities: self.required_capabilities,
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

fn pokemon(species: u32, speed: u16) -> PokemonState {
    PokemonState {
        species: SpeciesId::new(species),
        form: None,
        level: 50,
        stats: BattleStats {
            hp: 120,
            atk: 100,
            def: 100,
            spa: 100,
            spd: 100,
            spe: speed,
        },
        current_hp: 120,
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

fn preview(seed: u64, picked_team_size: usize) -> BattleState {
    let p1 = [(1, 140), (2, 130), (3, 70), (4, 60)]
        .into_iter()
        .map(|(species, speed)| pokemon(species, speed))
        .collect();
    let p2 = [(101, 100), (102, 90), (103, 50), (104, 40)]
        .into_iter()
        .map(|(species, speed)| pokemon(species, speed))
        .collect();
    BattleState::new_with_seed(
        SideState::new(TeamState::new(p1)),
        SideState::new(TeamState::new(p2)),
        picked_team_size,
        seed,
    )
    .expect("transition fixture should be valid")
}

fn selection(size: u8) -> TeamSelection {
    TeamSelection::new((0..size).map(index).collect())
}

fn preview_action(size: u8) -> JointAction {
    JointAction::TeamPreview {
        p1: selection(size),
        p2: selection(size),
    }
}

fn move_at(target: ActiveSlot) -> PokemonAction {
    PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Foe(target),
        mega: false,
    }
}

fn attacking_turn() -> JointAction {
    JointAction::Turn {
        p1: TurnChoice {
            actions: [move_at(ActiveSlot::Left), move_at(ActiveSlot::Right)],
        },
        p2: TurnChoice {
            actions: [move_at(ActiveSlot::Left), move_at(ActiveSlot::Right)],
        },
    }
}

fn started(seed: u64) -> BattleState {
    let effects = EffectRegistry::new();
    step(
        &preview(seed, 4),
        &preview_action(4),
        &Mechanics::new(&TestData::basic(20), &effects),
    )
    .expect("preview should advance")
    .state
}

fn assert_roundtrip_continuation(
    state: &BattleState,
    action: &JointAction,
    mechanics: &Mechanics<'_, TestData>,
    label: &str,
) {
    state
        .validate_stable_state()
        .unwrap_or_else(|error| panic!("{label} input is unstable: {error}"));
    let serialized = serde_json::to_string(state).unwrap();
    let restored: BattleState = serde_json::from_str(&serialized).unwrap();
    restored
        .validate_stable_state()
        .unwrap_or_else(|error| panic!("{label} restored state is unstable: {error}"));
    assert_eq!(&restored, state, "{label} authoritative state round trip");
    match (
        step(state, action, mechanics),
        step(&restored, action, mechanics),
    ) {
        (Ok(original), Ok(resumed)) => assert_eq!(original, resumed, "{label} continuation"),
        (Err(original), Err(resumed)) => {
            assert_eq!(original.to_string(), resumed.to_string(), "{label} error");
        }
        (original, resumed) => panic!("{label} continuation mismatch: {original:?} vs {resumed:?}"),
    }
}

#[test]
fn step_advances_preview_turn_and_forced_switch_phases() {
    let effects = EffectRegistry::new();
    let data = TestData::basic(1_000);
    let mechanics = Mechanics::new(&data, &effects);
    let initial = preview(0x5eed, 4);

    let preview_result = step(&initial, &preview_action(4), &mechanics).unwrap();
    assert_eq!(initial.phase(), BattlePhase::TeamPreview);
    assert_eq!(preview_result.state.phase(), BattlePhase::TurnSelection);
    assert_eq!(preview_result.state.turn(), 1);
    assert_eq!(preview_result.diagnostics, StepDiagnostics::default());

    let turn_result = step(&preview_result.state, &attacking_turn(), &mechanics).unwrap();
    assert_eq!(turn_result.state.phase(), BattlePhase::ForcedSwitch);
    assert_eq!(turn_result.state.side(SideId::P2).active, [None, None]);
    assert!(turn_result.diagnostics.turn.is_some());

    let p1_pass = TurnChoice {
        actions: [PokemonAction::Pass, PokemonAction::Pass],
    };
    assert!(turn_result
        .state
        .legal_actions(SideId::P1)
        .unwrap()
        .choices
        .contains(&p1_pass));
    let replacements = JointAction::ForcedSwitch {
        p1: p1_pass,
        p2: TurnChoice {
            actions: [
                PokemonAction::Switch { pokemon: index(2) },
                PokemonAction::Switch { pokemon: index(3) },
            ],
        },
    };
    let replaced = step(&turn_result.state, &replacements, &mechanics).unwrap();
    assert_eq!(replaced.state.phase(), BattlePhase::TurnSelection);
    assert_eq!(
        replaced.state.side(SideId::P2).active,
        [Some(index(2)), Some(index(3))]
    );
    replaced.state.validate_stable_state().unwrap();
}

#[test]
fn queue_inspection_matches_canonical_transition_diagnostics() {
    let effects = EffectRegistry::new();
    let data = TestData::basic(20);
    let mechanics = Mechanics::new(&data, &effects);
    let current = started(0xcafe);
    let JointAction::Turn { p1, p2 } = attacking_turn() else {
        unreachable!();
    };

    let inspected = current.inspect_turn_queue(p1, p2, &mechanics).unwrap();

    let canonical = step(&current, &JointAction::Turn { p1, p2 }, &mechanics).unwrap();
    assert_eq!(canonical.diagnostics.queue_events, inspected.events);
    assert_eq!(current.phase(), BattlePhase::TurnSelection);
    assert_eq!(current.queue().actions, Vec::new());
}

#[test]
fn replay_and_serde_resume_are_deterministic_and_leave_input_unchanged() {
    let effects = EffectRegistry::new();
    let data = TestData::basic(20);
    let mechanics = Mechanics::new(&data, &effects);
    let current = started(0x1234);
    let before = current.clone();
    let restored: BattleState = serde_json::from_str(&serde_json::to_string(&current).unwrap())
        .expect("serialized battle should deserialize");

    let first = step(&current, &attacking_turn(), &mechanics).unwrap();
    let replay = step(&current, &attacking_turn(), &mechanics).unwrap();
    let resumed = step(&restored, &attacking_turn(), &mechanics).unwrap();

    assert_eq!(current, before);
    assert_eq!(first, replay);
    assert_eq!(first, resumed);
}

#[test]
fn authoritative_state_round_trips_and_resumes_across_every_stable_phase() {
    let effects = EffectRegistry::new();
    let low_damage = TestData::basic(20);
    let low_mechanics = Mechanics::new(&low_damage, &effects);
    let initial = preview(0x1322, 4);
    assert_roundtrip_continuation(&initial, &preview_action(4), &low_mechanics, "team preview");

    let selected = step(&initial, &preview_action(4), &low_mechanics)
        .unwrap()
        .state;
    assert_roundtrip_continuation(
        &selected,
        &attacking_turn(),
        &low_mechanics,
        "initial actives",
    );

    let ordinary = step(&selected, &attacking_turn(), &low_mechanics)
        .unwrap()
        .state;
    assert_roundtrip_continuation(
        &ordinary,
        &attacking_turn(),
        &low_mechanics,
        "ordinary turn selection",
    );

    let mut switch_builder = rotom_core::BattleStateBuilder::from_state(selected.clone()).unwrap();
    switch_builder
        .switch_active(SideId::P1, ActiveSlot::Left, index(2))
        .unwrap();
    let switched = switch_builder.build().unwrap();
    assert_roundtrip_continuation(&switched, &attacking_turn(), &low_mechanics, "after switch");

    let high_damage = TestData::basic(1_000);
    let high_mechanics = Mechanics::new(&high_damage, &effects);
    let fainted = step(&selected, &attacking_turn(), &high_mechanics)
        .unwrap()
        .state;
    let replacements = JointAction::ForcedSwitch {
        p1: TurnChoice {
            actions: [PokemonAction::Pass, PokemonAction::Pass],
        },
        p2: TurnChoice {
            actions: [
                PokemonAction::Switch { pokemon: index(2) },
                PokemonAction::Switch { pokemon: index(3) },
            ],
        },
    };
    assert_roundtrip_continuation(&fainted, &replacements, &high_mechanics, "after faint");

    let filled = step(&fainted, &replacements, &high_mechanics)
        .unwrap()
        .state;
    assert_roundtrip_continuation(
        &filled,
        &attacking_turn(),
        &high_mechanics,
        "after filling empty slots",
    );

    let terminal = step(&filled, &attacking_turn(), &high_mechanics)
        .unwrap()
        .state;
    assert_eq!(terminal.phase(), BattlePhase::BattleEnd);
    assert_roundtrip_continuation(
        &terminal,
        &attacking_turn(),
        &high_mechanics,
        "completed battle",
    );
}

#[test]
fn serialized_effect_orders_reject_corruption_and_overflow_atomically() {
    let effects = EffectRegistry::new();
    let data = TestData::basic(20);
    let mechanics = Mechanics::new(&data, &effects);
    let selected = started(0xe0ff);

    for field in ["active_effect_orders", "next_effect_order"] {
        let mut historical = serde_json::to_value(&selected).unwrap();
        historical.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<BattleState>(historical).is_err(),
            "snapshots missing authoritative {field} must be rejected"
        );
    }

    let mut corruptions = Vec::new();
    let mut missing = serde_json::to_value(&selected).unwrap();
    missing["active_effect_orders"][0][0] = serde_json::Value::Null;
    corruptions.push(("missing active order", missing));

    let mut duplicate = serde_json::to_value(&selected).unwrap();
    duplicate["active_effect_orders"][0][1] = duplicate["active_effect_orders"][0][0].clone();
    corruptions.push(("duplicate active order", duplicate));

    let mut future = serde_json::to_value(&selected).unwrap();
    future["active_effect_orders"][0][0] = future["next_effect_order"].clone();
    corruptions.push(("order at next counter", future));

    let high_damage = TestData::basic(1_000);
    let fainted = step(
        &selected,
        &attacking_turn(),
        &Mechanics::new(&high_damage, &effects),
    )
    .unwrap()
    .state;
    let mut stale_empty = serde_json::to_value(&fainted).unwrap();
    stale_empty["active_effect_orders"][1][0] = serde_json::json!(0);
    corruptions.push(("empty slot retained order", stale_empty));

    for (label, value) in corruptions {
        let corrupted: BattleState = serde_json::from_value(value).unwrap();
        let before = corrupted.clone();
        assert!(matches!(
            corrupted.validate_stable_state(),
            Err(BattleError::InvalidState(_))
        ));
        assert!(matches!(
            step(&corrupted, &attacking_turn(), &mechanics),
            Err(BattleError::InvalidState(_))
        ));
        assert_eq!(corrupted, before, "{label} rejection mutated its input");
    }

    let mut overflow_value = serde_json::to_value(&selected).unwrap();
    overflow_value["next_effect_order"] = serde_json::json!(u32::MAX);
    let overflow: BattleState = serde_json::from_value(overflow_value).unwrap();
    overflow.validate_stable_state().unwrap();
    let before = overflow.clone();
    let mut builder = rotom_core::BattleStateBuilder::from_state(overflow.clone()).unwrap();
    let error = builder
        .switch_active(SideId::P1, ActiveSlot::Left, index(2))
        .err()
        .expect("overflow switch should fail");
    assert!(error
        .to_string()
        .contains("effect-order counter overflowed"));
    assert_eq!(builder.state(), &before, "kernel overflow must be atomic");

    let switch = JointAction::Turn {
        p1: TurnChoice {
            actions: [
                PokemonAction::Switch { pokemon: index(2) },
                move_at(ActiveSlot::Right),
            ],
        },
        p2: TurnChoice {
            actions: [move_at(ActiveSlot::Left), move_at(ActiveSlot::Right)],
        },
    };
    let error = step(&overflow, &switch, &mechanics).unwrap_err();
    assert!(error
        .to_string()
        .contains("effect-order counter overflowed"));
    assert_eq!(overflow, before, "public overflow error mutated its input");
}

#[test]
fn transition_errors_are_atomic_and_terminal_battles_reject_actions() {
    let effects = EffectRegistry::new();
    let unsupported = TestData {
        base_power: 20,
        required_capabilities: MoveCapabilities::PR_13_1.with(MoveCapability::SecondaryEffect),
        target: MoveTarget::Normal,
    };
    let current = started(0xdead);
    let before = current.clone();
    let error = step(
        &current,
        &attacking_turn(),
        &Mechanics::new(&unsupported, &effects),
    )
    .unwrap_err();
    assert!(matches!(error, BattleError::UnsupportedMechanic(_)));
    assert_eq!(current, before);

    let data = TestData::basic(1_000);
    let mechanics = Mechanics::new(&data, &effects);
    let selected = step(&preview(7, 2), &preview_action(2), &mechanics)
        .unwrap()
        .state;
    let terminal = step(&selected, &attacking_turn(), &mechanics)
        .unwrap()
        .state;
    assert_eq!(terminal.phase(), BattlePhase::BattleEnd);
    let terminal_before = terminal.clone();
    assert!(matches!(
        step(&terminal, &attacking_turn(), &mechanics),
        Err(BattleError::IllegalChoice(_))
    ));
    assert_eq!(terminal, terminal_before);
}

#[test]
fn normal_moves_accept_the_other_ally_as_a_selected_target() {
    let effects = EffectRegistry::new();
    let data = TestData::basic(20);
    let current = started(0x4a11);
    let ally_action = PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Ally(ActiveSlot::Right),
        mega: false,
    };
    let action = JointAction::Turn {
        p1: TurnChoice {
            actions: [ally_action, move_at(ActiveSlot::Left)],
        },
        p2: TurnChoice {
            actions: [move_at(ActiveSlot::Left), move_at(ActiveSlot::Right)],
        },
    };

    let result = step(&current, &action, &Mechanics::new(&data, &effects)).unwrap();
    let ally = result
        .state
        .side(SideId::P1)
        .pokemon(index(1))
        .expect("right ally");
    assert!(ally.current_hp < ally.stats.hp);
    assert_eq!(
        result
            .state
            .side(SideId::P1)
            .pokemon(index(0))
            .expect("source")
            .moves[0]
            .pp,
        15
    );
}

#[test]
fn authoritative_target_mismatch_is_rejected_atomically() {
    let effects = EffectRegistry::new();
    let data = TestData {
        target: MoveTarget::AdjacentFoe,
        ..TestData::basic(20)
    };
    let current = started(0x7a26);
    let before = current.clone();
    let action = JointAction::Turn {
        p1: TurnChoice {
            actions: [
                PokemonAction::Move {
                    move_slot: MoveSlotIndex::new(0),
                    target: ActionTarget::Ally(ActiveSlot::Right),
                    mega: false,
                },
                move_at(ActiveSlot::Left),
            ],
        },
        p2: TurnChoice {
            actions: [move_at(ActiveSlot::Left), move_at(ActiveSlot::Right)],
        },
    };

    let error = step(&current, &action, &Mechanics::new(&data, &effects)).unwrap_err();
    assert!(matches!(error, BattleError::IllegalChoice(_)));
    assert!(error.to_string().contains("incompatible"));
    assert_eq!(
        current, before,
        "state, PP, HP, and RNG must remain unchanged"
    );
}

#[test]
fn removed_selected_foe_retargets_to_the_remaining_foe_reproducibly() {
    let effects = EffectRegistry::new();
    let current = started(0xfeed);
    let before = current.clone();
    let action = JointAction::Turn {
        p1: TurnChoice {
            actions: [move_at(ActiveSlot::Left), move_at(ActiveSlot::Left)],
        },
        p2: TurnChoice {
            actions: [move_at(ActiveSlot::Left), move_at(ActiveSlot::Right)],
        },
    };
    for target in [MoveTarget::Normal, MoveTarget::Any, MoveTarget::AdjacentFoe] {
        let data = TestData {
            target,
            ..TestData::basic(1_000)
        };
        let mechanics = Mechanics::new(&data, &effects);
        let first = step(&current, &action, &mechanics).unwrap();
        let replay = step(&current, &action, &mechanics).unwrap();
        assert_eq!(first, replay, "{target:?}");
        let outcomes = &first.diagnostics.turn.as_ref().unwrap().target_outcomes;
        assert_eq!(outcomes.len(), 2);
        assert_eq!(outcomes[1].target.side, SideId::P2);
        assert_eq!(outcomes[1].target.pokemon, PokemonIndex::new(1));
        assert!(outcomes
            .iter()
            .all(|outcome| outcome.result == rotom_core::TargetResult::Hit && outcome.fainted));
        assert_eq!(
            current, before,
            "{target:?} successful resolution must leave the input branch unchanged"
        );
    }
}

#[test]
fn rng_tracing_does_not_change_transition_outcomes_or_draws() {
    let effects = EffectRegistry::new();
    let data = TestData::basic(20);
    let mechanics = Mechanics::new(&data, &effects);
    let plain = started(0xabcd);
    let mut traced = plain.clone();
    traced.enable_rng_trace();

    let plain_result = step(&plain, &attacking_turn(), &mechanics).unwrap();
    let traced_result = step(&traced, &attacking_turn(), &mechanics).unwrap();
    assert_eq!(
        plain_result.state.rng().state(),
        traced_result.state.rng().state()
    );
    assert_eq!(
        plain_result.state.rng().draws(),
        traced_result.state.rng().draws()
    );
    assert_eq!(plain_result.diagnostics, traced_result.diagnostics);

    let mut traced_without_metadata = traced_result.state;
    traced_without_metadata.disable_rng_trace();
    assert_eq!(plain_result.state, traced_without_metadata);
}

#[test]
fn phase_mismatch_is_rejected_before_transition_work() {
    let effects = EffectRegistry::new();
    let data = TestData::basic(20);
    let initial = preview(1, 4);
    let error = step(
        &initial,
        &attacking_turn(),
        &Mechanics::new(&data, &effects),
    )
    .unwrap_err();

    assert!(matches!(error, BattleError::IllegalChoice(_)));
    assert_eq!(initial.phase(), BattlePhase::TeamPreview);
    assert_eq!(initial.rng().draws(), 0);
}

#[test]
fn differently_seeded_branches_are_independent_and_reproducible() {
    let effects = EffectRegistry::new();
    let data = TestData::basic(20);
    let mechanics = Mechanics::new(&data, &effects);
    let left = started(0x1111);
    let right = started(0x2222);

    let left_first = step(&left, &attacking_turn(), &mechanics).unwrap();
    let left_replay = step(&left, &attacking_turn(), &mechanics).unwrap();
    let right_first = step(&right, &attacking_turn(), &mechanics).unwrap();
    let right_replay = step(&right, &attacking_turn(), &mechanics).unwrap();

    assert_eq!(left_first, left_replay);
    assert_eq!(right_first, right_replay);
    assert_ne!(
        left_first.state.rng().state(),
        right_first.state.rng().state()
    );
    assert_eq!(left.rng().draws(), 0);
    assert_eq!(right.rng().draws(), 0);
}

#[test]
fn supported_battle_runs_from_preview_through_replacements_to_terminal() {
    let effects = EffectRegistry::new();
    let data = TestData::basic(1_000);
    let mechanics = Mechanics::new(&data, &effects);
    let preview = preview(0xf011, 4);

    let selected = step(&preview, &preview_action(4), &mechanics).unwrap();
    selected.state.validate_stable_state().unwrap();
    assert_eq!(selected.state.phase(), BattlePhase::TurnSelection);

    let first_turn = step(&selected.state, &attacking_turn(), &mechanics).unwrap();
    first_turn.state.validate_stable_state().unwrap();
    assert_eq!(first_turn.state.phase(), BattlePhase::ForcedSwitch);

    let replacements = JointAction::ForcedSwitch {
        p1: TurnChoice {
            actions: [PokemonAction::Pass, PokemonAction::Pass],
        },
        p2: TurnChoice {
            actions: [
                PokemonAction::Switch { pokemon: index(2) },
                PokemonAction::Switch { pokemon: index(3) },
            ],
        },
    };
    let replaced = step(&first_turn.state, &replacements, &mechanics).unwrap();
    replaced.state.validate_stable_state().unwrap();
    assert_eq!(replaced.state.phase(), BattlePhase::TurnSelection);

    let terminal = step(&replaced.state, &attacking_turn(), &mechanics).unwrap();
    terminal.state.validate_stable_state().unwrap();
    assert_eq!(terminal.state.phase(), BattlePhase::BattleEnd);
    assert_eq!(terminal.state.winner(), Some(SideId::P1));
    assert_eq!(terminal.state.turn(), 2);
    assert_eq!(terminal.state.queue().actions, Vec::new());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn successful_public_turns_are_pure_deterministic_and_stable(
        seed in any::<u64>(),
        base_power in 1_u16..=200,
    ) {
        let effects = EffectRegistry::new();
        let data = TestData::basic(base_power);
        let mechanics = Mechanics::new(&data, &effects);
        let current = started(seed);
        let before = current.clone();

        let first = step(&current, &attacking_turn(), &mechanics).unwrap();
        let replay = step(&current, &attacking_turn(), &mechanics).unwrap();

        prop_assert_eq!(&current, &before);
        prop_assert_eq!(&first, &replay);
        prop_assert!(first.state.validate_stable_state().is_ok());
        prop_assert_ne!(first.state.phase(), BattlePhase::ResolvingTurn);
        prop_assert_eq!(first.state.queue().actions.len(), 0);
        prop_assert_eq!(first.state.queue().events.len(), 0);
    }
}
