# AGENTS.md — Rotom Engine Codex Instructions

This file defines the working rules for Codex and other coding agents operating in the `rotom-engine` repository.

Read this file **before making changes**. Also read `ROTOM_ENGINE_IMPLEMENTATION_PLAN.md` for the broader architecture and milestone plan.

---

## 1. Project Mission

Build a **standalone, deterministic, high-performance Rust battle simulator** for **Pokémon Champions Regulation M-C**.

During Phase 1, the behavioral reference is the **pinned Pokémon Showdown Champions Regulation M-C implementation**.

The finished simulator must:

- run without Pokémon Showdown at runtime;
- support complete local battles from team preview through battle end;
- be deterministic from explicit inputs and RNG seed;
- expose typed, inspectable battle state;
- support legal-action enumeration;
- support cheap battle-state cloning;
- be suitable for large-scale self-play and future AI search;
- eventually expose a Python interface, but **Python/ML work is not part of Phase 1**.

Pokémon Champions itself is **not** the Phase 1 oracle. Direct comparison against the real game is future work.

---

## 2. Primary Engineering Rules

These rules override convenience and speed of implementation.

1. **Correctness before optimization.**
2. **Never silently approximate unsupported mechanics.** Return an explicit error or mark the mechanic unsupported.
3. **Do not invent Pokémon behavior.** If behavior is unclear, inspect the pinned Showdown implementation and add a focused test.
4. **Do not introduce a runtime dependency on Showdown.** Showdown is development/reference tooling only.
5. **All randomness must flow through battle-owned deterministic RNG.** Never use thread-local/global RNG inside mechanics.
6. **Keep mechanics independent from AI code.** The battle engine must not depend on PyTorch, neural networks, reward functions, policy/value models, MCTS, or training infrastructure.
7. **Use typed Rust state, not battle-log text, as the source of truth.**
8. **Keep static game data separate from runtime battle state.**
9. **Keep format legality separate from general mechanics.** Regulation M-C should be a format/configuration layer.
10. **Prefer small, testable changes over broad rewrites.**

---

## 3. Source of Truth During Phase 1

The Phase 1 reference is the Pokémon Showdown version recorded in `SHOWDOWN_VERSION`.

Expected metadata:

```text
repo=https://github.com/smogon/pokemon-showdown
commit=<PINNED_SHA>
format=gen9championsvgc2026regmc
mod=champions
```

Do not silently update the pinned Showdown revision.

If the reference needs to change:

1. update the pinned revision intentionally;
2. regenerate derived data if necessary;
3. run the differential suite;
4. inspect every behavioral difference;
5. update Rotom only where the new reference behavior is intentionally adopted.

Generated fixtures should record the Showdown commit when practical.

---

## 4. Scope Boundaries

### In scope for Phase 1

- Rust workspace and core libraries;
- Champions Regulation M-C data and legality;
- team parsing/representation;
- team preview and bring-six/pick-four;
- doubles battle state;
- move selection and switching;
- action legality;
- deterministic RNG;
- action ordering;
- damage calculation;
- statuses;
- stat stages;
- volatile conditions;
- weather;
- terrain;
- side conditions;
- items;
- abilities;
- moves;
- forms and Mega Evolution where applicable;
- fainting and replacement flow;
- battle completion/winner detection;
- serialization and snapshots;
- differential testing against Showdown;
- benchmarks for simulation and cloning.

### Explicitly out of scope unless a task says otherwise

- Python bindings;
- PyTorch;
- neural-network training;
- reinforcement learning;
- MCTS/search agents;
- team-building AI;
- Showdown ladder bots;
- GUI/web UI;
- databases;
- distributed simulation;
- direct Pokémon Champions automation;
- packet inspection or reverse engineering of Champions;
- Champions RNG reconstruction;
- Champions-as-oracle validation.

Do not implement future systems merely because the architecture mentions them.

---

## 5. Repository Architecture

Preserve separation of concerns. The intended shape is:

```text
crates/
  rotom-core/       battle state, actions, resolution, events, RNG
  rotom-dex/        static species/move/item/ability/etc. data
  rotom-formats/    Regulation M-C and future format rules
  rotom-protocol/   normalized snapshots/events/protocol types as needed
  rotom-python/     future; do not implement during Phase 1 unless asked

tools/
  showdown-reference/
  showdown-export/
  differential/
  corpus-generator/

data/
  generated/
  fixtures/

tests/
  mechanics/
  differential/
  regression/
  full_battles/
```

Do not collapse everything into `rotom-core` for convenience.

Do not create unnecessary crates either. Add a crate only when it has a clear architectural responsibility.

---

## 6. Battle State Rules

Runtime battle state must be structured and explicit.

Conceptually, preserve distinctions such as:

```text
TeamDefinition
    !=
PokemonState
    !=
PlayerView
```

`TeamDefinition` represents what entered the match.

`PokemonState` represents mutable in-battle state such as:

- current HP;
- status;
- stat stages;
- PP;
- item state;
- ability state;
- form changes;
- volatiles;
- active position;
- fainted state.

A future `PlayerView` may hide unrevealed opponent information, but the internal simulator should retain the complete authoritative state.

Do not implement hidden information by deleting information from the simulator state.

---

## 7. Battle Phase Model

Battle phases should be explicit rather than inferred from scattered flags.

Expected conceptual phases include:

```rust
enum BattlePhase {
    TeamPreview,
    TurnSelection,
    ResolvingTurn,
    ForcedSwitch,
    BattleEnd,
}
```

Only actions legal for the current phase may be accepted.

Invalid phase/action combinations must return explicit errors.

---

## 8. Action Model

Player choices must use typed action values, not arbitrary command strings.

Conceptual example:

```rust
pub enum PokemonAction {
    Move {
        move_slot: u8,
        target: Target,
        mega: bool,
    },
    Switch {
        pokemon: PokemonIndex,
    },
}
```

The engine owns action legality.

Expose or implement an equivalent of:

```rust
battle.legal_actions(side)
```

This function must be deterministic and complete for the current state.

Future AI code will rely on this as the action mask, so do not treat it as optional utility code.

---

## 9. RNG Rules

Every random decision must pass through battle-owned deterministic RNG.

Examples include:

- damage rolls;
- accuracy;
- critical hits;
- secondary effects;
- confusion/paralysis checks;
- multi-hit counts;
- speed ties;
- random targeting;
- random move selection.

Forbidden inside battle mechanics:

```rust
rand::thread_rng()
```

or any equivalent hidden/global entropy source.

A battle should be reproducible from:

```text
engine version
format
team A
team B
team-preview selections
ordered player choices
seed
```

When practical, support an optional RNG trace containing reason, range/probability, and selected result.

---

## 10. Event and Effect Architecture

Do not implement the entire game as one giant turn-resolution function or one giant `match move_id` block.

Mechanics repeatedly intercept battle operations. Use a structured event/effect system.

Typical event categories include:

```text
BeforeTurn
BeforeMove
ModifyPriority
ModifySpeed
ModifyAttack
ModifyDefense
ModifyBasePower
ModifyDamage
TryHit
Hit
AfterDamage
AfterMove
SwitchIn
SwitchOut
Faint
Residual
```

The exact behavior and ordering must come from the pinned Showdown implementation.

Keep effect implementations organized by concern, for example:

```text
effects/
  moves/
  abilities/
  items/
  statuses/
  weather/
  terrain/
  side_conditions/
  volatiles/
```

Favor static/typed Rust designs where possible. Do not mechanically reproduce Showdown's dynamic TypeScript architecture if a simpler Rust design preserves behavior.

---

## 11. Effect Ordering

Pokémon mechanics are highly sensitive to ordering.

Do not scatter ordering assumptions through unrelated functions.

Use explicit ordering metadata or another centralized mechanism sufficient to reproduce Showdown behavior.

Tests should cover ordering conflicts between:

- actions;
- priority modifiers;
- effective speed;
- abilities;
- items;
- statuses;
- weather;
- terrain;
- side conditions;
- move effects;
- residual effects.

When a discrepancy is found, add a regression test before or with the fix.

---

## 12. Damage System

Keep damage calculation in a dedicated subsystem.

Prefer a design capable of exposing intermediate values during debugging, such as:

- selected attacking stat;
- selected defending stat;
- base damage;
- STAB;
- type effectiveness;
- critical modifier;
- weather modifier;
- random modifier;
- final damage.

Do not duplicate damage formulas across move implementations.

Champions-specific stat calculation must be treated as its own mechanic and explicitly tested.

---

## 13. Static Data and Showdown Export

Do not manually transcribe large portions of Showdown data.

Prefer tooling that loads the pinned Showdown `champions` mod and exports **resolved** data into normalized machine-readable files.

Important: export the resolved Champions data, not merely base Gen 9 data plus assumptions about overrides.

Static data and behavior are separate concepts:

```text
MoveData != move behavior
ItemData != item behavior
AbilityData != ability behavior
```

A data record may refer to a typed effect implementation.

Generated files should be reproducible. Avoid editing generated files by hand.

---

## 14. Regulation M-C Format Layer

Do not bake Regulation M-C restrictions into unrelated mechanics.

The format layer should own concerns such as:

- legal species/forms;
- legal moves;
- legal abilities;
- legal items;
- team size;
- bring-six/pick-four;
- level behavior;
- species clause;
- item clause;
- format-specific restrictions.

Future Champions regulations should reuse the same battle engine wherever mechanics are unchanged.

---

## 15. Serialization and Snapshots

Gameplay-relevant state should be serializable where practical using `serde`.

Provide a stable normalized snapshot representation for testing and debugging.

Snapshots should contain gameplay state, not memory layout or incidental implementation details.

Useful snapshot data includes:

- turn and phase;
- active Pokémon;
- current/max HP;
- status;
- stat stages;
- item state;
- ability/form state where relevant;
- PP;
- weather;
- terrain;
- side conditions;
- relevant volatiles;
- fainted state;
- winner.

A failing randomized test should be saveable as a compact replay/regression fixture.

---

## 16. Differential Testing Against Showdown

Differential testing is a core development feature, not optional polish.

Use a small reference adapter under `tools/showdown-reference/` that drives the pinned Showdown simulator and emits normalized structured results.

Do not compare only the final winner.

Compare important state transitions including, where relevant:

- HP;
- status;
- stat stages;
- PP;
- item state;
- ability/form state;
- active slots;
- field state;
- side conditions;
- volatiles;
- faint state;
- legal choices;
- battle result.

Preferred progression:

1. focused mechanic fixtures;
2. randomized turn-level tests;
3. complete generated battles.

When exact RNG parity is not yet available, use controlled/forced random outcomes for focused mechanic comparisons where possible. Do not block basic mechanics development solely on perfect PRNG parity.

---

## 17. Regression Policy

Every confirmed mechanics bug should become a permanent regression test.

A good regression fixture contains enough information to reproduce the issue:

```text
engine version
Showdown commit
format
teams
seed/random controls
team selections if relevant
action sequence
expected normalized state
```

Do not fix a nontrivial mechanics discrepancy without adding a focused test unless there is a strong reason documented in the change.

---

## 18. Property and Invariant Tests

In addition to Showdown differential tests, assert internal invariants.

Examples:

- HP never becomes negative;
- HP does not exceed max HP except during a well-defined max-HP recalculation transition;
- fainted Pokémon have 0 HP;
- stat stages stay in legal bounds;
- PP never becomes negative;
- a Pokémon cannot occupy two active slots;
- inactive/fainted Pokémon cannot execute normal actions;
- an ended battle cannot continue resolving ordinary turns;
- action generation does not emit illegal actions.

Use property testing where it adds value.

---

## 19. Unsupported Mechanics Policy

During development, incomplete coverage is acceptable.

Silent fallback behavior is not.

If a legal mechanic is not implemented, use an explicit mechanism such as:

```rust
BattleError::UnsupportedMechanic(...)
```

or an equivalent typed error.

Maintain a machine-readable coverage registry if/when the project reaches broad mechanic implementation.

The eventual goal is 100% coverage of mechanics reachable in Regulation M-C.

---

## 20. Performance Rules

Do not sacrifice correctness for early micro-optimizations.

However, preserve architecture compatible with fast simulation:

- no unnecessary heap-heavy object graphs;
- no async inside a single battle;
- no global mutable state;
- immutable shared Dex data;
- battle-local mutable state;
- battle state should be cheap enough to clone/fork for search;
- parallelism belongs around independent battles, not inside one turn unless proven necessary.

Track benchmarks once the relevant systems exist.

Useful metrics include:

- state transitions/sec/core;
- full battles/sec;
- battle creation cost;
- battle-state clone/fork cost;
- battle-state memory size;
- legal-action generation cost;
- damage calculation cost.

Do not introduce an ECS merely for theoretical performance. The domain has a small, structured number of entities and should use direct typed structures unless evidence shows otherwise.

---

## 21. Coding Standards

Follow normal idiomatic Rust practices.

Before completing a coding task, run where applicable:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

If some feature combination is intentionally unsupported, use the repository's documented canonical commands instead and explain the difference.

Additional expectations:

- avoid `unwrap()`/`expect()` in library code for recoverable input/state errors;
- use typed IDs/enums instead of free-form strings in core mechanics where practical;
- document non-obvious ordering/mechanics assumptions;
- keep public APIs narrow and purposeful;
- prefer explicit state transitions over hidden side effects;
- avoid giant files when a subsystem has clear internal boundaries;
- do not add dependencies without a concrete need;
- do not suppress Clippy warnings broadly to make CI green.

---

## 22. Dependency Policy

Before adding a new crate dependency, ask:

1. Is this functionality substantial enough to justify a dependency?
2. Is the crate actively maintained and appropriate for a simulation library?
3. Can the same result be achieved simply with the standard library or an existing dependency?
4. Does it introduce unnecessary async/global/runtime behavior?

Do not add large frameworks for small utilities.

Expected reasonable dependencies may include things like `serde`, test/property-test tooling, benchmarking tooling, and a deliberate RNG implementation.

---

## 23. How to Investigate a Mechanic

When behavior is unclear:

1. inspect the pinned Showdown Champions implementation;
2. trace inherited behavior into the relevant base implementation if necessary;
3. determine the exact event/order/data path;
4. write a small research note in `docs/research/` when the mechanic is complex or non-obvious;
5. create a focused fixture/test;
6. implement the smallest reusable engine behavior that reproduces it;
7. compare against Showdown.

Do not infer complex behavior from memory when the reference implementation is available.

For especially complex systems, separate research and implementation into different tasks/commits.

---

## 24. How Codex Should Tackle Work

Do **not** attempt to implement the entire simulator in one pass.

Work one milestone or subsystem at a time.

For each task:

1. read `AGENTS.md`;
2. read the relevant sections of `ROTOM_ENGINE_IMPLEMENTATION_PLAN.md`;
3. inspect the existing repository before editing;
4. identify existing abstractions that should be reused;
5. implement only the requested scope;
6. add or update focused tests;
7. run formatting, linting, and tests;
8. fix failures caused by the change;
9. do not implement later milestones unless required for the requested scope;
10. report what changed and what remains intentionally deferred.

Avoid speculative architecture for systems that are not yet needed.

---

## 25. Preferred Milestone Order

Unless the repository has already progressed past them, prefer roughly this order:

1. Rust workspace, crates, CI, and CLI skeleton.
2. Pin Showdown and build resolved Champions data export tooling.
3. Regulation M-C format data, team representation, and validation.
4. Core `BattleState`, `SideState`, and `PokemonState`.
5. Team preview, pick-four selection, leads, switching, fainting, and winner detection.
6. Typed action model and legal-action enumeration.
7. Deterministic battle RNG and optional RNG tracing.
8. Action queue, priority, effective speed, and tie handling.
9. Core type system, accuracy, base damage, STAB, and type effectiveness.
10. Event/effect framework.
11. Major status, stat stages, and volatile effects.
12. Weather, terrain, and side conditions.
13. Moves, abilities, and items in focused batches.
14. Form changes, Mega Evolution, and other Champions-specific mechanics.
15. Mature Showdown normalization/differential harness.
16. Randomized turn tests and full-battle corpus testing.
17. Performance profiling and targeted optimization.

A minimal Showdown reference adapter may be introduced earlier than step 15 so completed mechanics can be checked immediately.

---

## 26. First End-to-End Vertical Slice

The first meaningful engine checkpoint should be intentionally small.

Aim to support this complete path:

```text
load two teams
    -> validate them
    -> team preview
    -> each player picks four
    -> establish two leads
    -> enumerate legal ordinary moves
    -> accept one action for each active Pokémon
    -> order actions by priority/speed
    -> execute basic damaging moves
    -> update HP/fainting
    -> emit normalized state snapshot
    -> compare that turn against Showdown
```

Do not wait for complete move/item/ability coverage before proving this vertical slice.

---

## 27. Git and Change Discipline

Keep changes reviewable.

Preferred behavior:

- one coherent milestone/fix per change set;
- no unrelated formatting churn;
- no mass renames without need;
- generated data changes should be clearly separated from hand-written logic when practical;
- do not delete existing tests merely because a new implementation breaks them;
- preserve backward-compatible public APIs when reasonable, but correctness takes priority during early development.

If a task reveals that the requested design conflicts with existing architecture, explain the conflict and make the smallest sound change rather than rewriting unrelated systems.

---

## 28. Completion Report Required From Codex

At the end of each implementation task, report concisely:

### Implemented

- major behavior added;
- important architecture changes.

### Files changed

- relevant files/directories only.

### Tests

- tests added or changed;
- what behavior they prove.

### Verification

List commands actually run and whether they passed, for example:

```text
cargo fmt --all -- --check       PASS
cargo clippy ...                 PASS
cargo test --workspace           PASS
```

Never claim a command passed if it was not run.

### Deferred / known limitations

Explicitly list anything intentionally not implemented.

### Reference behavior

For mechanics work, note whether behavior was:

- directly compared against Showdown;
- derived by inspecting Showdown but not yet differentially tested;
- currently Rotom-only infrastructure with no battle-semantics comparison required.

---

## 29. Definition of Done for a Typical Mechanics Task

A mechanics task is not done merely because the code compiles.

Unless the task explicitly says otherwise, done means:

- the mechanic is represented in the intended architecture;
- common/important edge cases are covered;
- focused tests exist;
- relevant differential testing exists when feasible;
- unsupported adjacent behavior fails explicitly rather than silently approximating;
- formatting/lint/tests pass;
- documentation/comments explain non-obvious ordering decisions;
- no unrelated future systems were added.

---

## 30. Future AI Compatibility

The engine should remain compatible with future AI work without implementing that AI now.

Preserve these capabilities:

- arbitrary legal teams rather than a single hard-coded team;
- complete legal-action enumeration;
- deterministic stepping;
- cheap clone/fork of battle state;
- state serialization/snapshots;
- stable typed identifiers;
- eventual observation projection for hidden information;
- no global mutable battle state;
- no Python-specific types in core simulation APIs.

The ideal conceptual interface remains:

```text
state + legal joint action + deterministic RNG state -> next state
```

---

## 31. Final Non-Negotiables

Before making any significant design choice, preserve these five constraints:

> **Correctness first.** Unsupported mechanics must be explicit.

> **Pinned Showdown Champions Reg M-C defines Phase 1 reference behavior.**

> **Showdown must never be required at normal simulator runtime.**

> **Battle state must remain deterministic, inspectable, serializable, and cloneable.**

> **Core APIs should serve high-throughput simulation and future AI search, not mimic Showdown's UI or text protocol.**

