use std::{error::Error, fmt};

/// Errors exposed by the battle engine as mechanics are implemented.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum BattleError {
    IllegalChoice(String),
    UnsupportedMechanic(String),
    InvalidTeam(String),
    InvalidState(String),
    DataMismatch(String),
}

impl fmt::Display for BattleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, detail) = match self {
            Self::IllegalChoice(detail) => ("illegal choice", detail),
            Self::UnsupportedMechanic(detail) => ("unsupported mechanic", detail),
            Self::InvalidTeam(detail) => ("invalid team", detail),
            Self::InvalidState(detail) => ("invalid battle state", detail),
            Self::DataMismatch(detail) => ("data mismatch", detail),
        };

        write!(formatter, "{kind}: {detail}")
    }
}

impl Error for BattleError {}

#[cfg(test)]
mod tests {
    use super::BattleError;

    #[test]
    fn unsupported_mechanics_are_explicit() {
        let error = BattleError::UnsupportedMechanic("example".to_owned());

        assert_eq!(error.to_string(), "unsupported mechanic: example");
    }
}
