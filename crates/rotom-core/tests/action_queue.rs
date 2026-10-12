use rotom_core::{
    AbilityId, ActionOrder, ActionTarget, ActiveSlot, BattleError, BattleState, BattleStateBuilder,
    BattleStats, CombatData, CombatMove, EffectRegistry, FormId, Mechanics, MoveId, MoveSlot,
    MoveSlotIndex, MoveTarget, PokemonAction, PokemonIndex, PokemonPosition, PokemonRef,
    PokemonState, PokemonType, PokemonTypes, SideId, SideState, SpeciesId, StatStages, StatusState,
    TeamState, TurnChoice,
};

struct QueueData;

impl CombatData for QueueData {
    fn move_data(&self, _: MoveId) -> Option<CombatMove> {
        None
    }
    fn pokemon_types(&self, _: SpeciesId, _: Option<FormId>) -> Option<PokemonTypes> {
        None
    }
    fn type_modifier(&self, _: PokemonType, _: PokemonType) -> Option<i8> {
        None
    }
}

fn mechanics() -> Mechanics<'static, QueueData> {
    static DATA: QueueData = QueueData;
    Mechanics::new(&DATA, &EffectRegistry::new())
}

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
        .expect("selections should be valid");
    builder.build().expect("fixture should be stable")
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
    let state = battle(
        5,
        [(50, 1), (120, 0), (10, 0), (20, 0)],
        [(80, 0), (90, 0), (10, 0), (20, 0)],
    );
    let p2 = TurnChoice {
        actions: [PokemonAction::Switch { pokemon: index(2) }, move_choice()],
    };

    let queue = state
        .inspect_turn_queue(both_moves(), p2, &mechanics())
        .expect("legal choices should queue");

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
    assert_eq!(state.rng().draws(), 0);
}

#[test]
fn exact_speed_ties_use_only_the_battle_rng_and_reproduce() {
    let stats = [(100, 0); 4];
    let first = battle(0xfeed_beef, stats, stats);
    let second = first.clone();

    let first_queue = first
        .inspect_turn_queue(both_moves(), both_moves(), &mechanics())
        .expect("tied actions should queue");
    let second_queue = second
        .inspect_turn_queue(both_moves(), both_moves(), &mechanics())
        .expect("same seeded tied actions should queue");

    assert_eq!(first_queue, second_queue);
    assert_eq!(first.rng().draws(), 0);
    assert_eq!(first.rng(), second.rng());
}

#[test]
fn priority_groups_only_shuffle_within_exact_ties() {
    let state = battle(
        17,
        [(100, 1), (100, 0), (10, 0), (20, 0)],
        [(100, 1), (100, 0), (10, 0), (20, 0)],
    );

    let queue = state
        .inspect_turn_queue(both_moves(), both_moves(), &mechanics())
        .expect("legal choices should queue");

    assert_eq!(queue.actions[0].priority, 1);
    assert_eq!(queue.actions[1].priority, 1);
    assert_eq!(queue.actions[2].priority, 0);
    assert_eq!(queue.actions[3].priority, 0);
    assert_eq!(state.rng().draws(), 0);
}

#[test]
fn invalid_choices_do_not_change_phase_queue_or_rng() {
    let state = battle(23, [(100, 0); 4], [(100, 0); 4]);
    let invalid = TurnChoice {
        actions: [
            PokemonAction::Switch { pokemon: index(2) },
            PokemonAction::Switch { pokemon: index(2) },
        ],
    };
    let rng_before = state.rng().clone();

    assert!(matches!(
        state.inspect_turn_queue(invalid, both_moves(), &mechanics()),
        Err(BattleError::IllegalChoice(_))
    ));
    assert_eq!(state.queue().actions, Vec::new());
    assert_eq!(state.rng(), &rng_before);
}

#[test]
fn deferred_mega_queue_expansion_fails_explicitly_and_atomically() {
    let mut builder = BattleStateBuilder::from_state(battle(29, [(100, 0); 4], [(100, 0); 4]))
        .expect("fixture should be stable");
    builder
        .edit_pokemon(PokemonRef::new(SideId::P1, index(0)), |pokemon| {
            pokemon.can_mega_evolve = true;
        })
        .expect("left lead should exist");
    let state = builder.build().expect("fixture edit should be stable");
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
    let rng_before = state.rng().clone();

    assert!(matches!(
        state.inspect_turn_queue(mega, both_moves(), &mechanics()),
        Err(BattleError::UnsupportedMechanic(_))
    ));
    assert_eq!(state.queue().actions, Vec::new());
    assert_eq!(state.rng(), &rng_before);
}

#[test]
fn inspected_queue_round_trips_without_mutating_authoritative_state() {
    let state = battle(
        31,
        [(140, 0), (120, 0), (10, 0), (20, 0)],
        [(100, 0), (80, 0), (10, 0), (20, 0)],
    );
    let queue = state
        .inspect_turn_queue(both_moves(), both_moves(), &mechanics())
        .expect("legal choices should queue");

    let json = serde_json::to_string(&queue).expect("queue should serialize");
    let decoded: rotom_core::ActionQueue =
        serde_json::from_str(&json).expect("queue should deserialize");
    assert_eq!(decoded, queue);
    assert_eq!(state.queue().actions, Vec::new());
}
