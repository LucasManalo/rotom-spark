use rotom_core::{
    AbilityId, ActiveSlot, BattleError, BattlePhase, BattleState, BattleStats, ConditionId,
    ConditionState, FaintTarget, MajorStatus, MoveId, MoveSlot, MoveTarget, PokemonIndex,
    PokemonPosition, PokemonState, SideId, SideState, SpeciesId, StatStages, StatusState,
    TeamState,
};

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

fn select(state: &mut BattleState, picked_team_size: u8) {
    let selection = (0..picked_team_size).map(index).collect::<Vec<_>>();
    state
        .select_team(SideId::P1, selection.clone())
        .expect("P1 selection should be legal");
    state
        .select_team(SideId::P2, selection)
        .expect("P2 selection should be legal");
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
    let mut state = battle(4);
    state
        .select_team(SideId::P1, vec![index(5), index(2), index(4), index(0)])
        .expect("P1 selection should be legal");

    assert_eq!(state.phase, BattlePhase::TeamPreview);
    assert_eq!(state.side(SideId::P1).active, [None, None]);

    state
        .select_team(SideId::P2, vec![index(1), index(3), index(0), index(5)])
        .expect("P2 selection should be legal");

    assert_eq!(state.phase, BattlePhase::TurnSelection);
    assert_eq!(state.turn, 1);
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
        let mut state = battle(4);
        assert!(matches!(
            state.select_team(SideId::P1, choice),
            Err(BattleError::IllegalChoice(_))
        ));
        assert_eq!(state.side(SideId::P1).selected_team, Vec::new());
        assert_eq!(state.phase, BattlePhase::TeamPreview);
    }
}

#[test]
fn voluntary_switch_updates_positions_and_clears_switch_out_state() {
    let mut state = battle(4);
    select(&mut state, 4);
    let outgoing = state
        .side_mut(SideId::P1)
        .pokemon_mut(index(0))
        .expect("fixture has its first Pokémon");
    outgoing.stat_stages.atk = 3;
    outgoing
        .volatiles
        .push(ConditionState::new(ConditionId::new(7), Some(2)));
    outgoing.moves[0].disabled = true;

    state
        .switch_active(SideId::P1, ActiveSlot::Left, index(2))
        .expect("selected bench Pokémon should switch in");

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

    assert!(matches!(
        state.switch_active(SideId::P1, ActiveSlot::Right, index(5)),
        Err(BattleError::IllegalChoice(_))
    ));
}

#[test]
fn fainting_an_active_requests_and_accepts_a_forced_replacement() {
    let mut state = battle(4);
    select(&mut state, 4);
    let doomed = state
        .side_mut(SideId::P1)
        .pokemon_mut(index(0))
        .expect("fixture has its first Pokémon");
    doomed.status = Some(MajorStatus::BadlyPoisoned);
    doomed.status_state.toxic_counter = 2;

    state
        .process_faints(&[FaintTarget::new(SideId::P1, ActiveSlot::Left)])
        .expect("active Pokémon should faint");

    assert_eq!(state.phase, BattlePhase::ForcedSwitch);
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

    state
        .replace_fainted(SideId::P1, ActiveSlot::Left, index(3))
        .expect("available bench Pokémon should replace the fainted lead");
    assert_eq!(state.phase, BattlePhase::TurnSelection);
    assert_eq!(state.side(SideId::P1).active[0], Some(index(3)));
}

#[test]
fn one_reserve_for_two_empty_slots_requires_only_one_chosen_replacement() {
    let mut state = battle(3);
    select(&mut state, 3);
    state
        .process_faints(&[
            FaintTarget::new(SideId::P1, ActiveSlot::Left),
            FaintTarget::new(SideId::P1, ActiveSlot::Right),
        ])
        .expect("both active Pokémon should faint");

    let request = state
        .replacement_request(SideId::P1)
        .expect("state should be valid")
        .expect("P1 should need one replacement");
    assert_eq!(request.slots, vec![ActiveSlot::Left, ActiveSlot::Right]);
    assert_eq!(request.available, vec![index(2)]);
    assert_eq!(request.required, 1);

    state
        .replace_fainted(SideId::P1, ActiveSlot::Right, index(2))
        .expect("the player may choose which empty slot to fill");
    assert_eq!(state.phase, BattlePhase::TurnSelection);
    assert_eq!(state.side(SideId::P1).active, [None, Some(index(2))]);
}

#[test]
fn eliminating_one_selected_team_ends_the_battle() {
    let mut state = battle(2);
    select(&mut state, 2);
    state
        .process_faints(&[
            FaintTarget::new(SideId::P1, ActiveSlot::Left),
            FaintTarget::new(SideId::P1, ActiveSlot::Right),
        ])
        .expect("both active Pokémon should faint");

    assert_eq!(state.phase, BattlePhase::BattleEnd);
    assert_eq!(state.winner, Some(SideId::P2));
    assert!(!state.is_tie());
    assert!(matches!(
        state.switch_active(SideId::P2, ActiveSlot::Left, index(1)),
        Err(BattleError::IllegalChoice(_))
    ));
}

#[test]
fn simultaneous_total_knockout_uses_showdown_faint_queue_order() {
    let mut state = battle(2);
    select(&mut state, 2);
    state
        .process_faints(&[
            FaintTarget::new(SideId::P1, ActiveSlot::Left),
            FaintTarget::new(SideId::P2, ActiveSlot::Left),
            FaintTarget::new(SideId::P1, ActiveSlot::Right),
            FaintTarget::new(SideId::P2, ActiveSlot::Right),
        ])
        .expect("ordered faint batch should resolve");

    assert_eq!(state.phase, BattlePhase::BattleEnd);
    assert_eq!(state.winner, Some(SideId::P2));
}

#[test]
fn invalid_faint_batch_is_rejected_before_any_state_changes() {
    let mut state = battle(4);
    select(&mut state, 4);
    let target = FaintTarget::new(SideId::P1, ActiveSlot::Left);

    assert!(matches!(
        state.process_faints(&[target, target]),
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
