use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TeamParseError {
    pub line: usize,
    pub message: String,
}

impl TeamParseError {
    pub(crate) fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for TeamParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "team import line {}: {}",
            self.line, self.message
        )
    }
}

impl Error for TeamParseError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationError {
    pub problems: Vec<ValidationProblem>,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "team has {} validation problem(s)",
            self.problems.len()
        )
    }
}

impl Error for ValidationError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationProblem {
    TeamSize {
        expected: usize,
        actual: usize,
    },
    UnknownReference {
        slot: usize,
        kind: &'static str,
    },
    IllegalSpecies {
        slot: usize,
        name: String,
    },
    IllegalForm {
        slot: usize,
        name: String,
    },
    FormSpeciesMismatch {
        slot: usize,
    },
    IllegalAbility {
        slot: usize,
        name: String,
    },
    AbilityNotAvailable {
        slot: usize,
        name: String,
    },
    IllegalItem {
        slot: usize,
        name: String,
    },
    RequiredItem {
        slot: usize,
        name: String,
    },
    WrongMegaStone {
        slot: usize,
        name: String,
    },
    IllegalNature {
        slot: usize,
        name: String,
    },
    InvalidLevel {
        slot: usize,
        level: u8,
    },
    InvalidMoveCount {
        slot: usize,
        count: usize,
    },
    IllegalMove {
        slot: usize,
        name: String,
    },
    MoveNotLearnable {
        slot: usize,
        name: String,
    },
    UnsupportedEventMove {
        slot: usize,
        name: String,
    },
    RequiredMove {
        slot: usize,
        name: String,
    },
    InvalidIv {
        slot: usize,
        stat: &'static str,
        value: u8,
    },
    StatPointLimit {
        slot: usize,
        stat: &'static str,
        value: u8,
    },
    StatPointTotal {
        slot: usize,
        total: u16,
    },
    AmbiguousZeroStats {
        slot: usize,
    },
    GenderMismatch {
        slot: usize,
    },
    NicknameTooLong {
        slot: usize,
    },
    DuplicateSpecies {
        first_slot: usize,
        second_slot: usize,
    },
    DuplicateItem {
        first_slot: usize,
        second_slot: usize,
    },
}
