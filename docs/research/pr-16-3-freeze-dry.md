# PR 16.3 — Freeze-Dry type-effectiveness exception

## Pinned behavioral basis

Reference commit: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`, Champions format
`gen9championsvgc2026regmc`.

- `data/moves.ts::freezedry.onEffectiveness` returns `1` for the Water
  component of a target's typing.
- This is a replacement for that component, not a blanket increment to the
  combined modifier. Water becomes weak to Freeze-Dry; every other defending
  type retains ordinary Ice effectiveness.
- `data/mods/champions/moves.ts::freezedry` removes the inherited freeze
  secondary but preserves the effectiveness callback.

Consequently, pure Water is 2×, Water/Dragon is 4×, Water/Ice is neutral, and
non-Water typing follows the ordinary Ice chart.

## Rotom design

Freeze-Dry is represented as the narrow typed `MoveIntrinsic::FreezeDry` and
the verified `unique_type_effectiveness` capability. Both canonical live
execution and the immutable `calculate_move_damage` seam compose modifiers per
defending type through the same helper. Only the Water component is replaced.

The exception does not perform damage, draw RNG, dispatch events, or mutate
state. It feeds the already resolved type exponent into the shared damage
pipeline. HP application remains kernel-owned.

## Evidence and limits

`freeze-dry.json` records complete pinned Showdown turns and callback results
for Vaporeon, Kingdra, Lapras, and Snorlax. The Rust transition test confirms
the expected 2×, 4×, neutral, and ordinary compositions plus deterministic
serialization replay. These are reference-derived assertions, not PR 19
full-state differential comparisons.

Other unique effectiveness callbacks, including Flying Press and Thousand
Arrows, remain unsupported and require separate evidence.
