# Regulation M-C mechanics manifest research

## Reference

- Pokémon Showdown commit: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`
- Format: `gen9championsvgc2026regmc`
- Resolved mod: `champions`
- Format declaration: `config/formats.ts`, entry
  `[Gen 9 Champions] VGC 2026 Reg M-C`
- Set legality: `sim/team-validator.ts`, `TeamValidator.validateSet` and
  `TeamValidator.checkCanLearn`
- Resolved learnsets: `sim/dex-species.ts`, `DexSpecies.getMovePool` and
  `DexSpecies.getFullLearnset`
- Form transformation validation: `sim/team-validator.ts`,
  `TeamValidator.getValidationSpecies` and `validateForme`

## Inventory method

The broad resolved export remains `data/generated/champions-reg-mc.json`. The
PR 13.0 manifest does not use its per-entry `legal` flags as sufficient proof
of reachability.

For each resolved non-battle-only form, the generator constructs complete
Champions sets with a legal nature, IVs, Stat Points, ability slot, required
item/move, and one candidate learnset move. It passes each set through
`TeamValidator.validateSet`. A form is a direct starting form only when the
validator preserves that exact species identity; this prevents Showdown's
`-Gmax` input normalization from creating false starting-form entries.

A move or ability is direct only when at least one complete set validates with
it. An item is direct only when at least one already validated starting set
also validates while holding it. The current format has Species Clause and
Item Clause but no complex team bans, so those uniqueness rules do not remove
individual set reachability.

Battle-only forms are added when their `battleOnly` source is directly
reachable and any `requiredItem` is itself legal. Structured status,
volatile, weather, terrain, side, slot, and pseudo-weather references are
closed over from reachable moves, abilities, and items.

`Copycat` and `Sleep Talk` are the only directly reachable moves whose resolved
data has `callsMove`. They select moves already present in a legal battle set
or its move history, so they do not widen the move-ID set. Callback bodies that
can dynamically add conditions or invoke moves are listed separately under
`unresolved_reachability`; the manifest does not pretend structured metadata
proved those closures.

## Coverage claims

`data/mechanics-coverage.json` is the reviewed claim input. The generator
cross-checks implemented move claims against `BASIC_DAMAGE_MOVES`, requires
evidence and an owner for non-unsupported claims, and verifies differential
claims against the pinned basic-damage fixture. All other reachable entries
default to `unsupported` and `not_tested`.

This PR inventories scope only. It does not claim behavior parity for an
ability, item, form, condition, or unimplemented move.
