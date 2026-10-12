# Authoritative mutation ownership inventory

Status: Updated for PR E encapsulation. Line references are
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
  tests. Ordinary fixture setup uses the validating builder; intentionally
  invalid-state tests use explicit serialized corruption.

## Production writes

| Location | Authoritative writes | Current class | Planned owner |
| --- | --- | --- | --- |
| `rotom-dex/src/combat.rs:78-132` | Constructs runtime `PokemonState` and `MoveSlot` values from immutable resolved Dex data | Construction | Dex-to-state constructor; no running-battle mutation |
| `state/battle.rs:39-105` | Constructs turn, picked size, sides, field, embedded RNG, queue, phase, winner | Construction | State constructor; stable validation at return |
| `state/side.rs:16-27`, `state/pokemon.rs:375-379` | Constructs pre-preview side and runtime team containers | Construction | State constructors |
| `state/transition.rs` | Stores one side's selected-team order | Transition write | Justified crate-internal transition-owned preview input |
| `state/transition.rs`, `kernel/mod.rs` | Voluntary switches, forced replacements, and initial leads | Kernel-routed | Transition/mechanics validate and order; kernel atomically performs cleanup and coupled position/slot writes |
| `state/transition.rs`, `kernel/mod.rs` | Faint cleanup: HP/status/counters/volatiles/position/active slot | Kernel-routed | Mechanics owns faint ordering and settlement; kernel owns atomic structural representation |
| `state/transition.rs` | Writes turn, winner, and post-preview/faint/replacement phase | Transition write | Justified phase settlement; not an effect or primitive mechanics write |
| `state/queue.rs` | Clones state to construct a queue for inspection | Inspection adapter | Returns only the queue and never commits state or RNG advancement |
| `state/queue.rs:110-111` | Installs action queue and enters `ResolvingTurn` | Transition write | Justified queue/phase settlement |
| `state/queue.rs:190-213` | Advances embedded RNG while shuffling exact speed ties | Mechanics write | Ordering mechanics requests RNG; RNG remains embedded and authoritative |
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

`EffectCallback` receives `&mut EffectState`, not `&mut BattleState`.
`EffectState` permits immutable observation, documented mechanics RNG requests,
and narrow typed kernel-backed mutations. Custom handlers cannot obtain mutable
authoritative state or the internal `StateKernel`.

## Public mutation surface

`BattleState` fields are crate-private and expose read-only accessors. Its
`side_mut`, `SideState::pokemon_mut`, and `TeamState::pokemon_mut` paths are
crate-private, so public leaf state fields remain useful for initial detached
construction without allowing mutation through an authoritative battle.
The pending queue and embedded RNG are read-only through `BattleState`; trace
toggle methods change diagnostic metadata only. There is no second RNG.

`BattleStateBuilder` is the public controlled fixture boundary. It owns a
detached battle and validates stable invariants on `build`; it is not an
advancement protocol. Low-level queue/resolution, preview, switch, faint, and
replacement mutators are crate-private. `step` is the sole public advancement
operation.

## Test and fixture writes

Every `PokemonState`, `SideState`, and `BattleState` struct literal in the test
suite is fixture construction. The following additional mutations occur after
construction:

| Test area | Direct setup or corruption | Class / migration |
| --- | --- | --- |
| `rotom-core/tests/battle_state.rs` | HP and PP mutation on a clone | Validating scenario builder; serialized corruption for negative coverage |
| `rotom-core/tests/battle_flow.rs` | Stages, volatile, disable flag, status/counter setup | Validating scenario builder |
| `rotom-core/tests/legal_actions.rs` | Move targets/PP/disabled flags, trapped and Mega flags, side Mega-used flag | Validating scenario builder |
| `rotom-core/tests/action_queue.rs` | Mega capability and queue inspection | Validating scenario builder plus pure queue inspection |
| `rotom-core/tests/common_conditions.rs` | Status/stage/volatile/HP setup and trace configuration | Validating scenario builder |
| `rotom-core/tests/event_effects.rs` | Ability/item/field/side/volatile setup and callback effects | Validating builder/event-fixture dispatch; serialized corruption for invalid state |
| `rotom-core/tests/rng.rs` | Direct RNG exercise and battle-owned RNG advancement | Standalone RNG tests plus builder-owned replay setup |
| `rotom-core/tests/state_transition_baseline.rs` | Status, stage, and volatile setup plus trace configuration | Validating scenario builder; PR A regression baseline preserved |
| `rotom-dex/tests/combat.rs` | Selected battle and RNG trace configuration | Validating differential scenario builder |
| `rotom-protocol/tests/snapshots.rs` | HP mutation and RNG advancement for snapshot isolation | Validating builder/RNG fixture seam |

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
calls crate-internal queue/resolver operations on one cloned input. PR E removes
the public queue/resolve compatibility path and retains only pure queue
inspection. PR C also makes a no-op forced-switch choice legal
for the side with no replacement due, enabling complete two-sided joint
replacement actions.

Remaining direct authoritative writes are justified crate-internal transition
settlement (`selected_team`, queue, turn, phase, winner), construction, kernel
primitives, or mechanics-owned RNG advancement. Integration fixtures now use
`BattleStateBuilder`; deliberate invalid-state coverage uses explicit serialized
corruption and verifies rejection. Registry composition remains immutable
behavior outside battle state; `Mechanics` composes built-in and caller-provided
effects once and reuses that registry across transitions.

