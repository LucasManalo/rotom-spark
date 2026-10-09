use crate::{parser::to_id, ValidationError, ValidationProblem};
use rotom_core::{Gender, PokemonSet, TeamDefinition};
use rotom_dex::{DexData, SpeciesData};
use std::collections::{HashMap, HashSet};

pub(crate) fn validate_reg_mc_team(
    team: &TeamDefinition,
    dex: &DexData,
) -> Result<(), ValidationError> {
    let mut problems = Vec::new();
    if team.members.len() != usize::from(dex.format.max_team_size) {
        problems.push(ValidationProblem::TeamSize {
            expected: usize::from(dex.format.max_team_size),
            actual: team.members.len(),
        });
    }

    let mut seen_species = HashMap::new();
    let mut seen_items = HashMap::new();
    for (index, set) in team.members.iter().enumerate() {
        let slot = index + 1;
        let Some(base) = dex.species(set.species) else {
            problems.push(ValidationProblem::UnknownReference {
                slot,
                kind: "species",
            });
            continue;
        };
        let selected = selected_species(set, base, dex, slot, &mut problems);
        validate_species_and_form(set, base, selected, dex, slot, &mut problems);
        validate_set_fields(set, base, selected, dex, slot, &mut problems);

        let species_key = to_id(&base.base_species);
        if let Some(first_slot) = seen_species.insert(species_key, slot) {
            problems.push(ValidationProblem::DuplicateSpecies {
                first_slot,
                second_slot: slot,
            });
        }
        if let Some(item) = set.item {
            if let Some(first_slot) = seen_items.insert(item, slot) {
                problems.push(ValidationProblem::DuplicateItem {
                    first_slot,
                    second_slot: slot,
                });
            }
        }
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(ValidationError { problems })
    }
}

fn selected_species<'a>(
    set: &PokemonSet,
    base: &'a SpeciesData,
    dex: &'a DexData,
    slot: usize,
    problems: &mut Vec<ValidationProblem>,
) -> Option<&'a SpeciesData> {
    let Some(form_id) = set.form else {
        return Some(base);
    };
    let Some(form) = dex.form(form_id) else {
        problems.push(ValidationProblem::UnknownReference { slot, kind: "form" });
        return None;
    };
    if to_id(&form.base_species) != base.id {
        problems.push(ValidationProblem::FormSpeciesMismatch { slot });
    }
    let Some((_, species)) = dex.resolve_species(&form.id) else {
        problems.push(ValidationProblem::UnknownReference {
            slot,
            kind: "form species",
        });
        return None;
    };
    Some(species)
}

fn validate_species_and_form(
    set: &PokemonSet,
    base: &SpeciesData,
    selected: Option<&SpeciesData>,
    dex: &DexData,
    slot: usize,
    problems: &mut Vec<ValidationProblem>,
) {
    if !base.legal {
        problems.push(ValidationProblem::IllegalSpecies {
            slot,
            name: base.name.clone(),
        });
    }
    let Some(selected) = selected else {
        return;
    };
    if set.form.is_some() && !selected.legal {
        problems.push(ValidationProblem::IllegalForm {
            slot,
            name: selected.name.clone(),
        });
    }
    if let Some(required_item) = &selected.required_item {
        let held_name = set
            .item
            .and_then(|id| dex.item(id))
            .map(|item| item.name.as_str());
        if held_name != Some(required_item.as_str()) {
            problems.push(ValidationProblem::RequiredItem {
                slot,
                name: required_item.clone(),
            });
        }
    }
    if let Some(required_move) = &selected.required_move {
        let required_id = to_id(required_move);
        if !set
            .moves
            .iter()
            .filter_map(|id| dex.move_data(*id))
            .any(|move_data| move_data.id == required_id)
        {
            problems.push(ValidationProblem::RequiredMove {
                slot,
                name: required_move.clone(),
            });
        }
    }
}

fn validate_set_fields(
    set: &PokemonSet,
    base: &SpeciesData,
    selected: Option<&SpeciesData>,
    dex: &DexData,
    slot: usize,
    problems: &mut Vec<ValidationProblem>,
) {
    let starting_species = selected
        .filter(|species| species.battle_only.is_empty())
        .unwrap_or(base);
    validate_ability(set, starting_species, dex, slot, problems);
    validate_item(set, base, dex, slot, problems);
    validate_nature(set, dex, slot, problems);
    validate_level_and_stats(set, dex, slot, problems);
    validate_moves(set, starting_species, dex, slot, problems);
    validate_gender(set, starting_species, slot, problems);
    if set
        .nickname
        .as_ref()
        .is_some_and(|name| name.chars().count() > 18)
    {
        problems.push(ValidationProblem::NicknameTooLong { slot });
    }
}

fn validate_ability(
    set: &PokemonSet,
    species: &SpeciesData,
    dex: &DexData,
    slot: usize,
    problems: &mut Vec<ValidationProblem>,
) {
    let Some(ability) = dex.ability(set.ability) else {
        problems.push(ValidationProblem::UnknownReference {
            slot,
            kind: "ability",
        });
        return;
    };
    if !ability.legal {
        problems.push(ValidationProblem::IllegalAbility {
            slot,
            name: ability.name.clone(),
        });
    }
    if !species
        .abilities
        .iter()
        .any(|entry| to_id(&entry.value) == ability.id)
    {
        problems.push(ValidationProblem::AbilityNotAvailable {
            slot,
            name: ability.name.clone(),
        });
    }
}

fn validate_item(
    set: &PokemonSet,
    base: &SpeciesData,
    dex: &DexData,
    slot: usize,
    problems: &mut Vec<ValidationProblem>,
) {
    let Some(item_id) = set.item else {
        return;
    };
    let Some(item) = dex.item(item_id) else {
        problems.push(ValidationProblem::UnknownReference { slot, kind: "item" });
        return;
    };
    if !item.legal {
        problems.push(ValidationProblem::IllegalItem {
            slot,
            name: item.name.clone(),
        });
    }
    if !item.mega_stone.is_empty()
        && !item
            .mega_stone
            .iter()
            .any(|mapping| to_id(&mapping.key) == to_id(&base.base_species))
    {
        problems.push(ValidationProblem::WrongMegaStone {
            slot,
            name: item.name.clone(),
        });
    }
}

fn validate_nature(
    set: &PokemonSet,
    dex: &DexData,
    slot: usize,
    problems: &mut Vec<ValidationProblem>,
) {
    let Some(nature) = dex.nature(set.nature) else {
        problems.push(ValidationProblem::UnknownReference {
            slot,
            kind: "nature",
        });
        return;
    };
    if !nature.legal {
        problems.push(ValidationProblem::IllegalNature {
            slot,
            name: nature.name.clone(),
        });
    }
    if set.stat_points.total() == 0 && nature.id == "serious" {
        problems.push(ValidationProblem::AmbiguousZeroStats { slot });
    }
}

fn validate_level_and_stats(
    set: &PokemonSet,
    dex: &DexData,
    slot: usize,
    problems: &mut Vec<ValidationProblem>,
) {
    if u16::from(set.level) < dex.format.min_level || u16::from(set.level) > dex.format.max_level {
        problems.push(ValidationProblem::InvalidLevel {
            slot,
            level: set.level,
        });
    }
    for (stat, value) in set.ivs.named_values() {
        if value != 31 {
            problems.push(ValidationProblem::InvalidIv { slot, stat, value });
        }
    }
    for (stat, value) in set.stat_points.named_values() {
        if value > 32 {
            problems.push(ValidationProblem::StatPointLimit { slot, stat, value });
        }
    }
    let total = set.stat_points.total();
    if total > dex.format.ev_limit.unwrap_or(66) {
        problems.push(ValidationProblem::StatPointTotal { slot, total });
    }
}

fn validate_moves(
    set: &PokemonSet,
    species: &SpeciesData,
    dex: &DexData,
    slot: usize,
    problems: &mut Vec<ValidationProblem>,
) {
    let count = set.moves.len();
    if count == 0 || count > usize::from(dex.format.max_move_count) {
        problems.push(ValidationProblem::InvalidMoveCount { slot, count });
    }
    let mut seen_moves = HashSet::new();
    let mut reported_duplicates = HashSet::new();
    for move_id in &set.moves {
        let Some(move_data) = dex.move_data(*move_id) else {
            problems.push(ValidationProblem::UnknownReference { slot, kind: "move" });
            continue;
        };
        if !seen_moves.insert(*move_id) && reported_duplicates.insert(*move_id) {
            problems.push(ValidationProblem::DuplicateMove {
                slot,
                name: move_data.name.clone(),
            });
        }
        if !move_data.legal {
            problems.push(ValidationProblem::IllegalMove {
                slot,
                name: move_data.name.clone(),
            });
        }
        if !species.move_pool.iter().any(|id| id == &move_data.id) {
            problems.push(ValidationProblem::MoveNotLearnable {
                slot,
                name: move_data.name.clone(),
            });
        } else if species
            .event_only_moves
            .iter()
            .any(|id| id == &move_data.id)
        {
            problems.push(ValidationProblem::UnsupportedEventMove {
                slot,
                name: move_data.name.clone(),
            });
        }
    }
}

fn validate_gender(
    set: &PokemonSet,
    species: &SpeciesData,
    slot: usize,
    problems: &mut Vec<ValidationProblem>,
) {
    let mismatch = matches!(
        (species.gender.as_deref(), set.gender),
        (Some("M"), Some(Gender::Female)) | (Some("F"), Some(Gender::Male)) | (Some("N"), Some(_))
    );
    if mismatch {
        problems.push(ValidationProblem::GenderMismatch { slot });
    }
}
