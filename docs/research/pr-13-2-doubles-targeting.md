# PR 13.2 doubles targeting research

Reference: Pokémon Showdown commit `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`,
format `gen9championsvgc2026regmc`, mod `champions`.

## Target sets and hit order

- `sim/pokemon.ts:Pokemon.getMoveTargets` expands `allAdjacent` as adjacent
  allies followed by adjacent foes, expands `allAdjacentFoes` as adjacent foes,
  and performs `RedirectTarget` only for the default single-target branch.
- `sim/battle.ts:Battle.getTarget` and `Battle.getRandomTarget` retarget a
  missing/fainted foe to an available foe. A missing ally does not normally
  retarget; `adjacentAllyOrSelf` falls back to the user.
- `sim/battle-actions.ts:BattleActions.trySpreadMoveHit` sets `spreadHit` when
  the resolved set has more than one target, before protection, immunity, or
  accuracy removes individual targets. Its Gen 9 per-target order is TryHit,
  type immunity, move-specific immunity, accuracy, protection breaking, then
  damage.
- `data/mods/champions/scripts.ts:BattleActions.modifyDamage` applies the
  doubles spread modifier as 0.75 immediately after base damage plus two and
  before weather, critical, random, STAB, and type modifiers.

## Protect and redirection

- `data/moves.ts:protect` installs a one-turn `protect` volatile. Its TryHit
  handler uses `checkMoveBypassesProtect`; `breaksProtect` moves remove guards
  in `sim/battle-actions.ts:hitStepBreakProtect`.
- `data/conditions.ts:stall` starts at counter 3, succeeds with probability
  `1 / counter`, triples the counter on a successful restart, and uses duration
  two so one intervening non-stalling turn resets the chain.
- `data/moves.ts:followme` and `ragepowder` install one-turn redirect
  volatiles. Both require the redirector to remain a valid target. Rage Powder
  additionally calls powder status immunity; the PR 13.2 state model covers
  Grass typing, Overcoat, Safety Goggles, and Magic Room item suppression.
- `sim/battle.ts:Battle.priorityEvent`, `resolvePriority`, and
  `compareRedirectOrder` order competing redirect handlers by event priority,
  effective action Speed, then the holder's ability-state creation order. For
  Follow Me and Rage Powder this last value describes when the Pokémon most
  recently became active, not when the redirecting move was used. Reversing
  two tied users' activation order therefore does not reverse the winner;
  switching one user does. Rotom persists this minimal active-entry order and
  updates it only through position kernel operations.

## Psychic Terrain and per-target failures

- `data/moves.ts:psychicterrain.condition.onTryHit` is a priority-4 TryHit
  handler. It rejects only opposing positive-priority hits whose current
  target is grounded. Its handler precedes Protect's priority-3 TryHit handler,
  type immunity, accuracy, critical-hit and damage draws.
- A mixed grounded/airborne spread priority move can therefore be blocked for
  one target and continue for the other. A grounded protected target is
  reported as terrain-blocked without consuming accuracy/damage RNG; an
  airborne protected target is reported as protected.
- Rotom records terrain rejection as `TargetResult::PriorityBlocked`; it does
  not cancel unaffected targets or emit a whole-move failure.

## Differential verification scope

`tools/showdown-reference/doubles-targeting.cjs` now executes pinned
`BattleActions.useMove` cases against live Showdown battle state for spread
damage, protection chains, retargeting, redirection conflicts and immunities,
independent accuracy, applied HP loss, fainting, and Psychic Terrain. Where
Rotom's SplitMix64 cannot share raw seed parity with Showdown, the fixture
forces equivalent accuracy/critical/damage or Protect branches and records the
operations. These are formula, individual-mechanic, and interaction
comparisons; they are deliberately not labeled complete Showdown turn
transitions.

## Deliberate boundaries

Side/field targets, random/scripted targets, semi-invulnerability, Sky Drop,
ability suppression, and protection variants with contact punishments remain
explicitly unsupported unless their complete intrinsic capabilities are
implemented. The executor does not promote those moves merely because target
sets and Protect exist.
