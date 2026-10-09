//! Format rules, team parsing, and team-legality boundaries for Rotom Spark.

mod error;
mod format;
mod parser;
mod reg_mc;
mod validation;

pub use error::{TeamParseError, ValidationError, ValidationProblem};
pub use format::{BattleFormat, FormatId, GameType};
pub use parser::parse_showdown_team;
pub use reg_mc::ChampionsRegMC;

/// The crate's stable boundary name, useful for workspace smoke checks.
pub const CRATE_NAME: &str = "rotom-formats";

#[cfg(test)]
mod tests {
    #[test]
    fn crate_boundary_is_available() {
        assert_eq!(super::CRATE_NAME, "rotom-formats");
    }
}
