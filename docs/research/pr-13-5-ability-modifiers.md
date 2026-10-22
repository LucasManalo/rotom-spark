# PR 13.5 ability modifier batches

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

The next cohesive batch extends those relays to Speed and critical-hit rules:

- `data/abilities.ts::quickfeet` multiplies Speed by 1.5 while statused, while
  `data/conditions.ts::par` skips the ordinary paralysis Speed penalty for a
  Quick Feet holder. Rotom models these as two independent descriptors on the
  same stable ability identity.
- `data/abilities.ts::superluck` adds one critical-hit stage through
  `onModifyCritRatio`.
- `data/abilities.ts::battlearmor` and `shellarmor` reject a rolled critical
  hit through `onCriticalHit`. Both callbacks are marked `breakable`.
- `data/abilities.ts::sniper` multiplies final damage by 1.5 when the resolved
  hit is critical.
- `data/abilities.ts::moldbreaker` sets `move.ignoreAbility` during
  `onModifyMove`, suppressing breakable target ability callbacks for that move.

The third batch adds field-dependent Speed and non-mutating hit immunities:

- `chlorophyll`, `swiftswim`, and `slushrush` double Speed in sun, rain, and
  snow respectively; `surgesurfer` doubles Speed in Electric Terrain.
- `bulletproof` rejects moves carrying Showdown's `bullet` flag.
- `soundproof` rejects moves carrying Showdown's `sound` flag.
- `telepathy` rejects non-status moves from an ally.
- Bulletproof, Soundproof, and Telepathy are `breakable`; the four Speed
  handlers are not. Mold Breaker therefore bypasses the hit immunities without
  suppressing unrelated source-owned callbacks.

The fourth batch implements the two deferred compound abilities:

- `data/abilities.ts::sandrush` doubles Speed in sandstorm and returns false
  from `onImmunity('sandstorm')`. Rotom registers one declarative behavior for
  both `ModifySpeed` and the typed `WeatherImmunity` hook used before residual
  weather damage.
- `data/abilities.ts::prankster` adds one priority to status moves and marks
  the active move as Prankster-boosted. `sim/battle-actions.ts::hitStepTryImmunity`
  then rejects opposing Dark-type targets after Protect and type immunity but
  before accuracy. Rotom carries that queue-time provenance into a typed
  `TryImmunity` event and resolves it independently for every target.
- Showdown's `TryHit` ordering places Psychic Terrain at priority 4, Protect at
  priority 3, and ordinary Bulletproof/Soundproof/Telepathy handlers later.
  Rotom's shared hit lifecycle now preserves that order, so a protected holder
  reports `Protected` rather than exposing its ability immunity.

Showdown's modifier application uses its fixed-point `chainModify`/`modify`
rounding. Rotom Spark applies the same 4096-denominator rounding to each typed
integer relay.

## Rotom Spark mapping and registration

`CombatData::ability_handlers` declaratively joins stable generated-Dex
identities to native `AbilityBehavior` families. The generated Dex remains the
identity/metadata authority; `rotom-core` remains the behavior authority.
`Mechanics::new` expands those descriptors into stateless registrations and
composes them with common condition handlers and caller-provided effects.
Common families share Rust callbacks, while genuinely exceptional behaviors
may retain narrow callbacks. This table is neither a parallel Dex nor an
interpreted ability language.

One ability identity may produce multiple independent `AbilityHandler`
descriptors. Registration enumerates the table without identity-specific
branches, so Quick Feet's status multiplier and paralysis exception compose
through the same path used by single-handler abilities.

Every descriptor now requires an explicit `AbilitySuppression` classification.
The common registration helper derives suppressible registry entries from that
classification, so adding a callback without choosing breakable or unbreakable
behavior is a compile-time error.

`EffectRegistry` indexes registrations by event and concrete source identity.
Dispatch selects one event bucket, visits each active holder once, performs
direct ability/item/status lookups, then applies the existing deterministic
ordering comparator. The handlers inspect authoritative battle state through
`EffectState`, modify only ephemeral integer relays, and perform no direct
gameplay-state writes.

Move execution carries a narrow suppression flag derived from the source
ability's declarative behavior. The registry omits only suppressible,
target-owned ability callbacks; unrelated source abilities, conditions, and
non-breakable callbacks retain their ordinary ordering and semantics. This is
not a global ability-activation model and does not claim support for Neutralizing
Gas, Gastro Acid, ability copying, or replacement.

Queue events carry typed weather/terrain context, and move `TryHit` events carry
typed category and compact bullet/sound flags. Ability callbacks consume those
shared facts rather than adding ability identity checks to `combat.rs`.
Prankster additionally carries a derived queue-time marker into `TryImmunity`,
and target typing is supplied only at that typed immunity stage. Sandstorm
residuals use `WeatherImmunity` before requesting kernel-backed HP damage.

Physical and special calculations now dispatch distinct Attack/Special Attack
and Defense/Special Defense events. Stat-override moves select the hook for the
actual effective stat, allowing Defense-based offense to reuse Defense
ability handlers rather than inventing a move-specific path.

## Verification boundary

The pinned `ability-modifiers.json` cases exercise isolated Showdown damage,
Speed, priority, critical-ratio, immunity, and `TryHit` calculations. They support each ability's
`verification: differential_verified` claim, but they are not full
cross-mechanic transition comparisons. The current end-to-end Rotom tests
provide interaction-level unit evidence only, so all twenty-two entries use
`interaction_verification: unit_tested`. A future
`interaction_verification: differential_verified` claim requires a normalized
fixture that actually executes and compares the relevant ability interaction.

## Deferred PR 13.5 families

These twenty-two abilities are incremental PR 13.5 batches, not completion of
the milestone. The other 193 reachable Regulation M-C abilities remain
explicitly unsupported. The remaining families and prerequisites are:

| Family | Required event/mechanics capability |
| --- | --- |
| Switch-in/out | complete switch lifecycle, ordered field/side changes, stat and form consequences |
| Priority and remaining Speed | side-condition context, item-dependent Speed, probabilistic priority, suppression and tie-order coverage |
| Attack type and damage | typed move/source context, immunity ordering, shared damage modifiers |
| Remaining immunity and absorption | type/grounded eligibility, cancellation/result contracts, kernel-backed healing, boosts, and persistent activation state |
| Status prevention | shared `TrySetStatus` reasons, source/target context and cure/prevention ordering |
| Weather and terrain | switch-in setters, duration/override rules and field-condition ordering |
| Contact and on-hit reactions | contact metadata, actual applied damage, per-hit callbacks and faint settlement |
| Global suppression/copy/override | authoritative activation state, ability replacement/copy rules and exception ordering beyond move-scoped Mold Breaker |

The original PR 13.5 requirements still apply: every reachable ability must use
the common mechanics/event fabric, have explicit ordering and activation rules,
and receive isolation plus representative cross-source interaction evidence.
