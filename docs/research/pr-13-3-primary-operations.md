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
