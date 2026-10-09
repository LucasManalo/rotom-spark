use crate::ValidationError;
use rotom_core::TeamDefinition;
use rotom_dex::DexData;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatId {
    ChampionsRegMC,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameType {
    Doubles,
}

/// Format-owned legality rules. Battle configuration is added with battle state.
pub trait BattleFormat {
    fn id(&self) -> FormatId;
    fn game_type(&self) -> GameType;
    fn team_size(&self) -> usize;
    fn picked_team_size(&self) -> usize;
    /// Validates one complete team against this format's rules.
    ///
    /// # Errors
    ///
    /// Returns every detected format or set-legality problem.
    fn validate_team(&self, team: &TeamDefinition, dex: &DexData) -> Result<(), ValidationError>;
}
