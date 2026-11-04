# Rotom Spark — Battle Engine Completion Gameplan

**Repository:** [LucasManalo/rotom-spark](https://github.com/LucasManalo/rotom-spark)  
**Historical review baseline:** `master` at `9d9f45d` (October 30, 2026)  
**Planning update:** PR 15.7 at `65207f8` (November 2026); refresh generated counts after each implementation batch.  
**Target:** Complete, correct, deterministic Regulation M-C battle simulation suitable for Rotom Brawl training.

## Executive summary

Rotom Spark has a strong foundation: an explicit battle state, a mutation kernel, deterministic RNG, event/effect dispatch, generated Regulation M-C mechanics reachability, and broad initial move, ability, and item support. The remaining challenge is **closing engine capabilities**, rather than merely adding more mechanic identities.

At the historical PR 14.4 review baseline, the generated mechanics-status report recorded (historical numbers, not current progress):

| Kind | Reachable | Implemented | Partial | Unsupported |
| --- | ---: | ---: | ---: | ---: |
| Abilities | 215 | 120 | 95 | 0 |
| Items | 166 | 47 | 119 | 0 |
| Moves | 510 | 223 | 0 | 287 |

These are implementation labels, **not** complete verification or interaction-parity guarantees. The recent PR 14.0–14.4 commits expanded held-item behaviors and Mega Stone identities; Mega Stone identity mapping does not by itself complete Mega Evolution in live battles.

**Current strategy:** PR 15.0–15.7 established shared lifecycle foundations and reporting; freeze PR 15. Begin PR 16 with a small, behavior-preserving architecture pass, then implement missing moves by capability family with incremental reference-backed testing. Close remaining abilities/items and forms, execute broad differential comparisons in PR 19, and optimize batched RL simulation in PR 20. Do not reopen PR 15 for nonblocking polish.

**Coverage reporting rule:** PR 14.4 should be described historically as **“Mega Stone metadata foundation”**, not “71 items implemented.” PR 15.4 subsequently verified the dedicated Mega eligibility, timing, form-change, and ability-transition capabilities and unlocked those identities; generic non-Mega `form_change` remains unsupported. `0 unsupported identities` only means every reachable identity has at least some recognized handling; it does **not** mean the engine is complete. Report identity status **and** capability status separately.

## Current planning snapshot (after PR 15.7)

Read `data/generated/reg-mc-capability-coverage.md` as the authoritative generated snapshot, not the historical PR 14.4 table above. At PR 15.7, it reports 123/215 abilities implemented (92 partial), 141/166 items implemented (25 partial), and 241/510 moves implemented (269 unsupported). Capability coverage is 12 verified, 21 implemented, 21 partial, and 17 unsupported out of 71. **These counts are a dated snapshot, never hard-coded acceptance thresholds:** regenerate before every batch and report both identity and capability deltas. Pinned-reference behavioral assertions are not full-state differential parity.

## Roadmap

| Milestone | Objective | Definition of done |
| --- | --- | --- |
| **PR 15 (freeze)** | Capability-driven closure and hardening | PR 15.7 completes reporting, representative lifecycle tests, and architectural ownership guardrails; remaining capability gaps are explicit |
| **PR 16** | Prepare move architecture, then finish reachable moves | PR 16.0 preserves behavior while extracting cohesive modules; move families pass unit, interaction, and pinned-reference tests; all 510 reachable moves become executable without unresolved required capability blockers |
| **PR 17** | Close abilities and items | All 215 reachable abilities and 166 reachable items are fully implemented, with known gaps resolved rather than relabeled |
| **PR 18** | Finish remaining forms and Champions-specific transformation behavior | Build on the PR 15 Mega vertical slice; all other reachable form paths work end-to-end |
| **PR 19** | Full-state differential correctness campaign | Independent-engine normalized state comparisons, generated interactions, and full-battle replays agree with pinned Showdown within documented format differences |
| **PR 20** | Production-grade RL interface and performance | Deterministic battle stepping, legal-action queries, observations, and measured high-throughput batching |

The milestones are capability-oriented; exact PR boundaries can change as dependency evidence accumulates.

---

## PR 15 — Capability-driven mechanics closure

**Implement reusable capabilities rather than batches of identity names.** A capability is complete only when its trigger conditions, ordering, state transitions, cleanup, interacting abilities/items/moves, and verification evidence are complete. Do not promote dependent identities merely because they are registered.

### PR 15.0 — Capability coverage and dependency report

Generate a machine-readable dependency map for every partial ability/item and unsupported move. For each capability, report:

- `status`: unsupported / partial / implemented / verified;
- `blocked_identities`: the mechanics that currently depend on it;
- `unlocked_identities`: identities whose **remaining** blockers are fully resolved;
- `sub_capabilities`, lifecycle/ordering requirements, and known gaps;
- tests, differential fixtures, and pinned-reference evidence.

The headline report must show **identity coverage separately from executable capability coverage**, including outstanding partial/unsupported counts. Example dependencies: `choicescarf -> choice_lock`, `persimberry -> confusion`, `ejectbutton -> forced_switch`, and Mega Stones -> `mega_eligibility`, `mega_timing`, `mega_form_change`, `mega_ability_transition`.

**Exit:** Reports distinguish metadata-only, partially executable, fully executable, and verified behavior. There are no misleading “0 unsupported = 100% complete” summaries.

### PR 15.1 — Complete berry activation framework

Implement one shared berry lifecycle, not 28 one-off implementations:

1. Evaluate trigger and eligibility at the correct timing point (including pre-existing status on entry, damage, residual effects, and update paths).
2. Apply item suppression/Unnerve/Klutz/Magic Room and applicable ability interactions.
3. Consume through the kernel, record the consumed item, apply the effect, and fire follow-up effects.
4. Resolve effects such as Ripen, Cheek Pouch, Cud Chew, Harvest, and Symbiosis through reusable hooks where reachable.
5. Test Substitute-sensitive activation and exact ordering against pinned Showdown.

**Potential impact:** Up to **28 partial item identities** may be closable, *subject to a per-item blocker audit*. This is an opportunity estimate, not a promised implemented count.

**Exit:** Every promoted berry has all reachable trigger paths and interactions tested; remaining berries retain explicit partial status with precise blockers.

### PR 15.2 — Mid-turn forced switching

Build one switch-request and switch-resolution lifecycle that preserves action-queue semantics, entry/exit order, hazard/ability activation, replacement choice, fainting, and redirection/target validity. Cover Eject Button and Red Card first; use the same foundation for pivot moves, Emergency Exit, phazing, and other forced switches.

Conceptual operations:

```rust
request_switch(...)
force_switch(...)
replace_fainted(...)
```

**Exit:** Eject Button and Red Card work end-to-end; future moves and abilities use this shared machinery rather than their own queue mutation.

### PR 15.3 — Shared volatile lifecycles and Mental Herb

Consolidate volatile representation, activation, duration, expiry, switch-out cleanup, prevention/cure, and turn-boundary timing. Prioritize **flinch, confusion, Taunt, Encore, Disable, and other Mental Herb-cured effects** that are reachable. Extend the same state model to Substitute, Protect-family effects, trapping, recharge, redirection, and temporary invulnerability as the dependency graph requires.

Conceptual kernel operations:

```rust
add_volatile(...)
remove_volatile(...)
update_volatile(...)
```

**Exit:** Flinch and Mental Herb-related conditions are verified using canonical lifecycle machinery, not ad hoc flags.

### PR 15.4 — Mega Evolution as one vertical slice

Treat the existing Mega Stone catalog as **metadata foundation**. Implement one reusable, end-to-end Mega transformation workflow with explicit sub-capabilities:

- Eligibility: species/stone compatibility, once-per-battle rules, request validation, and prohibitions.
- Action timing: Mega activation relative to move selection/order and other effects.
- Transformation: species/form, typing, stats, HP rules, and state persistence through kernel-mediated operations.
- Ability transition: deactivate/replace/activate abilities with reference-correct ordering.
- Item semantics: stones remain attached and follow applicable restriction rules.
- Replay and comparison: deterministic fixtures for multiple Megas, failed requests, and ability interactions.

Keep stone-to-form mappings declarative; **do not create 81 independent item tasks**. The exact stone count should come from the generated manifest, not be assumed by the implementation.

**Exit:** Legal Mega requests execute completely for all reachable eligible combinations, with tests for timing and transformed state. Other forms remain PR 18 scope.

### PR 15.5 — Complete generic item and ability lifecycles

Unify item removal, theft, exchange, restoration, suppression, and consumed-item tracking; and ability copying, replacement, suppression, ignoring, entry activation, and exit cleanup. This should unlock Knock Off, Trick/Switcheroo, Pickpocket, Magician, Sticky Hold, Unburden, Trace, Mummy, Wandering Spirit, and similar mechanics without direct state writes in handlers.

**Exit:** Shared mutation/eligibility/timing infrastructure is exercised by representative move-item-ability interactions.

### PR 15.6 — Canonical move lifecycle and history

Converge action execution on a canonical sequence for priority, before-move checks, target resolution, accuracy, multi-hit processing, damage, on-hit effects, secondaries, and after-move effects. The exact ordering must be derived from pinned reference behavior. Centralize move-history fields such as last move, last successful move, last hit/damage source, action flags, consecutive-use count, and turn resets.

**Exit:** Moves do not bypass the shared lifecycle and history predicates; mechanical state modifications go through the kernel, while phase/turn/private-queue orchestration remains transition-owned.

### PR 15.7 — Acceptance-gate hardening and freeze

Reconcile the capability report after the Mega vertical slice, document the
transition-owned orchestration exception, add adversarial lifecycle and
interrupted-queue coverage, guard canonical move/switch/kernel pathways, and
label pinned-reference assertions separately from PR 19 full-state
differential comparison.

**Exit:** PR 15's claims match its executable evidence and ownership model.
Freeze the milestone after this pass; additional mechanic families belong to
PR 16 and later milestones.

### PR 15 acceptance gate

A PR must state: **(a)** capability/sub-capability completed, **(b)** identities genuinely unlocked, **(c)** remaining blockers, and **(d)** unit, interaction, and differential evidence. Re-run the dependency report after each merge. A partial identity becomes implemented only when **every reachable behavior and relevant interaction** is covered; do not treat a metadata registration or one supported callback as completion.

---

## PR 16 — Prepare the architecture and close the remaining move families

**Starting snapshot:** 269 unsupported of 510 reachable Regulation M-C moves after PR 15.7. Always regenerate the reachable-mechanics and capability reports before reporting progress; 269 is not a permanent task count. **Do not implement 269 independent callbacks.** Group moves by missing *capability* and its prerequisite dependencies, with family counts generated from the actual manifest.

### PR 16.0 — Behavior-preserving move architecture preparation

Before large move batches, perform **one bounded extraction** from the approximately 6,190-line `combat.rs` into cohesive modules. Separate canonical move execution and damage from switching, transformation, and item lifecycle orchestration where doing so materially improves navigation. Suggested layout (illustrative; do not manufacture empty files or unnecessary abstraction):

```text
rotom-core/src/combat/
  mod.rs              # Single canonical orchestration surface
  move_execution.rs   # BeforeMove -> PP/history -> target/hit -> AfterMove
  damage.rs           # Shared damage/hit execution
  switching.rs        # Interruption/resumption and entry/exit ordering
  transformation.rs   # Mega and reusable form-change orchestration
  item_lifecycle.rs   # Shared activation/removal/transfer
```

Maintain `StateKernel` as the only owner of **mechanical** state mutation. Keep the PR 15.7 exception explicit: canonical transition/resolution code may manage turn, phase, private queue, pending switch requests, and terminal settlement on the cloned working state. Move/ability/item/condition handlers may **not** use that exception to write state directly. Preserve the existing external API and internal event contract, plus serialized authoritative state, deterministic RNG draw ordering, diagnostics, and existing fixtures.

Do **not** implement new mechanics during PR 16.0. Extract incrementally, keep `execute_move` as the one queued-move entry, and move the current source-level architecture guards with the code without weakening their intent. Add missing behavior-level invariants (state-validity at stable boundaries, error atomicity, deterministic replay/serde continuation, interrupted queue resumption). Source-text guards are regression aids, **not** semantic proof.

**Exit:** No intended behavior or generated coverage-status changes, unchanged reference fixtures and deterministic traces, a clearly documented ownership boundary, no parallel execution/switch pipeline, passing `cargo test --workspace`, and passing existing manifest/capability-report checks.

### PR 16.1 — Declarative common move effects

Represent ordinary behavior as structured data (damage, boosts, major status, secondary chance, recoil, drain, and field effect) where possible. Flamethrower, Thunderbolt, Ice Beam, Close Combat, Icy Wind, Snarl, and Rock Slide should not each need bespoke execution logic. Implement common categories in batches that use the existing move lifecycle, mechanical rule checks, and kernel operations. Recalculate the per-family backlog after every batch.

### PR 16.2 — Shared effect families

Implement reusable behaviors in coherent, dependency-ordered batches: recoil/drain, multi-hit and variable power, priority checks, Protect, redirection, pivot/phazing/forced switch, two-turn mechanics, self-KO, field/side conditions, item/ability changes, called moves, and transformations as reachable. Extend canonical shared lifecycles rather than implementing a competing route to damage, switching, targeting, or state mutation.

When a new capability unlocks old partial mechanics, explicitly revisit its related move × item × ability interactions. For example, Substitute damage routing should trigger regression checks for resistance Berries, secondary effects, drain, and on-hit item/ability effects. Unresolved interactions remain blockers, not silent approximations.

### PR 16.3 — Irreducible exceptions

Keep a small, explicit unique-move layer only where behavior cannot be described with existing structured effects or reusable mechanics. Exceptions may orchestrate checks and kernel calls, but **must not** write mechanical state directly or create a separate action execution pipeline. Add regression evidence whenever an exception alters ordering/history/targeting.

### Mandatory PR 16 test and coverage policy (applies to every move-family batch)

Every move-family PR must include:

1. **Unit/mechanics tests:** behavior and calculation details, including boundaries, rounding, failure paths, PP, and move-history semantics.
2. **Interaction/transition tests:** ordering with representative abilities/items/volatiles, multi-target behavior where applicable, switch/faint/queue interruption, and relevant negative paths. Include deterministic replay, serialization-resumption, and error-atomicity tests when a batch changes lifecycle boundaries.
3. **Pinned-reference behavioral evidence:** a reproducible Showdown scenario at the pinned commit/format and assertions of the relevant normalized observable facts. Keep fixture generators and checked outputs consistent. Label these **reference-derived assertions**, not `full_state_differential_comparison`. The independent, comprehensive state oracle belongs to PR 19.

A move may become `implemented` **only when every required capability for all of its reachable behavior is executable**. Treat missing required sub-capabilities, unmodeled timing or interaction, and unsupported callbacks as explicit blockers. Do not mark a move implemented merely because it has an entry in the dex, a handler, or a passing happy-path fixture. A verified claim additionally needs qualifying pinned-reference evidence, and no claim of full parity is allowed before the corresponding PR 19 tests.

Every batch must regenerate the dependency graph and summarize: capabilities added/changed, move identities promoted, identities still blocked and why, unit/interaction/reference tests, new regression scenarios, and any limitations. Preserve the existing common pipelines; reviewers should reject parallel move dispatch, unchecked direct state writes, non-deterministic behavior, or opaque coverage promotion.

**PR 16 exit:** All reachable moves (currently 510) have executable implementations, no unsupported move identities or unresolved required capability blockers remain, all family testing gates are satisfied, and remaining broader interaction-parity risk is explicitly deferred to PR 19. Fail clearly on out-of-scope/unsupported behavior rather than silently approximating it.
---

## PR 17 — Resolve partial abilities and items

After PR 15 and PR 16, revisit the **current generated** partial ability/item identities (92 abilities and 25 items in the PR 15.7 snapshot; regenerate before planning PR 17). For every entry, capture its reason for incompleteness and dependencies in machine-readable coverage metadata:

```json
{
  "choicescarf": ["choice_lock"],
  "persimberry": ["confusion"],
  "cudchew": ["consumed_item_history", "delayed_effect"],
  "emergencyexit": ["forced_switch"],
  "stancechange": ["form_change"],
  "shadowtag": ["switch_restriction"]
}
```

Prioritize blockers by the number and importance of mechanics they unlock. Reuse new substrate operations instead of rewriting previously valid callbacks.

**Exit targets:** All reachable abilities and items executable (215 abilities and 166 items in the PR 15.7 manifest), with no unresolved required capabilities or partial entries attributable to unmodeled lifecycle/timing rules. Use the PR 16 unit → interaction → pinned-reference test policy for every promoted family, and retain explicit limitations until verified.

---

## PR 18 — Complete remaining forms and Champions mechanics

Build on PR 15.4's verified Mega transformation lifecycle. Extend shared form/species transformation mechanisms, with form-specific rules, to Stance Change, Hunger Switch, Zero to Hero, Forecast, Mimicry, Disguise, Imposter/Transform, and all other reachable Champions-specific transformations.

Avoid separate state mutation engines for each form. Each transformation must define eligibility, timing, stat/type/ability changes, HP and volatile persistence, and switch/exit restoration where applicable.

**Exit:** Every reachable Regulation M-C form transformation runs end-to-end, and Mega Evolution remains covered by its PR 15 vertical-slice regression suite.

---

## PR 19 — Independent full-state differential correctness and reliability

Implementation coverage and PR 16's reference-derived behavioral assertions are **not proof of full-state correctness**. PR 19 builds an independent normalized full-state differential harness comparing Rotom Spark and pinned Showdown with equivalent initial state, decisions, and controlled randomness. Compare normalized HP, status, boosts, items, abilities, fields, side/volatile conditions, active Pokémon, PP, fainting, turns, and winner.

### Three testing layers (build on, do not replace, PR 16 tests)

1. **Deterministic fixtures:** Hundreds/thousands of focused interactions (e.g., Fake Out + Inner Focus, Earthquake + Levitate, Knock Off + Sticky Hold, Sitrus + Ripen, Intimidate + Clear Body, Mega + ability start).
2. **Generated interactions:** Prioritize move×ability, move×item, ability×ability, weather×item, and terrain×ability combinations that share event hooks; avoid an impractical full Cartesian product.
3. **Full-battle corpus:** Aim for 10,000+ randomized/replayed complete battles with state checks at decision/action boundaries, automatic mismatch capture, minimization, and permanent regression fixtures.

Different RNG implementations may require controlled/random-decision injection or event-level parity instead of assuming identical seeds produce identical random draws across engines. Record the complete initial-state/action/RNG setup, normalize deliberate format differences, report each mismatch with minimized replay artifacts, and turn every confirmed bug into a permanent regression test. Do not relabel pinned-reference spot assertions as full differential tests.

**Exit:** A documented differential pass threshold, zero unresolved high-severity mismatches in the supported scope, and reproducible replay artifacts for every discovered discrepancy.

---

## PR 20 — RL-ready interface and optimization

Expose a minimal deterministic API:

```rust
legal_actions(state)
step(state, joint_action, rng)
terminal(state)
step_batch(states, actions)
```

Have Rust progress battles from one decision boundary to the next, rather than routing mechanics through Python. The intended flow:

```text
Batch observations -> Python / neural network
                   -> Batch joint actions
                   -> Rust simulates many battles
                   -> Batch observations
```

Profile before optimizing: battle cloning, ID representation, heap allocations, cache locality, serialization, observation encoding, and stepping throughput. Preserve determinism and differential verification throughout.

**Exit:** Stable Python-facing interface, validated batching semantics, performance benchmarks, and reproducible high-throughput simulation appropriate for Rotom Brawl.

---

## Architecture guardrails

The central dependency direction should remain:

```text
Moves / Abilities / Items (declarative effect definitions)
                     |
                     v
              Mechanics (rules)
                     |
                     v
              Kernel (mutation)
                     |
                     v
                 BattleState
```

Do not let effect handlers mutate `BattleState` directly. Mechanical changes go through `StateKernel`; only canonical transition/resolution helpers own phase, turn, private queue, pending-switch, and terminal orchestration on a cloned working state. Maintain a single canonical move pipeline and shared switch, item, ability, and form lifecycles. Large files such as `combat.rs`, `abilities.rs`, and `items.rs` warrant **conceptual module boundaries**, not a proliferation of arbitrary microfiles.

A possible eventual organization:

```text
rotom-core/
  kernel/
    hp.rs
    status.rs
    stats.rs
    field.rs
    pokemon.rs
  mechanics/
    damage.rs
    accuracy.rs
    switching.rs
    targeting.rs
    items.rs
    abilities.rs
    transformation.rs
    conditions.rs
  effects/
    moves.rs
    abilities.rs
    items.rs
  battle/
    queue.rs
    turn.rs
    lifecycle.rs
```

Treat this tree as **illustrative**; PR 16.0's smaller combat-focused extraction is the immediate target. Refactor only when it preserves behavior and reduces complexity. Keep source-level architectural checks narrow and pair them with behavioral invariants rather than building new AST-checker infrastructure.

## Immediate next action — PR 16.0 and ongoing acceptance gates

PR 15.0–15.7 is frozen. **Next implementation PR:** `PR 16.0 — Behavior-preserving move architecture preparation`. Keep its diff bounded; do not couple the extraction with move implementation. After PR 16.0, prioritize the capability dependency graph and implement moves in shared families.

**Standard PR completion note:** Include (a) capability and sub-capability deltas, (b) genuinely unlocked identities, (c) still-blocked identities and prerequisites, (d) unit, interaction, and pinned-reference assertions, (e) any new replay/error-atomicity evidence, and (f) deterministic and generated-report checks run. Label full-state differential parity only after a corresponding independent PR 19 comparison exists.

**Working rules until completion:** Do not reopen PR 15 for cosmetic cleanup; do not build a standalone architecture-lint framework; do not postpone all correctness checks to PR 19; do not optimize batching before correctness and determinism are established. Each new mechanic should make the existing engine more general rather than growing `combat.rs` into a collection of special cases.

### Completion criteria for Rotom Spark 1.0

- Every reachable Regulation M-C move, ability, item, and form has an accurate executable implementation and explicit evidence.
- No silent approximation for unsupported mechanics; unresolved behavior fails clearly.
- Canonical deterministic stepping and replay work from battle start to terminal result.
- Mechanical state mutations respect kernel boundaries; transition-owned phase/turn/private queue orchestration stays within its documented exception.
- Representative full battles agree with the pinned reference under normalized comparison.
- Rust supports efficient batched stepping and Python-facing observations/actions.

**Guiding principle:** Measure progress in **missing capabilities eliminated and verified interactions completed**, not simply the number of identities registered.
