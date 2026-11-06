# Showdown reference adapter

This directory contains development-only adapters around the pinned Pokémon
Showdown checkout. It is never used by Rotom Spark at runtime.

The checked outputs are pinned-reference behavioral fixtures: some invoke
callbacks with controlled inputs and PR 15.2--15.6 execute selected complete
Showdown turns, after which Rotom Spark tests assert the relevant normalized
facts. They are reference-derived assertions, not a full differential state
comparison between independent engines. That term is reserved for the planned
PR 19 harness.

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

Regenerate or verify PR 15.1 Berry consumption, Cheek Pouch, Cud Chew,
Harvest, and Symbiosis callback outcomes with:

```text
node tools/showdown-reference/berry-lifecycle.cjs
node tools/showdown-reference/berry-lifecycle.cjs --check
```

Regenerate or verify PR 15.2 Eject Button queue interruption, Red Card random
dragging, and Suction Cups prevention using behavioral assertions derived from
complete pinned Showdown turns:

```text
node tools/showdown-reference/forced-switch-lifecycle.cjs
node tools/showdown-reference/forced-switch-lifecycle.cjs --check
```

PR 15.3 records complete-turn flinch, King's Rock, Inner Focus, Steadfast,
and Mental Herb timing:

```text
node tools/showdown-reference/volatile-lifecycle.cjs
node tools/showdown-reference/volatile-lifecycle.cjs --check
```

PR 15.4 records complete-turn Mega form, stat, ability, item, transformed-Speed,
and incompatible-stone behavior:

```text
node tools/showdown-reference/mega-evolution-lifecycle.cjs
node tools/showdown-reference/mega-evolution-lifecycle.cjs --check
```

PR 15.5 records complete-turn Knock Off, Trick, Sticky Hold, Unburden, Mummy,
and Wandering Spirit behavior:

```text
node tools/showdown-reference/item-ability-lifecycle.cjs
node tools/showdown-reference/item-ability-lifecycle.cjs --check
```

PR 15.6 records complete-turn successful multi-hit, priority-blocked, and
protected move history and damage-source behavior:

```text
node tools/showdown-reference/move-lifecycle-history.cjs
node tools/showdown-reference/move-lifecycle-history.cjs --check
```

PR 16.1 records declarative guaranteed critical hits, critical prevention, and
pinned maximum-roll damage for Flower Trick, Frost Breath, and Storm Throw:

```text
node tools/showdown-reference/guaranteed-critical.cjs
node tools/showdown-reference/guaranteed-critical.cjs --check
```

PR 16.2 records complete-turn target phazing and Suction Cups prevention for
Circle Throw, Dragon Tail, Roar, and Whirlwind:

```text
node tools/showdown-reference/move-phazing.cjs
node tools/showdown-reference/move-phazing.cjs --check
```

PR 16.3 records Freeze-Dry's per-type Water override and dual-type
composition through complete pinned Showdown turns:

```text
node tools/showdown-reference/freeze-dry.cjs
node tools/showdown-reference/freeze-dry.cjs --check
```
