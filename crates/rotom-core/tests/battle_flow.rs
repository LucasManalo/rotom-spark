use rotom_core::{
    step, AbilityId, ActiveSlot, BattleError, BattlePhase, BattleState, BattleStateBuilder,
    BattleStats, CombatData, CombatMove, ConditionId, ConditionState, EffectRegistry, FaintTarget,
    FormId, JointAction, MajorStatus, Mechanics, MoveId, MoveSlot, MoveTarget, PokemonIndex,
    PokemonPosition, PokemonRef, PokemonState, PokemonType, PokemonTypes, SideId, SideState,
    SpeciesId, StatStages, StatusState, TeamSelection, TeamState,
};

struct NoCombatData;

impl CombatData for NoCombatData {
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

fn index(value: u8) -> PokemonIndex {
    PokemonIndex::new(value)
}

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
        last_consumed_berry: None,
        cud_chew_berry: None,
        cud_chew_turns: 0,
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

fn battle(picked_team_size: usize) -> BattleState {
    let p1 = (1..=6).map(pokemon).collect();
    let p2 = (101..=106).map(pokemon).collect();
    BattleState::new(
        SideState::new(TeamState::new(p1)),
        SideState::new(TeamState::new(p2)),
        picked_team_size,
    )
    .expect("fixture is a valid preview state")
}

fn select(state: BattleState, picked_team_size: u8) -> BattleState {
    let selection = (0..picked_team_size).map(index).collect::<Vec<_>>();
    let mut builder = BattleStateBuilder::from_state(state).expect("fixture should be stable");
    builder
        .select_teams(selection.clone(), selection)
        .expect("selections should be legal");
    builder.build().expect("selected fixture should be stable")
}

#[test]
fn battle_creation_rejects_invalid_doubles_pick_sizes() {
    let p1 = SideState::new(TeamState::new((1..=6).map(pokemon).collect()));
    let p2 = SideState::new(TeamState::new((101..=106).map(pokemon).collect()));

    assert!(matches!(
        BattleState::new(p1.clone(), p2.clone(), 1),
        Err(BattleError::InvalidState(_))
    ));
    assert!(matches!(
        BattleState::new(p1, p2, 7),
        Err(BattleError::InvalidState(_))
    ));
}

#[test]
fn ordered_pick_four_establishes_two_leads_after_both_sides_submit() {
    let initial = battle(4);
    let data = NoCombatData;
    let result = step(
        &initial,
        &JointAction::TeamPreview {
            p1: TeamSelection::new(vec![index(5), index(2), index(4), index(0)]),
            p2: TeamSelection::new(vec![index(1), index(3), index(0), index(5)]),
        },
        &Mechanics::new(&data, &EffectRegistry::new()),
    )
    .expect("preview should be legal");
    let state = result.state;

    assert_eq!(state.phase(), BattlePhase::TurnSelection);
    assert_eq!(state.turn(), 1);
    assert_eq!(
        state.side(SideId::P1).active,
        [Some(index(5)), Some(index(2))]
    );
    assert_eq!(
        state.side(SideId::P2).active,
        [Some(index(1)), Some(index(3))]
    );
    assert_eq!(
        state.side(SideId::P1).pokemon(index(5)).map(|p| p.position),
        Some(PokemonPosition::Active {
            slot: ActiveSlot::Left
        })
    );
    assert_eq!(
        state.side(SideId::P1).pokemon(index(4)).map(|p| p.position),
        Some(PokemonPosition::Team)
    );
}

#[test]
fn team_preview_rejects_wrong_size_duplicates_and_unknown_indexes_atomically() {
    let invalid_choices = [
        vec![index(0), index(1), index(2)],
        vec![index(0), index(1), index(1), index(2)],
        vec![index(0), index(1), index(2), index(9)],
    ];

    for choice in invalid_choices {
        let state = battle(4);
        let data = NoCombatData;
        assert!(matches!(
            step(
                &state,
                &JointAction::TeamPreview {
                    p1: TeamSelection::new(choice),
                    p2: TeamSelection::new(vec![index(0), index(1), index(2), index(3)]),
                },
                &Mechanics::new(&data, &EffectRegistry::new()),
            ),
            Err(BattleError::IllegalChoice(_))
        ));
        assert_eq!(state.side(SideId::P1).selected_team, Vec::new());
        assert_eq!(state.phase(), BattlePhase::TeamPreview);
    }
}

#[test]
fn voluntary_switch_updates_positions_and_clears_switch_out_state() {
    let state = select(battle(4), 4);
    let mut builder = BattleStateBuilder::from_state(state).expect("fixture should be stable");
    builder
        .edit_pokemon(PokemonRef::new(SideId::P1, index(0)), |outgoing| {
            outgoing.stat_stages.atk = 3;
            outgoing
                .volatiles
                .push(ConditionState::new(ConditionId::new(7), Some(2)));
            outgoing.moves[0].disabled = true;
        })
        .expect("fixture has its first Pokémon");
    builder
        .switch_active(SideId::P1, ActiveSlot::Left, index(2))
        .expect("selected bench Pokémon should switch in");
    let state = builder.build().expect("switched fixture should be stable");

    assert_eq!(state.side(SideId::P1).active[0], Some(index(2)));
    let outgoing = state
        .side(SideId::P1)
        .pokemon(index(0))
        .expect("fixture has its first Pokémon");
    assert_eq!(outgoing.position, PokemonPosition::Team);
    assert_eq!(outgoing.stat_stages, StatStages::default());
    assert_eq!(outgoing.volatiles, Vec::new());
    assert!(!outgoing.moves[0].disabled);
    assert_eq!(
        state.side(SideId::P1).pokemon(index(2)).map(|p| p.position),
        Some(PokemonPosition::Active {
            slot: ActiveSlot::Left
        })
    );

    let mut builder = BattleStateBuilder::from_state(state).expect("fixture should be stable");
    assert!(matches!(
        builder.switch_active(SideId::P1, ActiveSlot::Right, index(5)),
        Err(BattleError::IllegalChoice(_))
    ));
}

#[test]
fn fainting_an_active_requests_and_accepts_a_forced_replacement() {
    let state = select(battle(4), 4);
    let mut builder = BattleStateBuilder::from_state(state).expect("fixture should be stable");
    builder
        .set_major_status(
            PokemonRef::new(SideId::P1, index(0)),
            MajorStatus::BadlyPoisoned,
            StatusState {
                toxic_counter: 2,
                ..StatusState::default()
            },
        )
        .expect("fixture status should be coherent");
    builder
        .process_faints(&[FaintTarget::new(SideId::P1, ActiveSlot::Left)])
        .expect("active Pokémon should faint");
    let state = builder.build().expect("fainted fixture should be stable");

    assert_eq!(state.phase(), BattlePhase::ForcedSwitch);
    assert_eq!(state.side(SideId::P1).active[0], None);
    let fainted = state
        .side(SideId::P1)
        .pokemon(index(0))
        .expect("fixture has its first Pokémon");
    assert_eq!(fainted.current_hp, 0);
    assert!(fainted.is_fainted());
    assert_eq!(fainted.status, None);
    assert_eq!(fainted.status_state, StatusState::default());

    let request = state
        .replacement_request(SideId::P1)
        .expect("state should be valid")
        .expect("P1 should need a replacement");
    assert_eq!(request.slots, vec![ActiveSlot::Left]);
    assert_eq!(request.available, vec![index(2), index(3)]);
    assert_eq!(request.required, 1);
    assert_eq!(
        state
            .replacement_request(SideId::P2)
            .expect("state should be valid"),
        None
    );

    let mut builder = BattleStateBuilder::from_state(state).expect("fixture should be stable");
    builder
        .replace_fainted(SideId::P1, ActiveSlot::Left, index(3))
        .expect("available bench Pokémon should replace the fainted lead");
    let state = builder.build().expect("replacement should be stable");
    assert_eq!(state.phase(), BattlePhase::TurnSelection);
    assert_eq!(state.side(SideId::P1).active[0], Some(index(3)));
}

#[test]
fn one_reserve_for_two_empty_slots_requires_only_one_chosen_replacement() {
    let state = select(battle(3), 3);
    let mut builder = BattleStateBuilder::from_state(state).expect("fixture should be stable");
    builder
        .process_faints(&[
            FaintTarget::new(SideId::P1, ActiveSlot::Left),
            FaintTarget::new(SideId::P1, ActiveSlot::Right),
        ])
        .expect("both active Pokémon should faint");
    let state = builder.build().expect("fainted fixture should be stable");

    let request = state
        .replacement_request(SideId::P1)
        .expect("state should be valid")
        .expect("P1 should need one replacement");
    assert_eq!(request.slots, vec![ActiveSlot::Left, ActiveSlot::Right]);
    assert_eq!(request.available, vec![index(2)]);
    assert_eq!(request.required, 1);

    let mut builder = BattleStateBuilder::from_state(state).expect("fixture should be stable");
    builder
        .replace_fainted(SideId::P1, ActiveSlot::Right, index(2))
        .expect("the player may choose which empty slot to fill");
    let state = builder.build().expect("replacement should be stable");
    assert_eq!(state.phase(), BattlePhase::TurnSelection);
    assert_eq!(state.side(SideId::P1).active, [None, Some(index(2))]);
}

#[test]
fn eliminating_one_selected_team_ends_the_battle() {
    let state = select(battle(2), 2);
    let mut builder = BattleStateBuilder::from_state(state).expect("fixture should be stable");
    builder
        .process_faints(&[
            FaintTarget::new(SideId::P1, ActiveSlot::Left),
            FaintTarget::new(SideId::P1, ActiveSlot::Right),
        ])
        .expect("both active Pokémon should faint");
    let state = builder.build().expect("terminal fixture should be stable");

    assert_eq!(state.phase(), BattlePhase::BattleEnd);
    assert_eq!(state.winner(), Some(SideId::P2));
    assert!(!state.is_tie());
    assert!(matches!(
        BattleStateBuilder::from_state(state)
            .expect("terminal fixture should be stable")
            .switch_active(SideId::P2, ActiveSlot::Left, index(1)),
        Err(BattleError::IllegalChoice(_))
    ));
}

#[test]
fn simultaneous_total_knockout_uses_showdown_faint_queue_order() {
    let state = select(battle(2), 2);
    let mut builder = BattleStateBuilder::from_state(state).expect("fixture should be stable");
    builder
        .process_faints(&[
            FaintTarget::new(SideId::P1, ActiveSlot::Left),
            FaintTarget::new(SideId::P2, ActiveSlot::Left),
            FaintTarget::new(SideId::P1, ActiveSlot::Right),
            FaintTarget::new(SideId::P2, ActiveSlot::Right),
        ])
        .expect("ordered faint batch should resolve");
    let state = builder.build().expect("terminal fixture should be stable");

    assert_eq!(state.phase(), BattlePhase::BattleEnd);
    assert_eq!(state.winner(), Some(SideId::P2));
}

#[test]
fn invalid_faint_batch_is_rejected_before_any_state_changes() {
    let state = select(battle(4), 4);
    let mut builder =
        BattleStateBuilder::from_state(state.clone()).expect("fixture should be stable");
    let target = FaintTarget::new(SideId::P1, ActiveSlot::Left);

    assert!(matches!(
        builder.process_faints(&[target, target]),
        Err(BattleError::IllegalChoice(_))
    ));
    assert_eq!(state.side(SideId::P1).active[0], Some(index(0)));
    assert_eq!(
        state
            .side(SideId::P1)
            .pokemon(index(0))
            .map(|pokemon| pokemon.current_hp),
        Some(100)
    );
}
