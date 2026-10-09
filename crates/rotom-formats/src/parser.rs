use crate::TeamParseError;
use rotom_core::{
    AbilityId, FormId, Gender, ItemId, MoveId, NatureId, PokemonSet, SpeciesId, StatAllocation,
    TeamDefinition,
};
use rotom_dex::DexData;

struct SetBuilder {
    start_line: usize,
    nickname: Option<String>,
    species: SpeciesId,
    form: Option<FormId>,
    item: Option<ItemId>,
    gender: Option<Gender>,
    level: u8,
    ability: Option<AbilityId>,
    nature: NatureId,
    stat_points: StatAllocation,
    ivs: StatAllocation,
    moves: Vec<MoveId>,
    shiny: bool,
}

impl SetBuilder {
    fn finish(self) -> Result<PokemonSet, TeamParseError> {
        let ability = self.ability.ok_or_else(|| {
            TeamParseError::new(self.start_line, "each Pokémon must specify an Ability")
        })?;
        Ok(PokemonSet {
            nickname: self.nickname,
            species: self.species,
            form: self.form,
            level: self.level,
            ability,
            item: self.item,
            nature: self.nature,
            stat_points: self.stat_points,
            ivs: self.ivs,
            moves: self.moves,
            gender: self.gender,
            shiny: self.shiny,
        })
    }
}

/// Parses a strict Pokémon Showdown text export into typed IDs.
///
/// Unrecognized fields are rejected instead of being silently discarded.
///
/// # Errors
///
/// Returns the one-based source line and a description when syntax or a name is invalid.
pub fn parse_showdown_team(input: &str, dex: &DexData) -> Result<TeamDefinition, TeamParseError> {
    let serious = dex
        .resolve_nature("serious")
        .map(|(id, _)| id)
        .ok_or_else(|| TeamParseError::new(1, "generated data is missing Serious nature"))?;
    let mut members = Vec::new();
    let mut current: Option<SetBuilder> = None;

    for (index, raw_line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line == "---" {
            if let Some(builder) = current.take() {
                members.push(builder.finish()?);
            }
            continue;
        }
        if line.starts_with("===") && current.is_none() {
            continue;
        }

        if current.is_none() {
            current = Some(parse_header(line, line_number, serious, dex)?);
            continue;
        }

        if let Some(builder) = current.as_mut() {
            parse_set_line(builder, line, line_number, dex)?;
        } else {
            return Err(TeamParseError::new(
                line_number,
                "internal parser state lost the current Pokémon",
            ));
        }
    }
    if let Some(builder) = current {
        members.push(builder.finish()?);
    }
    if members.is_empty() {
        return Err(TeamParseError::new(1, "team export contains no Pokémon"));
    }
    Ok(TeamDefinition { members })
}

fn parse_header(
    line: &str,
    line_number: usize,
    serious: NatureId,
    dex: &DexData,
) -> Result<SetBuilder, TeamParseError> {
    let (identity, item_name) = line
        .split_once(" @ ")
        .map_or((line, None), |(left, right)| (left, Some(right)));
    let (identity, gender) = if let Some(value) = identity.strip_suffix(" (M)") {
        (value, Some(Gender::Male))
    } else if let Some(value) = identity.strip_suffix(" (F)") {
        (value, Some(Gender::Female))
    } else {
        (identity, None)
    };
    let (nickname, species_name) = if let Some((nickname, species)) = identity.rsplit_once(" (") {
        let species = species
            .strip_suffix(')')
            .ok_or_else(|| TeamParseError::new(line_number, "malformed nickname/species header"))?;
        (Some(nickname.to_owned()), species)
    } else {
        (None, identity)
    };
    let species_key = to_id(species_name);
    let (_, species_data) = dex.resolve_species(&species_key).ok_or_else(|| {
        TeamParseError::new(line_number, format!("unknown species `{species_name}`"))
    })?;
    let (species, form) = if species_data.forme.is_some() {
        let form = dex
            .resolve_form(&species_data.id)
            .map(|(id, _)| id)
            .ok_or_else(|| TeamParseError::new(line_number, "generated form data is incomplete"))?;
        let base_key = to_id(&species_data.base_species);
        let base = dex
            .resolve_species(&base_key)
            .map(|(id, _)| id)
            .ok_or_else(|| {
                TeamParseError::new(line_number, "generated base-species data is incomplete")
            })?;
        (base, Some(form))
    } else {
        let base = dex
            .resolve_species(&species_data.id)
            .map(|(id, _)| id)
            .ok_or_else(|| {
                TeamParseError::new(line_number, "generated species data is incomplete")
            })?;
        (base, None)
    };
    let item = match item_name {
        None | Some("No Item") => None,
        Some(name) => Some(
            dex.resolve_item(&to_id(name))
                .map(|(id, _)| id)
                .ok_or_else(|| {
                    TeamParseError::new(line_number, format!("unknown item `{name}`"))
                })?,
        ),
    };

    Ok(SetBuilder {
        start_line: line_number,
        nickname,
        species,
        form,
        item,
        gender,
        level: 100,
        ability: None,
        nature: serious,
        stat_points: StatAllocation::ZERO,
        ivs: StatAllocation::PERFECT_IVS,
        moves: Vec::new(),
        shiny: false,
    })
}

fn parse_set_line(
    builder: &mut SetBuilder,
    line: &str,
    line_number: usize,
    dex: &DexData,
) -> Result<(), TeamParseError> {
    if let Some(name) = line
        .strip_prefix("Ability: ")
        .or_else(|| line.strip_prefix("Trait: "))
    {
        builder.ability = Some(
            dex.resolve_ability(&to_id(name))
                .map(|(id, _)| id)
                .ok_or_else(|| {
                    TeamParseError::new(line_number, format!("unknown ability `{name}`"))
                })?,
        );
    } else if let Some(value) = line.strip_prefix("Level: ") {
        builder.level = value.parse().map_err(|_| {
            TeamParseError::new(line_number, "level must be an integer from 1 to 100")
        })?;
    } else if let Some(value) = line.strip_prefix("Shiny: ") {
        builder.shiny = match value {
            "Yes" => true,
            "No" => false,
            _ => return Err(TeamParseError::new(line_number, "Shiny must be Yes or No")),
        };
    } else if let Some(value) = line.strip_prefix("EVs: ") {
        builder.stat_points = parse_stats(value, StatAllocation::ZERO, line_number)?;
    } else if let Some(value) = line.strip_prefix("IVs: ") {
        builder.ivs = parse_stats(value, StatAllocation::PERFECT_IVS, line_number)?;
    } else if let Some(name) = line.strip_suffix(" Nature") {
        builder.nature = dex
            .resolve_nature(&to_id(name))
            .map(|(id, _)| id)
            .ok_or_else(|| TeamParseError::new(line_number, format!("unknown nature `{name}`")))?;
    } else if let Some(name) = line.strip_prefix("- ").or_else(|| line.strip_prefix("~ ")) {
        let move_name = normalize_move_name(name);
        builder.moves.push(
            dex.resolve_move(&to_id(&move_name))
                .map(|(id, _)| id)
                .ok_or_else(|| {
                    TeamParseError::new(line_number, format!("unknown move `{name}`"))
                })?,
        );
    } else {
        return Err(TeamParseError::new(
            line_number,
            format!("unsupported or unrecognized team field `{line}`"),
        ));
    }
    Ok(())
}

fn parse_stats(
    value: &str,
    mut result: StatAllocation,
    line_number: usize,
) -> Result<StatAllocation, TeamParseError> {
    for component in value.split(" / ") {
        let (number, stat) = component.split_once(' ').ok_or_else(|| {
            TeamParseError::new(
                line_number,
                format!("malformed stat allocation `{component}`"),
            )
        })?;
        let number = number.parse::<u8>().map_err(|_| {
            TeamParseError::new(line_number, format!("invalid stat value `{number}`"))
        })?;
        match stat {
            "HP" => result.hp = number,
            "Atk" => result.atk = number,
            "Def" => result.def = number,
            "SpA" => result.spa = number,
            "SpD" => result.spd = number,
            "Spe" => result.spe = number,
            _ => {
                return Err(TeamParseError::new(
                    line_number,
                    format!("unknown stat `{stat}`"),
                ));
            }
        }
    }
    Ok(result)
}

fn normalize_move_name(name: &str) -> String {
    if let Some(move_type) = name
        .strip_prefix("Hidden Power [")
        .and_then(|value| value.strip_suffix(']'))
    {
        format!("Hidden Power {move_type}")
    } else {
        name.to_owned()
    }
}

pub(crate) fn to_id(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}
