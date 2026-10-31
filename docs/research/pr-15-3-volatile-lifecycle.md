# PR 15.3 — shared volatile lifecycle and Mental Herb

Pinned Showdown commit: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`  
Format: `gen9championsvgc2026regmc` (mod `champions`)

## Pinned behavior

- `sim/pokemon.ts:Pokemon.addVolatile` and `Pokemon.removeVolatile` own the
  unique volatile identity and start/end lifecycle.
- `data/conditions.ts:Conditions.flinch` has duration one and a priority-eight
  `BeforeMove` callback. It emits the Flinch event and prevents the move.
- `data/abilities.ts:Abilities.innerfocus.onTryAddVolatile` prevents flinch;
  `Abilities.steadfast.onFlinch` raises Speed by one stage.
- `data/items.ts:Items.kingsrock.onModifyMove` appends one ten-percent flinch
  secondary to a damaging move only when it does not already have one.
- `data/items.ts:Items.mentalherb.onUpdate` consumes the item when Attract,
  Taunt, Encore, Torment, Disable, or Heal Block is present, then removes all
  six conditions. Item suppression still prevents activation.

The generated fixture at
`data/fixtures/differential/volatile-lifecycle.json` executes complete pinned
Showdown doubles turns for Air Slash with Steadfast and Inner Focus, King's
Rock flinch, and Mental Herb curing Taunt.

## Rotom implementation

The existing `ConditionState` remains the single authoritative volatile
representation. `StateKernel::add_volatile`, `remove_volatile`, duration ticks,
and switch cleanup continue to own structural writes. PR 15.3 adds stable
mechanics identities for flinch and the six Mental Herb conditions without
introducing parallel booleans.

The resolved-data exporter now retains declarative secondary
`volatile_status` values. Flinch secondaries become ordinary
`PrimaryMoveEffect::AddVolatile` operations. The configured effect registry
registers flinch as an ordered volatile `BeforeMove` handler, between the
existing sleep/freeze and paralysis checks. Inner Focus prevents application;
Steadfast uses the same event pass to boost Speed before flinch cancels the
move. Unused one-turn flinch expires through ordinary residual duration
upkeep.

King's Rock appends one canonical flinch secondary before Sheer Force removes
secondaries. Mental Herb participates in the shared update path used at turn,
residual, and switch-in boundaries, and also runs immediately after volatile
application. Consumption and every removal go through `StateKernel`.

## Coverage decisions

Promoted to implemented:

- Flinch lifecycle for declarative move secondaries.
- Inner Focus's flinch immunity and Steadfast's Flinch response.
- King's Rock's non-duplicating flinch secondary.
- Mental Herb's complete six-condition cure set, suppression, consumption,
  and update timing.

Still partial or unsupported:

- Confusion requires its random duration, before-move decrement, self-hit
  damage, prevention, and Berry cure.
- Taunt, Encore, Disable, Torment, Attract, and Heal Block can be represented
  and cured, but their move/action restriction semantics remain explicit
  blockers. Their move identities are not promoted by this PR.
- Substitute, trapping, recharge, temporary invulnerability, and the other
  Protect-family variants remain separate lifecycle work.
- Fake Out and other moves with custom activation predicates remain blocked
  by their individual callbacks even though their flinch consequence can use
  this lifecycle.

No production volatile, ability, or item mechanic directly writes the
volatile vector, held item, or stat stages. Event ordering remains mechanics
owned; structural mutation remains kernel owned.
