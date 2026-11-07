# PR 16.6 — Semantic damaging moves and stat-dependent power

## Scope

PR 16.6 first corrects the PR 16.5 classification of zero-listed-base-power
damaging moves. Standard-damage semantics now drive both operation selection
and the shared hit, Protect, retargeting, and redirection requirements. Flail
and Reversal therefore cannot bypass ordinary negative paths merely because
their exported listed power is zero.

The batch then extends the typed `BasePowerRule` family to Stored Power and
Power Trip. Both add 20 power for each positive stage across Attack, Defense,
Special Attack, Special Defense, Speed, accuracy, and evasion. Negative stages
do not offset positive stages. The rule is resolved before ordinary
`ModifyBasePower` handlers, preserving Technician and held-item composition.

Payback, Assurance, and Stomping Tantrum remain deferred as a separate
contextual-history family because they depend on queue state, same-turn target
damage history, and prior-turn move failure respectively.

## Pinned Showdown basis

- `data/moves.ts:Moves.storedpower.basePowerCallback` and
  `Moves.powertrip.basePowerCallback` calculate
  `move.basePower + 20 * pokemon.positiveBoosts()`.
- `sim/pokemon.ts:Pokemon.positiveBoosts` sums only positive values from all
  seven battle stat stages.
- `sim/battle-actions.ts:BattleActions.getDamage` resolves callback power before
  ordinary base-power modifiers and the damage formula.
- `data/moves.ts:Moves.flail` and `Moves.reversal` are damaging moves whose
  callback supplies nonzero execution power despite a listed power of zero.

The fixture includes mixed stages, maximum stages, Technician thresholds, and
an Unaware target. Unaware ignores effective attacking stages but does not
change the positive-stage sum used by the callback. These are pinned-reference
behavioral assertions, not PR 19 full-state differential parity.

## Ownership

Mechanics classifies moves and calculates base power from authoritative state.
The existing event and damage lifecycle applies modifiers. Protection, HP, PP,
and history mutations remain on the canonical transition/kernel paths; no new
direct battle-state writes were added.
