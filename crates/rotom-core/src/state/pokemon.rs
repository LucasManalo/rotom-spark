use super::ConditionState;
use crate::{AbilityId, BattleError, ConditionId, FormId, ItemId, MoveId, MoveTarget, SpeciesId};
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
    pub target: MoveTarget,
    /// Base move priority from the pinned static Dex data.
    #[serde(default)]
    pub priority: i8,
    pub pp: u8,
    pub max_pp: u8,
    pub disabled: bool,
}

/// Major non-volatile status conditions representable by the engine.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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
    #[serde(default)]
    pub freeze_turns: u8,
    pub toxic_counter: u8,
}

/// A battle stat affected by the standard -6..=6 stage system.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleStat {
    Attack,
    Defense,
    SpecialAttack,
    SpecialDefense,
    Speed,
    Accuracy,
    Evasion,
}

/// The bounded result of one requested stat-stage change.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatStageChange {
    pub stat: BattleStat,
    pub previous: i8,
    pub current: i8,
    pub requested_delta: i8,
    pub applied_delta: i8,
}

/// Current stat-stage modifiers, always bounded to the legal -6..=6 range
/// when changed through [`StatStages::change`].
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

impl StatStages {
    pub const MIN: i8 = -6;
    pub const MAX: i8 = 6;

    #[must_use]
    pub const fn get(self, stat: BattleStat) -> i8 {
        match stat {
            BattleStat::Attack => self.atk,
            BattleStat::Defense => self.def,
            BattleStat::SpecialAttack => self.spa,
            BattleStat::SpecialDefense => self.spd,
            BattleStat::Speed => self.spe,
            BattleStat::Accuracy => self.accuracy,
            BattleStat::Evasion => self.evasion,
        }
    }

    /// Applies a stat-stage delta and reports how much of it took effect.
    pub fn change(&mut self, stat: BattleStat, delta: i8) -> StatStageChange {
        let previous = self.get(stat);
        let current = previous.saturating_add(delta).clamp(Self::MIN, Self::MAX);
        *self.value_mut(stat) = current;
        StatStageChange {
            stat,
            previous,
            current,
            requested_delta: delta,
            applied_delta: current - previous,
        }
    }

    #[must_use]
    pub fn modified_stat(self, stat: BattleStat, raw: u16) -> u32 {
        let stage = self.get(stat).clamp(Self::MIN, Self::MAX);
        let base = match stat {
            BattleStat::Accuracy | BattleStat::Evasion => 3,
            BattleStat::Attack
            | BattleStat::Defense
            | BattleStat::SpecialAttack
            | BattleStat::SpecialDefense
            | BattleStat::Speed => 2,
        };
        if stage >= 0 {
            u32::from(raw) * (base + u32::from(stage.unsigned_abs())) / base
        } else {
            u32::from(raw) * base / (base + u32::from(stage.unsigned_abs()))
        }
    }

    fn value_mut(&mut self, stat: BattleStat) -> &mut i8 {
        match stat {
            BattleStat::Attack => &mut self.atk,
            BattleStat::Defense => &mut self.def,
            BattleStat::SpecialAttack => &mut self.spa,
            BattleStat::SpecialDefense => &mut self.spd,
            BattleStat::Speed => &mut self.spe,
            BattleStat::Accuracy => &mut self.accuracy,
            BattleStat::Evasion => &mut self.evasion,
        }
    }
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
    /// Most recent Berry consumed through the authoritative item lifecycle.
    /// Harvest may restore this value while the holder has no item.
    #[serde(default)]
    pub last_consumed_berry: Option<ItemId>,
    /// Berry and residual countdown scheduled by Cud Chew.
    #[serde(default)]
    pub cud_chew_berry: Option<ItemId>,
    #[serde(default)]
    pub cud_chew_turns: u8,
    pub moves: Vec<MoveSlot>,
    pub status: Option<MajorStatus>,
    pub status_state: StatusState,
    pub stat_stages: StatStages,
    pub volatiles: Vec<ConditionState>,
    /// Whether this Pokémon's current form/item combination can Mega Evolve.
    pub can_mega_evolve: bool,
    /// Authoritative trapping state used to reject voluntary switches.
    pub trapped: bool,
    pub position: PokemonPosition,
}

impl PokemonState {
    #[must_use]
    pub const fn is_fainted(&self) -> bool {
        matches!(self.position, PokemonPosition::Fainted)
    }

    /// Installs a major status and its initialized counters.
    ///
    /// # Errors
    ///
    /// Returns an error if another major status is already present or the
    /// supplied counters do not match the status being installed.
    pub fn set_major_status(
        &mut self,
        status: MajorStatus,
        state: StatusState,
    ) -> Result<(), BattleError> {
        if let Some(existing) = self.status {
            return Err(BattleError::IllegalChoice(format!(
                "cannot apply {status:?} while {existing:?} is active"
            )));
        }
        validate_status_state(status, state)?;
        self.status = Some(status);
        self.status_state = state;
        Ok(())
    }

    /// Clears the current major status and all status-specific counters.
    pub fn cure_major_status(&mut self) -> Option<MajorStatus> {
        let cured = self.status.take();
        self.status_state = StatusState::default();
        cured
    }

    /// Adds a volatile without replacing an existing instance.
    ///
    /// # Errors
    ///
    /// Returns an error for a zero-duration condition or duplicate identity.
    pub fn add_volatile(&mut self, condition: ConditionState) -> Result<(), BattleError> {
        if condition.turns_remaining == Some(0) || condition.layers == 0 {
            return Err(BattleError::InvalidState(
                "a volatile condition cannot start with zero duration or layers".to_owned(),
            ));
        }
        if self.has_volatile(condition.id) {
            return Err(BattleError::IllegalChoice(format!(
                "volatile condition {} is already active",
                condition.id.get()
            )));
        }
        self.volatiles.push(condition);
        Ok(())
    }

    #[must_use]
    pub fn has_volatile(&self, condition: ConditionId) -> bool {
        self.volatiles.iter().any(|state| state.id == condition)
    }

    pub fn remove_volatile(&mut self, condition: ConditionId) -> Option<ConditionState> {
        let index = self
            .volatiles
            .iter()
            .position(|state| state.id == condition)?;
        Some(self.volatiles.remove(index))
    }

    /// Decrements timed volatile conditions and returns those that expired.
    pub fn tick_volatile_durations(&mut self) -> Vec<ConditionId> {
        let mut expired = Vec::new();
        for volatile in &mut self.volatiles {
            if let Some(turns) = &mut volatile.turns_remaining {
                *turns = turns.saturating_sub(1);
                if *turns == 0 {
                    expired.push(volatile.id);
                }
            }
        }
        self.volatiles
            .retain(|volatile| volatile.turns_remaining != Some(0));
        expired
    }

    /// Returns Speed after stat stages and paralysis, before effect hooks.
    #[must_use]
    pub fn effective_speed(&self) -> u16 {
        let staged = u32::from(self.staged_speed());
        let status_modified = if self.status == Some(MajorStatus::Paralysis) {
            staged / 2
        } else {
            staged
        };
        u16::try_from(status_modified.min(u32::from(u16::MAX))).unwrap_or(u16::MAX)
    }

    /// Returns Speed after stat stages, before status and effect modifiers.
    #[must_use]
    pub fn staged_speed(&self) -> u16 {
        let speed = self
            .stat_stages
            .modified_stat(BattleStat::Speed, self.stats.spe);
        u16::try_from(speed.min(u32::from(u16::MAX))).unwrap_or(u16::MAX)
    }

    pub(crate) fn clear_on_exit(&mut self) {
        self.stat_stages = StatStages::default();
        self.volatiles.clear();
        self.trapped = false;
        self.cud_chew_berry = None;
        self.cud_chew_turns = 0;
        for move_slot in &mut self.moves {
            move_slot.disabled = false;
        }
    }

    pub(crate) fn prepare_switch_in(&mut self) {
        if self.status == Some(MajorStatus::BadlyPoisoned) {
            self.status_state.toxic_counter = 0;
        }
    }
}

pub(crate) fn validate_status_state(
    status: MajorStatus,
    state: StatusState,
) -> Result<(), BattleError> {
    let valid = match status {
        MajorStatus::Sleep => {
            state.sleep_turns > 0 && state.freeze_turns == 0 && state.toxic_counter == 0
        }
        MajorStatus::Freeze => {
            state.freeze_turns > 0 && state.sleep_turns == 0 && state.toxic_counter == 0
        }
        MajorStatus::BadlyPoisoned => {
            state.sleep_turns == 0 && state.freeze_turns == 0 && state.toxic_counter <= 15
        }
        MajorStatus::Burn | MajorStatus::Paralysis | MajorStatus::Poison => {
            state == StatusState::default()
        }
    };
    if valid {
        Ok(())
    } else {
        Err(BattleError::InvalidState(format!(
            "status counters are invalid for {status:?}"
        )))
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
    pub(crate) fn pokemon_mut(&mut self, index: PokemonIndex) -> Option<&mut PokemonState> {
        self.members.get_mut(usize::from(index.get()))
    }
}
