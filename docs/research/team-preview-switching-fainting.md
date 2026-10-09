# Team Preview, Switching, Fainting, and Victory

Reference revision: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`

Format: `gen9championsvgc2026regmc` (`champions` mod)

## Format rules

- `config/formats.ts` defines Champions VGC 2026 Regulation M-C as a doubles
  format using `Flat Rules`.
- `data/mods/champions/rulesets.ts` defines Champions `Flat Rules` with
  `Picked Team Size = Auto` and team preview.
- `sim/dex-formats.ts` resolves an automatic picked-team size to four for
  doubles.

Rotom therefore stores the picked-team size on each battle rather than baking
four into generic transition code. Regulation M-C supplies four.

## Team preview and leads

`Side.chooseTeam` in `sim/side.ts` validates unique, in-bounds positions and
adds ordered `team` actions. `Battle.runAction` in `sim/battle.ts` rebuilds the
battle roster in that order. The `start` action switches the first Pokémon into
each active position, so the first two ordered choices are the doubles leads.
`Battle.endTurn` advances the first ordinary choice request to turn 1.

Showdown's text parser can auto-fill omitted preview slots. Rotom's typed API
does not reproduce command-parser convenience: callers submit exactly the
format-required number of unique indexes.

Rotom retains all six authoritative team members and separately records the
ordered four selected for battle. This intentionally differs from Showdown's
internal array rewrite while preserving battle behavior and complete state.

## Switching and forced replacement

`Side.chooseSwitch` rejects active, fainted, repeated, and out-of-range switch
targets. `Side.clearChoice` computes forced switches as the minimum of active
positions needing replacement and living reserve Pokémon; remaining empty
positions become forced passes. This permits the player to choose which slot is
filled when two positions are empty but only one reserve remains.

`BattleActions.switchIn` and `Pokemon.clearVolatile` in
`sim/battle-actions.ts` and `sim/pokemon.ts` clear ordinary switch-out volatile
state and stat stages. Rotom's current basic switch transition clears modeled
stat stages, volatile conditions, and move-disable flags. Switch event hooks,
ability/item lifecycle hooks, copied volatiles, trapping, and form regression
remain unsupported until their dedicated mechanics exist.

## Faint resolution and victory

`Pokemon.faint` queues a faint rather than finalizing it immediately.
`Battle.faintMessages` drains the ordered queue and calls `Battle.checkWin`
after the batch. Rotom likewise accepts an ordered faint batch so simultaneous
knockouts are not incorrectly evaluated one at a time.

For Generation 9, `Battle.checkWin` handles both sides reaching zero by passing
the side of the final faint-queue target to `win`. Consequently, the side whose
Pokémon faints last wins this specific simultaneous-elimination case. Rotom
preserves this reference behavior using the final target in the submitted faint
batch.
