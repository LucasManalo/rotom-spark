# Showdown reference adapter

This directory contains development-only adapters around the pinned Pokémon
Showdown checkout. It is never used by Rotom Spark at runtime.

Validate the shared Regulation M-C fixture with:

```text
node tools/showdown-reference/validate-team.cjs data/fixtures/teams/valid-reg-mc.txt
```

Regenerate or verify the forced-maximum-roll base-damage fixture with:

```text
node tools/showdown-reference/basic-damage.cjs
node tools/showdown-reference/basic-damage.cjs --check
```
