# Common conditions research

Reference: Pokémon Showdown commit
`b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`, Champions mod,
`gen9championsvgc2026regmc`.

## Source paths

- `data/conditions.ts` defines base burn, paralysis, sleep, freeze, poison,
  toxic, confusion, flinch, and other volatile behavior.
- `data/mods/champions/conditions.ts` overrides paralysis, sleep, and freeze.
- `sim/battle.ts` `fieldEvent('Residual')` collects residual handlers, sorts
  them by event order and holder Speed, and expires timed effects.
- `sim/battle-actions.ts` applies the burn damage reduction after type and
  critical modifiers and before the general `ModifyDamage` event.

## Implemented behavior

Stat stages change through a bounded `BattleStat` API and use the standard
Gen 9 numerator/denominator table. Speed stages feed action ordering; accuracy
and evasion continue to feed hit checks. Switching clears stages.

Major status application is exclusive and initializes serialized state.
Champions sleep samples `[2, 3, 3]`. Champions freeze starts at three move
attempts and has a one-in-four early thaw check. Champions paralysis halves
Speed and uses a one-in-eight full-paralysis check. Every random decision uses
the battle RNG and its dedicated trace reason.

Burn halves ordinary physical damage. Poison deals one eighth max HP, burn one
sixteenth, and toxic increments from one sixteenth through a capped stage 15.
Residual handlers use Showdown orders 9 for poison/toxic and 10 for burn, then
resolve fainting as one ordered batch. Toxic's counter resets on switch-in.

Volatiles have duplicate-safe add/remove/query operations. Timed active
volatiles decrement during upkeep and expired identities are reported in the
turn result. The checked fixture at
`data/fixtures/differential/common-conditions.json` directly invokes the
resolved pinned Champions callbacks for status odds, durations, residual
fractions, and order metadata.

## Deferred behavior

Move-specific and ability-specific exceptions belong to their focused effect
batches. This includes Facade and Guts bypassing the burn penalty, sleep-usable
and defrosting moves, Fire-move thawing, Early Bird, Quick Feet, status/type
immunities, and concrete confusion/flinch behavior. Volatile end callbacks and
their interleaving with field and side-condition durations are part of the
next field-effect milestone. These cases are not silently selected by the
common APIs; the requesting move, ability, or item must implement them.
