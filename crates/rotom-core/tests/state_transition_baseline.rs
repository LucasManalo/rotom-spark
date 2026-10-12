use rotom_core::{
    AbilityId, ActionTarget, ActiveSlot, BattlePhase, BattleState, BattleStats, CombatData,
    CombatMove, ConditionId, ConditionState, FormId, MajorStatus, MoveAccuracy, MoveCategory,
    MoveId, MoveSlot, MoveSlotIndex, MoveSupport, MoveTarget, PokemonAction, PokemonIndex,
    PokemonPosition, PokemonState, PokemonType, PokemonTypes, RngReason, SideId, SideState,
    SpeciesId, StatStages, StatusState, TeamState, TurnChoice,
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
    let mut state = BattleState::new_with_seed(
        SideState::new(TeamState::new(p1)),
        SideState::new(TeamState::new(p2)),
        4,
        seed,
    )
    .expect("baseline fixture should create a valid battle");
    let selected = vec![index(0), index(1), index(2), index(3)];
    state
        .select_team(SideId::P1, selected.clone())
        .expect("P1 preview should be valid");
    state
        .select_team(SideId::P2, selected)
        .expect("P2 preview should be valid");
    state
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
    first.rng.enable_trace();
    let mut replay = first.clone();
    let (p1, p2) = attacking_choices();

    first.queue_turn(p1, p2).expect("turn should queue");
    replay.queue_turn(p1, p2).expect("replay should queue");
    let first_resolution = first
        .resolve_turn(&TestData { base_power: 20 })
        .expect("turn should resolve");
    let replay_resolution = replay
        .resolve_turn(&TestData { base_power: 20 })
        .expect("replay should resolve");

    assert_eq!(first_resolution, replay_resolution);
    assert_eq!(first, replay);
    assert_eq!(first.phase, BattlePhase::TurnSelection);
    first
        .validate_stable_state()
        .expect("resolved turn should be a stable state");
    assert_eq!(first.turn, 2);
    assert_eq!(first.queue.actions, Vec::new());
    assert_eq!(first.queue.events, Vec::new());
    assert_eq!(first_resolution.actions.len(), 4);
    for side in [SideId::P1, SideId::P2] {
        for pokemon in [index(0), index(1)] {
            let pokemon = first.side(side).pokemon(pokemon).expect("active exists");
            assert_eq!(pokemon.moves[0].pp, 15);
            assert!((1..120).contains(&pokemon.current_hp));
        }
    }
    let reasons = first
        .rng
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
    let mut state = battle(0x51de);
    let outgoing = state
        .side_mut(SideId::P1)
        .pokemon_mut(index(0))
        .expect("left lead exists");
    outgoing
        .set_major_status(
            MajorStatus::BadlyPoisoned,
            StatusState {
                toxic_counter: 3,
                ..StatusState::default()
            },
        )
        .expect("toxic fixture is valid");
    outgoing.stat_stages.atk = 4;
    outgoing
        .add_volatile(ConditionState::new(ConditionId::new(9), Some(2)))
        .expect("volatile fixture is valid");
    state
        .side_mut(SideId::P1)
        .pokemon_mut(index(1))
        .expect("right lead exists")
        .set_major_status(MajorStatus::Poison, StatusState::default())
        .expect("poison fixture is valid");

    let p1 = TurnChoice {
        actions: [
            PokemonAction::Switch { pokemon: index(2) },
            move_at(ActiveSlot::Right),
        ],
    };
    let (_, p2) = attacking_choices();
    state.queue_turn(p1, p2).expect("turn should queue");
    let resolution = state
        .resolve_turn(&TestData { base_power: 20 })
        .expect("turn should resolve");

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
    let mut state = battle(0xfa17);
    let (p1, p2) = attacking_choices();
    state.queue_turn(p1, p2).expect("turn should queue");
    let resolution = state
        .resolve_turn(&TestData { base_power: 1_000 })
        .expect("turn should resolve");

    assert_eq!(state.phase, BattlePhase::ForcedSwitch);
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
