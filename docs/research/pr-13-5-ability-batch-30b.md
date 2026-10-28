# PR 13.5.11 ability batch: switch-in, contact, and item mechanics

The behavioral reference remains the unchanged `SHOWDOWN_VERSION` pin,
`b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`. The primary source is
`vendor/pokemon-showdown/data/abilities.ts`; the generated Champions ability
set remains the reachability source.

This batch registers 30 reachable Regulation M-C abilities:

- Queue and weather rules: Analytic, Cloud Nine, Drizzle, Drought, Sand
  Stream, and Snow Warning.
- Switch-in and stage rules: Curious Medicine, Guard Dog, Inner Focus,
  Intimidate, Moody, Oblivious, Own Tempo, Screen Cleaner, and Supersweet
  Syrup.
- Hit and status reactions: Damp, Effect Spore, Flame Body, Lightning Rod,
  Poison Point, Poison Touch, Static, and Synchronize.
- Item and ability state: Klutz, Magician, Pickpocket, Receiver, Trace, and
  Unburden.
- Indirect-damage protection: Magic Guard.

The implementation reuses declarative ability-to-behavior registrations.
Ordinary queued switch-ins use one shared switch-in helper for weather,
screens, stage changes, and Intimidate. Hit reactions remain ordered event
callbacks, RNG uses `RngReason::Ability`, and all authoritative state changes
go through the mutation kernel or an invariant-preserving state operation
called by `EffectState`.

Pinned details checked directly include Analytic's all-active
`queue.willMove` scan and exact 5325/4096 modifier; Effect Spore's single
0..99 draw with 11/10/9 status buckets; Moody's two samples over only Attack,
Defense, Special Attack, Special Defense, and Speed; and Damp's move list plus
Aftermath protection.

## Deliberate partial coverage

Coverage is conservative because several supporting mechanics are not yet
represented:

- Switch-in behaviors run for ordinary queued switches, but initial lead and
  forced-replacement entry do not yet share that lifecycle path.
- Damp blocks explosion-flagged moves, but Aftermath damage is not represented.
- Contact status abilities and Synchronize do not yet route through every type,
  powder, terrain, item, and ability status-immunity rule.
- Lightning Rod provides immunity and its boost, but move redirection is not
  implemented.
- Magician and Pickpocket do not yet enforce every unstealable-item and Sticky
  Hold case. Klutz currently covers item-dependent grounding and powder
  redirection, not every held-item callback.
- Receiver and Trace do not yet consume the generated `noreceiver` and
  `notrace` exclusion flags.
- Screen Cleaner lacks the unresolved initial-lead lifecycle. Supersweet Syrup
  lacks its per-Pokémon once-per-battle marker and Substitute interaction.
- Unburden currently detects an empty item slot, not whether the holder lost or
  consumed an item while active.
- Magic Guard covers supported weather residual damage only; the remaining
  indirect-damage sources are deferred.
- Inner Focus, Oblivious, and Own Tempo cover Intimidate immunity in this slice;
  their flinch, infatuation, and confusion rules remain outside the supported
  event surface.

These gaps are labeled `partial`. Analytic and Moody are the only abilities in
this batch claimed as implemented, and no new differential-equivalence claim
is made.
