# Rotom Spark — Battle-State-Centered Architecture Refactor

**Status:** Proposed architecture milestone (11.5), after milestone 11 and before expanding mechanics coverage.  
**Scope:** Architectural refactor only; preserve supported behavior and pinned-Showdown differential compatibility.  
**Reference:** `ROTOM_SPARK_IMPLEMENTATION_PLAN.md` and `AGENTS.md`.

## 1. Mission

Rotom Spark is a deterministic **battle-state transition engine**, not an interactive game application. Given complete battle state, joint actions, a deterministic random stream, and immutable mechanics configuration, it computes the next state.

The two governing principles are:

1. **State ownership:** The battle state is the core data model. The mutation kernel owns *how* it may be changed. The mechanics layer owns *why* and *when* changes occur. The transition function composes these into a deterministic next state.
2. **Invariant ownership:** The kernel enforces **representation invariants**; the mechanics layer enforces **Pokémon rules**.

Formally:

```text
(B_next, R_next, Events) = S(B_current, JointAction, R_current; Mechanics)
```

- `B_current` / `B_next`: authoritative structured battle data.
- `JointAction`: phase-appropriate combined decisions for both sides.
- `R_current` / `R_next`: one authoritative deterministic RNG stream, including its advanced state.
- `Mechanics`: immutable Pokémon data, regulation, and effect registration.
- `Events`: optional diagnostic output, never an authoritative input to replay.

The simulation must not depend on Showdown, UI, networking, or an AI framework at runtime. Showdown remains a pinned development oracle.

## 2. Hard architectural boundaries

| Component | Owns | Must not own |
| --- | --- | --- |
| Battle state | Complete mutable game data, inspection, cloning, serialization | Pokémon effect decisions, orchestration |
| Mutation kernel | Small atomic state writes and structural invariants | Type immunities, damage formulas, event dispatch, RNG |
| Mechanics/effects | Pokémon rules, calculation, eligibility, effect order and consequences | Arbitrary direct writes that bypass the kernel |
| Transition | Phase validation, joint-action ingestion, ordered mechanics execution, atomic commit | Individual Pokémon rule special cases |
| RNG | Deterministic random choices and optional tracing | Independently duplicated RNG streams |
| Static mechanics context | Dex, regulation, registered effects | Per-battle mutable state |

**Important distinctions:**

- The kernel is *mechanically ignorant*, **not unrestricted**. For example it can clamp HP to `[0, max_hp]`, reject duplicate volatile identities, and keep active-slot mappings consistent; it does not decide whether a move may damage a target.
- Some intermediate states (such as `HP == 0` immediately before faint processing) may be legal *inside a transition*. Clearly distinguish local mutation invariants from stable output invariants.
- Do not perform whole-battle validation after every primitive write. Validate affected structures locally, and run comprehensive validation at public transition boundaries.
- The kernel should not dispatch events. The mechanics layer handles events and follow-up consequences.
- Prefer typed Rust methods and static dispatch. Do **not** create a giant mutation-command interpreter, ECS framework, scripting engine, or blanket getters/setters for every field.

## 3. Current-code starting point

Audit at least:

```text
crates/rotom-core/src/state/battle.rs
crates/rotom-core/src/state/transition.rs
crates/rotom-core/src/state/decision.rs
crates/rotom-core/src/state/pokemon.rs
crates/rotom-core/src/state/queue.rs
crates/rotom-core/src/combat.rs
crates/rotom-core/src/conditions.rs
crates/rotom-core/src/event.rs
crates/rotom-core/src/rng.rs
crates/rotom-core/src/action.rs
crates/rotom-core/tests/
tools/showdown-reference/
data/fixtures/differential/
```

Existing strengths to **preserve**, not rewrite:

- Structured, cloneable, serializable `BattleState`.
- Typed `PokemonAction`, `TurnChoice`, `LegalActions`, and phase model.
- Deterministic `BattleRng` with traced reasons.
- Explicit queue and ordered, typed effect registry.
- Transactional queue/turn resolution.
- `StatStages::change`, status and volatile state methods, switch cleanup.
- Existing tests and pinned-Showdown fixtures.

Known refactor targets:

- `queue_turn` plus `resolve_turn` currently split a logical transition and clone state separately.
- `BattleState` exposes publicly mutable fields and `side_mut`.
- Some mechanics directly write HP, status counters, positions, etc.
- The RNG lives in `BattleState`; decide whether to separate it or retain it, **but never maintain two independently authoritative RNG states**.
- The built-in effect registry may be reconstructed for each resolution; consider sharing an immutable configured registry.

## 4. Execution plan: six focused PRs

**Merge sequentially.** A verification agent may develop test specifications concurrently, but multiple agents should not independently rewrite overlapping core files.

### PR A — Architectural contract and regression baseline

**Goal:** Establish invariants and freeze current supported behavior before changing implementation.

**Tasks:**

1. Read `AGENTS.md`, `ROTOM_SPARK_IMPLEMENTATION_PLAN.md`, source, tests, and relevant pinned Showdown research.
2. Write `docs/architecture/state-transition.md` documenting the equation, exact transition semantics, ownership, phase boundaries, RNG policy, and error behavior.
3. Create a **mutation ownership inventory** listing every direct write to authoritative state, classified as construction, kernel candidate, mechanics write, or test/fixture-only write.
4. Define representation invariants, stable-boundary invariants, and Pokémon-specific rules separately.
5. Add regression cases covering currently supported normal turns, status resolution, switching, fainting, and RNG behavior. Record existing fixtures and baseline command results.

**Acceptance:** No production mechanic changes; architectural contract and current behavior are documented; regression tests pass. Get agreement on RNG ownership and public API before PR B/C.

### PR B — Mutation kernel and invariant validation

**Goal:** Centralize state-writing primitives without introducing game rules.

**Suggested internal layout (adapt to existing project):**

```text
crates/rotom-core/src/kernel/
  mod.rs
  hp.rs
  status.rs
  stats.rs
  volatile.rs
  position.rs
  field.rs
```

**Tasks:**

1. Introduce a small internal `StateKernel<'a>` or equivalent typed operations on `&mut BattleState`.
2. Prefer **semantic, invariant-preserving operations**: `damage_hp`, `heal_hp`, `change_stat_stage`, `set_major_status`, `cure_status`, `add/remove_volatile`, `set_pp`, `switch_active_position`, etc. Not every field needs a generic setter.
3. Return meaningful mutation results (actual HP lost, applied stage delta, replaced/expired condition) where needed.
4. Reuse current stat/status/volatile methods instead of duplicating them.
5. Add `validate_stable_state()` or equivalent checking output invariants.
6. Keep public behavior unchanged; do not add Pokémon-specific immunity or damage calculations to kernel.

**Invariant examples:**

- HP never exceeds maximum or falls below zero.
- Stage values stay between `-6..=6` when changed through supported APIs.
- Status-specific counters are coherent.
- Volatile identities are unique where required.
- PP remains within valid slot limits.
- Active references and Pokémon positions are mutually consistent at stable boundaries.
- Selected-team references are unique and valid.
- Stable faint, winner, replacement, and phase relationships are consistent.

**Acceptance:** Focused kernel tests cover bounds, invalid references, coupled position updates, duplicate conditions, and stable-state validation. Existing mechanic tests unchanged in result.

### PR C — Canonical deterministic `step()`

**Goal:** Expose one authoritative, phase-aware transition API.

**Conceptual signature:**

```rust
// Illustrative only: finalize types and exact ownership in PR A.
fn step(
    current: &SimulationState,
    action: &JointAction,
    mechanics: &Mechanics,
) -> Result<StepResult, BattleError>;
```

Where `SimulationState` may contain `BattleState` plus `BattleRng`, **or** `BattleState` may continue to own RNG. Pick one and clearly document the actual formal inputs and outputs. Avoid retaining redundant synchronized copies.

`JointAction` should cover:

- `TeamPreview { p1, p2 }`
- `Turn { p1, p2 }`
- `ForcedSwitch { p1, p2 }`

**Tasks:**

1. Validate phase and complete joint action at transition entry.
2. Clone input state only once for the pure transition where practical.
3. Perform queue construction, resolution, status upkeep, faints/replacements, and phase settlement internally.
4. Ensure deterministic RNG consumption, including speed ties, and return the advanced RNG state.
5. Expose structured optional events; tracing must not change RNG or state outcomes.
6. Ensure failures leave the caller's state and RNG unchanged.
7. Keep old `queue_turn`/`resolve_turn` interfaces as temporary internal adapters if useful; avoid breaking tests abruptly.
8. Clearly specify what constitutes **one step**: one complete decision phase, not necessarily one battle turn. Partial submission may be supported only through an explicit documented alternative.

**Acceptance:** Supported battle flows advance using only `step()`; error atomicity, deterministic replay, and phase correctness tests pass. No doubled state clones in the canonical turn path.

### PR D — Mechanics/event-to-kernel integration

**Goal:** Remove scattered authoritative writes from the supported mechanics paths.

**Tasks:**

1. Route existing HP damage, status counters, stage changes, volatile lifecycle, PP, switches, and faint cleanup through the kernel.
2. Preserve existing event callbacks, ordering keys, queue semantics, and mechanic formulas.
3. Event handlers decide consequences and call kernel operations; kernel operations never dispatch effects themselves.
4. Keep immutable Dex/regulation/effect registration outside per-battle mutable state and avoid rebuilding registries unnecessarily where practical.
5. Reuse all pinned Showdown fixtures and preserve RNG draw sequence for identical supported cases.
6. Update mutation ownership inventory to show remaining justified direct writes.

**Acceptance:** No unintended mechanic changes, event ordering differences, or fixture drift. Each migration is covered by focused regression tests.

### PR E — Encapsulation and fixture construction

**Goal:** Prevent production consumers from bypassing the transition engine.

**Tasks:**

1. Restrict public mutable access to authoritative state fields and `side_mut`-style unrestricted escapes where feasible.
2. Preserve ergonomic read-only inspection, snapshotting, `Clone`, and Serde.
3. Introduce controlled state builders/constructors for artificial but valid differential scenarios.
4. Provide an explicit test-only path for intentionally inconsistent states used in error tests; do not normalize them silently.
5. Do not expose the internal kernel as the normal user-facing API.

**Acceptance:** A normal downstream crate can inspect and step state but cannot freely bypass mutation invariants. Existing tests and fixture creation have supported replacements.

### PR F — End-to-end verification and architecture gate

**Goal:** Demonstrate the new design is a deterministic transition system, not merely rearranged code.

**Required test matrix:**

| Test | Required property |
| --- | --- |
| Determinism | Same complete input yields bit-for-bit/equality-equivalent next state, RNG, and events |
| Pure API | Input state unchanged |
| Error atomicity | Invalid action, unsupported mechanic, or callback error leaves input/RNG unchanged |
| Replay | Serialize/deserialize and resume yields identical future |
| Branch isolation | Independent clones do not share mutable state |
| RNG branching | Different RNG streams support independent reproducible continuations |
| Diagnostics isolation | Trace enabled/disabled does not change gameplay or RNG draws |
| Phase coverage | Preview, normal turn, forced replacement, battle end |
| Structural invariants | All successful public outputs satisfy stable-state validation |
| Sequential parity | Canonical step agrees with old supported queue/resolve sequence |
| Mechanics parity | Existing damage/status/switch/faint tests maintain expected results |
| Showdown parity | Existing pinned differential fixtures continue to match |
| Full supported battle | A battle using only supported mechanics runs from preview through terminal state via `step()` |

Add focused property-based tests (e.g. `proptest`) for HP bounds, stages, references, conditions, and relevant transaction invariants. Avoid generating invalid input and then treating explicit validation errors as mechanic mismatches.

**Acceptance:** All required tests and quality gates pass, architectural boundaries have been checked against the inventory, and known limitations are documented.

## 5. Non-goals

Do **not**:

- Rewrite the event/effect system just because Showdown also has events.
- Replace callbacks with a giant dynamic mutation interpreter.
- Rebuild the battle engine from scratch.
- Introduce ECS, scripting, async, networking, a UI, or Python bindings.
- Add new Pokémon mechanics, abilities, moves, items, or field effects.
- Allow the kernel to calculate Pokémon rules or consume RNG.
- Turn event logs into authoritative state.
- Maintain duplicate authoritative RNG state.
- Reorder RNG draws or change existing mechanic semantics without explicit evidence and review.
- Claim full-battle completeness beyond the intentionally supported mechanic subset.
- Optimize without preserving testability and determinism.

## 6. Mandatory quality gates per PR

Run from workspace root:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Also run relevant pinned Showdown fixture generators with `--check` (including `tools/showdown-reference/common-conditions.cjs --check`) when the required local Showdown build is available. Report any unavailable tool/dependency rather than claiming those checks passed.

**Every PR report must include:**

1. Files changed.
2. Architectural changes and any API-breaking changes.
3. Current behavior preserved and fixtures checked.
4. Tests added, commands actually run, and results.
5. Remaining direct authoritative state writes, with justification.
6. Known limitations and next-PR dependencies.

Do not advance to the next PR with failing tests or unresolved contract contradictions.

## 7. Agent orchestration

| Order | Agent | Deliverable |
| --- | --- | --- |
| 1 | Architecture agent | PR A and approved transition/RNG contract |
| 2 | Kernel agent | PR B |
| 3 | Transition agent | PR C |
| 4 | Mechanics integration agent | PR D |
| 5 | Encapsulation agent | PR E |
| 6 | Verification agent | PR F and reviews of earlier PRs |

Do not have multiple implementation agents edit the same core files against incompatible contracts. A test reviewer may work concurrently.

## 8. Final definition of done

The milestone is complete only when:

- Battle state is clearly the authoritative model.
- A public phase-aware `step()` advances supported simulations deterministically.
- State and RNG remain unchanged on failure.
- The mutation kernel owns production state writes and representation invariants.
- Mechanics own Pokémon rules and event sequencing.
- Static mechanic data is separate from mutable battle state.
- The engine can clone, branch, serialize, replay, and inspect states.
- Ordinary consumers cannot arbitrarily mutate authoritative state.
- Existing supported behavior, deterministic traces, and pinned Showdown fixtures are preserved.
- Test and quality gates pass.

**Core principle for all future mechanics PRs:** read battle state → evaluate mechanics/events → compute intended changes → apply through kernel → resolve consequences → validate stable next state.
