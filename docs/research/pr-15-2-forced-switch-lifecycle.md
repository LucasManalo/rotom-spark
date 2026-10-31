# PR 15.2 — mid-turn forced-switch lifecycle

Pinned Showdown commit: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`  
Format: `gen9championsvgc2026regmc` (mod `champions`)

## Pinned behavior

- `data/items.ts:ejectbutton.onAfterMoveSecondary` consumes the item after a
  damaging move, sets the holder's `switchFlag`, and cancels the attacker's
  competing switch flag.
- `data/items.ts:redcard.onAfterMoveSecondary` consumes the item after a
  damaging move and sets the attacker's `forceSwitchFlag` when `DragOut`
  succeeds. The item is still consumed when Suction Cups prevents the drag.
- `sim/battle.ts:Battle.runAction` resolves `forceSwitchFlag` through
  `BattleActions.dragIn`, while `switchFlag` produces a player switch request
  without discarding the remaining action queue.
- `sim/battle-actions.ts:BattleActions.switchIn` runs switch-out cleanup,
  cancels the outgoing Pokémon's queued action, installs the incoming Pokémon,
  and then runs switch-in effects.

The generated fixture at
`data/fixtures/differential/forced-switch-lifecycle.json` executes complete
pinned Showdown doubles turns. It records Eject Button's paused request and
preserved queue, Red Card's immediate random drag, and Suction Cups consuming
Red Card while preventing the drag.

## Rotom implementation

`BattleState` now owns typed, serializable mid-turn `SwitchRequest` values.
The canonical transition may expose `ForcedSwitch` with a remaining action
queue; submitting the requested replacement runs the shared switch-out,
kernel position change, direct entry abilities, terrain seeds, `SwitchIn`, and
Berry update path before resuming that exact queue.

Random requests use the sole authoritative `BattleRng` with
`RngReason::ForcedSwitch` and settle internally. Player-choice requests remain
stable decision boundaries. Faint replacements also pause and resume the same
queue, so a later attack aimed at a replaced slot hits the incoming Pokémon
rather than resolving against an artificially empty slot.

Eject Button and Red Card run through a typed `AfterMoveSecondary` event once
per successfully damaged target. Item removal and switch-request writes use
`StateKernel`; switch position changes continue to use the kernel's atomic
position operation.

## Coverage decisions

Promoted to implemented:

- Eject Button: damaging-hit trigger, item suppression, consumption, player
  replacement request, queue preservation, exit/entry hooks, and replay.
- Red Card: damaging-hit trigger, consumption, deterministic random bench
  selection, immediate drag, and Suction Cups prevention.
- Suction Cups: its reachable forced-drag prevention behavior.

Still partial or unsupported:

- Emergency Exit requires its half-HP crossing trigger.
- Pivot moves and move-driven phazing require their individual move behavior
  wiring even though they can reuse this lifecycle.
- Entry hazards remain blocked by the separate hazard lifecycle. Switch-in
  ordering is shared, but this PR does not claim hazard implementation.
- Broader item-transfer interactions remain PR 15.5 scope.

No production item or switch mechanic writes authoritative battle state
directly outside the mutation kernel. Phase/queue orchestration remains owned
by the transition and combat layers.
