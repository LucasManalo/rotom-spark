use crate::{
    AbilityId, BattleError, BattleState, CombatData, ConditionId, ItemId, MajorStatus,
    MoveCategory, MoveId, PokemonRef, PokemonType, SideId,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherEnvironment {
    Rain,
    Sun,
    Sandstorm,
    Snow,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerrainEnvironment {
    Electric,
    Grassy,
    Psychic,
    Misty,
}

/// Static identities used to attach field mechanics to compact condition IDs.
///
/// The IDs live in immutable mechanics configuration rather than mutable
/// battle state. Missing identities disable only that mechanic, which lets
/// focused tests and partial Dex implementations opt into a supported slice.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEffectIds {
    pub rain: Option<ConditionId>,
    pub sun: Option<ConditionId>,
    pub sandstorm: Option<ConditionId>,
    pub snow: Option<ConditionId>,
    pub electric_terrain: Option<ConditionId>,
    pub grassy_terrain: Option<ConditionId>,
    pub psychic_terrain: Option<ConditionId>,
    pub misty_terrain: Option<ConditionId>,
    pub reflect: Option<ConditionId>,
    pub light_screen: Option<ConditionId>,
    pub aurora_veil: Option<ConditionId>,
    pub tailwind: Option<ConditionId>,
    pub trick_room: Option<ConditionId>,
    pub gravity: Option<ConditionId>,
    pub magic_room: Option<ConditionId>,
    pub wonder_room: Option<ConditionId>,
    pub ingrain: Option<ConditionId>,
    pub smack_down: Option<ConditionId>,
    pub magnet_rise: Option<ConditionId>,
    pub telekinesis: Option<ConditionId>,
    pub levitate: Option<AbilityId>,
    pub iron_ball: Option<ItemId>,
    pub air_balloon: Option<ItemId>,
    pub earthquake: Option<MoveId>,
    pub bulldoze: Option<MoveId>,
    pub magnitude: Option<MoveId>,
    pub thunder: Option<MoveId>,
    pub hurricane: Option<MoveId>,
    pub blizzard: Option<MoveId>,
    pub protect: Option<ConditionId>,
    pub follow_me: Option<ConditionId>,
    pub rage_powder: Option<ConditionId>,
    pub stall: Option<ConditionId>,
    pub overcoat: Option<AbilityId>,
    pub safety_goggles: Option<ItemId>,
}

impl FieldEffectIds {
    pub(crate) fn weather_environment(&self, battle: &BattleState) -> Option<WeatherEnvironment> {
        let weather = battle.field.weather?.id;
        [
            (self.rain, WeatherEnvironment::Rain),
            (self.sun, WeatherEnvironment::Sun),
            (self.sandstorm, WeatherEnvironment::Sandstorm),
            (self.snow, WeatherEnvironment::Snow),
        ]
        .into_iter()
        .find_map(|(id, environment)| (id == Some(weather)).then_some(environment))
    }

    pub(crate) fn effective_weather_environment<D: CombatData>(
        &self,
        battle: &BattleState,
        data: &D,
    ) -> Option<WeatherEnvironment> {
        let suppressed = [SideId::P1, SideId::P2].into_iter().any(|side| {
            battle.side(side).active.into_iter().flatten().any(|index| {
                battle.side(side).pokemon(index).is_some_and(|pokemon| {
                    data.ability_has_behavior(pokemon.ability, crate::AbilityBehavior::CloudNine)
                })
            })
        });
        (!suppressed)
            .then(|| self.weather_environment(battle))
            .flatten()
    }

    pub(crate) fn terrain_environment(&self, battle: &BattleState) -> Option<TerrainEnvironment> {
        let terrain = battle.field.terrain?.id;
        [
            (self.electric_terrain, TerrainEnvironment::Electric),
            (self.grassy_terrain, TerrainEnvironment::Grassy),
            (self.psychic_terrain, TerrainEnvironment::Psychic),
            (self.misty_terrain, TerrainEnvironment::Misty),
        ]
        .into_iter()
        .find_map(|(id, environment)| (id == Some(terrain)).then_some(environment))
    }

    pub(crate) fn field_has(&self, battle: &BattleState, id: Option<ConditionId>) -> bool {
        if let Some(id) = id {
            debug_assert!(self.contains_condition(id));
        }
        id.is_some_and(|id| {
            battle.field.weather.is_some_and(|state| state.id == id)
                || battle.field.terrain.is_some_and(|state| state.id == id)
                || battle.field.conditions.iter().any(|state| state.id == id)
        })
    }

    pub(crate) fn side_has(
        &self,
        battle: &BattleState,
        side: SideId,
        id: Option<ConditionId>,
    ) -> bool {
        if let Some(id) = id {
            debug_assert!(self.contains_condition(id));
        }
        id.is_some_and(|id| {
            battle
                .side(side)
                .side_conditions
                .iter()
                .any(|state| state.id == id)
        })
    }

    pub(crate) fn item_is_active(&self, battle: &BattleState) -> bool {
        !self.field_has(battle, self.magic_room)
    }

    pub(crate) fn item_is_active_for<D: CombatData>(
        &self,
        battle: &BattleState,
        data: &D,
        target: PokemonRef,
    ) -> bool {
        self.item_is_active(battle)
            && battle
                .side(target.side)
                .pokemon(target.pokemon)
                .is_some_and(|pokemon| {
                    !data.ability_has_behavior(pokemon.ability, crate::AbilityBehavior::Klutz)
                })
    }

    pub(crate) fn action_speed(&self, battle: &BattleState, side: SideId, speed: u16) -> u16 {
        let speed = if self.side_has(battle, side, self.tailwind) {
            speed.saturating_mul(2).min(10_000)
        } else {
            speed
        };
        if self.field_has(battle, self.trick_room) {
            10_000_u16.saturating_sub(speed)
        } else {
            speed
        }
    }

    pub(crate) fn gravity_accuracy(&self, battle: &BattleState, accuracy: u16) -> u16 {
        if self.field_has(battle, self.gravity) {
            accuracy.saturating_mul(5) / 3
        } else {
            accuracy
        }
    }

    pub(crate) fn weather_accuracy(
        &self,
        battle: &BattleState,
        move_id: MoveId,
        accuracy: u16,
    ) -> WeatherAccuracy {
        let rain_move = [self.thunder, self.hurricane]
            .into_iter()
            .flatten()
            .any(|candidate| candidate == move_id);
        if rain_move && self.field_has(battle, self.rain) {
            return WeatherAccuracy::Always;
        }
        if rain_move && self.field_has(battle, self.sun) {
            return WeatherAccuracy::Percent(50);
        }
        if self.blizzard == Some(move_id) && self.field_has(battle, self.snow) {
            return WeatherAccuracy::Always;
        }
        WeatherAccuracy::Percent(accuracy)
    }

    pub(crate) fn grassy_weakens(&self, move_id: MoveId) -> bool {
        [self.earthquake, self.bulldoze, self.magnitude]
            .into_iter()
            .flatten()
            .any(|candidate| candidate == move_id)
    }

    pub(crate) fn field_residual_sub_order(&self, id: ConditionId) -> i16 {
        if self.trick_room == Some(id) {
            1
        } else if self.gravity == Some(id) {
            2
        } else if self.wonder_room == Some(id) {
            5
        } else if self.magic_room == Some(id) {
            6
        } else {
            7
        }
    }

    fn contains_condition(&self, id: ConditionId) -> bool {
        [
            self.rain,
            self.sun,
            self.sandstorm,
            self.snow,
            self.electric_terrain,
            self.grassy_terrain,
            self.psychic_terrain,
            self.misty_terrain,
            self.reflect,
            self.light_screen,
            self.aurora_veil,
            self.tailwind,
            self.trick_room,
            self.gravity,
            self.magic_room,
            self.wonder_room,
            self.ingrain,
            self.smack_down,
            self.magnet_rise,
            self.telekinesis,
            self.protect,
            self.follow_me,
            self.rage_powder,
            self.stall,
        ]
        .into_iter()
        .flatten()
        .any(|candidate| candidate == id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WeatherAccuracy {
    Always,
    Percent(u16),
}

/// Shared mechanics query used by terrain, hazards, and Ground interactions.
///
/// This implements the common static cases available in the current state
/// model: Gravity/Ingrain/Smack Down and Iron Ball ground a target; Flying,
/// Levitate, Magnet Rise, Telekinesis, and Air Balloon make it airborne.
/// Magic Room suppresses the two item checks. Ability suppression and
/// semi-invulnerable move states remain separate mechanics.
pub(crate) fn grounded<D: CombatData>(
    battle: &BattleState,
    data: &D,
    ids: &FieldEffectIds,
    target: PokemonRef,
) -> Result<bool, BattleError> {
    let pokemon = battle
        .side(target.side)
        .pokemon(target.pokemon)
        .ok_or_else(|| BattleError::InvalidState("grounded target is out of bounds".to_owned()))?;
    let has_volatile = |id: Option<ConditionId>| {
        id.is_some_and(|id| pokemon.volatiles.iter().any(|state| state.id == id))
    };
    if ids.field_has(battle, ids.gravity)
        || has_volatile(ids.ingrain)
        || has_volatile(ids.smack_down)
        || (ids.item_is_active_for(battle, data, target)
            && ids.iron_ball.is_some_and(|item| pokemon.item == Some(item)))
    {
        return Ok(true);
    }
    let types = data
        .pokemon_types(pokemon.species, pokemon.form)
        .ok_or_else(|| BattleError::DataMismatch("missing grounded-query type data".to_owned()))?;
    if types.primary == PokemonType::Flying || types.secondary == Some(PokemonType::Flying) {
        return Ok(false);
    }
    if ids
        .levitate
        .is_some_and(|ability| pokemon.ability == ability)
        || has_volatile(ids.magnet_rise)
        || has_volatile(ids.telekinesis)
        || (ids.item_is_active_for(battle, data, target)
            && ids
                .air_balloon
                .is_some_and(|item| pokemon.item == Some(item)))
    {
        return Ok(false);
    }
    Ok(true)
}

pub(crate) fn terrain_allows_status<D: CombatData>(
    battle: &BattleState,
    data: &D,
    ids: &FieldEffectIds,
    target: PokemonRef,
    status: MajorStatus,
) -> Result<bool, BattleError> {
    if !grounded(battle, data, ids, target)? {
        return Ok(true);
    }
    if ids.field_has(battle, ids.misty_terrain) {
        return Ok(false);
    }
    Ok(!(status == MajorStatus::Sleep && ids.field_has(battle, ids.electric_terrain)))
}

pub(crate) fn screen_modifier(
    battle: &BattleState,
    ids: &FieldEffectIds,
    target_side: SideId,
    category: MoveCategory,
    critical: bool,
) -> Option<(u32, u32)> {
    if critical {
        return None;
    }
    let matching_screen = match category {
        MoveCategory::Physical => ids.side_has(battle, target_side, ids.reflect),
        MoveCategory::Special => ids.side_has(battle, target_side, ids.light_screen),
        MoveCategory::Status => false,
    };
    if !matching_screen && !ids.side_has(battle, target_side, ids.aurora_veil) {
        return None;
    }
    let active = battle
        .side(target_side)
        .active
        .into_iter()
        .flatten()
        .count();
    Some(if active > 1 { (2732, 4096) } else { (1, 2) })
}

pub(crate) fn weather_damage_modifier(
    battle: &BattleState,
    ids: &FieldEffectIds,
    move_type: PokemonType,
) -> Option<(u32, u32)> {
    if ids.field_has(battle, ids.rain) {
        return match move_type {
            PokemonType::Water => Some((3, 2)),
            PokemonType::Fire => Some((1, 2)),
            _ => None,
        };
    }
    if ids.field_has(battle, ids.sun) {
        return match move_type {
            PokemonType::Fire => Some((3, 2)),
            PokemonType::Water => Some((1, 2)),
            _ => None,
        };
    }
    None
}
