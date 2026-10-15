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
  type immunity, move-specific immunity, accuracy, then damage.
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

## Deliberate boundaries

Side/field targets, random/scripted targets, semi-invulnerability, Sky Drop,
ability suppression, and protection variants with contact punishments remain
explicitly unsupported unless their complete intrinsic capabilities are
implemented. The executor does not promote those moves merely because target
sets and Protect exist.
