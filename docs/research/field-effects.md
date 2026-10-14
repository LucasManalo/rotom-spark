# Field effects (PR 12)

## Scope and representation

PR 12 keeps `ConditionId` as the compact state identity and resolves semantic
weather, terrain, room, and side-condition identities once in immutable
`FieldEffectIds`. `BattleState` remains authoritative. Mechanics inspect the
field and request HP/duration writes through `StateKernel`; the kernel does not
decide weather immunity, groundedness, damage modifiers, or ordering.

`ConditionState::layers` was added for a truthful representation of layered
conditions. Entry-hazard switch effects are deliberately not claimed yet: the
current supported move slice cannot establish them, and Toxic Spikes needs the
status-immunity/source pipeline planned with later move and ability batches.

## Pinned Showdown basis

Reference commit: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`.

- `data/conditions.ts`: `raindance`, `sunnyday`, `sandstorm`, and `snowscape`.
  These define five-turn weather, Fire/Water damage changes, Sandstorm's
  residual order and Rock Sp. Def boost, and Snow's Ice Defense boost.
- `data/moves.ts`: `electricterrain`, `grassyterrain`, `psychicterrain`, and
  `mistyterrain`. These define grounded-only power/status/priority behavior,
  Grassy recovery, and field residual order 27.
- `data/moves.ts`: `reflect`, `lightscreen`, `auroraveil`, and `tailwind`.
  In doubles the screens use `2732/4096`; Tailwind doubles Speed and lasts four
  turns; side conditions use residual order 26.
- `data/moves.ts`: `gravity`, `magicroom`, `trickroom`, and `wonderroom`.
  These last five turns at field residual order 27, with sub-orders 2, 6, 1,
  and 5 respectively.
- `sim/pokemon.ts`: `Pokemon#isGrounded`, `Pokemon#calculateStat`, and
  `Pokemon#getActionSpeed`. These are the basis for the shared grounded query,
  Wonder Room's pre-stage raw-defense swap, and the `10000 - Speed` Trick Room
  ordering key.

`tools/showdown-reference/field-effects.cjs` invokes the resolved Champions
callbacks and records their constants in
`data/fixtures/differential/field-effects.json`.

## Supported behavior

- Rain/Sun Fire and Water damage plus Rain/Sun/Snow accuracy overrides for
  Thunder, Hurricane, and Blizzard.
- Sandstorm residual damage and Rock special-defense modification; Snow's Ice
  physical-defense modification.
- Electric, Grassy, Psychic, and Misty Terrain power rules; Grassy recovery;
  Electric/Misty status eligibility; Psychic priority blocking.
- Shared groundedness for Flying, Levitate, Iron Ball, Air Balloon, Gravity,
  Ingrain, Smack Down, Magnet Rise, and Telekinesis. Magic Room suppresses item
  contributions to this query.
- Reflect, Light Screen, Aurora Veil, Tailwind, Trick Room, Gravity, Magic Room,
  and Wonder Room behavior expressible by the current basic-damage slice.
- Deterministic weather, terrain, field-condition, and side-condition expiry,
  with residual HP effects occurring before the condition expires.

## Deferred behavior

Weather/terrain-setting moves, recovery moves, item duration extension,
Persistent, weather/terrain abilities, ability-based weather immunity or
suppression, semi-invulnerable states, Infiltrator, entry-hazard switch effects,
and full Magic Room item callbacks belong to later move/ability/item batches.
They must fail explicitly when they become reachable; this PR does not mark
those moves or effects supported.
