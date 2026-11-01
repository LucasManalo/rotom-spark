# PR 15.4 — Mega Evolution vertical slice

## Scope

PR 15.4 turns the generated Mega Stone catalog into one executable lifecycle.
The implementation derives compatibility from each item's resolved `megaStone`
mapping and does not maintain a second hand-authored stone table. The generated
Dex currently contains 81 legal Mega Stone identities; the implementation and
tests discover that count from data rather than embedding it in runtime code.

## Pinned Showdown basis

Behavior was checked at commit `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`:

- `sim/battle-actions.ts:BattleActions.canMegaEvo` resolves the held stone
  against the current/base species and rejects an unchanged or incompatible
  target.
- `sim/battle-queue.ts:BattleQueue.resolveAction` expands a Mega move request
  into an order-104 Mega action followed by its ordinary move action.
- `sim/battle.ts:Battle.runAction` executes the Mega action before moves and,
  for Gen 8+, refreshes remaining action priorities and Speeds before sorting.
- `sim/battle-actions.ts:BattleActions.runMegaEvo` performs the permanent form
  change and removes Mega eligibility from the side.
- `sim/pokemon.ts:Pokemon.formeChange`, `Pokemon.setSpecies`, and
  `Pokemon.updateMaxHp` replace form, types, calculated stats, and ability while
  preserving HP loss and the held Mega Stone.
- `sim/pokemon.ts:Pokemon.setAbility` ends the prior ability, installs the new
  ability, and executes its `Start` hook.

The project-owned fixture
`data/fixtures/differential/mega-evolution-lifecycle.json` records complete
pinned doubles turns for Charizard-Mega-X, Abomasnow-Mega, transformed-Speed
ordering, Snow Warning activation, HP/item persistence, and an incompatible
stone.

## Rotom Spark architecture

- `PokemonState::mega_evolution` stores a prevalidated target form, transformed
  stats, and target ability. Stats are computed from the original immutable set
  inputs while constructing battle state, so runtime mechanics never infer a
  nature or stat-point allocation from rounded battle stats.
- Legal-action enumeration exposes the existing typed `mega` move variant only
  when that descriptor exists and the side has not Mega Evolved.
- Queue expansion creates a separate order-104 action. Resolution applies one
  atomic kernel transformation, emits ability end/start events, activates the
  new ability, and refreshes the remaining move queue while preserving its
  previously sampled fractional-priority and tie ordering.
- The kernel updates form, stats, HP, ability, side usage, and team eligibility
  together. It does not decide stone compatibility or transformation timing.
- The held item is not consumed or suppressed by the transformation.

## Deferred behavior

- Mega Rayquaza's move-based path is not reachable in Regulation M-C and is not
  implemented.
- Illusion presentation, Truant's move-result bookkeeping, and other incomplete
  general ability lifecycle interactions remain explicit later capabilities.
- Non-Mega transformations—including Stance Change, Hunger Switch, Zero to
  Hero, Forecast, Mimicry, Disguise, Imposter, and Transform—remain PR 18 scope.
- General-purpose ability replacement/suppression remains partial; PR 15.4
  verifies the dedicated Mega ability transition only.
