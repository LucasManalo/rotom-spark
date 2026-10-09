use rotom_core::{
    AbilityId, BattlePhase, BattleState, BattleStats, MoveId, MoveSlot, PokemonIndex,
    PokemonPosition, PokemonState, SideId, SideState, SpeciesId, StatStages, StatusState, TeamState,
};

fn pokemon(species: u32, hp: u16) -> PokemonState {
    PokemonState {
        species: SpeciesId::new(species),
        form: None,
        level: 50,
        stats: BattleStats {
            hp,
            atk: 100,
            def: 100,
            spa: 100,
            spd: 100,
            spe: 100,
        },
        current_hp: hp,
        ability: AbilityId::new(1),
        item: None,
        moves: vec![MoveSlot {
            move_id: MoveId::new(1),
            pp: 16,
            max_pp: 16,
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
        SideState::new(TeamState::new(vec![pokemon(1, 150)])),
        SideState::new(TeamState::new(vec![pokemon(2, 175)])),
    )
}

#[test]
fn battle_state_round_trips_through_json() {
    let state = battle();
    let json = serde_json::to_string_pretty(&state).expect("battle state should serialize");
    let decoded: BattleState =
        serde_json::from_str(&json).expect("battle state should deserialize");

    assert_eq!(decoded, state);
    assert_eq!(decoded.phase, BattlePhase::TeamPreview);
    assert_eq!(decoded.turn, 0);
    assert_eq!(decoded.winner, None);
}

#[test]
fn cloned_state_has_independent_mutable_storage() {
    let original = battle();
    let mut cloned = original.clone();
    let cloned_pokemon = cloned
        .side_mut(SideId::P1)
        .pokemon_mut(PokemonIndex::new(0))
        .expect("fixture has a first Pokémon");
    cloned_pokemon.current_hp = 1;
    cloned_pokemon.moves[0].pp = 0;

    let original_pokemon = original
        .side(SideId::P1)
        .pokemon(PokemonIndex::new(0))
        .expect("fixture has a first Pokémon");
    assert_eq!(original_pokemon.current_hp, 150);
    assert_eq!(original_pokemon.moves[0].pp, 16);
}
