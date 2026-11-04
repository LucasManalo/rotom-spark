# PR 15.7 — Acceptance-gate hardening

PR 15.7 closes the capability-driven milestone without adding mechanic
families or changing supported identity counts.

## Architecture and reporting

- Dedicated Mega eligibility, timing, form change, and ability transition are
  verified. Generic non-Mega form change remains unsupported.
- Mechanical writes remain kernel-owned. The canonical transition controller
  is the documented narrow owner of phase, turn, private queue, pending switch,
  and terminal orchestration on its cloned working state.
- PR 15.2--15.6 Showdown outputs are pinned-reference behavioral fixtures.
  Their Rust consumers assert selected normalized facts; they are not the full
  independent-engine state comparison reserved for PR 19.

## Hardening coverage

The added adversarial cases cover Magician and Pickpocket contact, occupancy,
and Mega Stone restrictions; Mummy and Wandering Spirit restoration on
switch-out; Unnerve and Magic Room berry suppression; Ripen's final rounding;
and error-atomic retry from a serialized interrupted-turn queue.

Source-level architecture tests additionally guard the single queued-move
executor, kernel-mediated atomic switch path, and the absence of direct
mechanical or transition-orchestration writes in production effect modules.

## Pinned basis and limits

The behavioral fixtures remain pinned to Pokémon Showdown commit
`b79ce06d06b8f0dc8de8bb348bc9cab5d931553f` and format
`gen9championsvgc2026regmc`. This pass does not promote partial capabilities,
implement deferred identities, or claim full differential parity.
