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

The fifth batch adds twenty-five reachable abilities through the same typed
relays and hit/status gates:

- `blaze`, `overgrow`, `torrent`, and `swarm` multiply the appropriate
  attacking stat by 1.5 at one-third HP or below for Fire, Grass, Water, and
  Bug moves.
- `ironfist`, `strongjaw`, `sharpness`, `toughclaws`, and `reckless` consume
  pinned move metadata for punch, bite, slicing, contact, and recoil/crash
  base-power modifiers. `punkrock` combines its sound base-power boost with
  target-side sound damage reduction.
- `sandforce` combines a sand-dependent Rock/Ground/Steel power boost with
  sandstorm immunity. `steelyspirit` applies its Steel boost to the holder and
  active allies; multiple holders stack through ordinary registry ordering.
- `filter` and `solidrock` reduce only super-effective final damage. `fluffy`
  combines contact resistance with Fire weakness, `thickfat` modifies the
  attack stat for Fire/Ice moves, and `grasspelt` modifies Defense in Grassy
  Terrain at the priorities declared by pinned Showdown.
- `galewings` modifies Flying-move priority only at full HP. `goodasgold`
  rejects opposing status moves at `TryHit`.
- `immunity`, `insomnia`, `vitalspirit`, `limber`, and `magmaarmor` prevent
  their corresponding major statuses and cure an already-present prohibited
  status through kernel-backed update hooks. `sweetveil` prevents sleep for
  the holder's active side.

The sixth batch adds thirty abilities around typed immunity, lifecycle,
damaging-hit, and accuracy hooks:

- `waterabsorb`, `voltabsorb`, and `eartheater` cancel matching external hits
  and heal one quarter max HP; `motordrive` cancels Electric hits and raises
  Speed. These target-side callbacks are breakable and therefore use the
  existing per-move Mold Breaker suppression contract.
- `overcoat` combines sandstorm immunity with powder-move immunity.
  `waterbubble` and `thermalexchange` share burn prevention and
  cure hooks. Water Bubble additionally doubles Water offense and halves the
  attacking stat used by incoming Fire moves; Thermal Exchange raises Attack
  after a damaging Fire hit.
- `naturalcure` and `regenerator` run on `SwitchOut`; `hospitality` runs after
  the incoming holder occupies its active slot. `hydration`, `shedskin`, and
  `healer` run at pinned residual order 5/sub-order 3. Random ability
  activations consume the battle-owned stream under `RngReason::Ability`.
- `stamina`, `weakarmor`, `justified`, `thermalexchange`, `gooey`, and
  `angerpoint` consume actual applied damage and typed hit metadata through
  `AfterDamage`. They do not activate after a knockout. Anger Point consumes
  the already-resolved critical result and clamps Attack through the kernel.
- `friendguard`, `megalauncher`, `merciless`, `compoundeyes`, and `hustle` use
  the shared damage, base-power, critical-ratio, Attack, and new final-accuracy
  relays. Pulse and powder are compact move flags exported from the pinned
  resolved Dex.
- `icebody` and `raindish` heal at the weather residual boundary; `moxie`
  consumes the existing source-aware `Faint` event and boosts once per foe
  fainted by a move.
- `anticipation`, `forewarn`, and `frisk` only reveal information in pinned
  Showdown, while `runaway` has no trainer-battle state effect. Because
  diagnostics are non-authoritative in Rotom, these identities deliberately
  map to a no-authoritative-effect behavior with no registered callback.

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
ability's declarative behavior. The registry omits suppressible defensive
callbacks owned by the target's active side; unrelated source abilities,
conditions, and non-breakable callbacks retain their ordinary ordering and semantics. This is
not a global ability-activation model and does not claim support for Neutralizing
Gas, Gastro Acid, ability copying, or replacement.

Queue and calculation events carry typed weather/terrain, move type, combined
type effectiveness, requested major status, and compact interaction flags.
Ability callbacks consume those shared facts rather than adding ability
identity checks to `combat.rs`.
Prankster additionally carries a derived queue-time marker into `TryImmunity`,
and target typing is supplied only at that typed immunity stage. Sandstorm
residuals use `WeatherImmunity` before requesting kernel-backed HP damage.

Physical and special calculations now dispatch distinct Attack/Special Attack
and Defense/Special Defense events. Stat-override moves select the hook for the
actual effective stat, allowing Defense-based offense to reuse Defense
ability handlers rather than inventing a move-specific path.

## Verification boundary

The pinned `ability-modifiers.json` cases exercise isolated Showdown damage,
Speed, accuracy, priority, critical-ratio, immunity, and `TryHit` calculations.
They are not full cross-mechanic transition comparisons. Coverage only uses
`verification: differential_verified` when those fixtures exercise the whole
claimed behavior; compound, lifecycle, random, and damaging-hit abilities
remain `unit_tested` even when one component has pinned differential evidence.
Interaction verification is likewise `unit_tested` only where a test combines
the ability with another source, lifecycle boundary, suppression, or multiple
holders; otherwise it remains `not_tested`. A future
`interaction_verification: differential_verified` claim requires a normalized
fixture that actually executes and compares the relevant ability interaction.

### PR 13.5.6 interaction audit

The follow-up audit keeps the fifth batch's coverage count unchanged and adds
evidence at the boundaries identified during review:

- Pinned calculation fixtures now cover Fire Punch against Fluffy (the Fire
  weakness and contact resistance cancel), Iron Fist against Filter, Tough
  Claws against Fluffy, and Mold Breaker against Filter, Solid Rock, Fluffy,
  Thick Fat, Grass Pelt, and Punk Rock's defensive callback. A separate case
  confirms that Mold Breaker leaves a source-side Steely Spirit callback
  active. These remain calculation-level differentials, not full transition
  comparisons.
- End-to-end Rotom tests put poisoned, badly poisoned, sleeping, paralyzed, and
  frozen holders onto the field through an ordinary switch and verify that the
  `SwitchIn` dispatch cures the prohibited status and resets its associated
  counters. A turn-boundary test starts two active holders with independently
  installed statuses and verifies deterministic `BeforeTurn` curing. Because
  status provenance is not authoritative battle state, this also exercises
  curing independently of whether the original source was a move or another
  mechanic.
- A callback-scope test combines source-side Steely Spirit, target-side Filter,
  and Mold Breaker. It verifies from the dispatch trace that the unbreakable
  source-side base-power callback remains collected while the breakable
  target-side damage callback is omitted. Punk Rock's offensive and defensive
  callbacks share the same breakable descriptor, but suppression remains
  owner-scoped: the source callback is not globally disabled.

The pinned basis is `data/abilities.ts`: Filter and the other defensive
modifiers use breakable `onSourceModify*` callbacks; Punk Rock combines
`onBasePower` and `onSourceModifyDamage`; Immunity, Insomnia, Vital Spirit,
Limber, and Magma Armor cure prohibited state through `onUpdate`. Rotom maps
the update lifecycle onto typed `BeforeTurn`, `SwitchIn`, and `AfterSetStatus`
events while retaining kernel-only status mutation.

## Deferred PR 13.5 families

These seventy-seven abilities are incremental PR 13.5 batches, not completion
of the milestone. The other 138 reachable Regulation M-C abilities remain
explicitly unsupported. The remaining families and prerequisites are:

| Family | Required event/mechanics capability |
| --- | --- |
| Remaining switch-in/out | ordered weather/terrain/side changes, stat and form consequences |
| Priority and remaining Speed | side-condition context, item-dependent Speed, probabilistic priority, suppression and tie-order coverage |
| Attack type and damage | typed move/source context, immunity ordering, shared damage modifiers |
| Remaining immunity and absorption | grounded eligibility, redirection, persistent activation state, and item/form consequences |
| Remaining status prevention | source/target context, volatile status gates, and cure/prevention ordering |
| Weather and terrain | switch-in setters, duration/override rules and field-condition ordering |
| Contact and on-hit reactions | contact metadata, actual applied damage, per-hit callbacks and faint settlement |
| Global suppression/copy/override | authoritative activation state, ability replacement/copy rules and exception ordering beyond move-scoped Mold Breaker |

The original PR 13.5 requirements still apply: every reachable ability must use
the common mechanics/event fabric, have explicit ordering and activation rules,
and receive isolation plus representative cross-source interaction evidence.
