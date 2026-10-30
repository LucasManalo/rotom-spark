use crate::BattleError;
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// The battle mechanic responsible for a random decision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RngReason {
    Accuracy,
    CriticalHit,
    DamageRoll,
    SecondaryEffect,
    Paralysis,
    SleepDuration,
    Freeze,
    Confusion,
    MultiHit,
    SpeedTie,
    Protect,
    RandomTarget,
    RandomMove,
    /// Random activation owned by an ability callback.
    Ability,
    /// Random activation owned by a held-item callback.
    Item,
    /// A temporary typed label for mechanics that do not have a dedicated
    /// variant yet. Stable mechanics should add a specific variant instead.
    Other(String),
}

/// The operation represented by one diagnostic RNG trace entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RngOperation {
    Range { start: u64, end_exclusive: u64 },
    Chance { numerator: u64, denominator: u64 },
    Sample { len: usize },
}

/// One externally meaningful random decision made by the battle RNG.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RngTraceEntry {
    /// One-based index of the final raw PRNG draw used for this decision.
    /// Gaps indicate rejection sampling discarded one or more biased values.
    pub draw: u64,
    pub reason: RngReason,
    pub operation: RngOperation,
    /// Zero-based sampled value. For [`RngOperation::Chance`], the chance
    /// succeeds when this value is less than `numerator`.
    pub result: u64,
}

/// Cloneable, serializable battle-owned deterministic random number generator.
///
/// The generator uses `SplitMix64`. This is an engine-internal deterministic
/// stream, not an assertion of seed-for-seed Pokémon Showdown PRNG parity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleRng {
    seed: u64,
    state: u64,
    draws: u64,
    trace: Option<Vec<RngTraceEntry>>,
}

impl BattleRng {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self {
            seed,
            state: seed,
            draws: 0,
            trace: None,
        }
    }

    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    #[must_use]
    pub const fn state(&self) -> u64 {
        self.state
    }

    #[must_use]
    pub const fn draws(&self) -> u64 {
        self.draws
    }

    /// Enables tracing without changing the random stream.
    pub fn enable_trace(&mut self) {
        self.trace.get_or_insert_with(Vec::new);
    }

    /// Disables tracing and discards recorded entries without changing the stream.
    pub fn disable_trace(&mut self) {
        self.trace = None;
    }

    #[must_use]
    pub fn trace(&self) -> Option<&[RngTraceEntry]> {
        self.trace.as_deref()
    }

    /// Removes recorded entries while preserving whether tracing is enabled.
    pub fn clear_trace(&mut self) {
        if let Some(trace) = &mut self.trace {
            trace.clear();
        }
    }

    /// Samples uniformly from a half-open integer range.
    ///
    /// # Errors
    ///
    /// Returns [`BattleError::InvalidState`] when the range is empty.
    pub fn range(&mut self, reason: RngReason, range: Range<u64>) -> Result<u64, BattleError> {
        let start = range.start;
        let end_exclusive = range.end;
        let width = end_exclusive
            .checked_sub(start)
            .filter(|width| *width > 0)
            .ok_or_else(|| {
                BattleError::InvalidState("RNG range must be non-empty and ordered".to_owned())
            })?;
        let result = start + self.sample_below(width);
        self.record(
            reason,
            RngOperation::Range {
                start,
                end_exclusive,
            },
            result,
        );
        Ok(result)
    }

    /// Returns true with exactly `numerator / denominator` probability.
    ///
    /// # Errors
    ///
    /// Returns [`BattleError::InvalidState`] for a zero denominator or a
    /// numerator greater than the denominator.
    pub fn chance(
        &mut self,
        reason: RngReason,
        numerator: u64,
        denominator: u64,
    ) -> Result<bool, BattleError> {
        if denominator == 0 || numerator > denominator {
            return Err(BattleError::InvalidState(
                "RNG chance requires 0 <= numerator <= denominator and denominator > 0".to_owned(),
            ));
        }
        let result = self.sample_below(denominator);
        self.record(
            reason,
            RngOperation::Chance {
                numerator,
                denominator,
            },
            result,
        );
        Ok(result < numerator)
    }

    /// Selects one value uniformly from a non-empty slice.
    ///
    /// # Errors
    ///
    /// Returns [`BattleError::InvalidState`] when `values` is empty or its
    /// length cannot be represented by the RNG's integer domain.
    pub fn sample<'a, T>(
        &mut self,
        reason: RngReason,
        values: &'a [T],
    ) -> Result<&'a T, BattleError> {
        let len = u64::try_from(values.len()).map_err(|_| {
            BattleError::InvalidState("sample population length does not fit in u64".to_owned())
        })?;
        if len == 0 {
            return Err(BattleError::InvalidState(
                "cannot sample an empty population".to_owned(),
            ));
        }
        let result = self.sample_below(len);
        self.record(reason, RngOperation::Sample { len: values.len() }, result);
        let index = usize::try_from(result).map_err(|_| {
            BattleError::InvalidState("sample index does not fit in usize".to_owned())
        })?;
        values.get(index).ok_or_else(|| {
            BattleError::InvalidState("sample index is outside the population".to_owned())
        })
    }

    fn sample_below(&mut self, upper_exclusive: u64) -> u64 {
        // Rejection sampling avoids modulo bias. `upper_exclusive` is always
        // non-zero at call sites, so wrapping negation is well-defined here.
        let threshold = upper_exclusive.wrapping_neg() % upper_exclusive;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return value % upper_exclusive;
            }
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        self.draws = self.draws.wrapping_add(1);
        value ^ (value >> 31)
    }

    fn record(&mut self, reason: RngReason, operation: RngOperation, result: u64) {
        if let Some(trace) = &mut self.trace {
            trace.push(RngTraceEntry {
                draw: self.draws,
                reason,
                operation,
                result,
            });
        }
    }
}

impl Default for BattleRng {
    fn default() -> Self {
        Self::new(0)
    }
}
