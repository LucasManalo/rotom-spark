# Typed Actions and Legal Choice Enumeration

Reference revision: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`

Format: `gen9championsvgc2026regmc` (`champions` mod)

## Target representation

`sim/dex-moves.ts` defines fifteen move-target categories. `BattleActions`
marks `normal`, `any`, `adjacentAlly`, `adjacentAllyOrSelf`, and
`adjacentFoe` as player-selectable. The other ten categories reject a supplied
target location and are represented by Rotom as an automatic target.

`Battle.validTargetLoc` establishes the doubles geometry. In particular,
`normal` can select any adjacent Pokémon, including the user's partner, while
`adjacentFoe` is restricted to either opposing position. Target-location
validity is geometric and does not depend on whether the position is currently
occupied, so Rotom enumerates positions rather than inspecting target HP.

## Move and switch availability

`Pokemon.getMoveRequestData` and `Side.chooseMove` exclude disabled moves and
moves without PP. If none remain, Showdown exposes Struggle with
`randomNormal` targeting; Struggle therefore needs no selected target. The
request does not expose Mega Evolution for this locked Struggle choice.

`Side.chooseSwitch` rejects fainted, active, repeated, and trapped switch
choices. Rotom enumerates only living selected-team members whose authoritative
position is the bench. A trapped active receives no voluntary switch actions.

## Combined doubles choices

Showdown records switch-ins already selected in the side choice and rejects a
second active choosing the same reserve. It also records whether Mega Evolution
has already been selected and rejects a second Mega action. Rotom applies both
constraints while building complete two-slot `TurnChoice` values, so consumers
do not need to reconstruct cross-slot legality from independent action lists.

During a forced-switch request, Showdown requires the minimum of empty active
positions and available reserves. If only one reserve exists for two empty
positions, either slot may receive it and the other position passes. Rotom
enumerates those two combined choices explicitly.

## Deferred behavior

This milestone validates and enumerates decisions only. It does not collect
simultaneous player submissions, order actions, spend PP, execute switches or
moves, perform Mega Evolution, or select Struggle's random target. Those belong
to later action-queue, RNG, and mechanics milestones.
