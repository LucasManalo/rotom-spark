use crate::{Accuracy, DexData, MoveData, SpeciesData, Stats};
use rotom_core::{
    BattleError, BattleStats, CombatData, CombatMove, ConditionId, FieldEffectIds, MoveAccuracy,
    MoveCategory, MoveId, MoveSlot, MoveSupport, PokemonPosition, PokemonSet, PokemonState,
    PokemonType, PokemonTypes, StatAllocation, StatStages, StatusState,
};

const BASIC_DAMAGE_MOVES: &[&str] = &[
    "aquajet",
    "aquatail",
    "dragonpulse",
    "highhorsepower",
    "powergem",
    "powerwhip",
    "seedbomb",
];

impl CombatData for DexData {
    fn move_data(&self, move_id: MoveId) -> Option<CombatMove> {
        let data = self.move_data(move_id)?;
        Some(CombatMove {
            move_type: parse_type(&data.move_type)?,
            category: parse_category(&data.category)?,
            base_power: data.base_power,
            accuracy: match data.accuracy {
                Accuracy::Always => MoveAccuracy::Always,
                Accuracy::Percent { value } => MoveAccuracy::Percent { value },
            },
            crit_ratio: data.crit_ratio,
            target: data.target,
            support: if BASIC_DAMAGE_MOVES.contains(&data.id.as_str()) {
                MoveSupport::BasicDamage
            } else {
                MoveSupport::Unsupported
            },
        })
    }

    fn pokemon_types(
        &self,
        species: rotom_core::SpeciesId,
        form: Option<rotom_core::FormId>,
    ) -> Option<PokemonTypes> {
        let species = selected_species(self, species, form)?;
        let mut types = species.types.iter().filter_map(|value| parse_type(value));
        Some(PokemonTypes {
            primary: types.next()?,
            secondary: types.next(),
        })
    }

    fn type_modifier(&self, attacking: PokemonType, defending: PokemonType) -> Option<i8> {
        let defending = self.resolve_type(type_id(defending))?;
        let value = defending
            .damage_taken
            .iter()
            .find(|entry| entry.key == type_id(attacking))
            .map_or(0, |entry| entry.value);
        match value {
            0 => Some(0),
            1 => Some(1),
            2 => Some(-1),
            3..=u8::MAX => None,
        }
    }

    fn field_effect_ids(&self) -> FieldEffectIds {
        let condition = |id: &str| {
            self.resolve_move(id)
                .map(|(move_id, _)| ConditionId::new(move_id.get()))
        };
        FieldEffectIds {
            rain: condition("raindance"),
            sun: condition("sunnyday"),
            sandstorm: condition("sandstorm"),
            snow: condition("snowscape"),
            electric_terrain: condition("electricterrain"),
            grassy_terrain: condition("grassyterrain"),
            psychic_terrain: condition("psychicterrain"),
            misty_terrain: condition("mistyterrain"),
            reflect: condition("reflect"),
            light_screen: condition("lightscreen"),
            aurora_veil: condition("auroraveil"),
            tailwind: condition("tailwind"),
            trick_room: condition("trickroom"),
            gravity: condition("gravity"),
            magic_room: condition("magicroom"),
            wonder_room: condition("wonderroom"),
            ingrain: condition("ingrain"),
            smack_down: condition("smackdown"),
            magnet_rise: condition("magnetrise"),
            telekinesis: condition("telekinesis"),
            levitate: self.resolve_ability("levitate").map(|(id, _)| id),
            iron_ball: self.resolve_item("ironball").map(|(id, _)| id),
            air_balloon: self.resolve_item("airballoon").map(|(id, _)| id),
            earthquake: self.resolve_move("earthquake").map(|(id, _)| id),
            bulldoze: self.resolve_move("bulldoze").map(|(id, _)| id),
            magnitude: self.resolve_move("magnitude").map(|(id, _)| id),
            thunder: self.resolve_move("thunder").map(|(id, _)| id),
            hurricane: self.resolve_move("hurricane").map(|(id, _)| id),
            blizzard: self.resolve_move("blizzard").map(|(id, _)| id),
        }
    }
}

impl DexData {
    /// Builds mutable battle state from a validated Regulation M-C set.
    ///
    /// This applies the pinned Champions stat-point formula and resolved move
    /// PP rules. Team legality remains the format layer's responsibility.
    ///
    /// # Errors
    ///
    /// Returns a data-mismatch error when the set references missing static
    /// data or calculated move PP cannot be represented by runtime state.
    pub fn pokemon_state(&self, set: &PokemonSet) -> Result<PokemonState, BattleError> {
        let species = selected_species(self, set.species, set.form).ok_or_else(|| {
            BattleError::DataMismatch("set species/form is missing from the Dex".to_owned())
        })?;
        let nature = self.nature(set.nature).ok_or_else(|| {
            BattleError::DataMismatch("set nature is missing from the Dex".to_owned())
        })?;
        let stats = champions_stats(
            species.base_stats,
            set.stat_points,
            set.level,
            nature.plus.as_deref(),
            nature.minus.as_deref(),
        );
        let moves = set
            .moves
            .iter()
            .copied()
            .map(|move_id| self.move_slot(move_id))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PokemonState {
            species: set.species,
            form: set.form,
            level: set.level,
            stats,
            current_hp: stats.hp,
            ability: set.ability,
            item: set.item,
            moves,
            status: None,
            status_state: StatusState::default(),
            stat_stages: StatStages::default(),
            volatiles: Vec::new(),
            can_mega_evolve: false,
            trapped: false,
            position: PokemonPosition::Team,
        })
    }

    fn move_slot(&self, move_id: MoveId) -> Result<MoveSlot, BattleError> {
        let data = self.move_data(move_id).ok_or_else(|| {
            BattleError::DataMismatch(format!("move {move_id} is missing from the Dex"))
        })?;
        let max_pp = calculate_pp(data)?;
        Ok(MoveSlot {
            move_id,
            target: data.target,
            priority: data.priority,
            pp: max_pp,
            max_pp,
            disabled: false,
        })
    }
}

fn selected_species(
    dex: &DexData,
    species: rotom_core::SpeciesId,
    form: Option<rotom_core::FormId>,
) -> Option<&SpeciesData> {
    match form {
        None => dex.species(species),
        Some(form) => dex
            .form(form)
            .and_then(|data| dex.resolve_species(&data.id).map(|(_, value)| value)),
    }
}

fn calculate_pp(data: &MoveData) -> Result<u8, BattleError> {
    if data.no_pp_boosts {
        return Ok(data.pp);
    }
    let value = (u16::from(data.pp) / 5 + 1) * 4;
    u8::try_from(value)
        .map_err(|_| BattleError::DataMismatch("calculated PP exceeds u8".to_owned()))
}

fn champions_stats(
    base: Stats,
    points: StatAllocation,
    level: u8,
    nature_plus: Option<&str>,
    nature_minus: Option<&str>,
) -> BattleStats {
    let hp = champion_stat(base.hp, points.hp, level, true);
    let mut atk = champion_stat(base.atk, points.atk, level, false);
    let mut def = champion_stat(base.def, points.def, level, false);
    let mut spa = champion_stat(base.spa, points.spa, level, false);
    let mut spd = champion_stat(base.spd, points.spd, level, false);
    let mut spe = champion_stat(base.spe, points.spe, level, false);
    for (name, stat) in [
        ("atk", &mut atk),
        ("def", &mut def),
        ("spa", &mut spa),
        ("spd", &mut spd),
        ("spe", &mut spe),
    ] {
        if nature_plus == Some(name) {
            *stat = u16::try_from(u32::from(*stat) * 110 / 100).unwrap_or(u16::MAX);
        } else if nature_minus == Some(name) {
            *stat = u16::try_from(u32::from(*stat) * 90 / 100).unwrap_or(u16::MAX);
        }
    }
    BattleStats {
        hp,
        atk,
        def,
        spa,
        spd,
        spe,
    }
}

fn champion_stat(base: u16, points: u8, level: u8, hp: bool) -> u16 {
    let point_term = u32::from(points).saturating_mul(2).saturating_sub(1);
    let scaled = (u32::from(base) * 2 + 31 + point_term) * u32::from(level) / 100;
    let value = if hp {
        scaled + u32::from(level) + 10
    } else {
        scaled + 5
    };
    u16::try_from(value).unwrap_or(u16::MAX)
}

fn parse_category(value: &str) -> Option<MoveCategory> {
    match value {
        "Physical" => Some(MoveCategory::Physical),
        "Special" => Some(MoveCategory::Special),
        "Status" => Some(MoveCategory::Status),
        _ => None,
    }
}

fn parse_type(value: &str) -> Option<PokemonType> {
    match value.to_ascii_lowercase().as_str() {
        "bug" => Some(PokemonType::Bug),
        "dark" => Some(PokemonType::Dark),
        "dragon" => Some(PokemonType::Dragon),
        "electric" => Some(PokemonType::Electric),
        "fairy" => Some(PokemonType::Fairy),
        "fighting" => Some(PokemonType::Fighting),
        "fire" => Some(PokemonType::Fire),
        "flying" => Some(PokemonType::Flying),
        "ghost" => Some(PokemonType::Ghost),
        "grass" => Some(PokemonType::Grass),
        "ground" => Some(PokemonType::Ground),
        "ice" => Some(PokemonType::Ice),
        "normal" => Some(PokemonType::Normal),
        "poison" => Some(PokemonType::Poison),
        "psychic" => Some(PokemonType::Psychic),
        "rock" => Some(PokemonType::Rock),
        "steel" => Some(PokemonType::Steel),
        "stellar" => Some(PokemonType::Stellar),
        "water" => Some(PokemonType::Water),
        _ => None,
    }
}

const fn type_id(value: PokemonType) -> &'static str {
    match value {
        PokemonType::Bug => "bug",
        PokemonType::Dark => "dark",
        PokemonType::Dragon => "dragon",
        PokemonType::Electric => "electric",
        PokemonType::Fairy => "fairy",
        PokemonType::Fighting => "fighting",
        PokemonType::Fire => "fire",
        PokemonType::Flying => "flying",
        PokemonType::Ghost => "ghost",
        PokemonType::Grass => "grass",
        PokemonType::Ground => "ground",
        PokemonType::Ice => "ice",
        PokemonType::Normal => "normal",
        PokemonType::Poison => "poison",
        PokemonType::Psychic => "psychic",
        PokemonType::Rock => "rock",
        PokemonType::Steel => "steel",
        PokemonType::Stellar => "stellar",
        PokemonType::Water => "water",
    }
}
