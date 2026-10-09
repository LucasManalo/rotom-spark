use crate::{AbilityId, FormId, ItemId, MoveId, NatureId, SpeciesId};
use serde::{Deserialize, Serialize};

/// Immutable choices that a player brings to a match.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeamDefinition {
    pub members: Vec<PokemonSet>,
}

/// An immutable Pokémon configuration, separate from future mutable battle state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PokemonSet {
    pub nickname: Option<String>,
    pub species: SpeciesId,
    pub form: Option<FormId>,
    pub level: u8,
    pub ability: AbilityId,
    pub item: Option<ItemId>,
    pub nature: NatureId,
    /// Champions stat points. Showdown currently transports these in its EV fields.
    pub stat_points: StatAllocation,
    pub ivs: StatAllocation,
    pub moves: Vec<MoveId>,
    pub gender: Option<Gender>,
    pub shiny: bool,
}

/// Six-stat allocation in HP/Attack/Defense/Sp. Atk/Sp. Def/Speed order.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatAllocation {
    pub hp: u8,
    pub atk: u8,
    pub def: u8,
    pub spa: u8,
    pub spd: u8,
    pub spe: u8,
}

impl StatAllocation {
    pub const ZERO: Self = Self {
        hp: 0,
        atk: 0,
        def: 0,
        spa: 0,
        spd: 0,
        spe: 0,
    };

    pub const PERFECT_IVS: Self = Self {
        hp: 31,
        atk: 31,
        def: 31,
        spa: 31,
        spd: 31,
        spe: 31,
    };

    #[must_use]
    pub const fn total(self) -> u16 {
        self.hp as u16
            + self.atk as u16
            + self.def as u16
            + self.spa as u16
            + self.spd as u16
            + self.spe as u16
    }

    #[must_use]
    pub const fn named_values(self) -> [(&'static str, u8); 6] {
        [
            ("HP", self.hp),
            ("Atk", self.atk),
            ("Def", self.def),
            ("SpA", self.spa),
            ("SpD", self.spd),
            ("Spe", self.spe),
        ]
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
}

#[cfg(test)]
mod tests {
    use super::StatAllocation;

    #[test]
    fn stat_allocation_total_counts_all_stats() {
        let allocation = StatAllocation {
            hp: 2,
            atk: 3,
            def: 5,
            spa: 7,
            spd: 11,
            spe: 13,
        };

        assert_eq!(allocation.total(), 41);
    }
}
