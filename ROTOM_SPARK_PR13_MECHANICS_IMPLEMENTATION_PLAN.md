# Rotom Spark — PR 13+ Unified Mechanics Implementation Plan

**Status:** Proposed replacement for `ROTOM_SPARK_PR13_MECHANICS_IMPLEMENTATION_PLAN.md`  
**Target:** Pokémon Champions Regulation M-C, as represented by the pinned Pokémon Showdown Champions format  
**Prerequisites:** Milestone 11.5 state-transition refactor, PR 12 field-effects primitives, PR 13.0 mechanics inventory  
**Architectural references:** `AGENTS.md`, `ROTOM_SPARK_STATE_TRANSITION_REFACTOR_PLAN.md`, `docs/architecture/state-transition.md`, `SHOWDOWN_VERSION`  
**Principal change:** PR 13.1 establishes the **final unified move execution architecture** using single-target damage as its first vertical slice. All later mechanics extend it rather than replace it.

## 1. Mission

Implement **all mechanics reachable in legal Champions Regulation M-C battles**, including move-, ability-, item-, condition-, and form-driven interactions, through a deterministic event-driven state-transition engine. Do not implement the entire broad Showdown Dex merely because the exporter contains it.

The core transition remains:

```text
(B_next, diagnostics) = step(B_current, JointAction, Mechanics)
```

`BattleState` embeds the only authoritative `BattleRng`. `Mechanics` contains immutable data and effect registrations. `step()` is the sole canonical public advancement boundary: phase-aware, deterministic, validated, and error-atomic. Diagnostics cannot influence outcomes.

### The governing separation

> **Mechanics decide whether, when, and why a change happens. The kernel alone performs authoritative state mutations and enforces representation invariants.**

Moves, abilities, held items, statuses, field effects and volatiles are **sources of effect behavior**, not competing state-mutation architectures. Events identify opportunities to execute ordered handlers. Not every handler mutates state, and not every operation needs an emitted event.

## 2. Final target architecture — non-negotiable

```text
                            Immutable Mechanics
               Resolved Dex + Format + Effect Registry
                                      |
(BattleState, JointAction)             |
           |                          |
           v                          v
    +---------------------------------------------------+
    | Canonical step / Transition Orchestrator          |
    | phase validation, action queue, turn flow, commit |
    | and stable-state validation                        |
    +----------------------+----------------------------+
                           |
                           v
    +---------------------------------------------------+
    | Mechanics Execution                               |
    | Move Executor + shared mechanics services         |
    | target/hit resolution, formulas, rule eligibility  |
    | RNG requests, effect sequencing, faint/switch flow|
    +----------------------+----------------------------+
                           |
                +----------+-----------+
                | Event Dispatcher     |
                | ordered registrations|
                | move/ability/item/   |
                | status/field effects |
                +----------+-----------+
                           |
                   rule outcomes, modifiers,
                   typed mutation requests
                           |
                           v
    +---------------------------------------------------+
    | EffectState / controlled mechanics context        |
    | read-only inspection, RNG service, typed writes   |
    +----------------------+----------------------------+
                           |
                           v
    +---------------------------------------------------+
    | StateKernel                                       |
    | HP, status, PP, boosts, volatiles, items, fields,  |
    | positions/counters; representation invariants     |
    | NO rule decisions, events, formulas, or RNG       |
    +----------------------+----------------------------+
                           |
                           v
                   Working BattleState
                           |
                settle + validate + commit
                           v
                 Next BattleState + Diagnostics
```

The dispatcher is a service used **inside** mechanics, not an alternative path around mechanics. Handlers may update *ephemeral execution context* directly, but authoritative gameplay writes must go through controlled kernel-backed operations. Avoid duplicating kernel mutation facades: extend existing `EffectState` and `StateKernel` facilities.

### Responsibility contract

| Layer | Owns | Must never own |
| --- | --- | --- |
| `BattleState` | Mutable authoritative battle facts, phases, embedded RNG, serialization | Rules, callback registration |
| `StateKernel` | Typed invariant-preserving updates, reporting actual applied changes | Immunity/eligibility, damage formulas, events, RNG, move-specific logic |
| Mechanics services | Checks, effective stats, accuracy, power/damage, status eligibility, ordering, consequences | Uncontrolled direct state writes |
| Move executor | One common move lifecycle, target/hit/effect sequence, cancellation/repetition/continuation | Separate executors for physical/status/special moves |
| Effect definitions | Declarative behavior and narrowly scoped exceptional handlers for moves, abilities, items, etc. | Direct unrestricted state mutation or independent battle loops |
| Event registry/dispatcher | Handler collection and deterministically ordered invocation | Owning authoritative state or executing arbitrary commands |
| Transition orchestrator | Phase/action validation, queue, faint/replacement scheduling, atomic final result | Per-move rule branches or duplicate battle mechanics |

### Critical distinctions

1. **Definition vs event:** Flamethrower is a *move definition*. `TryHit`, `ModifyBasePower`, `AfterHit`, etc. are *events/hook points*. The definition describes operations and can register handlers for the appropriate hooks.
2. **Primary operation vs damage class:** Damage is an optional primary operation, **not** a fundamental class of move. Swords Dance, Protect, Flamethrower and Facade all enter the same move executor. Fixed damage and OHKO moves use alternative *damage operations*, not alternative full executors.
3. **Ephemeral modification vs persistent mutation:** Facade's conditional base-power modifier changes `MoveContext`/damage calculation. A stat boost changes `BattleState` through the kernel.
4. **Shared mechanics vs kernel:** `try_inflict_status(Burn)` checks immunities, prevention, active conditions, and hooks; only then does it call a kernel operation. `kernel.apply_status` only preserves storage invariants.
5. **Error atomicity vs individual writes:** A move can perform several kernel writes while resolving on a private working state. Unexpected errors must return **no** partly committed public state or RNG. Ordinary in-battle failures (misses, blocked effects) are modeled as successful outcomes, not engine errors.
6. **No invented universal event order:** Exact ordering and cancellations come from the pinned Champions Showdown behavior. The pipeline is a semantic framework; it is not a promise that every effect executes in seven unconditional loops.

## 3. Unified move pipeline contract

### Logical phases and deferred capability boundaries

| Stage | Meaning | PR 13.1 | Later work |
| --- | --- | --- | --- |
| Action/move eligibility | Phase, restrictions, PP, before-move blockers | Supported existing basics | Complex restrictions, ability/item/volatile rules |
| Target resolution | Determine and validate applicable targets | Basic selected single target | Spread, redirection, retargeting, random/ally/field targeting |
| Before-execution / prior operations | Setup, charging, power changes, context adjustments | Interfaces and compatible existing hooks only | Conditional power, boosts, charge, consumption |
| Hit attempts | Accuracy/evasion, immunities, blocking, per-target/per-hit checks | Basic single-target accuracy and applicable checks | Protect, multiaccuracy, per-hit and per-target variants |
| Primary operations | Damage, status, boost, healing, field conditions, switching | **Shared ordinary formula and kernel HP application only** | Non-damage, fixed damage, special damage operations |
| Secondary / reactive effects | Chance-based and on-hit effects; reactions from other effect sources | Defined hook/ordering seams, no false execution | Status chances, stat drops, on-hit/contact reactions |
| Completion / continuation | Drain, recoil, self-switch, queue changes, cleanup | Existing relevant cleanup/faints | Complex after-effects, multi-turn and pivot control |

**These stages are logical ownership boundaries, not a rigid unconditional sequence.** Mechanics must support guarded branching, skipped steps, repeated hits, interrupted actions, and well-defined callback/hook points *within* damage calculation and HP application. Some effects (e.g., abilities/items) affect a calculation before damage is committed; others react to actual applied damage. All ordering is explicit and tested.

### Proposed *conceptual* interfaces (not mandatory exact Rust shapes)

```rust
// Illustrative types only. Reuse and evolve existing rotom-core interfaces.
struct MoveDefinition {
    targeting: TargetRule,
    hit_sequence: HitSequenceRule,
    operations: Vec<MoveOperation>,
    required_capabilities: CapabilitySet,
}

enum MoveOperation {
    StandardDamage,
    FixedDamage(FixedDamageRule),
    ModifyPower(PowerRule),
    ApplyMajorStatus(StatusRule),
    ModifyStatStage(StatStageRule),
    Heal(HealingRule),
    ApplyCondition(ConditionRule),
    // Additional typed operations as the manifest requires.
}

struct MoveContext {
    // Ephemeral source/target/action context, effective power/modifiers,
    // hit and damage outcomes, cancellation/continuation state.
}

enum ExecutionFlow {
    Continue,
    SkipOperation,
    StopMove,
    SuspendForFutureTurn,
    RepeatHit,
    RequestDecision,
}
```

Design review must decide concrete storage and representation **based on existing Rust data structures and cost of cloning**. A typed, compact descriptor and shared operation helpers are preferred over allocations or dynamic dispatch in every damage event. Do not add a general bytecode interpreter, scripting runtime, redundant Dex, or generic mutation command bus. Most moves should be declarative/shared; truly exceptional behaviors get narrow handler implementations with explicit hook ownership.

`required_capabilities` is a *coverage guard*, not a move taxonomy: avoid treating `OrdinaryDamage`, `StatusMove`, `SpecialDamage`, etc. as competing executor types. Capability reports identify exactly which mechanic is unsupported. Before accepting an action as executable, verify **all** reachable operations, relevant hooks and required behaviors, including behavior from current items, abilities and conditions as appropriate to supported scope. Missing behavior must not silently become a no-op or plain damage.

### Worked examples to verify composition

| Move/effect | Prior/context operations | Primary operation | Later/reaction operations |
| --- | --- | --- | --- |
| Dragon Pulse | Basic modifiers | Standard damage | Normal hit consequences |
| Facade | Conditional **base-power** modifier for qualifying user status | Standard damage | Normal hit consequences |
| Flamethrower | Standard modifiers | Standard damage | Chance to burn, eligibility checked by mechanics, status applied through kernel |
| Shadow Ball | Standard modifiers | Standard damage | Chance to lower Sp. Def through mechanics/kernel |
| Swords Dance | None | Self Attack boost through mechanics/kernel | None |
| Surf | Multi-target selection/spread context | Per-target standard damage | Per-target reactions |
| Bullet Seed | Determine hit count | Repeated per-hit standard damage | Per-hit hooks, faint handling, RNG ordering |
| Giga Drain | Standard modifiers | Standard damage | Heal from **actual applicable damage**, via mechanics/kernel |
| Electro Shot | Charge/Sp. Atk boost; rain/other skip-charge rules | Standard damage on attack turn | Move continuation/cancellation handling |
| Protect | Priority/eligibility/repeat-use checks | Apply guarding volatile/condition via kernel | Block appropriate incoming attempts |

These examples are design fixtures, **not claims of implementation or complete pinned rule details**. Differential research determines precise execution order, immunities, conditions and RNG draws. Do not model spread modifiers solely from targets whose HP decreased; do not implement multi-hit as `damage * n` or reroll only a final modifier.

### Mechanic service pattern

```text
Move operation: AttemptBurn(target)
   -> Mechanics: check trigger + target status eligibility + ordered prevention hooks
   -> Mechanics: request battle RNG roll at the reference-defined point
   -> (if allowed and chance succeeds) EffectState -> StateKernel.apply_status
   -> Mechanics: dispatch successful status/reaction events as required
```

The kernel **never** decides whether a target is immune to burn; it does reject structurally invalid status representations. `ModifyBasePower`, `ModifyDamage`, `TryHit`, `AfterDamage`, etc. should operate through existing or narrowly extended typed event contexts. Use actual applied HP delta when downstream behavior depends on it. Events must not accidentally recurse without controlled ordering and a termination guard.

## 4. Scope and coverage: preserve PR 13.0

PR 13.0's generated inventory and coverage reporting remain the source of *what must eventually work*. Keep `data/generated/champions-reg-mc.json` as the resolved broad export and `data/generated/reg-mc-mechanics-manifest.json` (or existing equivalent) as the distinct reachability/coverage product. Do not rebuild 13.0 because 13.1 changes execution architecture.

Scope is derived from legal starting species/forms, legal move learnsets, ability slots, usable items, format/team constraints, battle-only forms and transformations, and **reachability closure** for effects invoking/generated moves or other mechanics. Ambiguous reachability stays `needs_research`; no automatic claim of legality or support from a broad Dex record.

Maintain independent `implementation` (`unsupported`/`partial`/`implemented`) and `verification` (`not_tested`/`unit_tested`/`differential_verified`) statuses. **Also track required operation/hook capabilities** and interaction coverage for each reachable identity. Coverage is granted only when the move and all of its behavior—not simply the damage operation—are supported. `ordinary_damage` may remain temporarily as an informational classification for migration, but must not become an intrinsic runtime category or the final support criterion.

**Acceptance:** deterministic byte-identical regeneration; pinned provenance; accurate reachability; no unsupported entries falsely marked implemented; report can answer "which required operation/hook is missing?" for each reachable move/ability/item.

## 5. Sequencing and dependency plan

```text
PR 12 existing field/effect lifecycle + Milestone 11.5 state kernel/step
                          |
                 PR 13.0 inventory (retain)
                          |
          PR 13.1 FINAL UNIFIED MOVE EXECUTOR FOUNDATION
            single-target standard damage vertical slice
                          |
       +------------------+------------------+
       |                  |                  |
  PR 13.2 targeting   PR 13.3 primary    PR 13.4 conditional,
  and protection      non-damage ops     secondary, hit-sequence
       |                  |                  |
       +------------------+------------------+
                          |
                 PR 13.5 abilities
                          |
                 PR 13.6 items
                          |
          PR 13.7 complex move/queue flow
                          |
         PR 13.8 remaining reachable effects
                          |
       Mega/Champions-special gate if necessary
                          |
           PR 13.9 full interaction/completeness
```

Tasks inside 13.2–13.4 may overlap **only after** the 13.1 shared interfaces are reviewed and agents own disjoint code. The graph expresses logical dependencies, not mandatory serialization of independent fixture research. Abilities/items can require new hooks introduced by earlier phases; revise event interfaces narrowly without introducing a new executor.

## 6. PR-by-PR implementation specifications

### PR 13.0 — Scope inventory and coverage registry (retain)

**Goal:** Finite legal Regulation M-C target set with pinned-Showdown provenance.

**Work:** Preserve exporter, legality/reachability closure, deterministic manifest, evidence, owners, human-readable coverage, fixture checks. Adapt schema/report for shared operations and hooks without destroying existing reachability work.

**Done:** Every directly legal and conservatively included battle-reachable entry is represented; unresolved research and coverage are explicit.

### PR 13.1 — Unified move executor foundation: single-target damage

**Goal:** Establish **the permanent move execution architecture**, using one-target, one-hit standard damage as its first verified vertical slice. Do not create a separate `OrdinaryDamage` runtime class or a seven-move-specific pipeline.

**Implementation:**

1. Inventory existing `combat.rs`, `event.rs`, `EffectState`, `StateKernel`, Dex/combat mapping, queue and `step()`; document which functions will be **reused**, **adapted** and **retired**. Preserve the Milestone 11.5 contract.
2. Introduce/reshape a single `MoveExecutor` (name flexible) invoked by the existing transition/queue machinery for all supported move actions, regardless of category. Define minimal typed move descriptor/operation data and ephemeral execution context, with well-owned seams for targeting, before-effects, hit resolution, primary effects, later effects and completion/control flow.
3. For **this PR only**, execute basic player-selected single-target **one-hit** Physical/Special operations whose entire intrinsic behavior and currently applicable supporting hooks are implemented. Reuse the existing standard damage formula, status/stat-stage effects already in the engine, type chart, applicable weather/field hooks, accuracy/PP, faint processing and kernel-backed HP changes. Preserve correct rounding and RNG ordering.
4. No manual per-move allowlist as the final behavior definition. Generate or derive support from complete implemented *capabilities/requirements*, including base-power callbacks, intrinsic effects, flags that require active cross-effects, and applicable target behavior. Reuse existing move metadata instead of inventing a parallel Dex or copying long move definitions by hand.
5. Establish **narrow, tested extension points** for conditional power, non-damage primary operations, secondaries, repeated hit sequences and control flow. Defer their runtime behavior; don't build broad placeholder systems or empty no-op handlers that falsely mark support.
6. Move temporary calculation values into execution context; persistent state updates go through `EffectState`/kernel. The engine's rule checks, RNG and event dispatch remain outside the kernel.
7. Retire/migrate `MoveSupport::BasicDamage` and `ordinary_damage` as architectural identity/execution switches. If a temporary compatibility field is required, document its deprecation and ensure it cannot be used to claim entire-move support incorrectly.
8. Preflight missing required mechanics **before irreversible internal resolution**, returning a clear unsupported-mechanic error without exposing partial public state or RNG. Normal misses/immunities are successful move outcomes, not unsupported errors.
9. Keep one canonical `step()` and deterministic action queue. Do not change public phase semantics, implement Mega, or add a second run loop.

**Representative tests:** Dragon Pulse/Dragon Claw or equivalent eligible standard attacks; physical/special, various types, STAB, resist/immunity, accuracy/evasion, misses, critical rolls, field modifiers, priority, PP exhaustion, fainting, skipped actions, invalid targets; equality of replay/trace modes. Explicitly **reject** Flamethrower (burn secondary), Shadow Ball (stat-drop secondary), Facade (conditional power), Surf (spread), Bullet Seed (multi-hit), Giga Drain (drain), and Electro Shot (charge), pending capabilities. These are acceptance examples subject to resolved Champions data and format reachability; if an example isn't reachable, use a comparable reachable fixture plus a unit-level representation test.

**Acceptance gates:**

- One executor entry path for all supported move resolution; status/non-damage operations can be represented or correctly rejected but do not need to execute yet.
- Complete **supported** single-target damage semantics match pinned differential fixtures; no missing side effects are silently discarded.
- Existing Milestone 11.5/PR 12 tests remain green; damage algorithm is not rewritten gratuitously.
- Instrumented tests prove kernel-only authoritative writes and mechanics-owned checks/RNG.
- Next PR can add a status/boost operation **inside the same executor** without replacing damage architecture.

### PR 13.2 — Target resolution, protection and redirection

**Goal:** Upgrade the same move pipeline from single-target to doubles-aware target sets.

**Implement:** target-set and per-target execution context; spread targeting and modifier rules; valid ally/self/field targets if reachable; Protect and related guards, eligibility and repeated-use odds; Follow Me/Rage Powder and other reachable redirection; retargeting and missing-target cases; target-/hit-level immunity ordering. Distinguish selected targets, attempted targets, actual hits and applied HP loss.

**Tests:** mixed protected/unprotected targets, two/one target modifier cases, switching/fainting before execution, redirection conflicts, independent accuracy, immunities and RNG draws; source-backed differential corpus.

**Done:** Expansion is through target resolution and hit hooks, not a `SpreadMoveExecutor` or separate spread damage formula.

### PR 13.3 — Reusable primary non-damage operations

**Goal:** Use the unified executor for moves that do **not** deal ordinary damage.

**Implement:** status eligibility/infliction/cure, stat boosts/drops, healing, volatiles, weather, terrain and side conditions (reuse PR 12 lifecycles), move failure conditions, target distinctions. Operations call mechanics services, which invoke kernel writes only after rules permit. Reuse effects for primary and later secondary applications.

**Tests:** Swords Dance and reachable comparable moves, status immunity/ability prevention, already-statused targets, stat bounds, heal caps, field duration, failed effects, primary vs secondary timing; no standard-damage operation required.

**Done:** Both damaging and non-damaging moves work through **one** executor and event system, with zero category-specific turn loops.

### PR 13.4 — Prior effects, secondary effects and hit-sequence operations

**Goal:** Extend the same pipeline with flexible sequencing and shared specialized operations.

**Split by cohesive behavior if large:**

- **13.4a:** `ModifyBasePower`/conditional power and effective offensive/defensive stat overrides where reachable; ephemeral context versus kernel writes; Facade-like examples.
- **13.4b:** Chance-based secondaries, per-target effects, status and stat-drop reactions; Flamethrower/Shadow Ball-like examples. Trigger only where reference rules allow; do not always roll or always apply.
- **13.4c:** Multi-hit sequence control, fixed/level damage, multiaccuracy, crit/damage/ability/item checks and faint handling per hit as appropriate.
- **13.4d:** Draining, recoil, self-damage, self-KO, damage-dependent healing, and other completion operations. Base dependent effects on **actual** applied results, with reference rounding.
- **13.4e (if interfaces allow; otherwise PR 13.7):** Charging/recharge and conditional skip-charge, including Electro Shot-like prior stat changes; persisted multi-turn continuation state only when required.

**Tests:** RNG draw count and order, hook order, criticals and random damage per hit, HP deltas, partial interruptions, status/ability prevention, dependency on applied damage, KO between hits, sequencing against existing field mechanics.

**Done:** All effects are composable operations/hooks in the shared executor; no move-identity giant match and no formula duplication.

### PR 13.5 — Ability handlers using the same mechanics/event fabric

**Goal:** Implement legal reachable abilities as registrations responding to events and shared mechanic operations; they must not require a new mutation path.

**Suggested batches:** switch-in/out → stat/priority/speed modifiers → attack/defense/damage modifiers → hit immunity/absorption → status prevention → terrain/weather → contact/on-hit reactions → suppression/copy/override exceptions.

**Work:** stable handler identity and owner, activation/suppression checks, explicit event ordering, narrow typed context, accurate cross-source interactions. Prefer shared `try_inflict_status`, `modify_effective_stat`, `resolve_damage`, etc., rather than reimplementing rules per ability. Flash Fire-type checks must occur at the appropriate hit/immunity hook, not as a post-damage undo.

**Tests:** every verified ability in isolation and with representative moves, items, field conditions and other abilities; ordering/suppression and deterministic replay.

### PR 13.6 — Item handlers using the same mechanics/event fabric

**Goal:** Implement reachable held items as event handlers and rule capabilities, not a second item executor.

**Suggested batches:** stat/damage modifiers → move locks/action legality → consumables, recovery and status cure → survival and defensive effects → reactive/contact effects → edge cases and suppression.

**Work:** use existing state/kernel operations for item consume/transfer/removal and status/HP changes; model authoritative item activation state only if needed; distinguish timing of item trigger from timing of actual kernel writes. Lum Berry-like status cures should react to the appropriate event without Flamethrower knowing about the item.

**Tests:** one-time consumption, actual applied HP/status changes, switch/faint behavior, ability/item stacking and priority, legal-action masks, errors/replay.

### PR 13.7 — Complex move control, switching and decision phases

**Goal:** Extend move execution control outcomes without breaking canonical `step()` or unstable-state encapsulation.

**Implement:** pivoting, forced switching, trapping, ongoing multi-turn locks/charge/recharge, move interruption/cancellation, scheduled effects, any needed mid-turn choices, reachable move-calling/copying effects. Keep decisions explicit and orderly; represent continuation with minimal authoritative state and kernel operations.

**Design gate:** If a move demands player input during resolution, document an explicit stable decision-phase extension and its replay semantics before coding. Never silently auto-pick a replacement, expose `ResolvingTurn`, or store a hidden pending action outside authoritative transition/state.

**Tests:** queue continuation after pivot/KO, chained switches, target invalidation, canceled charges, simultaneous decisions, serialized replay, failure atomicity.

### PR 13.8 — Remaining reachable behaviors (repeatable batches)

**Goal:** Clear uncovered manifest requirements without implementing unrelated broad-Dex mechanics.

Choose the next missing *operation/hook/interaction family* from the manifest, record exact reachable IDs and pinned source evidence, then implement using existing mechanics services and kernel-backed effects. Exceptional move-specific handlers are acceptable only when generic composition does not faithfully represent the rule; they still use the **same** executor and event system. Track any newly discovered cross-source interaction as a coverage dependency.

**Done:** No unaccounted directly legal or battle-reachable move, ability, item or condition behavior remains.

### Transformation / Champions-specific gate (before final acceptance)

If reachable Mega Evolution, forms, transformation, or Champions-specific behavior require new authoritative state or turn sequencing, deliver a separate reviewed PR *before* PR 13.9. Preserve canonical API/error atomicity and reuse effect/transition/kernel boundaries. Do not declare PR 13 complete with reachable transformations unresolved.

### PR 13.9 — Differential and architectural completeness gate

**Goal:** Demonstrate faithful composition rather than a superficial count of implemented move IDs.

**Implement/test:** systematic cross-source coverage for move+ability+item+field+status interactions; ordering among hooks, hit/damage application and triggered reactions; opponent action order, speed ties, priority, immunities, Protect, multi-hit, status, switching, residuals and transformations; randomized legal-state comparisons; complete battle corpora; replay/serialization, structural invariants and failed-transition atomicity.

**Done:** Every reachable behavior is implemented and verified or has a formally documented, accepted exclusion; complete valid Regulation M-C battles progress through `step()` without unaccounted unsupported mechanics.

## 7. Safety, correctness and scope rules for every PR

1. **Pinned reference only:** use resolved Champions mod, pinned Showdown revision and selected format. Record source files/functions and relevant format overrides; live Champions parity must not be asserted without separate evidence.
2. **Reachability first:** legal Regulation M-C starting content + battle-generated closure; don't expand to all Showdown content.
3. **Kernel purity:** no move/ability/item/condition rule checks, event dispatch or RNG in `StateKernel`; no downstream direct authoritative writes outside controlled setup/kernel.
4. **Single execution fabric:** no duplicate move execution loops, per-move full resolvers, or opaque strings interpreted into state mutations.
5. **Explicit unsupported:** no auto-fallback to plain damage or no-op for omitted intrinsic effects; distinguish ordinary gameplay misses/failures from engine unsupported errors.
6. **Deterministic RNG:** one embedded stream; preserve Showdown-relevant order, reason traces and draw counts. Trace toggles cannot affect outcomes.
7. **Precise hook order:** target selection, ability/item prevention, calculations, HP application, consequences and faint settlement at correct points; add regression for each confirmed discrepancy.
8. **Atomic public boundary:** error returns no state/diagnostics/RNG mutation; `step()` validates stable output before exposing it.
9. **Lean interfaces:** reuse `CombatData`, `EffectRegistry`, `EffectState`, `StateKernel`, and existing queue/transition contracts. Add structs/enums only when ownership and measured needs justify them.
10. **Tests are implementation work:** source-backed focused cases, adversarial failures, interactions, differential fixtures, full workspace regressions.

## 8. Review checkpoints and acceptance commands

Every behavioral PR supplies exact results (not assumed results) for:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
# Pinned Showdown exporters and reference fixture generation/checks as applicable
# Mechanics-manifest regeneration and byte-for-byte reproducibility checks
# Applicable normalized differential comparisons
```

Reviewers must additionally answer:

- Does this feature use the **existing unified move executor**, or introduce a second one?
- Can every state mutation be traced to a kernel operation (`EffectState` may forward it)?
- Are checks/formulas/hooks outside the kernel and bounded by typed interfaces?
- Are the move's full effect requirements either implemented or explicitly unsupported?
- Does event ordering match pinned reference, including no-op/miss/reaction cases?
- Does replay from identical state + joint action + mechanics produce identical state and RNG?
- Is the public input unchanged on errors, and does serialization preserve continuation?
- Do legal-action masks reflect new locks, targeting and restrictions?
- Is implementation and verification status supported by actual evidence?

## 9. Codex agent handoff template

Every agent must report:

```markdown
## Scope and provenance
- PR / reachable mechanics IDs / effect capabilities:
- Pinned Showdown revision, format and source locations:
- Explicitly deferred adjacent behaviors:

## Architecture impact
- Existing functions/types reused:
- Executor phase / event hooks / execution-control changes:
- Mechanics rule checks and calculations:
- Kernel-backed mutation methods used:
- Ephemeral context vs authoritative state fields:
- No parallel executor or direct mutation paths (evidence):

## Behavioral implementation
- Success, failure, immunity, interruption and reaction semantics:
- Callback/event ordering and RNG ordering:
- Interaction with existing abilities/items/field effects:

## Verification
- Focused test cases and expected values:
- Normalized Showdown differential fixtures:
- Negative/unsupported and transactional-failure tests:
- Commands actually executed and actual results:

## Coverage
- Manifest entries and operation/hook requirements changed:
- Implementation and verification statuses supported by evidence:
- Known omissions, blockers and proposed next PR:
```

## 10. Parallelism and migration of existing PR 13.1

**Migration is a correction of architecture, not a reimplementation of all earlier milestones.** Reuse pinned Dex data, PR 13.0 manifest, PR 12 field lifecycle, existing battle damage calculation, event registry, state mutation kernel, replay contract, and valuable PR 13.1 tests. Remove or deprecate only the `ordinary_damage`/`BasicDamage` *architectural gating* that assumes a permanent move family.

Do **not** automatically reset/revert Git history as part of this plan; maintainers choose whether to revert PR 13.1 or refactor atop it. Preserve a reviewable commit and test baseline either way. PR 13.1 is accepted only once it satisfies the **new** gates, not merely the original 37-move classification count.

Parallel agents: research/fixtures can run alongside disjoint implementation; no unsynchronized simultaneous edits of `combat.rs`, event/registry, transition/queue, or kernel modules. The integrator owns final architecture audit and complete workspace checks.

## 11. Non-goals

- Runtime calls to Showdown, a second Dex, all-generation move implementation or speculative legality expansion.
- Separate damage/status/special executors or a plugin scripting VM.
- Putting status immunity, damage calculations, targeting, RNG or Pokémon rules into the mutation kernel.
- Treating an unimplemented secondary or prior operation as optional merely to expand supported ID counts.
- Implementing the training/pilot model, teambuilder, networking, Python bindings, or UI here.
- Unmeasured optimization that sacrifices readability, deterministic behavior or differential correctness.

## 12. Definition of completion

PR 13+ is complete when the pinned Regulation M-C reachability inventory is reproducible; all reachable move/ability/item/field/form behaviors and required interactions are implemented or formally excluded; every effect executes via shared mechanics/event contracts and kernel-controlled mutation; substantial normalized pinned-Showdown differential and full-battle suites pass; valid battles advance from preview to termination via the canonical `step()` without unaccounted unsupported behavior; and RNG determinism, atomic errors, serialization, legal-action enumeration, and state invariants remain intact.

**Guiding principle:** The engine does not implement *move categories*. It executes a deterministic sequence of rule-checked effects. Mechanics decide what is allowed; the kernel changes state; the transition function produces the next stable battle state.
