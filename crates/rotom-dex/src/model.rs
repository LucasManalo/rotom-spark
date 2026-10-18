use crate::DexLoadError;
use rotom_core::{AbilityId, FormId, ItemId, MoveId, MoveTarget, NatureId, SpeciesId};
use serde::{Deserialize, Serialize};

/// Complete normalized static data exported for one battle format.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DexData {
    pub schema_version: u32,
    pub source: SourceMetadata,
    pub format: FormatData,
    pub species: Vec<SpeciesData>,
    pub forms: Vec<FormData>,
    pub types: Vec<TypeData>,
    pub moves: Vec<MoveData>,
    pub items: Vec<ItemData>,
    pub abilities: Vec<AbilityData>,
    pub natures: Vec<NatureData>,
}

impl DexData {
    /// Parses normalized data produced by `tools/showdown-export`.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON does not match the normalized data schema.
    pub fn from_json(json: &str) -> Result<Self, DexLoadError> {
        serde_json::from_str(json).map_err(DexLoadError::new)
    }

    #[must_use]
    pub fn resolve_species(&self, key: &str) -> Option<(SpeciesId, &SpeciesData)> {
        resolve(&self.species, key, |entry| entry.id.as_str())
            .and_then(|(index, entry)| Some((SpeciesId::new(u32::try_from(index).ok()?), entry)))
    }

    #[must_use]
    pub fn resolve_form(&self, key: &str) -> Option<(FormId, &FormData)> {
        resolve(&self.forms, key, |entry| entry.id.as_str())
            .and_then(|(index, entry)| Some((FormId::new(u32::try_from(index).ok()?), entry)))
    }

    #[must_use]
    pub fn resolve_move(&self, key: &str) -> Option<(MoveId, &MoveData)> {
        resolve(&self.moves, key, |entry| entry.id.as_str())
            .and_then(|(index, entry)| Some((MoveId::new(u32::try_from(index).ok()?), entry)))
    }

    #[must_use]
    pub fn resolve_type(&self, key: &str) -> Option<&TypeData> {
        resolve(&self.types, key, |entry| entry.id.as_str()).map(|(_, entry)| entry)
    }

    #[must_use]
    pub fn resolve_item(&self, key: &str) -> Option<(ItemId, &ItemData)> {
        resolve(&self.items, key, |entry| entry.id.as_str())
            .and_then(|(index, entry)| Some((ItemId::new(u32::try_from(index).ok()?), entry)))
    }

    #[must_use]
    pub fn resolve_ability(&self, key: &str) -> Option<(AbilityId, &AbilityData)> {
        resolve(&self.abilities, key, |entry| entry.id.as_str())
            .and_then(|(index, entry)| Some((AbilityId::new(u32::try_from(index).ok()?), entry)))
    }

    #[must_use]
    pub fn resolve_nature(&self, key: &str) -> Option<(NatureId, &NatureData)> {
        resolve(&self.natures, key, |entry| entry.id.as_str())
            .and_then(|(index, entry)| Some((NatureId::new(u32::try_from(index).ok()?), entry)))
    }

    #[must_use]
    pub fn species(&self, id: SpeciesId) -> Option<&SpeciesData> {
        self.species.get(usize::try_from(id.get()).ok()?)
    }

    #[must_use]
    pub fn form(&self, id: FormId) -> Option<&FormData> {
        self.forms.get(usize::try_from(id.get()).ok()?)
    }

    #[must_use]
    pub fn move_data(&self, id: MoveId) -> Option<&MoveData> {
        self.moves.get(usize::try_from(id.get()).ok()?)
    }

    #[must_use]
    pub fn item(&self, id: ItemId) -> Option<&ItemData> {
        self.items.get(usize::try_from(id.get()).ok()?)
    }

    #[must_use]
    pub fn ability(&self, id: AbilityId) -> Option<&AbilityData> {
        self.abilities.get(usize::try_from(id.get()).ok()?)
    }

    #[must_use]
    pub fn nature(&self, id: NatureId) -> Option<&NatureData> {
        self.natures.get(usize::try_from(id.get()).ok()?)
    }
}

fn resolve<'a, T>(
    entries: &'a [T],
    key: &str,
    get_key: impl Fn(&T) -> &str,
) -> Option<(usize, &'a T)> {
    let index = entries
        .binary_search_by(|entry| get_key(entry).cmp(key))
        .ok()?;
    Some((index, &entries[index]))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMetadata {
    pub repo: String,
    pub commit: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FormatData {
    pub id: String,
    pub name: String,
    #[serde(rename = "mod")]
    pub mod_name: String,
    pub game_type: String,
    pub ruleset: Vec<String>,
    pub banlist: Vec<String>,
    pub restricted: Vec<String>,
    pub unbanlist: Vec<String>,
    pub min_team_size: u8,
    pub max_team_size: u8,
    pub picked_team_size: Option<u8>,
    pub max_move_count: u8,
    pub min_level: u16,
    pub max_level: u16,
    pub default_level: u16,
    pub adjust_level: Option<u16>,
    pub ev_limit: Option<u16>,
    pub rule_table: Vec<RuleEntry>,
    pub value_rules: Vec<ValueRule>,
    pub complex_bans: Vec<ComplexBan>,
    pub complex_team_bans: Vec<ComplexBan>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuleEntry {
    pub rule: String,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ValueRule {
    pub rule: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComplexBan {
    pub rule: String,
    pub source: String,
    pub limit: Option<u16>,
    pub entries: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SpeciesData {
    pub id: String,
    pub name: String,
    pub num: i32,
    pub gen: u8,
    pub base_species: String,
    pub forme: Option<String>,
    pub types: Vec<String>,
    pub base_stats: Stats,
    pub abilities: Vec<KeyValue<String>>,
    pub weight_kg: f64,
    pub height_m: f64,
    pub gender: Option<String>,
    pub gender_ratio: GenderRatio,
    pub prevo: Option<String>,
    pub evos: Vec<String>,
    pub other_formes: Vec<String>,
    pub required_item: Option<String>,
    pub required_move: Option<String>,
    pub battle_only: Vec<String>,
    pub changes_from: Option<String>,
    pub is_mega: bool,
    pub is_primal: bool,
    pub is_nonstandard: Option<String>,
    pub tags: Vec<String>,
    pub tier: String,
    pub move_pool: Vec<String>,
    pub event_only_moves: Vec<String>,
    pub legal: bool,
    pub ban_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FormData {
    pub id: String,
    pub name: String,
    pub base_species: String,
    pub forme: String,
    pub required_item: Option<String>,
    pub required_move: Option<String>,
    pub battle_only: Vec<String>,
    pub changes_from: Option<String>,
    pub is_mega: bool,
    pub is_primal: bool,
    pub legal: bool,
    pub ban_reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    pub hp: u16,
    pub atk: u16,
    pub def: u16,
    pub spa: u16,
    pub spd: u16,
    pub spe: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypeData {
    pub id: String,
    pub name: String,
    pub damage_taken: Vec<KeyValue<u8>>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenderRatio {
    pub male: f64,
    pub female: f64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeyValue<T> {
    pub key: String,
    pub value: T,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)]
pub struct MoveData {
    pub id: String,
    pub name: String,
    pub num: i32,
    pub gen: u8,
    #[serde(rename = "type")]
    pub move_type: String,
    pub category: String,
    pub base_power: u16,
    pub accuracy: Accuracy,
    pub pp: u8,
    pub no_pp_boosts: bool,
    pub priority: i8,
    pub target: MoveTarget,
    pub flags: Vec<String>,
    pub crit_ratio: u8,
    pub multi_hit: Option<MultiHit>,
    pub multi_accuracy: bool,
    pub drain: Option<Fraction>,
    pub recoil: Option<Fraction>,
    pub self_damage: Option<Fraction>,
    pub damage: Option<Damage>,
    pub status: Option<String>,
    pub volatile_status: Option<String>,
    pub boosts: Vec<KeyValue<i8>>,
    pub heal: Option<Fraction>,
    pub weather: Option<String>,
    pub terrain: Option<String>,
    pub pseudo_weather: Option<String>,
    pub side_condition: Option<String>,
    pub secondaries: Vec<MoveSecondaryData>,
    pub self_effect: Option<MoveSelfEffectData>,
    pub override_offensive_stat: Option<String>,
    pub override_defensive_stat: Option<String>,
    pub ignore_negative_offensive: bool,
    pub ignore_positive_defensive: bool,
    pub behavior: MoveBehaviorData,
    pub is_nonstandard: Option<String>,
    pub legal: bool,
    pub ban_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MoveSecondaryData {
    pub chance: u8,
    pub status: Option<String>,
    pub boosts: Vec<KeyValue<i8>>,
    pub self_boosts: Vec<KeyValue<i8>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MoveSelfEffectData {
    pub chance: Option<u8>,
    pub boosts: Vec<KeyValue<i8>>,
}

/// Resolved move behavior that is not captured by the ordinary numeric fields.
///
/// The exporter records these facts from the pinned resolved Showdown move so
/// runtime support can be derived from required capabilities instead of a
/// hand-maintained move-ID allowlist.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MoveBehaviorData {
    pub effect_kinds: Vec<MoveBehaviorKind>,
    pub callbacks: Vec<String>,
    pub intrinsic_flags: Vec<String>,
    pub special_properties: Vec<String>,
    pub structured_effects_supported: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveBehaviorKind {
    SecondaryEffect,
    SelfEffect,
    StatStage,
    Healing,
    FieldOrSideCondition,
    CallsMove,
    SwitchControl,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Accuracy {
    Always,
    Percent { value: u8 },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MultiHit {
    pub min: u8,
    pub max: u8,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fraction {
    pub numerator: u16,
    pub denominator: u16,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Damage {
    Fixed { value: u16 },
    Level,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ItemData {
    pub id: String,
    pub name: String,
    pub num: i32,
    pub gen: u8,
    pub mega_stone: Vec<KeyValue<String>>,
    pub mega_evolves: Option<String>,
    pub item_user: Vec<String>,
    pub fling: Option<Fling>,
    pub natural_gift: Option<NaturalGift>,
    pub boosts: Vec<KeyValue<i8>>,
    pub flags: Vec<ItemFlag>,
    pub is_nonstandard: Option<String>,
    pub legal: bool,
    pub ban_reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemFlag {
    Berry,
    Choice,
    Gem,
    Pokeball,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fling {
    pub base_power: u16,
    pub status: Option<String>,
    pub volatile_status: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NaturalGift {
    pub base_power: u16,
    #[serde(rename = "type")]
    pub gift_type: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbilityData {
    pub id: String,
    pub name: String,
    pub num: i32,
    pub gen: u8,
    pub rating: f64,
    pub is_nonstandard: Option<String>,
    pub legal: bool,
    pub ban_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NatureData {
    pub id: String,
    pub name: String,
    pub plus: Option<String>,
    pub minus: Option<String>,
    pub legal: bool,
    pub ban_reason: Option<String>,
}
