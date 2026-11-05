# PR 16.2 — Move-driven random phazing

## Pinned behavioral basis

Reference commit: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`, Champions format
`gen9championsvgc2026regmc`.

- `data/moves.ts` declares `forceSwitch: true` for Circle Throw, Dragon Tail,
  Roar, and Whirlwind.
- `sim/battle-actions.ts::moveHit` applies ordinary hit effects before step 6,
  `forceSwitch`.
- `sim/battle-actions.ts::forceSwitch` requires a living source and target plus
  an available reserve, runs the `DragOut` event, and marks the target for a
  random replacement only when that event succeeds.
- `data/abilities.ts::suctioncups.onDragOut` prevents the drag. Its ability is
  breakable, so the canonical move context's existing target-ability
  suppression remains authoritative.

## Rotom design

Resolved move data carries typed `SwitchEffect::ForceTargetRandom` metadata.
Damaging and status phazing moves both use the canonical hit lifecycle. A
successful eligible hit requests the existing kernel-owned random switch;
turn resolution settles that request through the same switch-out, random RNG,
position mutation, switch-in, ability-start, item-update, and remaining-queue
path already used by Red Card.

The new `force_target_switch` capability is intentionally narrower than
`switch_control`. Player-choice pivots, Baton Pass state transfer, Shed Tail,
Chilly Reception, Parting Shot, and Revival Blessing remain blocked instead of
being silently promoted.

## Evidence and limits

`move-phazing.json` executes complete pinned Showdown doubles turns for all
four moves and the Suction Cups negative interaction. The Rust transition test
checks deterministic serialization replay, forced-switch RNG, queue
preservation/skipping of the outgoing target's queued action, and prevention.
These are reference-derived behavioral assertions, not PR 19 full-state
differential comparisons.
