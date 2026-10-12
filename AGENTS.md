# AGENTS.md — Rotom Spark Coding Agent Rules

> **Read before editing.** This file is the permanent engineering contract for agents working in `rotom-spark`. Also read `ROTOM_SPARK_IMPLEMENTATION_PLAN.md` and, for architectural or mechanics work, `ROTOM_SPARK_STATE_TRANSITION_REFACTOR_PLAN.md`. If a referenced file is missing, report it instead of inventing its contents.

## 1. Mission and priority

Build **Rotom Spark**, a standalone, deterministic, high-performance Rust **battle-state transition engine** for Pokémon Champions Regulation M-C. This is a simulator for state branching, search, differential validation, and future large-scale AI self-play—not an interactive battle application.

Its conceptual contract is:

```text
(B_next, R_next, diagnostics) = S(B_current, JointAction, R_current; Mechanics)
```

- `B` is the complete authoritative battle state, not a player-facing observation.
- `JointAction` contains the decisions relevant to the current battle phase.
- `R` is the single authoritative deterministic random stream.
- `Mechanics` is immutable static game data, format rules, and effect definitions.
- Diagnostics are optional and **never** drive the simulation.

Use the **pinned Pokémon Showdown Champions Regulation M-C** as Phase 1's *behavioral* reference, never as a runtime dependency or an architectural mandate. Direct Pokémon Champions validation is future work.

**Priority order:** correct mechanics and invariants → reproducible transitions → testability and clarity → optimization after measurement.

## 2. Two governing architectural principles

1. **Battle state is the core data model.** The mutation kernel owns *how* state can change; mechanics own *why and when* it changes; the transition layer combines these into a deterministic next state.
2. **The kernel enforces representation invariants; mechanics enforce Pokémon rules.** Do not move immunity, damage formula, move legality, ordering, or other game-specific decisions into the kernel.

These principles apply to all new mechanics and every architectural refactor.

## 3. Ownership and boundaries

### State layer — authoritative data

Maintain distinct concepts: `TeamDefinition` (input set), `PokemonState` / `BattleState` (complete mutable battle information), and future `PlayerView` (information projection). The simulation must not forget hidden information to implement player observations.

State must remain inspectable, cloneable, serializable, and independent of any UI, Python runtime, network, Showdown process, or training framework. Keep immutable Dex and format data outside mutable battle state.

### Mutation kernel — structurally safe changes

The kernel provides small, statically dispatched, typed operations for HP, PP, stat stages, status/counters, volatile conditions, positions/active slots, field and side conditions, and related state writes.

- Check local structural invariants at the point of mutation; check whole-state invariants at stable transition boundaries.
- Prefer an operation that updates related fields atomically (`switch_positions`, `mark_fainted`) over independent setters that can leave contradictory references.
- Report actual applied changes (e.g. HP lost after clamping) when downstream mechanics need them.
- **No RNG draws, events, side-effect chains, damage calculations, immunity decisions, or Pokémon-specific rules inside the kernel.**
- The kernel is not a generic mutation-command interpreter or event bus.
- Existing invariant-preserving methods on state types may be retained and called by the kernel. Do not add abstraction solely to hide an assignment.

### Mechanics and event layer — rules and consequences

Mechanics read current state, apply Pokémon rules, resolve modifiers and ordering, request deterministic RNG outcomes, and invoke kernel operations. The mechanics layer owns consequences such as faint-processing after HP reaches zero. **Never bypass the kernel with direct writes in production mechanics.**

Retain the typed event/effect dispatch architecture when it accurately models mechanics. Do not copy Showdown's dynamic TypeScript object model, UI-oriented lifecycle, or protocol into Rust merely for resemblance.

### Transition layer — one authoritative public step

The canonical simulation API should accept a complete state, a phase-appropriate joint action, and a single RNG stream with immutable mechanics configuration; it should return the next state, advanced RNG, and optional diagnostics.

- A transition is **deterministic** from full inputs, including mechanics/version.
- Input state and RNG remain unmodified on failure. No partially committed public outputs.
- Ordinary consumers must not mutate production state directly or orchestrate queue/resolution/faint methods themselves.
- Team preview, turn resolution, forced replacements, and terminal behavior must have explicit phase-aware semantics.
- Keep low-level resolution helpers internal. One public `step()` may internally use queue construction, event dispatch, mechanics and kernel operations.
- Choose **one** authoritative RNG ownership model. Either an embedded RNG or a separate RNG is valid; do not maintain independently mutable duplicates. During migration, compatibility adapters must not cause double advancement.
- A pure `step()` may clone once; a later in-place variant is acceptable when error atomicity is preserved. Avoid unnecessary repeated cloning without premature performance rewrites.

**Refactor sequencing and acceptance criteria:** `ROTOM_SPARK_STATE_TRANSITION_REFACTOR_PLAN.md`. Follow its incremental PR approach; do not rewrite existing mechanics en masse.

## 4. State invariants

At stable public transition boundaries, verify at minimum:

- HP is between zero and max HP; fainted state, zero HP, and active-slot state agree.
- Every active slot references a valid distinct Pokémon, and positions agree bidirectionally with slots.
- Selected team indexes are valid and unique; active/bench/replacement membership is consistent.
- Stat stages remain in `[-6, +6]`; PP remains within the move-slot bounds.
- Major-status state and associated counters agree; volatile identifiers obey their declared uniqueness/duration constraints.
- The battle phase, winner and pending decisions/replacements are consistent.
- Terminal battles cannot accept ordinary actions.

Intermediate states during an internal transaction may temporarily await faint or switch settlement. Clearly document those transitional states; never expose them as stable successful transition output. Fixture construction should allow **arbitrary structurally valid scenarios**, with separately isolated negative tests for intentionally invalid states.

## 5. Behavioral source of truth and version pinning

The authoritative Phase 1 reference is `SHOWDOWN_VERSION`:

```text
repo=https://github.com/smogon/pokemon-showdown
commit=<PINNED_SHA>
format=gen9championsvgc2026regmc
mod=champions
```

Do not silently change the pinned commit. Updates require intentional version changes, regeneration of derived data/fixtures, running the differential suite, inspecting differences, and explicit adoption of behavior. Cite pinned source path/function/override in nontrivial research notes.

Showdown is **only** a specification, data-export source, and differential-test oracle. It must not be called during normal Rotom Spark simulation.

## 6. Scope and exclusions

Phase 1 supports Regulation M-C teams/data/legality; preview and pick-four; typed actions and legal-action enumeration; deterministic RNG; normal action ordering; damage; effects, statuses and stat stages; conditions, weather and terrain; items, abilities, moves, forms and Mega Evolution as implemented; fainting, replacement, victory; snapshots/serialization; differential tests; and later benchmarks.

Unless explicitly requested, **do not implement** Python bindings, PyTorch/ML, reinforcement learning, MCTS/search agents, team-building AI, ladder bots, UI, databases, distributed processing, Champions automation, packet inspection, Champions RNG reverse engineering, or Champions-as-oracle testing. Future compatibility does not authorize premature implementation.

## 7. Repository boundaries

Prefer existing workspace crates and typed modules:

```text
crates/rotom-core/       state, kernel, transition, mechanics, actions, events, RNG
crates/rotom-dex/        resolved immutable static data
crates/rotom-formats/    Regulation M-C format and legality
crates/rotom-protocol/   normalized snapshots and diagnostic events
crates/rotom-cli/        small development entry point
# rotom-python is future-only

tools/showdown-reference/  pinned reference adapter
tools/showdown-export/     resolved data exporter
tools/differential/        comparisons

data/generated/           generated data
data/fixtures/            test fixtures
docs/research/            source-backed mechanic notes
docs/architecture/        authoritative architecture contracts
```

Do not introduce an extra crate or huge abstraction without a demonstrated responsibility. Keep immutable data, mutable state, kernel writes, mechanics, and reference tooling distinct.

## 8. Phases, actions, legality

Represent phases explicitly (e.g. `TeamPreview`, `TurnSelection`, `ResolvingTurn`, `ForcedSwitch`, `BattleEnd`). Accept only choices legal for the current phase, with typed values rather than command strings.

The engine—not an agent—owns action legality, including PP, disabled moves, targeting, trapping, replacements, conflicting doubles switches, and special actions when supported. Legal-action enumeration must be deterministic and complete **for supported mechanics**. Reject unsupported legal cases explicitly; do not present unsupported actions as silently executable.

For the canonical transition API, use phase-aware joint decisions. Internal action queues remain inspectable for tests but must not become a required application-facing command protocol.

## 9. RNG, deterministic branching and replay

Every random outcome (accuracy, critical hits, damage, status, speed ties, multi-hit, random targets, etc.) uses the one authoritative `BattleRng` or equivalent explicit stream. Never use `thread_rng`, global RNG, hidden entropy, wall-clock randomness, or undocumented PRNG calls in mechanics.

A replay is defined by engine/version, format/mechanics version, teams, preview decisions, ordered joint actions, and initial RNG state. Preserve deterministic RNG traces with reason and operation when practical. Enabling/disabling diagnostics **must not** change results. Forking identical inputs must produce identical branches; separately seeded forks may diverge reproducibly.

## 10. Event and effect ordering

Preserve structured hooks such as `BeforeTurn`, `BeforeMove`, `ModifyPriority`, `ModifySpeed`, `ModifyAttack`, `ModifyDefense`, `ModifyBasePower`, `ModifyDamage`, `TryHit`, `Hit`, `AfterDamage`, `AfterMove`, `SwitchIn`, `SwitchOut`, `Faint`, and `Residual` where necessary.

Use explicit ordering metadata and pinned-Showdown-backed tests. Avoid a giant turn resolver or monolithic `match move_id`. Organize effects by moves, abilities, items, statuses, weather, terrain, side conditions, and volatiles as complexity warrants. **Events decide and sequence; kernel functions mutate.**

Test ordering interactions (priority, Speed/ties, items, abilities, statuses, residuals and field effects). A confirmed mismatch requires a regression test.

## 11. Damage subsystem

Calculate damage in a dedicated testable subsystem, with optional intermediate values: effective attacking/defending stats, base damage, STAB, type/critical/weather/random modifiers and final damage. Do not duplicate formulas across moves. The damage subsystem determines the result; the kernel applies actual HP changes. Champions-specific stat behavior must be source-backed and tested.

## 12. Static data, legality and formats

Export the **resolved Champions Dex** from pinned Showdown. Do not transcribe extensive data manually or infer Champions behavior from base Gen 9 overrides. Keep `MoveData`, `ItemData`, and `AbilityData` separate from their behaviors. Generated outputs are reproducible and should not be hand-edited.

Regulation M-C legality belongs to the format layer: species/forms, moves, abilities, items, level rules, team size/pick-four, item/species clauses and relevant restrictions. Reuse general mechanics across future regulations.

## 13. Serialization, snapshots and diagnostics

Use Serde for gameplay-relevant state. Normalized snapshots should capture meaningful battle state (phase/turn, actives, HP, status/counters, stat stages, PP, items, abilities, form, volatiles, field/side conditions, fainting, winner), not incidental Rust memory layout.

Logs and diagnostics are **not** authoritative. Randomized failures should be reproducible from a compact fixture containing sufficient inputs, versions, random controls and expected next state. Ensure serialize → deserialize → step maintains results.

## 14. Differential and regression testing

Differential testing is a required development tool. Use pinned Showdown under `tools/showdown-reference/` to produce project-owned, normalized results. Compare meaningful intermediate state, not just winners.

Testing progression:
1. Source-backed focused mechanic fixtures.
2. Randomized legal-state/turn comparisons.
3. Supported complete-battle corpus comparisons.

Use forced outcomes when seed parity isn't available. Do not equate one fixture checking callback constants with full battle equivalence. Every confirmed mechanics bug must have a permanent focused regression test; document exceptions.

## 15. Architecture test requirements

Beyond mechanic tests, test the mathematical transition contract:

- Same complete `(B, A, R, M)` → identical state, advanced RNG, diagnostics.
- Input immutability and error atomicity (including RNG).
- Clone/fork isolation and deterministic replay after serialization.
- Phase-aware progression from preview through supported turns, replacements and battle end.
- Output representation invariants, including position consistency and counters.
- Kernel edge cases: HP/stage/PP bounds, status exclusivity, volatile uniqueness and multi-field position updates.
- Diagnostic tracing not affecting outcomes.
- Equivalence with existing queue/resolve behavior and pinned differential fixtures.

Use property-based tests where useful. A full-battle test may use a narrow explicitly supported mechanic subset; do not fake completion by silently approximating unimplemented effects.

## 16. Unsupported mechanics policy

Incomplete mechanic coverage is acceptable during development; silent fallback is not. Return `BattleError::UnsupportedMechanic(...)` or an equivalent typed error when required behavior is absent. Track coverage accurately, updating it only when supported by tests. Target eventual complete coverage of Regulation M-C reachable mechanics.

## 17. Performance principles

Correctness first; architecture must permit high-throughput self-play and state branching. Favor typed compact state, immutable shared Dex/mechanics, no global mutable state, no async within a battle, and no giant heap-heavy object graph. Do not adopt ECS, scripting runtimes or mutation interpreters merely for theoretical performance.

Benchmark transitions/sec/core, full battles/sec, state clone cost, memory per state, legal-action enumeration, and relevant hot paths before optimizing. Do not sacrifice transaction atomicity to gain unmeasured speed.

## 18. Coding and dependency standards

Use idiomatic Rust, explicit typed IDs/enums, narrow public APIs, and documented non-obvious ordering. Avoid recoverable `unwrap()`/`expect()` in library paths and broad Clippy suppression. Don't add dependencies or crates without a concrete need, and don't perform mass renames or unrelated formatting churn.

Before marking implementation complete, run where applicable:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Run relevant Node/Showdown fixture-generation/check commands and differential checks when affected. Record commands **actually run**. Never claim success for checks not executed.

## 19. How to research mechanics

When unclear, inspect pinned Champions behavior and inherited Showdown code, identify the exact source function and hook order, document complex findings under `docs/research/`, add focused fixtures/tests, implement narrowly, then compare against the reference. Do not rely on memory for obscure rules or copy TypeScript architecture merely to reproduce its behavior.

## 20. Agent workflow

For each task:
1. Read this file, `ROTOM_SPARK_IMPLEMENTATION_PLAN.md`, and relevant architecture/refactor documents.
2. Inspect the current repository, code, tests, and existing abstractions **before editing**.
3. State the intended scope, ownership boundaries, and regression risks.
4. Work on **one small PR/milestone**; preserve behavior unless change is explicitly required.
5. Add tests alongside production changes.
6. Run formatting, Clippy, workspace tests and applicable differential tests.
7. Report files changed, architecture decisions, verified outcomes, remaining direct state writes, deferred behavior, and actual checks run.
8. Do not silently proceed into later phases or add unrequested systems.

For the state-transition refactor, follow the staged plan: **A** contract/baseline → **B** kernel → **C** unified transition → **D** mechanics integration → **E** encapsulation → **F** equivalence verification. Each phase should pass review before dependent production changes. Preserve the milestone-11 baseline.

## 21. Milestone planning

Historical milestones 1–11 established workspace/data/format/team/state/preview/actions/RNG/queue/base combat/event framework/common conditions. **Complete the state-transition architecture refactor (Milestone 11.5) before major new mechanics.** Then continue milestone 12 (field effects), milestone 13+ (move/ability/item batches), specialized Champions mechanics, advanced differential harness, randomized/full-battle validation, and finally profiling/optimization.

Do not implement later milestones as incidental additions. The first supported vertical slice should work end-to-end from team data and preview through actions, damage/fainting, structured snapshot and differential comparison, driven through the canonical transition interface after refactor.

## 22. Required task completion report

Every PR must include:

- **Implemented:** behavioral and architectural changes.
- **Files changed:** relevant files/directories.
- **Tests:** focused and regression tests and what they prove.
- **Verification:** exact commands executed and pass/fail result.
- **Showdown basis:** directly compared, derived from pinned source, or Rotom-only infrastructure.
- **Deferred / limitations:** unsupported mechanics and follow-up tasks.
- **Ownership audit:** any remaining direct battle-state writes outside the kernel, with reason or migration plan.

A task is **not done** merely because it compiles. It must preserve required invariants, intended semantics, deterministic behavior and relevant regressions.

## 23. Final non-negotiables

1. Battle state is authoritative; it is never reconstructed from logs.
2. Kernel owns structurally valid mutations; mechanics own Pokémon rules and consequences.
3. A single deterministic transition interface governs simulation and AI-facing advancement.
4. Exactly one authoritative RNG stream; errors must not partially advance state or RNG.
5. Phase 1 behavioral reference is pinned Champions Showdown; no runtime Showdown dependency.
6. State stays cloneable, serializable, inspectable and suited to inexpensive branching.
7. Unsupported mechanics fail explicitly.
8. Test the transition abstraction and invariants, not just individual moves.
9. No AI/UI/platform concerns inside the core engine.
10. Prefer disciplined incremental changes over a broad rewrite.
