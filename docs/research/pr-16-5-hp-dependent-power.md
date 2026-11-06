# PR 16.5 — HP-dependent base power

## Scope

PR 16.5 promotes the shared HP-dependent variable-power family. Eruption and
Water Spout scale their listed 150 power by the user's current-HP ratio, with
the damage pipeline enforcing the minimum power of one. Flail and Reversal use
the shared 48-step HP bracket table (20, 40, 80, 100, 150, or 200 power).

The rule is typed combat metadata resolved by the Dex adapter and runs in the
canonical damage path before field and `ModifyBasePower` hooks. This preserves
ordinary spread damage and lets Technician and held-item modifiers inspect the
resolved power. Stat-dependent and history/context-dependent power are deferred
as separate priority-1 batches.

## Pinned Showdown basis

- `data/moves.ts:Moves.eruption.basePowerCallback` and
  `Moves.waterspout.basePowerCallback` return listed power multiplied by
  `pokemon.hp / pokemon.maxhp`.
- `data/moves.ts:Moves.flail.basePowerCallback` and
  `Moves.reversal.basePowerCallback` calculate
  `max(floor(hp * 48 / maxhp), 1)` and select the six power brackets.
- `sim/battle-actions.ts:BattleActions.getDamage` resolves callback power before
  the ordinary base-power modifier and damage sequence.

The checked fixture records direct callback results and executes complete
pinned-format doubles turns. These are reference-derived assertions, not PR 19
full-state differential comparisons.

## Ownership

Mechanics calculates the rule from authoritative current HP. Existing event and
damage lifecycles apply later modifiers and spread damage. HP damage and move
history remain kernel-owned; this batch adds no direct battle-state writes.
