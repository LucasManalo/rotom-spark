use rotom_core::{
    AbilityId, BattleState, BattleStats, MoveId, MoveSlot, PokemonIndex, PokemonPosition,
    PokemonState, SideId, SideState, SpeciesId, StatStages, StatusState, TeamState,
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
            pp: 8,
            max_pp: 8,
            disabled: false,
        }],
        status: None,
        status_state: StatusState::default(),
        stat_stages: StatStages::default(),
        volatiles: Vec::new(),
        position: PokemonPosition::Team,
    }
}

fn battle() -> BattleState {
    BattleState::new(
        SideState::new(TeamState::new(vec![pokemon(11, 120)])),
        SideState::new(TeamState::new(vec![pokemon(12, 130)])),
    )
}

#[test]
fn snapshot_is_normalized_and_serializable() {
    let state = battle();
    let snapshot = state.snapshot();

    assert_eq!(snapshot.sides[0].side, SideId::P1);
    assert_eq!(snapshot.sides[1].side, SideId::P2);
    assert_eq!(snapshot.sides[0].team[0].species, 11);
    assert_eq!(snapshot.sides[0].team[0].current_hp, 120);
    assert_eq!(snapshot.sides[0].team[0].stats.max_hp, 120);

    let json = serde_json::to_string_pretty(&snapshot).expect("snapshot should serialize");
    let decoded: BattleSnapshot =
        serde_json::from_str(&json).expect("snapshot should deserialize");
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
