# Event and effect framework research

Reference: Pokémon Showdown commit
`b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`, Champions mod,
`gen9championsvgc2026regmc`.

## Source paths

- `sim/battle.ts` — `comparePriority`, `runEvent`, `resolvePriority`, and
  `priorityEvent` define handler collection, ordering, relay values, early
  termination, and default effect-type sub-orders.
- `sim/battle-actions.ts` — move execution supplies the initial hook sequence,
  including `BeforeMove`, `TryHit`, `ModifyDamage`, `Hit`, `AfterDamage`, and
  `AfterMove` behavior.
- `sim/battle.ts` and `sim/battle-actions.ts` — switching, fainting, and the
  residual request path establish the first non-move lifecycle hook sites.

## Implemented slice

`EffectRegistry` stores stateless Rust callbacks outside `BattleState` so
battles remain cloneable and serializable. At dispatch, the registry discovers
matching active abilities, items, major statuses, field/side/volatile
conditions, and the current move from typed authoritative state.

Collected handlers use Showdown's normal ordering directions: lower explicit
order, higher priority, higher holder Speed, lower sub-order, then stable
registration order. Default sub-orders distinguish conditions, field and side
conditions, abilities, and items. Dispatches carry a typed relay value, may be
cancelled, and retain the ordered invocation list for diagnostics.

Turn resolution currently emits `BeforeTurn`, move, damage, switch, faint, and
`Residual` dispatches. `ModifyDamage` is the first modifying relay integrated
into combat. Existing `resolve_turn` callers use an empty registry;
`resolve_turn_with_effects` enables behavior callbacks transactionally.

## Deferred behavior

PR 11 and later mechanics will register concrete condition, move, ability, and
item callbacks. More specialized Showdown event relations (`onSource`,
`onAlly`, `onFoe`, and similar), priority-event first-result behavior, duration
updates, and generation-specific exceptions remain deferred until a reachable
Regulation M-C mechanic requires them. These cases must be added explicitly,
not approximated through a generic callback.
