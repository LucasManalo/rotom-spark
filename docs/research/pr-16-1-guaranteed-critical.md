# PR 16.1 — Declarative guaranteed critical hits

## Pinned behavioral basis

Reference commit: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`, Champions mod
`gen9championsvgc2026regmc`.

- `data/moves.ts` declares `willCrit: true` for Flower Trick, Frost Breath,
  and Storm Throw.
- `sim/battle-actions.ts::getDamage` runs `ModifyCritRatio`, takes `willCrit`
  as the critical decision without a random draw, and then runs `CriticalHit`
  so Battle Armor, Shell Armor, and equivalent prevention can cancel it.
- The Champions damage override in `data/mods/champions/scripts.ts` consumes
  the resulting per-target critical flag and applies the ordinary 1.5×
  critical modifier and stage-bypass behavior.

The generated `guaranteed-critical.json` fixture records all three move
identities plus Flower Trick into Battle Armor. It is a pinned-reference
behavioral assertion, not PR 19 full-state differential comparison.

## Rotom Spark design

Resolved move data now exports `will_crit` as a boolean. `CombatMove` carries
that typed fact through the existing canonical executor and declares the
`guaranteed_critical` capability. Per target, the executor still dispatches
`ModifyCritRatio` in the established position. A guaranteed-critical move then
skips only the `CriticalHit` RNG draw and dispatches the existing
`CriticalHit` prevention event before base-power and damage modifiers.

No move identity branch or alternate damage pipeline was added. Critical
damage, ignored negative offensive stages, ignored positive defensive stages,
Sniper, prevention abilities, target outcomes, history, and HP application all
remain in their existing shared paths.

## Scope and limitations

This batch unlocks exactly Flower Trick, Frost Breath, and Storm Throw. It does
not implement conditional guaranteed criticals, new critical-ratio volatiles,
Lucky Chant, or unrelated `special_behavior` moves. Those identities retain
their explicit blockers.
