use rotom_core::{
    AbilityId, BattlePhase, BattleState, BattleStats, MoveId, MoveSlot, MoveTarget, PokemonIndex,
    PokemonPosition, PokemonState, SideId, SideState, SpeciesId, StatStages, StatusState,
    TeamState,
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

fn battle() -> BattleState {
    let p1 = (1..=4).map(|species| pokemon(species, 150)).collect();
    let p2 = (5..=8).map(|species| pokemon(species, 175)).collect();
    BattleState::new(
        SideState::new(TeamState::new(p1)),
        SideState::new(TeamState::new(p2)),
        4,
    )
    .expect("fixture is a valid preview state")
}

#[test]
fn battle_state_round_trips_through_json() {
    let state = battle();
    let json = serde_json::to_string_pretty(&state).expect("battle state should serialize");
    let decoded: BattleState =
        serde_json::from_str(&json).expect("battle state should deserialize");

    assert_eq!(decoded, state);
    assert_eq!(decoded.phase(), BattlePhase::TeamPreview);
    assert_eq!(decoded.turn(), 0);
    assert_eq!(decoded.winner(), None);
}

#[test]
fn cloned_state_has_independent_mutable_storage() {
    let original = battle();
    let mut builder = rotom_core::BattleStateBuilder::from_state(original.clone())
        .expect("fixture should be stable");
    builder
        .edit_pokemon(
            rotom_core::PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
            |cloned_pokemon| {
                cloned_pokemon.current_hp = 1;
                cloned_pokemon.moves[0].pp = 0;
            },
        )
        .expect("fixture has a first Pokémon");
    let cloned = builder.build().expect("edited clone should be stable");
    let cloned_pokemon = cloned
        .side(SideId::P1)
        .pokemon(PokemonIndex::new(0))
        .expect("fixture has a first Pokémon");
    assert_eq!(cloned_pokemon.current_hp, 1);
    assert_eq!(cloned_pokemon.moves[0].pp, 0);

    let original_pokemon = original
        .side(SideId::P1)
        .pokemon(PokemonIndex::new(0))
        .expect("fixture has a first Pokémon");
    assert_eq!(original_pokemon.current_hp, 150);
    assert_eq!(original_pokemon.moves[0].pp, 16);
}

#[test]
fn scenario_builder_rejects_invalid_edits_without_normalizing_them() {
    let original = battle();
    let mut builder = rotom_core::BattleStateBuilder::from_state(original.clone())
        .expect("fixture should be stable");
    builder
        .edit_pokemon(
            rotom_core::PokemonRef::new(SideId::P1, PokemonIndex::new(0)),
            |pokemon| pokemon.current_hp = 0,
        )
        .expect("fixture has a first Pokémon");

    assert!(matches!(
        builder.build(),
        Err(rotom_core::BattleError::InvalidState(_))
    ));
    assert_eq!(
        original
            .side(SideId::P1)
            .pokemon(PokemonIndex::new(0))
            .expect("fixture has a first Pokémon")
            .current_hp,
        150
    );
}

#[test]
fn deliberately_invalid_states_use_an_explicit_serialized_fixture_path() {
    let mut serialized = serde_json::to_value(battle()).expect("fixture should serialize");
    serialized["sides"][0]["active"][0] = serde_json::json!(99);
    let invalid: BattleState = serde_json::from_value(serialized)
        .expect("Serde preserves the deliberately invalid representation");

    assert!(matches!(
        invalid.validate_stable_state(),
        Err(rotom_core::BattleError::InvalidState(_))
    ));
}
