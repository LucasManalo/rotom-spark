use super::ConditionState;
use crate::{AbilityId, FormId, ItemId, MoveId, SpeciesId};
use serde::{Deserialize, Serialize};

/// Stable zero-based index into a side's battle team.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PokemonIndex(u8);

impl PokemonIndex {
    #[must_use]
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// One of the two active positions on a side in a doubles battle.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveSlot {
    Left,
    Right,
}

impl ActiveSlot {
    pub const ALL: [Self; 2] = [Self::Left, Self::Right];

    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}

/// A Pokémon's current relationship to the battlefield.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PokemonPosition {
    Team,
    Active { slot: ActiveSlot },
    Fainted,
}

/// Calculated battle stats in HP/Attack/Defense/Sp. Atk/Sp. Def/Speed order.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleStats {
    pub hp: u16,
    pub atk: u16,
    pub def: u16,
    pub spa: u16,
    pub spd: u16,
    pub spe: u16,
}

/// Mutable PP and availability for one known move.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MoveSlot {
    pub move_id: MoveId,
    pub pp: u8,
    pub max_pp: u8,
    pub disabled: bool,
}

/// Major non-volatile status conditions representable by the engine.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MajorStatus {
    Burn,
    Freeze,
    Paralysis,
    Poison,
    BadlyPoisoned,
    Sleep,
}

/// Status-specific counters that remain meaningful across turns.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatusState {
    pub sleep_turns: u8,
    pub toxic_counter: u8,
}

/// Current stat-stage modifiers. Mechanics later enforce the legal -6..=6 range.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatStages {
    pub atk: i8,
    pub def: i8,
    pub spa: i8,
    pub spd: i8,
    pub spe: i8,
    pub accuracy: i8,
    pub evasion: i8,
}

/// Mutable state for one Pokémon after it enters a battle.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PokemonState {
    pub species: SpeciesId,
    pub form: Option<FormId>,
    pub level: u8,
    pub stats: BattleStats,
    pub current_hp: u16,
    pub ability: AbilityId,
    pub item: Option<ItemId>,
    pub moves: Vec<MoveSlot>,
    pub status: Option<MajorStatus>,
    pub status_state: StatusState,
    pub stat_stages: StatStages,
    pub volatiles: Vec<ConditionState>,
    pub position: PokemonPosition,
}

impl PokemonState {
    #[must_use]
    pub const fn is_fainted(&self) -> bool {
        matches!(self.position, PokemonPosition::Fainted)
    }

    pub(crate) fn clear_on_exit(&mut self) {
        self.stat_stages = StatStages::default();
        self.volatiles.clear();
        for move_slot in &mut self.moves {
            move_slot.disabled = false;
        }
    }
}

/// Runtime Pokémon collection, distinct from the immutable team definition.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeamState {
    pub members: Vec<PokemonState>,
}

impl TeamState {
    #[must_use]
    pub fn new(members: Vec<PokemonState>) -> Self {
        Self { members }
    }

    #[must_use]
    pub fn pokemon(&self, index: PokemonIndex) -> Option<&PokemonState> {
        self.members.get(usize::from(index.get()))
    }

    #[must_use]
    pub fn pokemon_mut(&mut self, index: PokemonIndex) -> Option<&mut PokemonState> {
        self.members.get_mut(usize::from(index.get()))
    }
}
