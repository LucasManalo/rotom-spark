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

Regenerate or verify the event-listener ordering fixture produced directly by
the pinned `Battle.comparePriority` implementation with:

```text
node tools/showdown-reference/event-order.cjs
node tools/showdown-reference/event-order.cjs --check
```
