# PR 13.5 ability batch: shared modifiers and reactions

Behavioral reference remains the repository's unchanged `SHOWDOWN_VERSION` pin,
`b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`, using
`vendor/pokemon-showdown/data/abilities.ts` plus Champions overrides in
`vendor/pokemon-showdown/data/mods/champions/abilities.ts`.

This batch adds 30 reachable Regulation M-C abilities through reusable native
behaviors rather than identity branches:

- Normal conversion: Aerilate, Pixilate, Refrigerate, Dragonize.
- Shared offensive/order modifiers: Fairy Aura, Plus, Minus, Quick Draw,
  Supreme Overlord, Sheer Force, Stall.
- Accuracy, recoil, and stage rules: Illuminate, No Guard, Rock Head, Eelevate,
  Contrary, Defiant, Competitive, Unaware.
- Lifecycle and defensive reactions: Berserk, Speed Boost, Leaf Guard, Pressure,
  Rattled, Heatproof, Flower Veil, Mirror Armor, Opportunist, Mummy, and
  Wandering Spirit.

The implementation follows the pinned hook boundaries: type conversion occurs
before base-power modification; Quick Draw and Stall use a fractional-priority
queue key; Defiant, Competitive, and Opportunist observe an applied stat-stage
change; Berserk and contact reactions use `AfterDamage`; and ability replacement
uses the mutation kernel.

## Deliberate partial coverage

The coverage registry remains conservative where the engine does not yet expose
the complete Showdown substrate:

- Normal converters do not claim dynamic-type move interactions that are still
  rejected as unsupported by move preflight.
- No Guard does not yet bypass semi-invulnerable states.
- Speed Boost cannot distinguish a Pokémon switched in during the current turn
  because `activeTurns` is not represented.
- Leaf Guard and Flower Veil do not block the unimplemented Yawn volatile.
- Rattled does not yet react to Intimidate.
- Heatproof does not yet halve burn residual damage.
- Mirror Armor reflects through the kernel but does not recursively dispatch the
  reflected stage event; Opportunist applies supported boosts immediately rather
  than batching a multi-stat boost action.
- Mummy and Wandering Spirit cannot yet consult generated `cantsuppress` and
  `noability` flags, so their exceptional ability exclusions remain deferred.
- Eelevate provides its pinned airborne Ground immunity, but its post-faint
  best-stat boost remains deferred.

These gaps are labeled `partial`; no differential-equivalence claim is made for
this batch.
