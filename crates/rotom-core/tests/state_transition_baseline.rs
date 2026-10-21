use rotom_core::{
    step, AbilityId, ActionTarget, ActiveSlot, BattlePhase, BattleState, BattleStateBuilder,
    BattleStats, CombatData, CombatMove, ConditionId, ConditionState, EffectRegistry, FormId,
    JointAction, MajorStatus, Mechanics, MoveAccuracy, MoveCapabilities, MoveCategory, MoveId,
    MoveOperation, MoveSlot, MoveSlotIndex, MoveTarget, PokemonAction, PokemonIndex,
    PokemonPosition, PokemonRef, PokemonState, PokemonType, PokemonTypes, RngReason, SideId,
    SideState, SpeciesId, StatStages, StatusState, TeamState, TurnChoice,
};

const MOVE_ID: MoveId = MoveId::new(1);

#[derive(Clone, Copy)]
struct TestData {
    base_power: u16,
}

impl CombatData for TestData {
    fn move_data(&self, move_id: MoveId) -> Option<CombatMove> {
        (move_id == MOVE_ID).then_some(CombatMove {
            move_type: PokemonType::Normal,
            category: MoveCategory::Physical,
            base_power: self.base_power,
            accuracy: MoveAccuracy::Always,
            crit_ratio: 1,
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

fn battle(seed: u64) -> BattleState {
    let p1 = [(1, 140), (2, 120), (3, 70), (4, 60)]
        .into_iter()
        .map(|(species, speed)| pokemon(species, speed))
        .collect();
    let p2 = [(101, 100), (102, 80), (103, 50), (104, 40)]
        .into_iter()
        .map(|(species, speed)| pokemon(species, speed))
        .collect();
    let mut builder = BattleStateBuilder::new_with_seed(
        SideState::new(TeamState::new(p1)),
        SideState::new(TeamState::new(p2)),
        4,
        seed,
    )
    .expect("baseline fixture should create a valid battle");
    let selected = vec![index(0), index(1), index(2), index(3)];
    builder
        .select_teams(selected.clone(), selected)
        .expect("preview should be valid");
    builder.build().expect("baseline fixture should be stable")
}

fn move_at(target: ActiveSlot) -> PokemonAction {
    PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Foe(target),
        mega: false,
    }
}

fn attacking_choices() -> (TurnChoice, TurnChoice) {
    (
        TurnChoice {
            actions: [move_at(ActiveSlot::Left), move_at(ActiveSlot::Right)],
        },
        TurnChoice {
            actions: [move_at(ActiveSlot::Left), move_at(ActiveSlot::Right)],
        },
    )
}

#[test]
fn normal_turn_replays_state_resolution_and_rng_trace_exactly() {
    let mut first = battle(0x5eed);
    first.enable_rng_trace();
    let replay = first.clone();
    let (p1, p2) = attacking_choices();

    let data = TestData { base_power: 20 };
    let mechanics = Mechanics::new(&data, &EffectRegistry::new());
    let first_result =
        step(&first, &JointAction::Turn { p1, p2 }, &mechanics).expect("turn should resolve");
    let replay_result =
        step(&replay, &JointAction::Turn { p1, p2 }, &mechanics).expect("replay should resolve");
    let first_resolution = first_result.diagnostics.turn.as_ref().unwrap();
    let replay_resolution = replay_result.diagnostics.turn.as_ref().unwrap();
    let first = first_result.state;
    let replay = replay_result.state;

    assert_eq!(first_resolution, replay_resolution);
    assert_eq!(first, replay);
    assert_eq!(first.phase(), BattlePhase::TurnSelection);
    first
        .validate_stable_state()
        .expect("resolved turn should be a stable state");
    assert_eq!(first.turn(), 2);
    assert_eq!(first.queue().actions, Vec::new());
    assert_eq!(first.queue().events, Vec::new());
    assert_eq!(first_resolution.actions.len(), 4);
    for side in [SideId::P1, SideId::P2] {
        for pokemon in [index(0), index(1)] {
            let pokemon = first.side(side).pokemon(pokemon).expect("active exists");
            assert_eq!(pokemon.moves[0].pp, 15);
            assert!((1..120).contains(&pokemon.current_hp));
        }
    }
    let reasons = first
        .rng()
        .trace()
        .expect("trace should remain enabled")
        .iter()
        .map(|entry| entry.reason.clone())
        .collect::<Vec<_>>();
    assert_eq!(reasons.len(), 8);
    for pair in reasons.chunks_exact(2) {
        assert_eq!(pair, [RngReason::CriticalHit, RngReason::DamageRoll]);
    }
}

#[test]
fn switching_and_status_upkeep_preserve_the_current_state_contract() {
    let state = battle(0x51de);
    let mut builder = BattleStateBuilder::from_state(state).unwrap();
    builder
        .set_major_status(
            PokemonRef::new(SideId::P1, index(0)),
            MajorStatus::BadlyPoisoned,
            StatusState {
                toxic_counter: 3,
                ..StatusState::default()
            },
        )
        .expect("toxic fixture is valid");
    builder
        .edit_pokemon(PokemonRef::new(SideId::P1, index(0)), |outgoing| {
            outgoing.stat_stages.atk = 4;
            outgoing
                .add_volatile(ConditionState::new(ConditionId::new(9), Some(2)))
                .unwrap();
        })
        .unwrap();
    builder
        .set_major_status(
            PokemonRef::new(SideId::P1, index(1)),
            MajorStatus::Poison,
            StatusState::default(),
        )
        .expect("poison fixture is valid");
    let state = builder.build().unwrap();

    let p1 = TurnChoice {
        actions: [
            PokemonAction::Switch { pokemon: index(2) },
            move_at(ActiveSlot::Right),
        ],
    };
    let (_, p2) = attacking_choices();
    let data = TestData { base_power: 20 };
    let result = step(
        &state,
        &JointAction::Turn { p1, p2 },
        &Mechanics::new(&data, &EffectRegistry::new()),
    )
    .expect("turn should resolve");
    let resolution = result.diagnostics.turn.as_ref().unwrap();
    let state = result.state;

    assert_eq!(state.side(SideId::P1).active[0], Some(index(2)));
    assert_eq!(
        state.side(SideId::P1).pokemon(index(2)).unwrap().position,
        PokemonPosition::Active {
            slot: ActiveSlot::Left
        }
    );
    let outgoing = state.side(SideId::P1).pokemon(index(0)).unwrap();
    assert_eq!(outgoing.position, PokemonPosition::Team);
    assert_eq!(outgoing.stat_stages, StatStages::default());
    assert_eq!(outgoing.volatiles, Vec::new());
    assert_eq!(outgoing.status, Some(MajorStatus::BadlyPoisoned));
    assert_eq!(outgoing.status_state.toxic_counter, 3);
    assert_eq!(resolution.residual_damage.len(), 1);
    assert_eq!(resolution.residual_damage[0].status, MajorStatus::Poison);
    assert_eq!(resolution.residual_damage[0].damage, 15);
    state
        .validate_stable_state()
        .expect("switch and upkeep should settle to a stable state");
}

#[test]
fn damage_faints_settle_positions_and_request_replacements() {
    let state = battle(0xfa17);
    let (p1, p2) = attacking_choices();
    let data = TestData { base_power: 1_000 };
    let result = step(
        &state,
        &JointAction::Turn { p1, p2 },
        &Mechanics::new(&data, &EffectRegistry::new()),
    )
    .expect("turn should resolve");
    let resolution = result.diagnostics.turn.as_ref().unwrap();
    let state = result.state;

    assert_eq!(state.phase(), BattlePhase::ForcedSwitch);
    assert_eq!(state.side(SideId::P2).active, [None, None]);
    for pokemon in [index(0), index(1)] {
        let pokemon = state.side(SideId::P2).pokemon(pokemon).unwrap();
        assert_eq!(pokemon.current_hp, 0);
        assert_eq!(pokemon.position, PokemonPosition::Fainted);
        assert_eq!(pokemon.status, None);
        assert_eq!(pokemon.status_state, StatusState::default());
    }
    assert_eq!(resolution.actions.len(), 4);
    assert_eq!(
        state
            .replacement_request(SideId::P2)
            .expect("state should be valid")
            .expect("P2 should need replacements")
            .required,
        2
    );
    state
        .validate_stable_state()
        .expect("forced-switch output should be a stable state");
}
