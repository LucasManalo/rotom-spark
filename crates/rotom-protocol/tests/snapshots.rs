use rotom_core::{
    AbilityId, BattlePhase, BattleState, BattleStats, MoveId, MoveSlot, MoveTarget, PokemonIndex,
    PokemonPosition, PokemonState, RngReason, SideId, SideState, SpeciesId, StatStages,
    StatusState, TeamState,
};
use rotom_protocol::{BattleSnapshot, BattleStateSnapshot};

fn pokemon(species: u32, hp: u16) -> PokemonState {
    PokemonState {
        species: SpeciesId::new(species),
        form: None,
        level: 50,
        stats: BattleStats {
            hp,
            atk: 90,
            def: 91,
            spa: 92,
            spd: 93,
            spe: 94,
        },
        current_hp: hp,
        ability: AbilityId::new(2),
        item: None,
        moves: vec![MoveSlot {
            move_id: MoveId::new(3),
            target: MoveTarget::Normal,
            pp: 8,
            max_pp: 8,
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

fn battle() -> BattleState {
    let p1 = (11..=14).map(|species| pokemon(species, 120)).collect();
    let p2 = (21..=24).map(|species| pokemon(species, 130)).collect();
    BattleState::new(
        SideState::new(TeamState::new(p1)),
        SideState::new(TeamState::new(p2)),
        4,
    )
    .expect("fixture is a valid preview state")
}

#[test]
fn snapshot_is_normalized_and_serializable() {
    let state = battle();
    let snapshot = state.snapshot();

    assert_eq!(snapshot.picked_team_size, 4);
    assert_eq!(snapshot.sides[0].side, SideId::P1);
    assert_eq!(snapshot.sides[1].side, SideId::P2);
    assert_eq!(snapshot.sides[0].team[0].species, 11);
    assert_eq!(snapshot.sides[0].team[0].current_hp, 120);
    assert_eq!(snapshot.sides[0].team[0].stats.max_hp, 120);

    let json = serde_json::to_string_pretty(&snapshot).expect("snapshot should serialize");
    let decoded: BattleSnapshot = serde_json::from_str(&json).expect("snapshot should deserialize");
    assert_eq!(decoded, snapshot);
}

#[test]
fn snapshot_does_not_alias_mutable_battle_state() {
    let mut state = battle();
    let snapshot = state.snapshot();

    state
        .side_mut(SideId::P1)
        .pokemon_mut(PokemonIndex::new(0))
        .expect("fixture has a first Pokémon")
        .current_hp = 1;

    assert_eq!(snapshot.sides[0].team[0].current_hp, 120);
}

#[test]
fn snapshot_captures_replay_relevant_rng_state() {
    let mut state = battle();
    state
        .rng
        .range(RngReason::SpeedTie, 0..2)
        .expect("speed tie range should be valid");

    let snapshot = state.snapshot();
    assert_eq!(snapshot.rng.seed, 0);
    assert_eq!(snapshot.rng.state, state.rng.state());
    assert_eq!(snapshot.rng.draws, 1);
}

#[test]
fn snapshot_exposes_preview_order_and_initial_leads() {
    let mut state = battle();
    let p1 = vec![
        PokemonIndex::new(3),
        PokemonIndex::new(1),
        PokemonIndex::new(0),
        PokemonIndex::new(2),
    ];
    let p2 = vec![
        PokemonIndex::new(2),
        PokemonIndex::new(0),
        PokemonIndex::new(3),
        PokemonIndex::new(1),
    ];
    state
        .select_team(SideId::P1, p1)
        .expect("P1 preview choice should be valid");
    state
        .select_team(SideId::P2, p2)
        .expect("P2 preview choice should be valid");

    let snapshot = state.snapshot();
    assert_eq!(snapshot.turn, 1);
    assert_eq!(snapshot.phase, BattlePhase::TurnSelection);
    assert_eq!(snapshot.sides[0].selected_team, vec![3, 1, 0, 2]);
    assert_eq!(snapshot.sides[0].active, [Some(3), Some(1)]);
    assert_eq!(snapshot.sides[1].active, [Some(2), Some(0)]);
}
