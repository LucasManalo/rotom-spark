# PR 15.6 — Canonical move lifecycle and history

## Pinned behavioral basis

Reference commit: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`, Champions mod
`gen9championsvgc2026regmc`.

- `sim/battle-actions.ts::runMove` increments the action count, runs
  `BeforeMove`, deducts PP, calls `Pokemon::moveUsed`, resolves the move, stores
  its result, and only then runs `AfterMove`.
- `sim/battle-actions.ts::trySpreadMoveHit` defines the Gen 9 target pipeline:
  invulnerability, `TryHit`, type immunity, move-specific immunity, accuracy,
  protection breaking, and the per-hit loop.
- `sim/battle-actions.ts::hitStepMoveHitLoop` and the Champions override in
  `data/mods/champions/scripts.ts` perform multi-hit damage and only run
  `afterMoveSecondaryEvent` after the hit loop.
- `sim/pokemon.ts::moveUsed` writes `lastMove`, target location, and
  `moveThisTurn`; `Pokemon::gotAttacked`, `getLastAttackedBy`, and
  `getLastDamagedBy` own incoming move-source records.
- `sim/battle.ts::endTurn` rolls `moveThisTurnResult` into
  `moveLastTurnResult`, clears current-turn flags, and marks retained attacker
  records as no longer from this turn.
- `data/mods/champions/scripts.ts::pokemon.clearVolatile` clears move and attack
  history on switch-out.

The executable fixture `data/fixtures/differential/move-lifecycle-history.json`
records successful multi-hit, Armor Tail priority blocking, and Protect cases.
Notably, pinned Showdown rolls a fully protected move to its null/skipped result,
not successful or ordinary failure.

## Rotom Spark design

`BattleState::execute_move` is the only supported queued-move entry point. It
now owns this outer sequence:

1. Prepare immutable move data and resolved targets.
2. Begin the action and run `BeforeMove`.
3. Deduct PP and record the used move.
4. Execute the shared target/hit pipeline for the move operation.
5. Classify the completed action as succeeded, failed, or skipped.
6. Commit history through the mutation kernel.
7. Dispatch the single canonical `AfterMove` event.

Damage, fixed-damage, declarative status, item-swap, and narrow intrinsic moves
no longer dispatch their own `AfterMove` events.

`PokemonState::move_history` centralizes last move, last successful move,
current/previous result, consecutive uses, last move damage, incoming hit and
damage sources, per-turn damaging-hit count, persistent times-attacked count,
and active move-action count.
`BattleState::last_successful_move_this_turn` stores the battle-wide predicate.
All mechanical history writes use `StateKernel`; residual completion performs
the transition-owned turn rollover and queue settlement, and switch-out cleanup
clears the complete record through the kernel.

The Metronome held item now consumes the authoritative consecutive-use count
at `ModifyDamage`, using pinned modifiers `4096, 4915, 5734, 6553, 7372, 8192`
and resetting after a failed or different move. This is the one identity whose
remaining blocker is closed by PR 15.6.

## Deliberate scope and follow-ups

- Called moves (Dancer, Instruct, Sleep Talk, Metronome, Copycat) are not yet
  executable, so their distinction between external `runMove` and nested
  `useMove` remains a PR 16 identity-family concern.
- Two-turn locks, recharge, future moves, Pursuit-style interception, and
  target-location history remain blocked by their unsupported move families.
- The canonical lifecycle does not silently enable those identities; their
  capability dependencies remain partial/unsupported.
- `last_damage_dealt` follows Showdown's most recent hit damage. Aggregate move
  damage remains transient execution context for recoil and `AfterMove` hooks.
