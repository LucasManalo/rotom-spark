use crate::DexLoadError;
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
    pub moves: Vec<MoveData>,
    pub items: Vec<ItemData>,
    pub abilities: Vec<AbilityData>,
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
    pub priority: i8,
    pub target: String,
    pub flags: Vec<String>,
    pub crit_ratio: u8,
    pub multi_hit: Option<MultiHit>,
    pub drain: Option<Fraction>,
    pub recoil: Option<Fraction>,
    pub damage: Option<Damage>,
    pub status: Option<String>,
    pub volatile_status: Option<String>,
    pub is_nonstandard: Option<String>,
    pub legal: bool,
    pub ban_reason: Option<String>,
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
