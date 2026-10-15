use rotom_core::{CombatData, MoveCapabilities};
use rotom_dex::{champions_reg_mc, Accuracy};
use serde::Deserialize;
use std::collections::HashSet;

const SHOWDOWN_COMMIT: &str = "b79ce06d06b8f0dc8de8bb348bc9cab5d931553f";

#[derive(Deserialize)]
struct MechanicsManifest {
    entries: Vec<MechanicsEntry>,
}

#[derive(Deserialize)]
struct MechanicsEntry {
    id: String,
    kind: String,
    implementation: String,
}

#[test]
fn generated_data_has_expected_source_and_format() {
    let data = champions_reg_mc().expect("checked-in data should match the Rust schema");

    assert_eq!(data.schema_version, 4);
    assert_eq!(data.source.commit, SHOWDOWN_COMMIT);
    assert_eq!(data.format.id, "gen9championsvgc2026regmc");
    assert_eq!(data.format.mod_name, "champions");
    assert_eq!(data.format.game_type, "doubles");
    assert_eq!(data.format.min_team_size, 6);
    assert_eq!(data.format.max_team_size, 6);
    assert_eq!(data.format.picked_team_size, Some(4));
    assert_eq!(data.format.adjust_level, Some(50));
    assert_eq!(data.format.ev_limit, Some(66));
    assert_eq!(data.species.len(), 1_518);
    assert_eq!(data.forms.len(), 397);
    assert_eq!(data.types.len(), 19);
    assert_eq!(data.moves.len(), 954);
    assert_eq!(data.items.len(), 583);
    assert_eq!(data.abilities.len(), 321);
    assert_eq!(data.natures.len(), 25);
}

#[test]
fn generated_collections_are_sorted_and_unique() {
    let data = champions_reg_mc().expect("checked-in data should match the Rust schema");

    for ids in [
        data.species
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        data.forms
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        data.types
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        data.moves
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        data.items
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        data.abilities
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        data.natures
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
    ] {
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(ids.len(), ids.iter().copied().collect::<HashSet<_>>().len());
    }
}

#[test]
fn data_contains_resolved_champions_overrides() {
    let data = champions_reg_mc().expect("checked-in data should match the Rust schema");
    let anchor_shot = data
        .moves
        .iter()
        .find(|entry| entry.id == "anchorshot")
        .expect("Anchor Shot should exist in the resolved Champions Dex");

    assert_eq!(anchor_shot.base_power, 90);
    assert_eq!(anchor_shot.accuracy, Accuracy::Percent { value: 100 });
    assert!(data.moves.iter().all(|entry| entry.pp <= 20));
    assert!(data.forms.iter().any(|entry| entry.id == "charizardmegax"));
    assert!(data
        .species
        .iter()
        .find(|entry| entry.id == "charizard")
        .is_some_and(|entry| entry.move_pool.iter().any(|move_id| move_id == "protect")));
}

#[test]
fn format_legality_is_exported() {
    let data = champions_reg_mc().expect("checked-in data should match the Rust schema");
    let charizard = data
        .species
        .iter()
        .find(|entry| entry.id == "charizard")
        .expect("Charizard should exist");
    let mew = data
        .species
        .iter()
        .find(|entry| entry.id == "mew")
        .expect("Mew should exist");

    assert!(charizard.legal);
    assert!(!mew.legal);
    assert!(mew
        .ban_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("Mythical")));
}

#[test]
fn generated_data_resolves_core_field_effect_identities() {
    let data = champions_reg_mc().expect("checked-in data should match the Rust schema");
    let ids = data.field_effect_ids();

    assert!(ids.rain.is_some());
    assert!(ids.snow.is_some());
    assert!(ids.electric_terrain.is_some());
    assert!(ids.tailwind.is_some());
    assert!(ids.trick_room.is_some());
    assert!(ids.gravity.is_some());
    assert!(ids.magic_room.is_some());
    assert!(ids.wonder_room.is_some());
    assert!(ids.levitate.is_some());
    assert!(ids.iron_ball.is_some());
    assert!(ids.air_balloon.is_some());
}

#[test]
fn generated_move_coverage_matches_runtime_capability_preflight() {
    let data = champions_reg_mc().expect("checked-in data should match the Rust schema");
    let manifest: MechanicsManifest = serde_json::from_str(include_str!(
        "../../../data/generated/reg-mc-mechanics-manifest.json"
    ))
    .expect("mechanics manifest should deserialize");

    for entry in manifest.entries.iter().filter(|entry| entry.kind == "move") {
        let move_id = data
            .resolve_move(&entry.id)
            .expect("reachable move in Dex")
            .0;
        let definition = <rotom_dex::DexData as CombatData>::move_data(&data, move_id)
            .expect("reachable move has combat data");
        let runtime_supported = definition
            .required_capabilities
            .first_missing_from(MoveCapabilities::PR_13_2)
            .is_none();
        assert_eq!(
            entry.implementation == "implemented",
            runtime_supported,
            "{} coverage/runtime support drift",
            entry.id
        );
    }
}
