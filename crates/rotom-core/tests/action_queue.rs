use rotom_core::{
    AbilityId, ActionOrder, ActionTarget, ActiveSlot, BattleError, BattlePhase, BattleState,
    BattleStats, MoveId, MoveSlot, MoveSlotIndex, MoveTarget, PokemonAction, PokemonIndex,
    PokemonPosition, PokemonState, RngReason, SideId, SideState, SpeciesId, StatStages,
    StatusState, TeamState, TurnChoice,
};

fn index(value: u8) -> PokemonIndex {
    PokemonIndex::new(value)
}

fn pokemon(species: u32, speed: u16, priority: i8) -> PokemonState {
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
            spe: speed,
        },
        current_hp: 100,
        ability: AbilityId::new(1),
        item: None,
        moves: vec![MoveSlot {
            move_id: MoveId::new(1),
            target: MoveTarget::Normal,
            priority,
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

fn battle(seed: u64, p1: [(u16, i8); 4], p2: [(u16, i8); 4]) -> BattleState {
    let p1 = p1
        .into_iter()
        .enumerate()
        .map(|(species, (speed, priority))| {
            pokemon(
                u32::try_from(species).expect("fixture index should fit in u32") + 1,
                speed,
                priority,
            )
        })
        .collect();
    let p2 = p2
        .into_iter()
        .enumerate()
        .map(|(species, (speed, priority))| {
            pokemon(
                u32::try_from(species).expect("fixture index should fit in u32") + 101,
                speed,
                priority,
            )
        })
        .collect();
    let mut state = BattleState::new_with_seed(
        SideState::new(TeamState::new(p1)),
        SideState::new(TeamState::new(p2)),
        4,
        seed,
    )
    .expect("fixture should create a valid battle");
    let selection = vec![index(0), index(1), index(2), index(3)];
    state
        .select_team(SideId::P1, selection.clone())
        .expect("P1 selection should be valid");
    state
        .select_team(SideId::P2, selection)
        .expect("P2 selection should be valid");
    state
}

fn move_choice() -> PokemonAction {
    PokemonAction::Move {
        move_slot: MoveSlotIndex::new(0),
        target: ActionTarget::Foe(ActiveSlot::Left),
        mega: false,
    }
}

fn both_moves() -> TurnChoice {
    TurnChoice {
        actions: [move_choice(), move_choice()],
    }
}

#[test]
fn switches_move_priority_and_speed_form_the_normal_ordering_keys() {
    let mut state = battle(
        5,
        [(50, 1), (120, 0), (10, 0), (20, 0)],
        [(80, 0), (90, 0), (10, 0), (20, 0)],
    );
    let p2 = TurnChoice {
        actions: [PokemonAction::Switch { pokemon: index(2) }, move_choice()],
    };

    let queue = state
        .queue_turn(both_moves(), p2)
        .expect("legal choices should queue")
        .clone();

    assert_eq!(state.phase, BattlePhase::ResolvingTurn);
    assert_eq!(queue.actions.len(), 4);
    assert_eq!(queue.actions[0].order, ActionOrder::Switch);
    assert_eq!(queue.actions[0].side, SideId::P2);
    assert_eq!(queue.actions[0].slot, ActiveSlot::Left);
    assert_eq!(queue.actions[1].priority, 1);
    assert_eq!(queue.actions[1].side, SideId::P1);
    assert_eq!(queue.actions[1].slot, ActiveSlot::Left);
    assert_eq!(queue.actions[2].speed, 120);
    assert_eq!(queue.actions[2].side, SideId::P1);
    assert_eq!(queue.actions[3].speed, 90);
    assert_eq!(state.rng.draws(), 0);
}

#[test]
fn exact_speed_ties_use_only_the_battle_rng_and_reproduce() {
    let stats = [(100, 0); 4];
    let mut first = battle(0xfeed_beef, stats, stats);
    let mut second = first.clone();
    first.rng.enable_trace();
    second.rng.enable_trace();

    let first_queue = first
        .queue_turn(both_moves(), both_moves())
        .expect("tied actions should queue")
        .clone();
    let second_queue = second
        .queue_turn(both_moves(), both_moves())
        .expect("same seeded tied actions should queue")
        .clone();

    assert_eq!(first_queue, second_queue);
    assert_eq!(first.rng.draws(), 3);
    assert_eq!(first.rng, second.rng);
    assert!(first
        .rng
        .trace()
        .expect("tracing should be enabled")
        .iter()
        .all(|entry| entry.reason == RngReason::SpeedTie));
}

#[test]
fn priority_groups_only_shuffle_within_exact_ties() {
    let mut state = battle(
        17,
        [(100, 1), (100, 0), (10, 0), (20, 0)],
        [(100, 1), (100, 0), (10, 0), (20, 0)],
    );

    let queue = state
        .queue_turn(both_moves(), both_moves())
        .expect("legal choices should queue")
        .clone();

    assert_eq!(queue.actions[0].priority, 1);
    assert_eq!(queue.actions[1].priority, 1);
    assert_eq!(queue.actions[2].priority, 0);
    assert_eq!(queue.actions[3].priority, 0);
    assert_eq!(state.rng.draws(), 2);
}

#[test]
fn invalid_choices_do_not_change_phase_queue_or_rng() {
    let mut state = battle(23, [(100, 0); 4], [(100, 0); 4]);
    let invalid = TurnChoice {
        actions: [
            PokemonAction::Switch { pokemon: index(2) },
            PokemonAction::Switch { pokemon: index(2) },
        ],
    };
    let rng_before = state.rng.clone();

    assert!(matches!(
        state.queue_turn(invalid, both_moves()),
        Err(BattleError::IllegalChoice(_))
    ));
    assert_eq!(state.phase, BattlePhase::TurnSelection);
    assert_eq!(state.queue.actions, Vec::new());
    assert_eq!(state.rng, rng_before);
}

#[test]
fn deferred_mega_queue_expansion_fails_explicitly_and_atomically() {
    let mut state = battle(29, [(100, 0); 4], [(100, 0); 4]);
    state
        .side_mut(SideId::P1)
        .pokemon_mut(index(0))
        .expect("left lead should exist")
        .can_mega_evolve = true;
    let mega = TurnChoice {
        actions: [
            PokemonAction::Move {
                move_slot: MoveSlotIndex::new(0),
                target: ActionTarget::Foe(ActiveSlot::Left),
                mega: true,
            },
            move_choice(),
        ],
    };
    let rng_before = state.rng.clone();

    assert!(matches!(
        state.queue_turn(mega, both_moves()),
        Err(BattleError::UnsupportedMechanic(_))
    ));
    assert_eq!(state.phase, BattlePhase::TurnSelection);
    assert_eq!(state.queue.actions, Vec::new());
    assert_eq!(state.rng, rng_before);
}

#[test]
fn queued_turn_round_trips_with_authoritative_state() {
    let mut state = battle(
        31,
        [(140, 0), (120, 0), (10, 0), (20, 0)],
        [(100, 0), (80, 0), (10, 0), (20, 0)],
    );
    state
        .queue_turn(both_moves(), both_moves())
        .expect("legal choices should queue");

    let json = serde_json::to_string(&state).expect("queued battle should serialize");
    let decoded: BattleState =
        serde_json::from_str(&json).expect("queued battle should deserialize");
    assert_eq!(decoded, state);
}
