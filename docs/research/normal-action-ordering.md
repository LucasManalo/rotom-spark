# Normal action ordering

Reference revision: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`

Milestone 8 implements the normal, pre-effect action queue only. The relevant
pinned Pokémon Showdown behavior is:

- `sim/battle-queue.ts` assigns voluntary switches order `103` and ordinary
  moves order `200`; lower order executes first.
- `sim/battle.ts::comparePriority` then compares priority high-to-low and speed
  high-to-low.
- `sim/battle.ts::speedSort` identifies actions whose complete ordering keys
  tie and delegates to `sim/prng.ts::shuffle`.
- `sim/prng.ts::shuffle` uses an in-place Fisher–Yates shuffle and consumes one
  random draw for every position except the last in a tied group.
- Move priority begins with the static move priority in
  `sim/battle.ts::getActionSpeed` before later `ModifyPriority` effects.
- Normal action speed comes from `pokemon.getActionSpeed()`. The Champions mod
  overrides Trick Room handling in `data/mods/champions/scripts.ts`.

Rotom therefore stores base move priority on each runtime move slot, sorts
switches before moves, then sorts moves by base priority and current calculated
Speed, and resolves only exact ties through `BattleRng` using Fisher–Yates.

## Intentionally deferred

- priority-modifying moves, abilities, items, terrain, and other effects;
- fractional priority and sub-order hooks;
- Trick Room and other speed modifiers;
- Mega Evolution's separate queue entry and its speed recalculation;
- mid-turn queue insertion and re-sorting;
- action execution.

Those behaviors require the later event/effect and combat milestones. Until
then, `BattleStats::spe` is the queue's effective-speed input and the queue API
does not silently claim support for those modifiers.
