# Unified move executor foundation (PR 13.1)

Status: implemented PR 13.1 architecture review and first vertical slice.
Scope: selected-target, one-hit standard damage only. This document does not
claim PR 13.2 or later behavior.

## Repository mapping

| Existing area | PR 13.1 decision | Result |
| --- | --- | --- |
| `transition::step` and `Mechanics` | Reuse unchanged | Remain the only public transaction boundary and immutable mechanics context. |
| `state/queue.rs` | Reuse unchanged | Queue construction, priority/Speed hooks, and tie RNG continue to define action order. |
| `combat.rs::resolve_turn_inner` | Adapt | Every queued move now enters the single `execute_move` lifecycle. Switch, residual, and faint settlement remain turn-orchestrator responsibilities. |
| Former `resolve_basic_move` / `BasicMoveContext` | Retire as an architectural identity | Replaced by `execute_move`, `prepare_move_execution`, and `MoveExecutionContext`. Physical and Special are formula inputs, not executor classes. |
| `calculate_damage_inner` and field helpers | Reuse | The pinned formula, rounding, type chart, field modifiers, screens, critical and random draw order are unchanged. |
| `EffectRegistry` / `EffectState` | Reuse | Existing ordered hooks remain inside the move lifecycle. Callbacks retain controlled read/RNG/kernel-backed mutation access. No second dispatcher was added. |
| `StateKernel` | Reuse | PP and HP writes remain typed kernel operations. `AfterDamage` now relays actual clamped HP loss. |
| `rotom-dex::MoveData` | Extend, not duplicate | The resolved broad Dex now records behavior-bearing callback/metadata facts needed to derive execution requirements. |
| `MoveSupport::BasicDamage` and `BASIC_DAMAGE_MOVES` | Retire | Runtime support is derived from operation and capability requirements, never a move-ID allowlist. |
| PR 13.0 reachability manifest | Adapt | Reachability is unchanged; move entries now expose `operation` and `required_capabilities`, and compatible moves receive capability-derived implementation status. |

## Concrete minimum interfaces

`CombatMove` is the compact immutable runtime definition returned by
`CombatData`. Existing numeric/type/target data is joined by:

- `MoveOperation`: the primary operation shape. PR 13.1 executes only
  `StandardDamage`; fixed, level and non-damage operations remain representable
  and reject explicitly.
- `MoveCapabilities`: a compact requirement set for targeting, hit sequence,
  prior/later effects, callbacks, and control flow.
- `MoveCapability`: stable, named requirements used in unsupported-mechanic
  errors and generated coverage.

`MoveExecutionContext` is internal ephemeral data for one queued move: source,
target, action slot, resolved definition, and source identity. It is not
authoritative state and is never serialized into `BattleState`.

`execute_move` is the sole queued-move entry path. Its PR 13.1 stages are:

```text
lookup + capability preflight
  -> BeforeMove
  -> kernel PP decrement
  -> accuracy / current field hit checks
  -> TryHit
  -> execute_primary_operation(StandardDamage)
  -> ModifyDamage / Hit
  -> kernel HP application
  -> AfterDamage(actual applied HP)
  -> faint settlement
  -> AfterMove
```

Misses, cancellations, immunities and priority blocking remain successful
battle outcomes. Missing capabilities are engine errors. Because `step` owns a
private cloned working state, all callback, PP, HP and RNG work is discarded
on any error and the caller's state remains unchanged.

## Definition, handlers and mutation coexistence

The three extension mechanisms are complementary rather than parallel engines:

1. The resolved declarative move definition states its primary operation and
   every behavior capability it needs.
2. The one executor sequences lifecycle stages and dispatches the existing
   typed handlers at their established hooks.
3. Handlers use `EffectState`; built-in move mechanics use `StateKernel`
   directly. Both reach the same typed kernel primitives, and neither receives
   an unrestricted public mutation path.

This permits a later status or boost operation to be added as another
`MoveOperation` branch inside `execute_primary_operation`. Conditional power,
secondaries, multi-hit and continuation can extend their named lifecycle seams
without a category-specific or move-specific run loop.

## Plan/repository conflicts resolved

- The pre-13.1 generated Dex omitted secondaries and callback presence. Schema
  version 4 adds normalized `MoveBehaviorData`, sourced from the same pinned
  resolved move objects.
- The PR 13.0 manifest previously treated some callback moves as damage-only.
  It now records runtime operation/capability requirements independently of its
  broader research-oriented `effect_families`.
- The previous coverage generator parsed a Rust move-ID allowlist. It now uses
  the shared exporter capability analysis and rejects incompatible implemented
  claims.
- Existing `ModifyAttack`, `ModifyDefense`, and `ModifyBasePower` events remain
  reserved. PR 13.1 does not activate them because doing so would change
  ordering/rounding without a required reachable mechanic and pinned fixture.

## Deliberate deviations and deferred boundaries

- The conceptual plan names a `MoveExecutor` type. The minimum Rust shape is an
  internal `execute_move` entry plus narrowly named stage methods on the
  working `BattleState`; an otherwise state-forwarding wrapper type would add
  no ownership boundary. The architectural invariant is one entry/lifecycle,
  not a public executor object.
- Capability preflight covers the complete intrinsic move definition and the
  already implemented condition/field/event fabric. General ability/item
  interaction coverage is not claimed; it remains PR 13.5/13.6 scope.
- Legal-action enumeration remains the primary target selector. Move
  preparation independently checks the selected `ActionTarget` against the
  authoritative resolved definition so a stale or inconsistent move-slot
  target cannot cross the executor boundary. In the current doubles slice,
  `Normal` and `Any` accept either foe or the other ally, while `AdjacentFoe`
  accepts foes only; self/automatic selections are rejected. Empty or fainted
  selected slots still return the explicit deferred-retargeting error.
- Move flags are retained in the Dex. Flags with intrinsic execution behavior
  (charge, recharge, future move, and similar) require `special_behavior`.
  Cross-source flag interactions such as contact abilities remain deferred
  with their source family rather than falsely implemented here.
- Coverage marks 37 reachable definitions structurally executable by the same
  PR 13.1 capabilities. Verification remains independent: unit claims name an
  anchored Rust test, differential claims name a pinned fixture containing the
  move identity and pinned damage-calculation evidence, and interaction
  verification is tracked separately. The controlled-outcome `step()` fixtures
  validate Rotom Spark state consequences but do not compare an independently
  executed complete Showdown transition. No PR 13.1 move currently claims
  full battle-transition or complete ability/item interaction verification.

## Unsupported after PR 13.1

The executor rejects non-damage primary operations, advanced/spread/random
targeting, retargeting, fixed/level/OHKO damage, multi-hit, secondaries,
self-effects, status/volatile/stat-stage/healing operations, drain/recoil,
field/side conditions, move calls, switch control, move-specific callbacks,
charge/recharge and other special behavior. Struggle and Mega Evolution also
remain explicit unsupported mechanics.

