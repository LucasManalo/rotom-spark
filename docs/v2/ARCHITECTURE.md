# Rotom Spark v2 architecture boundaries

**Status:** Initial ownership contract. This document names boundaries; it does
not select concrete types, module layouts, or mechanics implementations.

Rotom Spark v2 is an independently compiled, deterministic battle-state
transition engine. It lives in `crates/rotom-engine`. Historical implementations
remain available in Git; v2 establishes independent runtime contracts.

## Ownership boundaries

### State

State owns the complete authoritative data needed to resume a battle. It is
inspectable, cloneable, serializable, and independent of logs, UI, networking,
Showdown, and training systems. State stores facts; it does not decide rules,
execute effects, or orchestrate a turn.

### Kernel

The kernel is the only production boundary allowed to mutate authoritative
state. It performs small typed changes and preserves local representation
invariants, including coupled updates that must be atomic. It does not calculate
damage, decide legality or immunity, consume RNG, dispatch effects, or encode
Pokémon-specific policy.

### Mechanics

Mechanics owns Pokémon rules: eligibility, calculations, modifiers, ordering
requirements, and the consequences that should be requested. It reads state and
immutable game data, requests randomness explicitly, and asks the kernel to
apply changes. Mechanics never writes authoritative state directly.

### Effect Interpreter

The effect interpreter executes a deliberately small, typed, declarative effect
language. It sequences ordinary move effects using mechanics decisions and
kernel operations. It is not a generic mutation-command bus, a scripting
runtime, an event-system port, or a copy of Showdown's object model. Exceptional
behavior may eventually use narrow typed extensions, but this PR defines no
operation set.

### Battle Controller

The battle controller owns the public phase-aware transition. It validates the
submitted joint decision, coordinates ordering and effect interpretation,
settles consequences such as fainting and required replacements, and commits one
stable next state atomically. It owns orchestration, not individual Pokémon
rules. Failed transitions do not expose partial state or RNG advancement.

## Boundary flow

```text
joint decision + current state + RNG + immutable mechanics data
                            |
                    Battle Controller
                            |
          Mechanics -> Effect Interpreter -> Kernel -> State
                            |
              next state + advanced RNG + diagnostics
```

Dependencies point toward stable data and mutation boundaries; control may
return results upward. Diagnostics observe this flow and never drive it.

## Deliberate non-decisions

This preparation PR does not define battle types, kernel operations, move
operations, event hooks, effect bytecode, registry shapes, or a coverage system.
It does not port v1 `combat.rs`, `event.rs`, or the v1 capability architecture.
Those choices require focused design and tests in later PRs.
