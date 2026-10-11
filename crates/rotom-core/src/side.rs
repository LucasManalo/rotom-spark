use serde::{Deserialize, Serialize};

/// Identifies one of the two players in a battle.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SideId {
    P1,
    P2,
}

impl SideId {
    /// Returns the opposing side.
    #[must_use]
    pub const fn opponent(self) -> Self {
        match self {
            Self::P1 => Self::P2,
            Self::P2 => Self::P1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SideId;

    #[test]
    fn opponent_is_symmetric() {
        assert_eq!(SideId::P1.opponent(), SideId::P2);
        assert_eq!(SideId::P2.opponent(), SideId::P1);
    }
}
