# Rotom Spark — Battle Engine Completion Gameplan

**Repository:** [LucasManalo/rotom-spark](https://github.com/LucasManalo/rotom-spark)  
**Review baseline:** `master` at `9d9f45d` (October 30, 2026)  
**Target:** Complete, correct, deterministic Regulation M-C battle simulation suitable for Rotom Brawl training.

## Executive summary

Rotom Spark has a strong foundation: an explicit battle state, a mutation kernel, deterministic RNG, event/effect dispatch, generated Regulation M-C mechanics reachability, and broad initial move, ability, and item support. The remaining challenge is **closing engine capabilities**, rather than merely adding more mechanic identities.

At the review baseline, the generated mechanics-status report recorded:

| Kind | Reachable | Implemented | Partial | Unsupported |
| --- | ---: | ---: | ---: | ---: |
| Abilities | 215 | 120 | 95 | 0 |
| Items | 166 | 47 | 119 | 0 |
| Moves | 510 | 223 | 0 | 287 |

These are implementation labels, **not** complete verification or interaction-parity guarantees. The recent PR 14.0–14.4 commits expanded held-item behaviors and Mega Stone identities; Mega Stone identity mapping does not by itself complete Mega Evolution in live battles.

**Main strategy:** Complete shared mechanics capabilities before counting individual identities as complete. Start with a dependency/coverage audit, berry activation, mid-turn forced switching, volatile lifecycles, and a Mega Evolution vertical slice; then implement missing moves by family, close remaining ability/item gaps, finish other forms, verify whole battles against pinned Showdown, and optimize batched RL simulation.

**Coverage reporting rule:** PR 14.4 should be described as **“Mega Stone metadata foundation”**, not “71 items implemented.” All stones remain partial until their complete battle behavior is supported and verified. `0 unsupported identities` only means every reachable identity has at least some recognized handling; it does **not** mean the engine is complete. Report identity status **and** capability status separately.

## Roadmap

| Milestone | Objective | Definition of done |
| --- | --- | --- |
| **PR 15** | Capability-driven closure (berries, forced switches, volatiles, Mega foundation) | Shared lifecycles work end-to-end; capability blockers, unlocked identities, and verification are reported explicitly |
| **PR 16** | Finish reachable moves | All 510 reachable moves have executable implementations with appropriate tests |
| **PR 17** | Close abilities and items | All 215 reachable abilities and 166 reachable items are fully implemented, with known gaps resolved rather than relabeled |
| **PR 18** | Finish remaining forms and Champions-specific transformation behavior | Build on the PR 15 Mega vertical slice; all other reachable form paths work end-to-end |
| **PR 19** | Differential correctness campaign | Broad deterministic fixtures, interaction tests, and full-battle comparisons agree with pinned Showdown within documented format differences |
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

The headline report must show **identity coverage separately from executable capability coverage**, including outstanding partial/unsupported counts. Example dependencies: `choicescarf -> choice_lock`, `persimberry -> confusion`, `ejectbutton -> forced_switch`, and Mega Stones -> `mega_eligibility`, `mega_timing`, `form_change`, `ability_transition`.

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

**Exit:** Moves do not bypass the shared lifecycle and history predicates; all state modifications go through the kernel.

### PR 15 acceptance gate

A PR must state: **(a)** capability/sub-capability completed, **(b)** identities genuinely unlocked, **(c)** remaining blockers, and **(d)** unit, interaction, and differential evidence. Re-run the dependency report after each merge. A partial identity becomes implemented only when **every reachable behavior and relevant interaction** is covered; do not treat a metadata registration or one supported callback as completion.

---

## PR 16 — Implement the 287 unsupported moves

**Do not approach this as 287 unrelated callbacks.** Generate a report grouping unsupported moves by missing mechanic family. Illustrative families include direct damage with secondary effects, recoil/drain, fixed or variable damage, multi-hit, Protect, redirection, pivoting, forced switching, two-turn execution, item/ability changes, field/side conditions, and move-copy/call effects. Family counts must be **generated from the actual manifest**, not guessed.

### PR 16.1 — Declarative common move effects

Represent ordinary behavior as structured data (damage, boosts, major status, secondary chance, recoil, drain, field effect) where possible. Flamethrower, Thunderbolt, Ice Beam, Close Combat, Icy Wind, Snarl, and Rock Slide should not each need a bespoke execution engine.

### PR 16.2 — Shared effect families

Implement missing reusable behaviors in coherent, testable batches: recoil/drain, multi-hit, variable power, priority checks, Protect, redirection, pivot, phazing, two-turn mechanics, self-KO, transformations, and more.

### PR 16.3 — True exceptions

Unique moves may orchestrate mechanics but **must not mutate `BattleState` directly**. Keep a small, explicit unique-move layer only for irreducible behavior.

**Exit:** `510/510` reachable moves executable, `0` unsupported, with focused tests and recorded limitations surfaced as failures rather than silent approximations. Full cross-interaction parity remains a PR 19 gate.

---

## PR 17 — Resolve partial abilities and items

After PR 15 and PR 16, revisit the 95 partial abilities and 119 partial items. For every entry, capture its reason for incompleteness and dependencies in machine-readable coverage metadata:

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

**Exit targets:** 215/215 abilities implemented; 166/166 items implemented; no lingering partial entries attributable to unmodeled common lifecycle or timing rules. Each claimed implementation must have meaningful verification evidence.

---

## PR 18 — Complete remaining forms and Champions mechanics

Build on PR 15.4's verified Mega transformation lifecycle. Extend shared form/species transformation mechanisms, with form-specific rules, to Stance Change, Hunger Switch, Zero to Hero, Forecast, Mimicry, Disguise, Imposter/Transform, and all other reachable Champions-specific transformations.

Avoid separate state mutation engines for each form. Each transformation must define eligibility, timing, stat/type/ability changes, HP and volatile persistence, and switch/exit restoration where applicable.

**Exit:** Every reachable Regulation M-C form transformation runs end-to-end, and Mega Evolution remains covered by its PR 15 vertical-slice regression suite.

---

## PR 19 — Differential correctness and reliability

Implementation coverage is **not proof of correctness**. Compare Rotom Spark and pinned Showdown with equivalent initial state, decisions, and controlled randomness. Compare normalized HP, status, boosts, items, abilities, fields, side/volatile conditions, active Pokémon, PP, fainting, turns, and winner.

### Three testing layers

1. **Deterministic fixtures:** Hundreds/thousands of focused interactions (e.g., Fake Out + Inner Focus, Earthquake + Levitate, Knock Off + Sticky Hold, Sitrus + Ripen, Intimidate + Clear Body, Mega + ability start).
2. **Generated interactions:** Prioritize move×ability, move×item, ability×ability, weather×item, and terrain×ability combinations that share event hooks; avoid an impractical full Cartesian product.
3. **Full-battle corpus:** Aim for 10,000+ randomized/replayed complete battles with state checks at decision/action boundaries, automatic mismatch capture, minimization, and permanent regression fixtures.

Different RNG implementations may require controlled/random-decision injection or event-level parity instead of assuming identical seeds produce identical random draws across engines.

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

Do not let effect handlers mutate `BattleState` directly. Maintain a single canonical move pipeline and shared switch, item, ability, and form lifecycles. Large files such as `combat.rs`, `abilities.rs`, and `items.rs` warrant **conceptual module boundaries**, not a proliferation of arbitrary microfiles.

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

Treat this tree as **illustrative**; refactor only when it preserves behavior and reduces complexity.

## First action: PR 15.0 — Capability audit and reporting

Before another identity batch, generate an explicit dependency graph for **all partial abilities/items and unsupported moves**. Group blockers by engine capability, count dependent identities, rank by prerequisite leverage and competitive importance, and keep pinned Showdown evidence for each gap.

**Next implementation batches, in order:** berry activation framework (potentially closing 28 items), mid-turn forced switching (Eject Button and Red Card), shared flinch/Mental Herb volatile lifecycles, then the Mega Evolution vertical slice. Re-rank later work from the updated dependency graph instead of assuming fixed family sizes.

**Immediate next PR:** `PR 15.0 — Capability coverage report and mechanics blocker graph`.

**Standard PR completion note:** State capability completed, sub-capabilities covered, unlocked identities, remaining blockers, and unit/interaction/differential evidence. Describe PR 14.4 as **“Mega Stone metadata foundation”** in progress reporting.

### Completion criteria for Rotom Spark 1.0

- Every reachable Regulation M-C move, ability, item, and form has an accurate executable implementation and explicit evidence.
- No silent approximation for unsupported mechanics; unresolved behavior fails clearly.
- Canonical deterministic stepping and replay work from battle start to terminal result.
- All state mutations respect kernel boundaries.
- Representative full battles agree with the pinned reference under normalized comparison.
- Rust supports efficient batched stepping and Python-facing observations/actions.

**Guiding principle:** Measure progress in **missing capabilities eliminated and verified interactions completed**, not simply the number of identities registered.
