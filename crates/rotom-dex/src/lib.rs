//! Static Pokémon game data for Rotom Spark.
//!
//! The checked-in data is generated from the resolved Champions Dex at the
//! exact Pokémon Showdown revision recorded in `SHOWDOWN_VERSION`. Showdown is
//! development tooling only and is never loaded by this crate.

mod combat;
mod error;
mod model;

pub use error::DexLoadError;
pub use model::{
    AbilityData, Accuracy, ComplexBan, Damage, DexData, Fling, FormData, FormatData, Fraction,
    GenderRatio, ItemData, ItemFlag, KeyValue, MoveBehaviorData, MoveBehaviorKind, MoveData,
    MultiHit, NaturalGift, NatureData, RuleEntry, SourceMetadata, SpeciesData, Stats, TypeData,
    ValueRule,
};

const CHAMPIONS_REG_MC_JSON: &str = include_str!("../../../data/generated/champions-reg-mc.json");

/// Loads the generated Champions Regulation M-C data bundled with this crate.
///
/// # Errors
///
/// Returns an error when the checked-in generated data does not match the Rust
/// schema. Such an error indicates a build/release defect rather than bad user
/// input.
pub fn champions_reg_mc() -> Result<DexData, DexLoadError> {
    DexData::from_json(CHAMPIONS_REG_MC_JSON)
}
