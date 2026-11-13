# Rotom Spark v2 engineering rules

These rules apply to work under `crates/rotom-engine`.

## Dependency direction

- State depends only on foundational value types needed to represent a battle.
- The kernel depends on state and exposes typed, invariant-preserving mutations.
- Mechanics may depend on read-only state, immutable game data, and explicit RNG
  interfaces; it requests changes through the kernel boundary.
- The effect interpreter depends on mechanics contracts and kernel operations.
- The battle controller composes the other boundaries and is the public
  transition entry point.
- Lower layers must not call the controller or interpreter. v2 must not depend
  on `rotom-core`, Showdown, UI, network, or training runtimes.

## State mutation ownership

- The kernel alone writes authoritative production state.
- Mechanics decides why and when; the kernel decides how to apply a structurally
  valid change.
- Construction and narrowly scoped test fixtures may build state directly, but
  are not production mutation paths.
- Stable controller outputs must satisfy whole-state invariants. Failed public
  transitions must leave caller-owned state unchanged.

## Deterministic RNG

- A battle has exactly one authoritative explicit random stream.
- Every random choice records a stable purpose at its call site; no global RNG,
  wall clock, hidden entropy, or diagnostic-dependent draw is permitted.
- Equal complete inputs—including engine/data versions and RNG state—must yield
  equal next state, advanced RNG, and gameplay-relevant ordering.
- Errors cannot partially advance the caller's RNG.

## Declarative move execution

- Ordinary moves are immutable data interpreted through a small typed operation
  vocabulary; they are not one handler or match arm per move.
- Operations express mechanic intent, not arbitrary field assignment.
- Ordering and targeting are explicit and testable. Exceptional mechanics use
  narrow typed extensions only after ordinary operations prove insufficient.
- Do not port v1 `combat.rs`, `event.rs`, or capability registries into v2. Add
  operations only with a concrete behavioral fixture and an ownership review.
