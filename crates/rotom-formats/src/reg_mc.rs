use crate::{validation::validate_reg_mc_team, BattleFormat, FormatId, GameType, ValidationError};
use rotom_core::TeamDefinition;
use rotom_dex::DexData;

#[derive(Clone, Copy, Debug, Default)]
pub struct ChampionsRegMC;

impl BattleFormat for ChampionsRegMC {
    fn id(&self) -> FormatId {
        FormatId::ChampionsRegMC
    }

    fn game_type(&self) -> GameType {
        GameType::Doubles
    }

    fn team_size(&self) -> usize {
        6
    }

    fn picked_team_size(&self) -> usize {
        4
    }

    fn validate_team(&self, team: &TeamDefinition, dex: &DexData) -> Result<(), ValidationError> {
        validate_reg_mc_team(team, dex)
    }
}
