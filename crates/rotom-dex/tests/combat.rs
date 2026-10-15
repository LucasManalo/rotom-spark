use rotom_core::{
    calculate_move_damage, step, ActionTarget, BattlePhase, BattleState, BattleStateBuilder,
    CombatData, EffectRegistry, FormId, Gender, JointAction, Mechanics, MoveCapabilities,
    MoveCapability, MoveOperation, MoveSlotIndex, PokemonAction, PokemonIndex, PokemonPosition,
    PokemonSet, RngOperation, RngReason, SideId, SideState, StatAllocation, StatStages, TeamState,
    TurnChoice,
};
use rotom_dex::champions_reg_mc;
use serde::Deserialize;

const SHOWDOWN_COMMIT: &str = "b79ce06d06b8f0dc8de8bb348bc9cab5d931553f";

#[derive(Debug, Deserialize)]
struct DamageFixture {
    showdown_commit: String,
    format: String,
    random: DamageRollRange,
    cases: Vec<DamageCase>,
}

#[derive(Debug, Deserialize)]
struct DamageRollRange {
    damage_percent_min: u8,
    damage_percent_max: u8,
}

#[derive(Debug, Deserialize)]
struct DamageCase {
    name: String,
    #[serde(rename = "move")]
    move_name: String,
    critical: bool,
    source: FixturePokemon,
    target: FixturePokemon,
    rolls: Vec<DamageRoll>,
}

#[derive(Debug, Deserialize)]
struct FixturePokemon {
    species: String,
    ability: String,
    nature: String,
    stat_points: StatAllocation,
    stages: FixtureStages,
    level: u8,
    stats: rotom_core::BattleStats,
    types: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct FixtureStages {
    hp: i8,
    atk: i8,
    def: i8,
    spa: i8,
    spd: i8,
    spe: i8,
    accuracy: i8,
    evasion: i8,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct DamageRoll {
    percent: u8,
    damage: u16,
}

#[derive(Debug, Deserialize)]
struct CanonicalStepFixture {
    showdown_commit: String,
    format: String,
    methodology: StepMethodology,
    cases: Vec<CanonicalStepCase>,
}

#[derive(Debug, Deserialize)]
struct StepMethodology {
    damage_source: String,
    integration_scope: String,
    full_transition_comparison: String,
    rng_comparison: String,
    move_draw_order: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CanonicalStepCase {
    name: String,
    #[serde(rename = "move")]
    move_name: String,
    damage_case: String,
    rotom_seed: u64,
    controlled_outcomes: ControlledOutcomes,
    expected: StepExpectation,
}

#[derive(Debug, Deserialize)]
struct ControlledOutcomes {
    accuracy: String,
    critical_hit: bool,
    damage_percent: u8,
}

#[derive(Debug, Deserialize)]
struct StepExpectation {
    damage: u16,
    applied_hp_loss: u16,
    target_hp: u16,
    pp_consumed: u8,
    fainted: bool,
    replacement_scheduled: bool,
}

impl From<FixtureStages> for StatStages {
    fn from(value: FixtureStages) -> Self {
        assert_eq!(value.hp, 0, "HP cannot have a battle stat stage");
        Self {
            atk: value.atk,
            def: value.def,
            spa: value.spa,
            spd: value.spd,
            spe: value.spe,
            accuracy: value.accuracy,
            evasion: value.evasion,
        }
    }
}

fn set(
    dex: &rotom_dex::DexData,
    species: &str,
    ability: &str,
    nature: &str,
    move_name: &str,
    points: StatAllocation,
) -> PokemonSet {
    PokemonSet {
        nickname: None,
        species: dex
            .resolve_species(species)
            .map(|(id, _)| id)
            .expect("fixture species"),
        form: None::<FormId>,
        level: 50,
        ability: dex
            .resolve_ability(ability)
            .map(|(id, _)| id)
            .expect("fixture ability"),
        item: None,
        nature: dex
            .resolve_nature(nature)
            .map(|(id, _)| id)
            .expect("fixture nature"),
        stat_points: points,
        ivs: StatAllocation::PERFECT_IVS,
        moves: vec![dex
            .resolve_move(move_name)
            .map(|(id, _)| id)
            .expect("fixture move")],
        gender: None::<Gender>,
        shiny: false,
    }
}

#[test]
fn champions_stats_and_damage_match_pinned_showdown_fixtures() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture: DamageFixture = serde_json::from_str(include_str!(
        "../../../data/fixtures/differential/basic-damage.json"
    ))
    .expect("Showdown damage fixture should deserialize");
    assert_eq!(fixture.showdown_commit, SHOWDOWN_COMMIT);
    assert_eq!(fixture.format, "gen9championsvgc2026regmc");
    assert_eq!(fixture.random.damage_percent_min, 85);
    assert_eq!(fixture.random.damage_percent_max, 100);
    assert!(fixture.cases.len() >= 12);
    assert!(fixture.cases.iter().any(|case| case.critical));
    assert!(fixture
        .cases
        .iter()
        .any(|case| case.source.stages.spa != 0 || case.target.stages.spd != 0));
    assert!(fixture
        .cases
        .iter()
        .any(|case| case.move_name == "dragonclaw"));

    for case in fixture.cases {
        let source_set = fixture_set(&dex, &case.source, &case.move_name);
        let target_set = fixture_set(&dex, &case.target, "splash");
        let mut source = dex.pokemon_state(&source_set).expect("source state");
        let mut target = dex.pokemon_state(&target_set).expect("target state");
        source.stat_stages = case.source.stages.into();
        target.stat_stages = case.target.stages.into();
        let move_id = source_set.moves[0];
        assert_eq!(
            source.level, case.source.level,
            "{} source level",
            case.name
        );
        assert_eq!(
            target.level, case.target.level,
            "{} target level",
            case.name
        );
        assert_eq!(
            source.stats, case.source.stats,
            "{} source stats",
            case.name
        );
        assert_eq!(
            target.stats, case.target.stats,
            "{} target stats",
            case.name
        );
        assert!(!case.source.types.is_empty(), "{} source types", case.name);
        assert!(!case.target.types.is_empty(), "{} target types", case.name);
        assert_eq!(case.rolls.len(), 16, "{} roll count", case.name);
        for roll in case.rolls {
            let actual =
                calculate_move_damage(&dex, &source, &target, move_id, case.critical, roll.percent)
                    .unwrap_or_else(|error| panic!("{} at {}%: {error}", case.name, roll.percent));
            assert_eq!(
                actual.final_damage, roll.damage,
                "{} at {}%",
                case.name, roll.percent
            );
        }
    }
}

fn fixture_set(dex: &rotom_dex::DexData, pokemon: &FixturePokemon, move_name: &str) -> PokemonSet {
    let mut result = set(
        dex,
        &pokemon.species,
        &pokemon.ability,
        &pokemon.nature,
        move_name,
        pokemon.stat_points,
    );
    result.level = pokemon.level;
    result
}

fn repeated_side(pokemon: &rotom_core::PokemonState) -> SideState {
    SideState::new(TeamState::new(vec![
        pokemon.clone(),
        pokemon.clone(),
        pokemon.clone(),
        pokemon.clone(),
    ]))
}

fn four_selection() -> Vec<PokemonIndex> {
    (0..4).map(PokemonIndex::new).collect()
}

fn fixture_battle(
    dex: &rotom_dex::DexData,
    case: &DamageCase,
    seed: u64,
    trace_rng: bool,
) -> (
    rotom_core::BattleState,
    rotom_core::PokemonState,
    rotom_core::PokemonState,
) {
    let source_set = fixture_set(dex, &case.source, &case.move_name);
    let target_set = fixture_set(dex, &case.target, &case.move_name);
    let mut source = dex.pokemon_state(&source_set).expect("source state");
    let mut target = dex.pokemon_state(&target_set).expect("target state");
    source.stat_stages = case.source.stages.into();
    target.stat_stages = case.target.stages.into();
    let mut builder =
        BattleStateBuilder::new_with_seed(repeated_side(&source), repeated_side(&target), 4, seed)
            .expect("battle");
    let selection = four_selection();
    builder
        .select_teams(selection.clone(), selection)
        .expect("preview");
    if trace_rng {
        builder.enable_rng_trace();
    }
    (builder.build().expect("selected battle"), source, target)
}

fn p1_damage(
    actions: &[rotom_core::ResolvedAction],
) -> (rotom_core::ActiveSlot, rotom_core::DamageResult) {
    actions
        .iter()
        .find_map(|action| match action {
            rotom_core::ResolvedAction::Damaged {
                side: SideId::P1,
                target_slot,
                damage,
                ..
            } => Some((*target_slot, *damage)),
            _ => None,
        })
        .expect("P1 damage result")
}

fn assert_step_state(
    result: &rotom_core::StepResult,
    input: &rotom_core::BattleState,
    source: &rotom_core::PokemonState,
    target: &rotom_core::PokemonState,
    damaged_slot: rotom_core::ActiveSlot,
    case: &CanonicalStepCase,
) {
    let damaged_index = match damaged_slot {
        rotom_core::ActiveSlot::Left => PokemonIndex::new(0),
        rotom_core::ActiveSlot::Right => PokemonIndex::new(1),
    };
    let target_state = result
        .state
        .side(SideId::P2)
        .pokemon(damaged_index)
        .expect("target");
    assert_eq!(
        target_state.current_hp, case.expected.target_hp,
        "{} applied HP loss",
        case.name
    );
    assert_eq!(
        case.expected.applied_hp_loss,
        target.stats.hp - target_state.current_hp,
        "{} actual applied HP loss",
        case.name
    );
    assert_eq!(
        case.expected.fainted,
        case.expected.target_hp == 0,
        "{}",
        case.name
    );
    if case.expected.fainted {
        assert_ko_state(result, target_state, damaged_slot, case);
    } else {
        assert_surviving_state(
            result,
            target_state,
            target,
            damaged_slot,
            damaged_index,
            case,
        );
    }
    assert_eq!(
        result
            .state
            .replacement_request(SideId::P2)
            .expect("settled state should have valid replacement indexes")
            .is_some(),
        case.expected.replacement_scheduled,
        "{} actual replacement scheduling",
        case.name
    );
    assert_eq!(
        result
            .state
            .side(SideId::P1)
            .pokemon(PokemonIndex::new(0))
            .expect("source")
            .moves[0]
            .pp,
        source.moves[0].max_pp - case.expected.pp_consumed,
        "{} PP",
        case.name
    );
    assert_eq!(input.rng().draws(), 0, "{} input RNG changed", case.name);
    assert_stable_serializable_state(&result.state, &case.name);
}

fn assert_ko_state(
    result: &rotom_core::StepResult,
    target_state: &rotom_core::PokemonState,
    damaged_slot: rotom_core::ActiveSlot,
    case: &CanonicalStepCase,
) {
    assert_eq!(target_state.current_hp, 0, "{} faint HP", case.name);
    assert_eq!(
        target_state.position,
        PokemonPosition::Fainted,
        "{} faint position",
        case.name
    );
    assert_eq!(
        result.state.side(SideId::P2).active[damaged_slot.index()],
        None,
        "{} cleared active slot",
        case.name
    );
    assert_eq!(
        result.state.phase(),
        BattlePhase::ForcedSwitch,
        "{} forced-switch phase",
        case.name
    );
    let request = result
        .state
        .replacement_request(SideId::P2)
        .expect("settled state should have valid replacement indexes")
        .expect("fainted target's side should need a replacement");
    assert!(
        request.slots.contains(&damaged_slot),
        "{} replacement request must include the cleared slot",
        case.name
    );
    assert!(request.required > 0, "{} replacement count", case.name);
    assert_eq!(
        result
            .state
            .replacement_request(SideId::P1)
            .expect("settled state should have valid replacement indexes"),
        None,
        "{} opposing side must not need a replacement",
        case.name
    );
}

fn assert_surviving_state(
    result: &rotom_core::StepResult,
    target_state: &rotom_core::PokemonState,
    initial_target: &rotom_core::PokemonState,
    damaged_slot: rotom_core::ActiveSlot,
    damaged_index: PokemonIndex,
    case: &CanonicalStepCase,
) {
    assert_eq!(
        target_state.position,
        PokemonPosition::Active { slot: damaged_slot },
        "{} surviving target position",
        case.name
    );
    assert_eq!(
        result.state.side(SideId::P2).active[damaged_slot.index()],
        Some(damaged_index),
        "{} surviving target active slot",
        case.name
    );
    assert_eq!(
        result.state.phase(),
        BattlePhase::TurnSelection,
        "{} non-KO phase",
        case.name
    );
    for side in [SideId::P1, SideId::P2] {
        assert_eq!(
            result
                .state
                .replacement_request(side)
                .expect("settled state should have valid replacement indexes"),
            None,
            "{} non-KO replacement request for {side:?}",
            case.name
        );
    }
    if case.expected.damage == 0 {
        assert_eq!(
            target_state.current_hp, initial_target.stats.hp,
            "{} immunity must preserve HP",
            case.name
        );
    }
}

fn assert_stable_serializable_state(state: &BattleState, case_name: &str) {
    state
        .validate_stable_state()
        .unwrap_or_else(|error| panic!("{case_name} unstable step output: {error}"));
    let serialized = serde_json::to_string(state)
        .unwrap_or_else(|error| panic!("{case_name} state serialization failed: {error}"));
    let restored: BattleState = serde_json::from_str(&serialized)
        .unwrap_or_else(|error| panic!("{case_name} state deserialization failed: {error}"));
    assert_eq!(restored, *state, "{case_name} state round trip");
}

fn opposing_targets_choice() -> TurnChoice {
    TurnChoice {
        actions: [rotom_core::ActiveSlot::Left, rotom_core::ActiveSlot::Right].map(|target| {
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(target),
                mega: false,
            }
        }),
    }
}

#[test]
fn queued_turn_consumes_pp_and_updates_hp() {
    let dex = champions_reg_mc().expect("generated Dex");
    let special_attacker = set(
        &dex,
        "goodra",
        "gooey",
        "modest",
        "dragonpulse",
        StatAllocation::ZERO,
    );
    let physical_attacker = set(
        &dex,
        "goodra",
        "gooey",
        "adamant",
        "aquatail",
        StatAllocation::ZERO,
    );
    let make_side = |set: &PokemonSet| {
        let pokemon = dex.pokemon_state(set).expect("state");
        SideState::new(TeamState::new(vec![
            pokemon.clone(),
            pokemon.clone(),
            pokemon.clone(),
            pokemon,
        ]))
    };
    let mut builder = BattleStateBuilder::new_with_seed(
        make_side(&special_attacker),
        make_side(&physical_attacker),
        4,
        19,
    )
    .expect("battle");
    let selection = vec![
        PokemonIndex::new(0),
        PokemonIndex::new(1),
        PokemonIndex::new(2),
        PokemonIndex::new(3),
    ];
    builder
        .select_teams(selection.clone(), selection)
        .expect("preview");
    builder.enable_rng_trace();
    let battle = builder.build().expect("selected battle");
    let p1 = TurnChoice {
        actions: [
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(rotom_core::ActiveSlot::Left),
                mega: false,
            },
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(rotom_core::ActiveSlot::Right),
                mega: false,
            },
        ],
    };
    let p2 = TurnChoice {
        actions: [
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(rotom_core::ActiveSlot::Left),
                mega: false,
            },
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(rotom_core::ActiveSlot::Right),
                mega: false,
            },
        ],
    };
    let replay = battle.clone();
    let mechanics = Mechanics::new(&dex, &EffectRegistry::new());
    let result = step(&battle, &JointAction::Turn { p1, p2 }, &mechanics).expect("resolution");
    let replay_result =
        step(&replay, &JointAction::Turn { p1, p2 }, &mechanics).expect("replayed resolution");
    let outcome = result.diagnostics.turn.as_ref().unwrap();
    let replay_outcome = replay_result.diagnostics.turn.as_ref().unwrap();
    let battle = result.state;

    assert_eq!(outcome, replay_outcome);
    assert_eq!(battle, replay_result.state);
    assert_eq!(outcome.actions.len(), 4);
    assert_eq!(battle.phase(), BattlePhase::TurnSelection);
    assert_eq!(battle.turn(), 2);
    assert_eq!(battle.queue().actions.len(), 0);
    for side in [SideId::P1, SideId::P2] {
        for index in [PokemonIndex::new(0), PokemonIndex::new(1)] {
            let pokemon = battle.side(side).pokemon(index).expect("active");
            assert_eq!(pokemon.moves[0].pp, pokemon.moves[0].max_pp - 1);
            assert!(pokemon.current_hp < pokemon.stats.hp);
            assert!(pokemon.current_hp > 0);
        }
    }
    assert_combat_rng_trace(battle.rng().trace().expect("enabled trace"));
}

#[test]
fn canonical_step_matches_an_independently_selected_pinned_showdown_roll() {
    let dex = champions_reg_mc().expect("generated Dex");
    let fixture: DamageFixture = serde_json::from_str(include_str!(
        "../../../data/fixtures/differential/basic-damage.json"
    ))
    .expect("Showdown damage fixture should deserialize");
    let case = fixture
        .cases
        .iter()
        .find(|case| case.name == "dragon-pulse-neutral-stab")
        .expect("neutral Dragon Pulse fixture");
    // Seed 19's SplitMix64 trace is asserted below. Rotom Spark does not claim
    // seed parity with Showdown; the independently generated Showdown fixture
    // supplies the expected result for the preselected 96% damage outcome.
    let expected = case
        .rolls
        .iter()
        .find(|roll| roll.percent == 96)
        .expect("fixture's independently selected 96% roll");
    let (battle, source, target) = fixture_battle(&dex, case, 19, true);
    let choice = opposing_targets_choice();

    let result = step(
        &battle,
        &JointAction::Turn {
            p1: choice,
            p2: choice,
        },
        &Mechanics::new(&dex, &EffectRegistry::new()),
    )
    .expect("canonical move lifecycle");
    let trace = result.state.rng().trace().expect("enabled RNG trace");
    assert_eq!(
        trace.iter().map(|entry| entry.result).collect::<Vec<_>>(),
        [0, 2, 53, 1, 99, 11, 1, 85, 89, 10, 96, 9, 23, 98]
    );
    assert_eq!(trace[10].reason, RngReason::DamageRoll);
    assert_eq!(
        trace[10].operation,
        RngOperation::Range {
            start: 85,
            end_exclusive: 101,
        }
    );
    let (_, damage) = p1_damage(
        &result
            .diagnostics
            .turn
            .as_ref()
            .expect("turn diagnostics")
            .actions,
    );
    assert_eq!(damage.critical_modifier, rotom_core::Modifier::NEUTRAL);
    assert_eq!(damage.random_modifier.numerator, 96);
    assert_eq!(damage.random_modifier.denominator, 100);
    assert_eq!(damage.final_damage, expected.damage);
    assert_eq!(
        result
            .state
            .side(SideId::P1)
            .pokemon(PokemonIndex::new(0))
            .expect("source")
            .moves[0]
            .pp,
        source.moves[0].max_pp - 1
    );
    assert_eq!(
        battle
            .side(SideId::P1)
            .pokemon(PokemonIndex::new(0))
            .expect("unchanged input source")
            .moves[0]
            .pp,
        source.moves[0].max_pp
    );
    assert_eq!(
        result
            .state
            .side(SideId::P2)
            .pokemon(PokemonIndex::new(0))
            .expect("damaged target")
            .current_hp,
        target.stats.hp - expected.damage
    );
}

#[test]
fn canonical_step_matches_preselected_physical_special_effective_immune_and_critical_cases() {
    let dex = champions_reg_mc().expect("generated Dex");
    let damage_fixture: DamageFixture = serde_json::from_str(include_str!(
        "../../../data/fixtures/differential/basic-damage.json"
    ))
    .expect("Showdown damage fixture should deserialize");
    let step_fixture: CanonicalStepFixture = serde_json::from_str(include_str!(
        "../../../data/fixtures/differential/canonical-step.json"
    ))
    .expect("canonical step fixture should deserialize");
    assert_eq!(step_fixture.showdown_commit, SHOWDOWN_COMMIT);
    assert_eq!(step_fixture.format, "gen9championsvgc2026regmc");
    assert!(step_fixture
        .methodology
        .rng_comparison
        .contains("no seed-level parity"));
    assert!(step_fixture
        .methodology
        .damage_source
        .contains("Battle.actions.getDamage"));
    assert!(step_fixture
        .methodology
        .integration_scope
        .contains("Rotom Spark public step()"));
    assert!(step_fixture
        .methodology
        .full_transition_comparison
        .contains("not performed"));
    assert_eq!(
        step_fixture.methodology.move_draw_order,
        ["accuracy", "critical_hit", "damage_roll"]
    );

    for step_case in step_fixture.cases {
        let case = damage_fixture
            .cases
            .iter()
            .find(|case| case.name == step_case.damage_case)
            .unwrap_or_else(|| panic!("missing fixture case {}", step_case.damage_case));
        assert_eq!(step_case.move_name, case.move_name, "{}", step_case.name);
        assert_eq!(step_case.controlled_outcomes.accuracy, "hit");
        assert_eq!(
            step_case.controlled_outcomes.critical_hit, case.critical,
            "{}",
            step_case.name
        );
        let expected = case
            .rolls
            .iter()
            .find(|roll| roll.percent == step_case.controlled_outcomes.damage_percent)
            .unwrap_or_else(|| panic!("missing preselected roll for {}", step_case.name));
        assert_eq!(
            expected.damage, step_case.expected.damage,
            "{}",
            step_case.name
        );
        let (battle, source, target) = fixture_battle(&dex, case, step_case.rotom_seed, false);
        let result = step(
            &battle,
            &JointAction::Turn {
                p1: opposing_targets_choice(),
                p2: opposing_targets_choice(),
            },
            &Mechanics::new(&dex, &EffectRegistry::new()),
        )
        .unwrap_or_else(|error| panic!("{}: {error}", step_case.name));
        let (damaged_slot, damage) = p1_damage(
            &result
                .diagnostics
                .turn
                .as_ref()
                .expect("turn diagnostics")
                .actions,
        );
        assert_eq!(
            damage.random_modifier.numerator,
            u16::from(step_case.controlled_outcomes.damage_percent),
            "{}",
            step_case.name
        );
        assert_eq!(
            damage.random_modifier.denominator, 100,
            "{}",
            step_case.name
        );
        assert_eq!(
            damage.critical_modifier != rotom_core::Modifier::NEUTRAL,
            step_case.controlled_outcomes.critical_hit,
            "{} critical outcome",
            step_case.name
        );
        assert_eq!(
            damage.final_damage, step_case.expected.damage,
            "{}",
            step_case.name
        );
        assert_step_state(&result, &battle, &source, &target, damaged_slot, &step_case);
    }
}

#[test]
fn resolved_dex_requirements_drive_support_without_a_move_id_allowlist() {
    let dex = champions_reg_mc().expect("generated Dex");
    let definition = |id: &str| {
        let move_id = dex.resolve_move(id).expect("move identity").0;
        <rotom_dex::DexData as CombatData>::move_data(&dex, move_id).expect("combat move")
    };

    let dragon_claw = definition("dragonclaw");
    assert_eq!(dragon_claw.operation, MoveOperation::StandardDamage);
    assert_eq!(
        dragon_claw
            .required_capabilities
            .first_missing_from(MoveCapabilities::PR_13_1),
        None
    );

    for (id, capability) in [
        ("flamethrower", MoveCapability::SecondaryEffect),
        ("shadowball", MoveCapability::SecondaryEffect),
        ("facade", MoveCapability::CustomCallback),
        ("surf", MoveCapability::AdvancedTargeting),
        ("bulletseed", MoveCapability::MultiHit),
        ("gigadrain", MoveCapability::Drain),
        ("electroshot", MoveCapability::CustomCallback),
    ] {
        let move_data = definition(id);
        assert!(
            move_data.required_capabilities.contains(capability),
            "{id} must retain its unsupported intrinsic behavior"
        );
        assert!(
            move_data
                .required_capabilities
                .first_missing_from(MoveCapabilities::PR_13_1)
                .is_some(),
            "{id} must not degrade to plain damage"
        );
    }
}

fn assert_combat_rng_trace(trace: &[rotom_core::RngTraceEntry]) {
    assert_eq!(trace.len(), 15);
    assert!(trace[..3]
        .iter()
        .all(|entry| entry.reason == RngReason::SpeedTie));
    let mut accuracy_numerators = Vec::new();
    for action_trace in trace[3..].chunks_exact(3) {
        assert_eq!(action_trace[0].reason, RngReason::Accuracy);
        let RngOperation::Chance {
            numerator,
            denominator,
        } = action_trace[0].operation
        else {
            panic!("accuracy must use a chance draw")
        };
        assert_eq!(denominator, 100);
        accuracy_numerators.push(numerator);
        assert_eq!(action_trace[1].reason, RngReason::CriticalHit);
        assert_eq!(
            action_trace[1].operation,
            RngOperation::Chance {
                numerator: 1,
                denominator: 24,
            }
        );
        assert_eq!(action_trace[2].reason, RngReason::DamageRoll);
        assert_eq!(
            action_trace[2].operation,
            RngOperation::Range {
                start: 85,
                end_exclusive: 101,
            }
        );
        assert!((85..=100).contains(&action_trace[2].result));
    }
    accuracy_numerators.sort_unstable();
    assert_eq!(accuracy_numerators, [90, 90, 100, 100]);
}

#[test]
fn unsupported_move_capabilities_are_explicit_and_atomic() {
    let dex = champions_reg_mc().expect("generated Dex");
    for (move_name, expected_capability) in [
        ("flamethrower", "secondary_effect"),
        ("shadowball", "secondary_effect"),
        ("facade", "custom_callback"),
        ("surf", "advanced_targeting"),
        ("bulletseed", "multi_hit"),
        ("gigadrain", "drain"),
        ("electroshot", "custom_callback"),
    ] {
        let unsupported = set(
            &dex,
            "goodra",
            "gooey",
            "serious",
            move_name,
            StatAllocation::ZERO,
        );
        let make_side = || {
            let pokemon = dex.pokemon_state(&unsupported).expect("state");
            SideState::new(TeamState::new(vec![
                pokemon.clone(),
                pokemon.clone(),
                pokemon.clone(),
                pokemon,
            ]))
        };
        let mut builder =
            BattleStateBuilder::new_with_seed(make_side(), make_side(), 4, 7).expect("battle");
        let selection = vec![
            PokemonIndex::new(0),
            PokemonIndex::new(1),
            PokemonIndex::new(2),
            PokemonIndex::new(3),
        ];
        builder
            .select_teams(selection.clone(), selection)
            .expect("preview");
        let battle = builder.build().unwrap();
        let targets = if move_name == "surf" {
            [ActionTarget::Automatic, ActionTarget::Automatic]
        } else {
            [
                ActionTarget::Foe(rotom_core::ActiveSlot::Left),
                ActionTarget::Foe(rotom_core::ActiveSlot::Right),
            ]
        };
        let choice = TurnChoice {
            actions: targets.map(|target| PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target,
                mega: false,
            }),
        };
        let before = battle.clone();

        let error = step(
            &battle,
            &JointAction::Turn {
                p1: choice,
                p2: choice,
            },
            &Mechanics::new(&dex, &EffectRegistry::new()),
        )
        .unwrap_err();
        assert!(
            matches!(error, rotom_core::BattleError::UnsupportedMechanic(_)),
            "{move_name}: {error}"
        );
        assert!(
            error.to_string().contains(expected_capability),
            "{move_name}: {error}"
        );
        assert_eq!(battle, before, "{move_name} mutated its public input");
    }
}

#[test]
fn combat_damage_processes_faints_and_requests_replacements() {
    let dex = champions_reg_mc().expect("generated Dex");
    let attacker = set(
        &dex,
        "glimmora",
        "corrosion",
        "modest",
        "powergem",
        StatAllocation {
            spa: 32,
            ..StatAllocation::ZERO
        },
    );
    let defender = set(
        &dex,
        "charizard",
        "blaze",
        "serious",
        "powergem",
        StatAllocation::ZERO,
    );
    let make_side = |set: &PokemonSet| {
        let pokemon = dex.pokemon_state(set).expect("state");
        SideState::new(TeamState::new(vec![
            pokemon.clone(),
            pokemon.clone(),
            pokemon.clone(),
            pokemon,
        ]))
    };
    let mut builder =
        BattleStateBuilder::new_with_seed(make_side(&attacker), make_side(&defender), 4, 31)
            .expect("battle");
    let selection = vec![
        PokemonIndex::new(0),
        PokemonIndex::new(1),
        PokemonIndex::new(2),
        PokemonIndex::new(3),
    ];
    builder
        .select_teams(selection.clone(), selection)
        .expect("preview");
    let battle = builder.build().unwrap();
    let attack = |target| PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Foe(target),
        mega: false,
    };
    let result = step(
        &battle,
        &JointAction::Turn {
            p1: TurnChoice {
                actions: [
                    attack(rotom_core::ActiveSlot::Left),
                    attack(rotom_core::ActiveSlot::Right),
                ],
            },
            p2: TurnChoice {
                actions: [
                    attack(rotom_core::ActiveSlot::Left),
                    attack(rotom_core::ActiveSlot::Right),
                ],
            },
        },
        &Mechanics::new(&dex, &EffectRegistry::new()),
    )
    .expect("resolution");
    let battle = result.state;

    assert_eq!(battle.phase(), BattlePhase::ForcedSwitch);
    assert!(battle
        .side(SideId::P2)
        .pokemon(PokemonIndex::new(0))
        .expect("target")
        .is_fainted());
    assert!(battle
        .side(SideId::P2)
        .pokemon(PokemonIndex::new(1))
        .expect("target")
        .is_fainted());
    assert_eq!(
        battle
            .replacement_request(SideId::P2)
            .expect("request")
            .expect("needed")
            .required,
        2
    );
}
