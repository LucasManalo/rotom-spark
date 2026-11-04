//! Deterministic static damage calculation seams.
//!
//! Live move execution remains owned by the single canonical executor in the
//! parent module. These functions calculate already-resolved damage inputs and
//! never mutate battle state or consume the battle RNG.

use super::{
    combined_type_modifier, preflight_move, showdown_modify, staged_stat, CombatData, CombatMove,
    DamageEnvironment, DamageResult, Modifier, MoveCapability, MoveCategory, MoveId, MoveIntrinsic,
    PokemonState, PokemonTypes,
};
use crate::{BattleError, MajorStatus};

#[allow(clippy::too_many_arguments)]
/// Calculates one normal Gen 9 damage result from already-resolved inputs.
///
/// # Errors
///
/// Returns an error for status moves or a random percentage outside 85–100.
pub fn calculate_damage(
    source: &PokemonState,
    target: &PokemonState,
    move_data: CombatMove,
    source_types: PokemonTypes,
    type_exponent: Option<i8>,
    critical: bool,
    random_percent: u8,
) -> Result<DamageResult, BattleError> {
    calculate_damage_inner(
        source,
        target,
        move_data,
        source_types,
        type_exponent,
        critical,
        random_percent,
        DamageEnvironment::default(),
    )
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn calculate_damage_inner(
    source: &PokemonState,
    target: &PokemonState,
    move_data: CombatMove,
    source_types: PokemonTypes,
    type_exponent: Option<i8>,
    critical: bool,
    random_percent: u8,
    environment: DamageEnvironment,
) -> Result<DamageResult, BattleError> {
    if !(85..=100).contains(&random_percent) {
        return Err(BattleError::InvalidState(
            "damage roll must be between 85 and 100".to_owned(),
        ));
    }
    let (raw_attack, raw_defense, attack_stage, defense_stage) = match move_data.category {
        MoveCategory::Physical => (
            source.stats.atk,
            target.stats.def,
            source.stat_stages.atk,
            target.stat_stages.def,
        ),
        MoveCategory::Special => (
            source.stats.spa,
            target.stats.spd,
            source.stat_stages.spa,
            target.stat_stages.spd,
        ),
        MoveCategory::Status => {
            return Err(BattleError::UnsupportedMechanic(
                "status moves do not use the damage formula".to_owned(),
            ));
        }
    };
    let attack_stage = if critical && attack_stage < 0 {
        0
    } else {
        attack_stage
    };
    let defense_stage = if critical && defense_stage > 0 {
        0
    } else {
        defense_stage
    };
    let attack = environment
        .attack_override
        .unwrap_or_else(|| staged_stat(raw_attack, attack_stage));
    let mut defense = environment
        .defense_override
        .unwrap_or_else(|| staged_stat(raw_defense, defense_stage))
        .max(1);
    if let Some((numerator, denominator)) = environment.defense_modifier {
        defense = showdown_modify(defense, numerator, denominator).max(1);
    }
    let level_factor = (2 * u32::from(source.level) / 5) + 2;
    let base_damage = level_factor
        .saturating_mul(u32::from(move_data.base_power))
        .saturating_mul(attack)
        / defense
        / 50;
    let mut final_damage = base_damage + 2;
    if environment.spread {
        final_damage = showdown_modify(final_damage, 3, 4);
    }
    if let Some((numerator, denominator)) = environment.weather {
        final_damage = showdown_modify(final_damage, numerator, denominator);
    }
    let critical_modifier = if critical {
        final_damage = final_damage.saturating_mul(3) / 2;
        Modifier {
            numerator: 3,
            denominator: 2,
        }
    } else {
        Modifier::NEUTRAL
    };
    final_damage = final_damage.saturating_mul(u32::from(random_percent)) / 100;
    let has_stab = source_types.primary == move_data.move_type
        || source_types.secondary == Some(move_data.move_type);
    let stab_modifier = if has_stab {
        let (numerator, denominator) = if environment.adaptability {
            (2, 1)
        } else {
            (3, 2)
        };
        final_damage = showdown_modify(final_damage, numerator, denominator);
        Modifier {
            numerator: u16::try_from(numerator).unwrap_or(u16::MAX),
            denominator: u16::try_from(denominator).unwrap_or(u16::MAX),
        }
    } else {
        Modifier::NEUTRAL
    };
    let (typed_damage, type_modifier) = apply_type_modifier(final_damage, type_exponent);
    final_damage = typed_damage;
    if source.status == Some(MajorStatus::Burn)
        && move_data.category == MoveCategory::Physical
        && !environment.ignore_burn
    {
        final_damage /= 2;
    }
    if type_exponent.is_some() && final_damage == 0 {
        final_damage = 1;
    }
    Ok(DamageResult {
        base_damage,
        attack_stat: attack,
        defense_stat: defense,
        stab_modifier,
        type_modifier,
        critical_modifier,
        random_modifier: Modifier {
            numerator: u16::from(random_percent),
            denominator: 100,
        },
        final_damage: u16::try_from(final_damage.min(u32::from(u16::MAX))).unwrap_or(u16::MAX),
    })
}

fn apply_type_modifier(damage: u32, exponent: Option<i8>) -> (u32, Modifier) {
    match exponent {
        None => (
            0,
            Modifier {
                numerator: 0,
                denominator: 1,
            },
        ),
        Some(value) if value > 0 => {
            let factor = 1_u16
                .checked_shl(u32::from(value.unsigned_abs()))
                .unwrap_or(u16::MAX);
            (
                damage.saturating_mul(u32::from(factor)),
                Modifier {
                    numerator: factor,
                    denominator: 1,
                },
            )
        }
        Some(value) if value < 0 => {
            let factor = 1_u16
                .checked_shl(u32::from(value.unsigned_abs()))
                .unwrap_or(u16::MAX);
            (
                damage / u32::from(factor),
                Modifier {
                    numerator: 1,
                    denominator: factor,
                },
            )
        }
        Some(_) => (damage, Modifier::NEUTRAL),
    }
}

/// Resolves static typing and calculates one forced-outcome damage value.
///
/// This is the deterministic seam used by focused Showdown reference-derived
/// assertions. Turn resolution supplies its rolled critical and random values.
///
/// # Errors
///
/// Returns an error when static data is missing, the move requires a capability
/// outside the static damage seam, or damage inputs are invalid. Moves with
/// state-dependent power or effective-stat behavior must run through the
/// authoritative transition executor.
pub fn calculate_move_damage<D: CombatData>(
    data: &D,
    source: &PokemonState,
    target: &PokemonState,
    move_id: MoveId,
    critical: bool,
    random_percent: u8,
) -> Result<DamageResult, BattleError> {
    let move_data = data.move_data(move_id).ok_or_else(|| {
        BattleError::DataMismatch(format!("missing combat data for move {move_id}"))
    })?;
    let transition_only = [
        MoveCapability::ConditionalPower,
        MoveCapability::EffectiveStatOverride,
    ]
    .into_iter()
    .find(|capability| move_data.required_capabilities.contains(*capability));
    if let Some(capability) = transition_only {
        return Err(BattleError::UnsupportedMechanic(format!(
            "move {move_id} requires transition-only capability {capability}"
        )));
    }
    let secondaries = data.secondary_move_effects(move_id);
    let self_effect = data.self_move_effect(move_id);
    preflight_move(
        move_id,
        move_data,
        MoveIntrinsic::Ordinary,
        &[],
        &secondaries,
        self_effect.as_ref(),
    )?;
    let source_types = data
        .pokemon_types(source.species, source.form)
        .ok_or_else(|| BattleError::DataMismatch("missing source type data".to_owned()))?;
    let target_types = data
        .pokemon_types(target.species, target.form)
        .ok_or_else(|| BattleError::DataMismatch("missing target type data".to_owned()))?;
    let type_exponent = combined_type_modifier(data, move_data.move_type, target_types)?;
    calculate_damage(
        source,
        target,
        move_data,
        source_types,
        type_exponent,
        critical,
        random_percent,
    )
}
