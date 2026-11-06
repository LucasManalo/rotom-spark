# PR 16.4 — Status-dependent base power

## Scope

PR 16.4 promotes the shared status-dependent base-power family: Hex and Infernal Parade double against any target with a major status, while Venoshock and Barb Barrage double only against poisoned or badly poisoned targets. The predicates are represented as typed combat metadata and execute inside the canonical transition damage path before `ModifyBasePower` hooks.

Comatose is not reachable in the pinned Regulation M-C manifest, so its status-equivalent branch remains outside this PR. Other variable-power callbacks remain explicitly unsupported.

## Pinned Showdown basis

- `data/moves.ts:Moves.hex.basePowerCallback` and `Moves.infernalparade.basePowerCallback` double base power when the target has a status or Comatose.
- `data/moves.ts:Moves.venoshock.onBasePower` and `Moves.barbbarrage.onBasePower` double base power for `psn` or `tox`.
- `mods/champions/moves.ts:Moves.infernalparade` sets the Champions base power to 65.
- `sim/battle-actions.ts:BattleActions.getDamage` resolves the callback-adjusted base power before the ordinary damage formula.

The pinned generator records callback-resolved base power and executes a complete doubles turn for each promoted move. The Rust regression checks the same predicates through `step()`, including serialization/replay determinism and unchanged input state.

## Ownership

Mechanics chooses whether the predicate activates. The existing damage path applies the adjusted power and the mutation kernel remains the only production writer of HP and status state. This PR adds no direct battle-state writes.
