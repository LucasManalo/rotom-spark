# Battle-state transition contract

Status: PR F equivalence verification completes milestone 11.5. PR A
established this contract, PR B added the internal mutation kernel and
stable-state validation, PR C implemented the canonical phase-aware API, PR D
routed supported mechanics writes through the kernel, and PR E closed ordinary
downstream mutation escapes. PR F verifies those boundaries without expanding
the supported mechanics set.

## Transition model

Rotom Spark is a deterministic state-transition engine:

```text
(B_next, R_next, diagnostics) = S(B_current, JointAction, R_current; Mechanics)
```

- `B` is the complete authoritative gameplay state: turn, phase, both sides,
  every loaded and selected Pokémon, active positions, field state, pending
  queue, and winner.
- `R` is the one authoritative deterministic random stream.
- `JointAction` is one complete, phase-appropriate decision for both sides.
- `Mechanics` is immutable static Dex data plus the configured effect registry
  and format/version identity.
- Diagnostics describe a completed transition. They never become an input to
  later mechanics and enabling them must not alter gameplay state or RNG draws.

The physical representation will continue to embed `BattleRng` in
`BattleState`. `BattleState::rng` is the sole authoritative `R`; PR B and PR C
must not introduce a second synchronized RNG. Its seed, internal stream state,
and draw count are replay-relevant. Its optional trace entries are diagnostic
metadata, not gameplay authority.

## Approved canonical API direction

PR C implements the following public contract in
`crates/rotom-core/src/transition.rs`:

```rust
pub enum JointAction {
    TeamPreview { p1: TeamSelection, p2: TeamSelection },
    Turn { p1: TurnChoice, p2: TurnChoice },
    ForcedSwitch { p1: TurnChoice, p2: TurnChoice },
}

pub struct Mechanics<'a, D> {
    data: &'a D,
    effects: &'a EffectRegistry,
}

pub struct StepResult {
    pub state: BattleState,
    pub diagnostics: StepDiagnostics,
}

pub fn step<D: CombatData>(
    current: &BattleState,
    action: &JointAction,
    mechanics: &Mechanics<'_, D>,
) -> Result<StepResult, BattleError>;
```

`Mechanics::new` accepts immutable combat data and an immutable custom effect
registry, composes it with the built-in effects once, and owns the resulting
immutable registry for reuse across transitions. The concrete data provider and the caller's replay metadata identify
the pinned format/mechanics version; no mutable or duplicated version state is
stored in `BattleState`. `StepDiagnostics` contains queue-construction events
and, for ordinary turns, the existing structured `TurnResolution`.

`step` is pure from the caller's perspective. It clones the complete input at
most once, advances only the clone, and returns it only after successful phase
settlement and stable-state validation. An error returns no partially advanced
state, RNG, queue, or diagnostics. A later in-place optimization may exist,
but only if it provides the same error atomicity.

Low-level preview, queue/resolver, switch, faint, and replacement methods are
crate-internal. The canonical normal-turn path calls those internals on one
cloned state. `BattleState::inspect_turn_queue` remains a pure public inspection
helper: it validates and clones the input, constructs a queue on the clone, and
returns only that queue without advancing authoritative state or RNG.

## Meaning of one step

One step consumes exactly one complete decision phase, not necessarily one
battle turn.

| Input phase | Required joint action | Successful output |
| --- | --- | --- |
| `TeamPreview` | Both ordered selections, each complete | Initial leads installed, turn 1, `TurnSelection` |
| `TurnSelection` | Both complete doubles `TurnChoice` values | Queue built and fully resolved, including upkeep and faint settlement; output is `TurnSelection`, `ForcedSwitch`, or `BattleEnd` |
| `ForcedSwitch` | Both sides' complete replacement choices, using explicit passes where no decision is due | All required replacements settled, normally `TurnSelection`, or `BattleEnd` if settlement proves terminal |
| `ResolvingTurn` | None accepted publicly | Invalid as a stable public input and output |
| `BattleEnd` | None accepted | Every ordinary action is rejected without mutation |

Partial submissions are not part of canonical `step`. If an interactive
adapter later accepts one side at a time, its pending choices are adapter state
outside the authoritative transition and it must call `step` only after the
joint action is complete.

## Ownership boundaries

### Battle state

Battle state owns complete mutable facts and supports inspection, cloning, and
Serde round trips. It does not decide Pokémon rules. Team definitions and
immutable Dex/format data remain outside it. A future player observation is a
projection and must never replace hidden authoritative data.

Authoritative `BattleState` fields and nested mutable access are crate-private.
Downstream callers use read-only getters and `side`; RNG trace toggles are the
only public in-place changes and affect diagnostics only. `BattleStateBuilder`
owns a detached state for structurally valid scenario and differential-fixture
setup and validates all stable invariants before returning it. Deliberately
invalid tests corrupt a serialized fixture explicitly and assert rejection;
there is no normalizing constructor.

### Mutation kernel

The mutation kernel owns how authoritative state changes. It provides
small typed operations for HP, PP, status and counters, stat stages,
volatiles, positions/active slots, field/side conditions, and related coupled
writes. It enforces local representation invariants and returns actual applied
changes where mechanics need them.

The kernel must not consume RNG, dispatch events, calculate damage, decide
immunity or legality, choose targets, or encode move/ability/item rules. It is
not a generic command interpreter. Existing invariant-preserving methods such
as `StatStages::change`, `PokemonState::set_major_status`, and volatile helpers
should be reused.

### Mechanics and events

Mechanics own why and when a mutation happens: eligibility, formulas,
ordering, random requests, callbacks, and consequences such as faint
processing after HP reaches zero. Event handlers use `EffectState`, which
provides immutable battle inspection, mechanics-owned RNG requests, and narrow
typed kernel-backed mutations. Callbacks no longer receive unrestricted
`&mut BattleState`. The typed event registry and its ordering semantics are
retained.

### Transition layer

The transition layer validates the input phase and joint action, creates and
resolves internal queues, invokes mechanics/events in order, settles faints
and replacements, validates the stable result, and commits atomically. It must
not accumulate individual Pokémon special cases.

## Invariant taxonomy

### Local representation invariants

These hold after each completed kernel operation, even while a transition is
still resolving:

- `0 <= current_hp <= stats.hp`.
- Move PP is in `0..=max_pp`.
- Stat stages are in `-6..=6`.
- A major status and its status counters are coherent; curing clears all
  associated counters.
- Volatile identities that are unique are not duplicated; a fixed duration is
  never stored as zero.
- A position/active-slot operation updates both references atomically.
- Index-bearing writes reject out-of-range references.

An HP write may leave an active Pokémon at zero HP until mechanics dispatches
the `Faint` consequences. This is a documented internal transitional state,
not a stable output.

### Stable public-boundary invariants

These hold for every successfully created input and every successful `step`
output:

- Every selected-team index is in bounds and unique.
- Every occupied active slot references a distinct selected Pokémon, and that
  Pokémon's `position` points back to the same slot.
- A living active Pokémon has positive HP. A fainted Pokémon has zero HP,
  `PokemonPosition::Fainted`, no active-slot reference, no major status, and
  default status counters.
- Living selected Pokémon are partitioned consistently between active slots
  and the bench. Unselected loaded team members remain outside battle
  membership.
- `TeamPreview` has turn 0, no selected teams or active slots, no winner, and
  an empty action queue.
- `TurnSelection` has no pending action queue and no unresolved mandatory
  replacements.
- `ForcedSwitch` has at least one valid replacement request and no pending
  ordinary action queue.
- `ResolvingTurn` is internal only and contains the queue being resolved.
- `BattleEnd` has an empty queue and a phase/winner/remaining-team relationship
  consistent with the supported victory rules; it accepts no actions.
- Field, side, status, volatile, PP, and counter structures satisfy their local
  invariants.

PR B encoded these checks in `BattleState::validate_stable_state`.
Fixture builders may create arbitrary structurally valid scenarios. Deliberate
corruption belongs in isolated negative tests and must not be silently fixed.

### Pokémon-specific rules

These are not representation invariants and therefore do not belong in the
kernel: type and ability immunities, damage and accuracy formulas, move PP
eligibility, trapping, targeting, move priority, Speed and tie ordering,
status application eligibility, switch/faint event order, residual order,
Mega Evolution rules, and victory tie behavior. They remain mechanics,
legality, event, or transition concerns backed by the pinned Showdown source.

## Determinism, diagnostics, and errors

- Equal complete state (including authoritative RNG stream), equal joint
  action, and equal mechanics/version must produce equality-equivalent next
  gameplay state, RNG stream, and diagnostics.
- All random outcomes use the embedded `BattleRng`. Diagnostics must never
  cause extra draws.
- Trace enabled versus disabled may change only diagnostic trace storage; it
  must not change gameplay fields, RNG stream state, draw count, or outcomes.
- Invalid phase/actions, unsupported mechanics, missing data, invalid state,
  and callback failures are errors. The caller's input remains byte-for-byte
  serializable to the same value it had before the call.
- Diagnostics are emitted only for a successful committed transition. Logs do
  not reconstruct or drive battle state.

## Current baseline frozen by PR A

The baseline includes ordinary damaging turns, PP consumption, action/event
ordering, status before-move and residual behavior, voluntary switching,
faint settlement and forced replacement, deterministic RNG traces, Serde
round trips, clone isolation, and transactional queue/resolve failures.
`crates/rotom-core/tests/state_transition_baseline.rs` adds an integrated
freeze over normal-turn replay, switch/status upkeep, and damage-driven faint
settlement.

The following checks passed before PR A edits on 2026-10-11:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
node tools/showdown-reference/basic-damage.cjs --check
node tools/showdown-reference/event-order.cjs --check
node tools/showdown-reference/common-conditions.cjs --check
```

The checked differential fixtures are pinned to Showdown commit
`b79ce06d06b8f0dc8de8bb348bc9cab5d931553f` and format
`gen9championsvgc2026regmc`. PR A changes no fixture or supported mechanic.

## PR C implementation notes

- `step` validates a stable input and phase/action pairing before cloning the
  complete embedded-RNG state once. Only a validated stable result is returned.
- Team preview accepts both complete ordered selections in one call.
- A normal turn composes built-in and custom effects once, queues and resolves
  on the same clone, and preserves the prior queue/resolver behavior and RNG
  order. Queue events are copied into diagnostics before resolver settlement
  clears the authoritative queue.
- Forced replacement validates both sides' complete choices before mutation,
  then settles all requested slots. A side with no replacement due submits
  explicit `Pass` actions while its opponent replaces.
- `ResolvingTurn` remains internal and `BattleEnd` rejects every joint action.
- PR C does not route existing mechanics writes through the kernel or restrict
  public mutable fields; those remain the explicit scopes of PR D and PR E.

## PR D implementation notes

- Basic damage HP loss and PP consumption use bounded kernel operations; damage
  calculation, RNG draws, callback order, and diagnostic payloads are unchanged.
- Common-condition handlers retain their eligibility, duration, residual, and
  RNG decisions while the kernel applies status installation/cure, sleep/freeze
  counters, toxic counters, and residual HP loss.
- Timed volatile upkeep, voluntary switches, initial leads, forced
  replacements, and faint cleanup use kernel operations. Switch and faint event
  dispatch remains in mechanics/transition code and the kernel dispatches no
  effects.
- `Mechanics` composes built-in and custom immutable effects once at
  construction and reuses that registry across transitions as well as within
  queue construction and resolution.
- PR E owns public-field encapsulation and replacement of unrestricted
  callback/fixture mutation escapes, as documented below.

## PR E implementation notes

- `BattleState` gameplay fields, `side_mut`, and nested `pokemon_mut` access are
  crate-private; read-only getters preserve inspection and snapshot support.
- Low-level queue/resolution and lifecycle mutation entry points are internal.
  Public advancement remains `step`; pure queue inspection is available for
  tooling and tests.
- `BattleStateBuilder` supports validated scenario setup, including status,
  switch/faint/replacement, field/side/Pokémon edits, and explicit replay RNG
  setup. `build` never repairs invalid state and returns an error instead.
- `EffectState` replaces unrestricted mutable battle access in callbacks and
  exposes only typed kernel operations plus immutable observation.
- Serde deserialization intentionally remains representationally faithful and
  unchecked. Negative tests use visibly corrupted serialized fixtures, while
  every public transition validates before work and rejects them atomically.

## PR F verification matrix

PR F closes the refactor with an executable mapping from each acceptance
property to focused coverage. Property tests use deterministic proptest seeds
on failure and do not replace the source-backed fixture suites.

| Property | Verification |
| --- | --- |
| Determinism and pure API | `successful_public_turns_are_pure_deterministic_and_stable` and `replay_and_serde_resume_are_deterministic_and_leave_input_unchanged` in `transition_step.rs` |
| Error atomicity | Invalid phase/action and unsupported-mechanic checks in `transition_step.rs`; callback rollback in `event_effects.rs` |
| Replay after serialization | `replay_and_serde_resume_are_deterministic_and_leave_input_unchanged` |
| State branch isolation | Clone-isolation coverage in `battle_state.rs` plus the transition property test |
| RNG branch independence and reproducibility | `differently_seeded_branches_are_independent_and_reproducible` |
| Diagnostic isolation | `rng_tracing_does_not_change_transition_outcomes_or_draws` |
| Phase coverage | `step_advances_preview_turn_and_forced_switch_phases` and `supported_battle_runs_from_preview_through_replacements_to_terminal` |
| Structural invariants | Kernel property tests for bounded HP, PP, stages, references, and conditions; every successful transition property case calls `validate_stable_state` |
| Sequential-path parity | `canonical_turn_matches_internal_queue_then_resolve_sequence` compares `step` with the crate-internal legacy sequence |
| Supported mechanics parity | `state_transition_baseline.rs`, `common_conditions.rs`, `event_effects.rs`, and `rotom-dex/tests/combat.rs` preserve damage, condition, event, switch, faint, and RNG behavior |
| Pinned Showdown parity | CI checks basic-damage, event-order, and common-condition fixtures after building the pinned Showdown commit |
| Complete supported battle | `supported_battle_runs_from_preview_through_replacements_to_terminal` runs preview, ordinary turn, forced replacements, a second turn, and terminal settlement only through `step` |

The PR F property cases intentionally target representation and transaction
invariants rather than inventing new Pokémon behavior. The complete-battle
case uses only the explicitly supported basic-damage vertical slice.

## Milestone 11.5 gate result

The state-transition refactor now has one public phase-aware advancement API,
one embedded authoritative RNG, kernel-routed supported mechanics writes,
validated stable outputs, error-atomic failure paths, and an encapsulated
authoritative state. The equivalence matrix above is the regression gate for
future mechanics work.

Known limitations remain explicit: the engine still covers only the supported
milestone-11 mechanics subset; Serde can deserialize representationally invalid
data but `step` rejects it before work; and current differential verification
uses the three checked pinned-Showdown adapters rather than a generalized
randomized `tools/differential` harness. Those are later-milestone concerns,
not silent fallbacks in this refactor.

## PR 13.1 move-executor extension

PR 13.1 preserves this complete transition contract while replacing the
seven-ID `BasicDamage` runtime gate with one capability-preflighted
`execute_move` lifecycle. The queue, embedded RNG, effect registry, kernel,
faint settlement and stable output validation remain the same authorities.
Intrinsic behavior missing from the executor is rejected before PP or move RNG
within the private transaction. See `unified-move-executor.md` for the detailed
mapping, interfaces, and deferred capabilities.

