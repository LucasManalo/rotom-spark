# Authoritative mutation ownership inventory

Status: Updated for PR D mechanics-to-kernel integration. Line references are
descriptive migration anchors; ownership classifications are authoritative.

This inventory treats battle fields, nested side/team/Pokémon/condition data,
the pending queue, and the embedded RNG stream as authoritative. Diagnostic
result vectors and event relay values are excluded because they do not drive
future simulation.

## Classification

- **Construction:** initializes a new state or test fixture; it does not mutate
  an already-running production battle.
- **Kernel candidate:** a structurally meaningful production write that PR B
  should represent with a typed, invariant-preserving operation.
- **Mechanics write:** a rule/effect currently writes state directly. PR D must
  route it through the kernel while preserving ordering and RNG use.
- **Test/fixture only:** direct setup, observation, or deliberate corruption in
  tests. PR E will replace ordinary fixture setup with controlled builders;
  intentionally invalid-state tests need an explicit test-only escape.

## Production writes

| Location | Authoritative writes | Current class | Planned owner |
| --- | --- | --- | --- |
| `rotom-dex/src/combat.rs:78-132` | Constructs runtime `PokemonState` and `MoveSlot` values from immutable resolved Dex data | Construction | Dex-to-state constructor; no running-battle mutation |
| `state/battle.rs:39-105` | Constructs turn, picked size, sides, field, embedded RNG, queue, phase, winner | Construction | State constructor; stable validation at return |
| `state/side.rs:16-27`, `state/pokemon.rs:375-379` | Constructs pre-preview side and runtime team containers | Construction | State constructors |
| `state/transition.rs` | Stores one side's selected-team order | Transition write | Justified transition-owned preview input; PR E will narrow the public field escape |
| `state/transition.rs`, `kernel/mod.rs` | Voluntary switches, forced replacements, and initial leads | Kernel-routed | Transition/mechanics validate and order; kernel atomically performs cleanup and coupled position/slot writes |
| `state/transition.rs`, `kernel/mod.rs` | Faint cleanup: HP/status/counters/volatiles/position/active slot | Kernel-routed | Mechanics owns faint ordering and settlement; kernel owns atomic structural representation |
| `state/transition.rs` | Writes turn, winner, and post-preview/faint/replacement phase | Transition write | Justified phase settlement; not an effect or primitive mechanics write |
| `state/queue.rs:75-85` | Clones and commits the whole battle transactionally | Compatibility adapter | Retained for migration compatibility; canonical `step` bypasses this extra clone |
| `state/queue.rs:110-111` | Installs action queue and enters `ResolvingTurn` | Transition write | Justified queue/phase settlement |
| `state/queue.rs:190-213` | Advances embedded RNG while shuffling exact speed ties | Mechanics write | Ordering mechanics requests RNG; RNG remains embedded and authoritative |
| `combat.rs:226-236` | Clones and commits the whole battle transactionally | Compatibility adapter | Retained for migration compatibility; canonical `step` bypasses this extra clone |
| `transition.rs` | Clones the complete stable input once, orchestrates preview/turn/replacement, then returns only a validated stable result | Transition owner | Canonical public transaction boundary introduced by PR C |
| `combat.rs:330-338` | Clears queue/events, increments turn, settles phase | Transition write | Justified end-of-turn settlement |
| `combat.rs` | Decrements move PP | Kernel-routed | Mechanics validates the move; kernel applies the bounded PP assignment |
| `combat.rs:487-492`, `632-646` | Advances RNG for critical, damage, and accuracy | Mechanics write | Mechanics-owned RNG requests; no kernel involvement |
| `combat.rs` | Applies calculated target HP loss | Kernel-routed | Damage mechanics calculates and orders events; kernel applies bounded HP loss |
| `combat.rs` | Resolves switch cleanup and coupled outgoing/incoming positions | Kernel-routed | Switch hooks/order remain mechanics; kernel applies the atomic structural change |
| `combat.rs` | Ticks/removes active timed volatiles | Kernel-routed | Residual ordering remains mechanics; kernel calls the retained invariant-preserving state operation |
| `conditions.rs` | Draws sleep duration and installs/cures major status | Kernel-routed | Status eligibility/duration stays mechanics; kernel owns coherent status writes |
| `conditions.rs` | Decrements sleep/freeze counters, cures status, advances freeze RNG | Kernel-routed | Condition callbacks decide; kernel applies counter/status changes; RNG stays mechanics-owned |
| `conditions.rs:191-201` | Advances paralysis RNG and may cancel action | Mechanics write | Condition mechanics; no kernel write except future consequence APIs |
| `conditions.rs` | Increments toxic counter and subtracts residual HP | Kernel-routed | Residual formula/order stays mechanics; kernel applies bounded counter and HP changes |
| `state/pokemon.rs:154-165` | Writes one bounded stat stage | Kernel primitive | Retained invariant method called by `StateKernel::change_stat_stage`; no supported production effect changes stages yet |
| `state/pokemon.rs:230-254` | Installs/cures status and coherent counters | Kernel primitive | Retained invariant methods called only through the kernel in supported production mechanics |
| `state/pokemon.rs:256-304` | Adds/removes/ticks unique volatile conditions | Kernel primitive | Retained invariant methods called through the kernel in supported production upkeep |
| `state/pokemon.rs:328-340` | Clears switch-out state; resets toxic counter on entry | Kernel primitive | Called inside atomic kernel position operations; mechanics decides switch timing |
| `rng.rs:87-107`, `rng.rs:207-223` | Enables/disables/clears trace; advances stream state/draw count and appends trace | Kernel candidate (RNG-local) | `BattleRng` remains the sole RNG authority; gameplay kernel never draws RNG |

`event.rs` itself does not directly write battle state, but
`EffectHandler` callbacks receive `&mut BattleState`. Production
common-condition handlers now route their supported writes through
`StateKernel`. Custom/downstream handlers can still write any public field;
PR E must prevent ordinary consumers from using this unrestricted path while
preserving a controlled internal callback context.

## Public mutation surface

All fields on `BattleState`, `SideState`, `PokemonState`, `TeamState`,
`FieldState`, `ConditionState`, `MoveSlot`, `StatusState`, and `StatStages` are
currently public. `BattleState::side_mut`, `SideState::pokemon_mut`, and
`TeamState::pokemon_mut` expose nested mutation. Consequently, direct writes
outside this repository cannot be enumerated. This is an explicit PR E target,
not something PR A or PR B should hide with blanket setters.

The pending queue and `BattleState::rng` are also publicly mutable. Canonical
`step` is now the normal advancement path; PR E will narrow these escapes.
There is no second RNG today, and none was introduced by PR D.

## Test and fixture writes

Every `PokemonState`, `SideState`, and `BattleState` struct literal in the test
suite is fixture construction. The following additional mutations occur after
construction:

| Test area | Direct setup or corruption | Class / migration |
| --- | --- | --- |
| `rotom-core/tests/battle_state.rs` | HP and PP mutation on a clone | Test/fixture only; future valid builder |
| `rotom-core/tests/battle_flow.rs` | Stages, volatile, disable flag, status/counter setup | Test/fixture only; future scenario builder |
| `rotom-core/tests/legal_actions.rs` | Move targets/PP/disabled flags, trapped and Mega flags, side Mega-used flag | Test/fixture only; future scenario builder |
| `rotom-core/tests/action_queue.rs` | Mega capability and RNG trace configuration | Test/fixture only; trace API remains supported |
| `rotom-core/tests/common_conditions.rs` | Status/stage/volatile/HP setup and trace configuration | Test/fixture only; future scenario builder |
| `rotom-core/tests/event_effects.rs` | Ability/item/field/side/volatile setup; callback mutations; deliberate invalid active index; direct turn mutation in rollback handler | Test/fixture only; valid builder plus explicit invalid-state/callback test escape |
| `rotom-core/tests/rng.rs` | Direct RNG exercise and battle-owned RNG advancement | Test/fixture only; public deterministic RNG tests remain |
| `rotom-core/tests/state_transition_baseline.rs` | Status, stage, and volatile setup plus trace configuration | Test/fixture only; PR A regression baseline |
| `rotom-dex/tests/combat.rs` | Stat-stage fixture injection and RNG trace configuration | Test/fixture only; differential scenario builder |
| `rotom-protocol/tests/snapshots.rs` | HP mutation and direct RNG advancement for snapshot isolation | Test/fixture only; future builder/RNG fixture seam |

Generated JSON under `data/fixtures/` is immutable test input rather than a
runtime mutation site. The Showdown adapters write those files only in
explicit regeneration mode; `--check` is read/compare-only.

## Non-authoritative mutable data excluded from kernel migration

- `TurnResolution` action, residual, expiry, and event vectors.
- `EventDispatch` relay value, cancellation flag, and invocation list.
- `EffectRegistry` registrations and composed-registry construction.
- Temporary vectors used to build legal choices, action queues, collected
  effects, faint targets, and diagnostics.
- Static/generated Dex and format data loaders.

These values may mutate during calculation, but they are not the battle state
from which future simulation proceeds.

## Ownership audit result and PR dependencies

PR B added the internal `StateKernel` in `crates/rotom-core/src/kernel/` with
typed operations for bounded HP and PP changes, stage changes, coherent major
status and volatile lifecycle, coupled active-position changes, atomic faint
representation, and field/side conditions. `BattleState::validate_stable_state`
now checks HP, PP, stages, status counters, condition uniqueness/durations,
selected and active references, phase settlement, replacements, and terminal
winner consistency.

PR D routes supported production mechanics through that boundary for PP, HP,
status installation/cure/counters, volatile upkeep, voluntary switches,
initial leads, forced replacements, and faint cleanup. Mechanics still owns
eligibility, formulas, event order, RNG requests, and consequences. The kernel
does not dispatch callbacks or draw RNG.

PR C established the single phase-aware transaction. Its normal-turn path
calls crate-internal queue/resolver operations on one cloned input; the old
public queue/resolve methods keep their independent transactional clones only
as compatibility adapters. PR C also makes a no-op forced-switch choice legal
for the side with no replacement due, enabling complete two-sided joint
replacement actions.

Remaining direct authoritative writes are justified transition settlement
(`selected_team`, queue, turn, phase, winner), construction, or RNG-local
advancement. Public fields, `side_mut`, nested `pokemon_mut`, and the
`EffectCallback(&mut BattleState, ...)` signature remain unrestricted until PR
E; tests also continue to use direct fixture setup and deliberate corruption.

PR E should close those public mutation escapes and provide valid/test-invalid
builders. Registry composition remains immutable behavior outside battle state;
`Mechanics` composes built-in and caller-provided effects once at construction
and reuses that registry across transitions.

