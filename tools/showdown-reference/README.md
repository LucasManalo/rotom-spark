# Showdown reference adapter

This directory contains development-only adapters around the pinned Pokémon
Showdown checkout. It is never used by Rotom Spark at runtime.

Validate the shared Regulation M-C fixture with:

```text
node tools/showdown-reference/validate-team.cjs data/fixtures/teams/valid-reg-mc.txt
```

Regenerate or verify the base-damage fixture (all 16 rolls, criticals, stat
stages, low-damage rounding, and immunity) with:

```text
node tools/showdown-reference/basic-damage.cjs
node tools/showdown-reference/basic-damage.cjs --check
```

Regenerate or verify the canonical-step case selection built from those pinned
damage outcomes. This records independently selected RNG outcomes and expected
HP/PP/faint consequences for controlled-outcome Rotom Spark integration tests.
It neither claims seed parity nor executes an independent complete Showdown
battle transition:

```text
node tools/showdown-reference/canonical-step.cjs
node tools/showdown-reference/canonical-step.cjs --check
```

Regenerate or verify the event-listener ordering fixture produced directly by
the pinned `Battle.comparePriority` implementation with:

```text
node tools/showdown-reference/event-order.cjs
node tools/showdown-reference/event-order.cjs --check
```

Regenerate or verify the common-condition fixture produced by invoking the
resolved Champions status callbacks with controlled random outcomes:

```text
node tools/showdown-reference/common-conditions.cjs
node tools/showdown-reference/common-conditions.cjs --check
```

Regenerate or verify the field-effect callback constants, durations, and
residual ordering metadata with:

```text
node tools/showdown-reference/field-effects.cjs
node tools/showdown-reference/field-effects.cjs --check
```

Regenerate or verify primary non-damage move success, failure, rounding,
reapplication, protection, and multi-effect behavior with:

```text
node tools/showdown-reference/primary-non-damage.cjs
node tools/showdown-reference/primary-non-damage.cjs --check
```

Regenerate or verify PR 13.4 move sequencing, including controlled hit-count,
later-hit accuracy, critical-hit, damage-roll, secondary-effect, self-effect,
drain, and recoil behavior with:

```text
node tools/showdown-reference/sequenced-effects.cjs
node tools/showdown-reference/sequenced-effects.cjs --check
```

Regenerate or verify the PR 13.5 stat, base-power, damage, priority, hit-gate,
weather-immunity, and major-status ability fixture with:

```text
node tools/showdown-reference/ability-modifiers.cjs
node tools/showdown-reference/ability-modifiers.cjs --check
```
