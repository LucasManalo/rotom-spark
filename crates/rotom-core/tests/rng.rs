use rotom_core::{
    AbilityId, BattleError, BattleRng, BattleState, BattleStats, MoveId, MoveSlot, MoveTarget,
    PokemonPosition, PokemonState, RngOperation, RngReason, SideState, SpeciesId, StatStages,
    StatusState, TeamState,
};

fn pokemon(species: u32) -> PokemonState {
    PokemonState {
        species: SpeciesId::new(species),
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

fn battle(seed: u64) -> BattleState {
    BattleState::new_with_seed(
        SideState::new(TeamState::new((1..=4).map(pokemon).collect())),
        SideState::new(TeamState::new((101..=104).map(pokemon).collect())),
        4,
        seed,
    )
    .expect("fixture should create a valid battle")
}

fn sequence(rng: &mut BattleRng) -> (u64, bool, u8) {
    let range = rng
        .range(RngReason::DamageRoll, 85..101)
        .expect("damage range should be valid");
    let chance = rng
        .chance(RngReason::CriticalHit, 1, 24)
        .expect("critical-hit chance should be valid");
    let sampled = *rng
        .sample(RngReason::RandomMove, &[2_u8, 4, 6, 8])
        .expect("population should be non-empty");
    (range, chance, sampled)
}

#[test]
fn equal_seeds_and_calls_reproduce_the_same_stream() {
    let mut first = BattleRng::new(0x1234_5678_9abc_def0);
    let mut second = first.clone();

    assert_eq!(sequence(&mut first), sequence(&mut second));
    assert_eq!(first, second);
    assert_eq!(first.draws(), 3);
}

#[test]
fn rng_is_battle_owned_cloneable_and_serializable() {
    let mut builder =
        rotom_core::BattleStateBuilder::from_state(battle(42)).expect("fixture should be stable");
    let first = builder
        .rng_mut()
        .range(RngReason::SpeedTie, 0..2)
        .expect("speed tie range should be valid");
    let original = builder.build().expect("rng draw preserves validity");
    let json = serde_json::to_string(&original).expect("battle should serialize");
    let restored: BattleState = serde_json::from_str(&json).expect("battle should deserialize");
    let cloned = original.clone();

    assert!(first < 2);
    let mut original_builder = rotom_core::BattleStateBuilder::from_state(original).unwrap();
    let mut restored_builder = rotom_core::BattleStateBuilder::from_state(restored).unwrap();
    let mut cloned_builder = rotom_core::BattleStateBuilder::from_state(cloned).unwrap();
    let expected = sequence(original_builder.rng_mut());
    assert_eq!(expected, sequence(restored_builder.rng_mut()));
    assert_eq!(expected, sequence(cloned_builder.rng_mut()));
}

#[test]
fn tracing_is_optional_diagnostic_state_and_does_not_change_results() {
    let mut traced = BattleRng::new(7);
    let mut untraced = BattleRng::new(7);
    traced.enable_trace();

    assert_eq!(sequence(&mut traced), sequence(&mut untraced));
    let trace = traced.trace().expect("tracing should be enabled");
    assert_eq!(trace.len(), 3);
    assert_eq!(trace[0].reason, RngReason::DamageRoll);
    assert_eq!(
        trace[0].operation,
        RngOperation::Range {
            start: 85,
            end_exclusive: 101,
        }
    );
    assert!((85..101).contains(&trace[0].result));
    assert_eq!(trace[2].operation, RngOperation::Sample { len: 4 });

    traced.clear_trace();
    assert_eq!(traced.trace(), Some([].as_slice()));
    traced.disable_trace();
    assert_eq!(traced.trace(), None);
}

#[test]
fn invalid_random_requests_fail_without_consuming_the_stream() {
    let mut rng = BattleRng::new(99);

    assert!(matches!(
        rng.range(RngReason::DamageRoll, 10..10),
        Err(BattleError::InvalidState(_))
    ));
    assert!(matches!(
        rng.chance(RngReason::Accuracy, 2, 1),
        Err(BattleError::InvalidState(_))
    ));
    assert!(matches!(
        rng.sample::<u8>(RngReason::RandomTarget, &[]),
        Err(BattleError::InvalidState(_))
    ));
    assert_eq!(rng.draws(), 0);
}
