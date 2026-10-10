//! Stable normalized snapshots and protocol boundaries for Rotom Spark.
//!
//! Snapshots intentionally copy gameplay state into protocol-owned structures.
//! This lets internal engine storage evolve without turning memory layout into a
//! replay or differential-testing contract.

use rotom_core::{
    ActionOrder, ActiveSlot, BattlePhase, BattleState, MajorStatus, MoveTarget, PokemonAction,
    PokemonIndex, PokemonPosition, SideId,
};
use serde::{Deserialize, Serialize};

pub const CRATE_NAME: &str = "rotom-protocol";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleSnapshot {
    pub turn: u32,
    pub picked_team_size: u8,
    pub phase: BattlePhase,
    pub field: FieldSnapshot,
    pub rng: RngSnapshot,
    pub queue: Vec<QueuedActionSnapshot>,
    pub sides: [SideSnapshot; 2],
    pub winner: Option<SideId>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RngSnapshot {
    pub seed: u64,
    pub state: u64,
    pub draws: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueuedActionSnapshot {
    pub side: SideId,
    pub slot: ActiveSlot,
    pub pokemon: u8,
    pub choice: PokemonAction,
    pub order: ActionOrder,
    pub priority: i8,
    pub speed: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldSnapshot {
    pub weather: Option<ConditionSnapshot>,
    pub terrain: Option<ConditionSnapshot>,
    pub conditions: Vec<ConditionSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SideSnapshot {
    pub side: SideId,
    pub active: [Option<u8>; 2],
    pub selected_team: Vec<u8>,
    pub side_conditions: Vec<ConditionSnapshot>,
    pub mega_evolution_used: bool,
    pub team: Vec<PokemonSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PokemonSnapshot {
    pub species: u32,
    pub form: Option<u32>,
    pub level: u8,
    pub stats: StatsSnapshot,
    pub current_hp: u16,
    pub ability: u32,
    pub item: Option<u32>,
    pub moves: Vec<MoveSnapshot>,
    pub status: Option<MajorStatus>,
    pub status_state: StatusSnapshot,
    pub stat_stages: StatStagesSnapshot,
    pub volatiles: Vec<ConditionSnapshot>,
    pub can_mega_evolve: bool,
    pub trapped: bool,
    pub position: PositionSnapshot,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatsSnapshot {
    pub max_hp: u16,
    pub atk: u16,
    pub def: u16,
    pub spa: u16,
    pub spd: u16,
    pub spe: u16,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MoveSnapshot {
    pub move_id: u32,
    pub target: MoveTarget,
    pub priority: i8,
    pub pp: u8,
    pub max_pp: u8,
    pub disabled: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatusSnapshot {
    pub sleep_turns: u8,
    pub toxic_counter: u8,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatStagesSnapshot {
    pub atk: i8,
    pub def: i8,
    pub spa: i8,
    pub spd: i8,
    pub spe: i8,
    pub accuracy: i8,
    pub evasion: i8,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionSnapshot {
    pub id: u32,
    pub turns_remaining: Option<u8>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PositionSnapshot {
    Team,
    Active { slot: ActiveSlot },
    Fainted,
}

/// Converts an authoritative state into the stable normalized snapshot schema.
#[must_use]
pub fn snapshot(state: &BattleState) -> BattleSnapshot {
    BattleSnapshot {
        turn: state.turn,
        picked_team_size: state.picked_team_size,
        phase: state.phase,
        field: FieldSnapshot {
            weather: state.field.weather.map(ConditionSnapshot::from),
            terrain: state.field.terrain.map(ConditionSnapshot::from),
            conditions: state
                .field
                .conditions
                .iter()
                .copied()
                .map(ConditionSnapshot::from)
                .collect(),
        },
        rng: RngSnapshot {
            seed: state.rng.seed(),
            state: state.rng.state(),
            draws: state.rng.draws(),
        },
        queue: state
            .queue
            .actions
            .iter()
            .map(|action| QueuedActionSnapshot {
                side: action.side,
                slot: action.slot,
                pokemon: action.pokemon.get(),
                choice: action.choice,
                order: action.order,
                priority: action.priority,
                speed: action.speed,
            })
            .collect(),
        sides: [
            SideSnapshot::from_state(SideId::P1, &state.sides[0]),
            SideSnapshot::from_state(SideId::P2, &state.sides[1]),
        ],
        winner: state.winner,
    }
}

/// Method-style access to [`snapshot`] without coupling `rotom-core` to this crate.
pub trait BattleStateSnapshot {
    fn snapshot(&self) -> BattleSnapshot;
}

impl BattleStateSnapshot for BattleState {
    fn snapshot(&self) -> BattleSnapshot {
        snapshot(self)
    }
}

impl SideSnapshot {
    fn from_state(side: SideId, state: &rotom_core::SideState) -> Self {
        Self {
            side,
            active: state.active.map(|index| index.map(PokemonIndex::get)),
            selected_team: state
                .selected_team
                .iter()
                .copied()
                .map(PokemonIndex::get)
                .collect(),
            side_conditions: state
                .side_conditions
                .iter()
                .copied()
                .map(ConditionSnapshot::from)
                .collect(),
            mega_evolution_used: state.mega_evolution_used,
            team: state
                .team
                .members
                .iter()
                .map(PokemonSnapshot::from)
                .collect(),
        }
    }
}

impl From<&rotom_core::PokemonState> for PokemonSnapshot {
    fn from(state: &rotom_core::PokemonState) -> Self {
        Self {
            species: state.species.get(),
            form: state.form.map(rotom_core::FormId::get),
            level: state.level,
            stats: StatsSnapshot {
                max_hp: state.stats.hp,
                atk: state.stats.atk,
                def: state.stats.def,
                spa: state.stats.spa,
                spd: state.stats.spd,
                spe: state.stats.spe,
            },
            current_hp: state.current_hp,
            ability: state.ability.get(),
            item: state.item.map(rotom_core::ItemId::get),
            moves: state
                .moves
                .iter()
                .map(|move_slot| MoveSnapshot {
                    move_id: move_slot.move_id.get(),
                    target: move_slot.target,
                    priority: move_slot.priority,
                    pp: move_slot.pp,
                    max_pp: move_slot.max_pp,
                    disabled: move_slot.disabled,
                })
                .collect(),
            status: state.status,
            status_state: StatusSnapshot {
                sleep_turns: state.status_state.sleep_turns,
                toxic_counter: state.status_state.toxic_counter,
            },
            stat_stages: StatStagesSnapshot {
                atk: state.stat_stages.atk,
                def: state.stat_stages.def,
                spa: state.stat_stages.spa,
                spd: state.stat_stages.spd,
                spe: state.stat_stages.spe,
                accuracy: state.stat_stages.accuracy,
                evasion: state.stat_stages.evasion,
            },
            volatiles: state
                .volatiles
                .iter()
                .copied()
                .map(ConditionSnapshot::from)
                .collect(),
            can_mega_evolve: state.can_mega_evolve,
            trapped: state.trapped,
            position: PositionSnapshot::from(state.position),
        }
    }
}

impl From<rotom_core::ConditionState> for ConditionSnapshot {
    fn from(state: rotom_core::ConditionState) -> Self {
        Self {
            id: state.id.get(),
            turns_remaining: state.turns_remaining,
        }
    }
}

impl From<PokemonPosition> for PositionSnapshot {
    fn from(position: PokemonPosition) -> Self {
        match position {
            PokemonPosition::Team => Self::Team,
            PokemonPosition::Active { slot } => Self::Active { slot },
            PokemonPosition::Fainted => Self::Fainted,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn crate_boundary_is_available() {
        assert_eq!(super::CRATE_NAME, "rotom-protocol");
    }
}
