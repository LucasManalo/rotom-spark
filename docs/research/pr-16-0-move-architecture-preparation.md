# PR 16.0 — Behavior-preserving move architecture preparation

## Scope

PR 16.0 performs one bounded extraction from the canonical combat module. The
deterministic static damage seams, `calculate_damage` and
`calculate_move_damage`, now live in `combat/damage.rs`. Live move execution,
hit sequencing, event dispatch, RNG draws, HP mutation, switching, Mega
Evolution, item lifecycle, queue interruption, and transition settlement stay
in the existing canonical `combat.rs` implementation.

This boundary is intentionally smaller than the illustrative directory tree in
the completion plan. Switching, transformation, and item activation are tightly
connected to queue/event orchestration today; extracting all of them in the
same PR would make behavior preservation harder to review. Future extraction
should happen only when a mechanics batch demonstrates a cohesive seam.

## Ownership boundary

`combat/damage.rs` accepts already-resolved immutable inputs. It does not own a
`BattleState`, call `StateKernel`, draw from `BattleRng`, dispatch events, or
resolve switches. The parent module remains the single queued-move executor and
uses the extracted internal calculation function after mechanics has resolved
effective stats, weather, critical state, spread state, and modifiers.

Mechanical state changes remain kernel-owned. The documented transition
exception remains limited to phase, turn, private queue, pending-switch, and
terminal settlement on the cloned working state. No public API, serialized
state, event contract, move capability, or generated coverage claim changes.

## Regression evidence

The source guard now counts canonical move executors across both combat source
files and rejects mutation, RNG, move execution, or switch resolution in the
static damage module. Existing behavior tests already cover the PR 16.0
semantic gates:

- `transition_step`: stable successful outputs, public error atomicity,
  deterministic replay, serialization continuation, and phase progression.
- `state_transition_baseline`: identical state resolution and RNG traces.
- `combat::eject_button_pauses_and_resumes_the_remaining_action_queue`:
  serialized interrupted-queue retry and deterministic resumption.
- `combat::champions_stats_and_damage_match_pinned_showdown_fixtures` and the
  canonical-step tests: reference-derived damage and RNG assertions.

These tests are behavior evidence, while the source-text checks remain narrow
regression aids rather than semantic proof.
