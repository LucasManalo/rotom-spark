# Authoritative mutation ownership inventory

Status: PR A inventory of the pre-kernel code. Line references describe the
source at PR A and should be refreshed as each migration PR lands.

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
| `state/transition.rs:63` | Stores one side's selected-team order | Kernel candidate | Selection/preview transition operation |
| `state/transition.rs:113-121` | Clears outgoing switch state, writes outgoing/incoming positions, replaces active-slot reference | Kernel candidate | Atomic position/switch operation |
| `state/transition.rs:187-194` | Sets HP to zero, clears status/counters and switch state, marks fainted, empties active slot | Kernel candidate | Atomic `mark_fainted`; mechanics retains faint ordering/consequences |
| `state/transition.rs:247-259` | Prepares incoming replacement, writes position/slot, settles phase | Kernel candidate | Position operation plus transition-owned phase settlement |
| `state/transition.rs:302-316` | Prepares both leads, writes positions/slots, sets turn 1 and phase | Kernel candidate | Preview activation operations plus transition settlement |
| `state/transition.rs:345-363` | Writes winner and post-faint phase | Kernel candidate | Transition settlement; simultaneous-faint rule remains mechanics/transition policy |
| `state/queue.rs:75-85` | Clones and commits the whole battle transactionally | Kernel candidate | Replaced by canonical transition commit in PR C |
| `state/queue.rs:110-111` | Installs action queue and enters `ResolvingTurn` | Kernel candidate | Transition-owned queue/phase operation |
| `state/queue.rs:190-213` | Advances embedded RNG while shuffling exact speed ties | Mechanics write | Ordering mechanics requests RNG; RNG remains embedded and authoritative |
| `combat.rs:226-236` | Clones and commits the whole battle transactionally | Kernel candidate | Replaced by canonical transition commit in PR C |
| `combat.rs:330-338` | Clears queue/events, increments turn, settles phase | Kernel candidate | Transition-owned turn settlement |
| `combat.rs:370-375` | Decrements move PP | Mechanics write | Mechanics validates/requests; kernel applies bounded PP change |
| `combat.rs:487-492`, `632-646` | Advances RNG for critical, damage, and accuracy | Mechanics write | Mechanics-owned RNG requests; no kernel involvement |
| `combat.rs:586-594` | Subtracts target HP | Mechanics write | Damage mechanics calculates; kernel applies and reports actual loss |
| `combat.rs:676-687` | Clears outgoing state and writes both positions plus active slot | Mechanics write | Switch hooks/order remain mechanics; atomic position write moves to kernel |
| `combat.rs:700-718` | Ticks/removes active timed volatiles through `PokemonState` | Mechanics write | Residual mechanics requests; kernel/retained invariant method applies |
| `conditions.rs:69-124` | Draws sleep duration and installs/cures major status through `PokemonState` | Mechanics write | Status eligibility/duration stays mechanics; kernel owns coherent status write |
| `conditions.rs:149-187` | Decrements sleep/freeze counters, cures status, advances freeze RNG | Mechanics write | Condition callbacks decide; kernel applies counter/status changes |
| `conditions.rs:191-201` | Advances paralysis RNG and may cancel action | Mechanics write | Condition mechanics; no kernel write except future consequence APIs |
| `conditions.rs:205-227` | Increments toxic counter and subtracts residual HP | Mechanics write | Residual formula/order stays mechanics; kernel applies counter and HP changes |
| `state/pokemon.rs:154-165` | Writes one bounded stat stage | Kernel candidate | Retain and call from kernel |
| `state/pokemon.rs:230-254` | Installs/cures status and coherent counters | Kernel candidate | Retain and call from kernel |
| `state/pokemon.rs:256-304` | Adds/removes/ticks unique volatile conditions | Kernel candidate | Retain or wrap in kernel according to required battle context |
| `state/pokemon.rs:328-340` | Clears switch-out state; resets toxic counter on entry | Kernel candidate | Kernel performs structural writes; mechanics decides when switching occurs |
| `rng.rs:87-107`, `rng.rs:207-223` | Enables/disables/clears trace; advances stream state/draw count and appends trace | Kernel candidate (RNG-local) | `BattleRng` remains the sole RNG authority; gameplay kernel never draws RNG |

`event.rs` itself does not directly write battle state, but
`EffectHandler` callbacks receive `&mut BattleState`. Production common
condition handlers use that escape today, and custom/downstream handlers can
write any public field. PR D must pass supported production mutations through
the kernel. PR E must prevent ordinary consumers from using this unrestricted
path while preserving a controlled internal callback context.

## Public mutation surface

All fields on `BattleState`, `SideState`, `PokemonState`, `TeamState`,
`FieldState`, `ConditionState`, `MoveSlot`, `StatusState`, and `StatStages` are
currently public. `BattleState::side_mut`, `SideState::pokemon_mut`, and
`TeamState::pokemon_mut` expose nested mutation. Consequently, direct writes
outside this repository cannot be enumerated. This is an explicit PR E target,
not something PR A or PR B should hide with blanket setters.

The pending queue and `BattleState::rng` are also publicly mutable. PR C must
make canonical `step` the normal advancement path before PR E narrows these
escapes. There is no second RNG today, and none should be introduced.

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

Production mechanics currently bypass the future kernel for PP, HP, status
counters, switch positions, volatile upkeep, and faint settlement. These are
known migration sites, not PR A defects. PR B should add the smallest typed
operations needed by this inventory without changing callers' behavior. PR C
should establish the single phase-aware transaction and remove duplicated
queue/resolve commits. PR D should migrate the mechanics writes. PR E should
close public mutation escapes and provide valid/test-invalid builders.

No production write was moved in PR A.

