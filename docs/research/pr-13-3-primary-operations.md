# PR 13.3 primary non-damage operations

Pinned reference: Pokémon Showdown `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`,
format `gen9championsvgc2026regmc`, mod `champions`.

The generic primary-effect order is based on
`sim/battle-actions.ts:moveHit` (boosts, healing, status, volatile status,
side condition, weather, terrain) and the corresponding rule services in
`sim/battle.ts` and `sim/pokemon.ts`. Representative declarative definitions
are `data/moves.ts:swordsdance`, `recover`, `willowisp`, `sunnyday`,
`electricterrain`, and `reflect`.

Rotom Spark normalizes those structured fields into typed
`PrimaryMoveEffect` values. Move execution performs TryHit/protection/accuracy
before primary effects, reports ordinary failed effects as successful
transition outcomes, and uses the kernel only after mechanics eligibility.
Major-status type and terrain immunities are checked outside the kernel;
ordered ability/item prevention can cancel `TrySetStatus`.

The exporter deliberately does not claim callback-driven moves or arbitrary
volatile conditions merely because they contain a status-shaped field.
Condition behavior without an implemented lifecycle remains
`special_behavior` and fails capability preflight. Secondary application and
chance timing remain PR 13.4; general ability and item registrations remain
PR 13.5/13.6.

## PR 13.3.1 source-backed corrections

`tools/showdown-reference/primary-non-damage.cjs` executes the pinned
`Battle.actions.useMove` path and records status success/failure, protection,
accuracy, sleep RNG, stage bounds, combined boosts, healing, field/side
conditions, Room toggling, and Magnet Rise behavior.

- `sim/battle-actions.ts:moveHit` uses `Math.round` for Gen 5+ move healing;
  Recover therefore heals 78 HP at an odd 155 base max HP. The later
  `Battle.heal` truncation does not undo that earlier rounding.
- `sim/field.ts:setWeather` and `setTerrain` reject identical weather or
  terrain without refreshing duration. `Side.addSideCondition` and
  `Pokemon.addVolatile` likewise reject duplicates without a restart hook, so
  screens, Tailwind, and Magnet Rise do not refresh.
- `data/moves.ts:trickroom`, `magicroom`, and `wonderroom` implement
  `onFieldRestart` by removing themselves. Their typed operation is therefore
  a toggle; Gravity has no restart hook and remains an ordinary add/fail.
- `data/moves.ts:magnetrise.onTry` rejects use during Gravity or while Ingrain
  or Smack Down is active. This narrow callback is modeled explicitly; other
  callback-driven volatiles remain unsupported.

Damage and non-damage target execution now share one staged TryHit, priority,
Protect, optional type-immunity, accuracy, and break-Protection helper. This
preserves the pinned all-targets-per-stage ordering while keeping effect
application in the primary operation.
