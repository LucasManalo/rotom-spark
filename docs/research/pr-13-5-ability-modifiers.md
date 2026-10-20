# PR 13.5 ability modifier handlers

Pinned reference: Pokémon Showdown commit
`b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`, resolved format
`gen9championsvgc2026regmc`, mod `champions`.

## Source findings

The first ability batch is limited to behavior already expressible by the
unified move executor's calculation relays:

- `data/abilities.ts::hugepower` and `purepower` use `onModifyAtk` at priority
  5 and double Attack.
- `data/abilities.ts::guts` uses `onModifyAtk` at priority 5, multiplies Attack
  by 1.5 while statused, and `data/conditions.ts::brn` omits its Attack penalty
  for Guts holders.
- `data/abilities.ts::furcoat` doubles Defense at priority 6.
- `data/abilities.ts::marvelscale` multiplies Defense by 1.5 at priority 6
  while statused.
- `data/abilities.ts::technician` modifies base power at priority 30 and
  multiplies by 1.5 when the base power after earlier modifiers is at most 60.
- `data/abilities.ts::multiscale` uses
  `onSourceModifyDamage` to halve damage while the target is at full HP.

Showdown's modifier application uses its fixed-point `chainModify`/`modify`
rounding. Rotom Spark applies the same 4096-denominator rounding to each typed
integer relay.

## Rotom Spark mapping

`CombatData::ability_effect_ids` resolves stable generated-Dex identities.
`Mechanics::new` composes their stateless registrations with common condition
handlers and caller-provided effects. The handlers inspect authoritative
battle state through `EffectState`, modify only ephemeral integer relays, and
perform no direct gameplay-state writes.

Physical and special calculations now dispatch distinct Attack/Special Attack
and Defense/Special Defense events. Stat-override moves select the hook for the
actual effective stat, allowing Defense-based offense to reuse Defense
ability handlers rather than inventing a move-specific path.

## Deferred ability families

Switch-in/out effects, priority and Speed changes, attack-type modifiers,
immunity/absorption, status prevention, weather/terrain setters, contact and
on-hit reactions, ability suppression, copying, replacement, and override
exceptions need additional typed context or lifecycle support. They remain
unsupported and are not included in this batch's coverage claims.
