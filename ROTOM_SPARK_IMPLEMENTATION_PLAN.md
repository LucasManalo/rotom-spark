# Rotom Spark — Battle Engine Implementation Plan

## 1. Goal

Build a **standalone, deterministic, high-performance Rust battle simulator** for **Pokémon Champions Regulation M-C**.

For the first implementation phase, **Pokémon Showdown's Champions Regulation M-C implementation is the development reference**. The engine should reproduce Showdown's behavior closely enough to support automated differential testing, but the completed simulator must **not depend on Showdown at runtime**.

Pokémon Champions itself is **not** part of the initial validation loop. Direct comparison against the real game is future work.

The battle engine will eventually support:

- large-scale local simulation;
- deterministic self-play;
- battle-state cloning and search;
- arbitrary legal Regulation M-C teams;
- full Pokémon sets including species, form, ability, item, moves, nature, and stat allocation;
- legal-action enumeration;
- hidden-information observations for AI agents;
- future Python bindings for machine learning.

Phase 1 ends when two legal Regulation M-C teams can be simulated locally from team preview through battle completion with mechanics that match the pinned Showdown reference implementation.

---

# 2. Design Principles

The implementation should follow these rules throughout development.

1. **Correctness before performance.**
2. **Never silently approximate an unsupported mechanic.**
3. **Use a pinned Pokémon Showdown commit as the Phase 1 reference.**
4. **The final battle engine must run without Showdown.**
5. **Every battle must be deterministic from its inputs and seed.**
6. **Battle state must be cloneable, serializable, and inspectable.**
7. **The engine must own action legality. AI layers should never construct arbitrary invalid commands.**
8. **Keep game mechanics separate from machine-learning code.**
9. **Prefer typed Rust state over textual protocols or object graphs.**
10. **Design for eventual massive self-play and search.**

---

# 3. High-Level Architecture

```text
                Pokemon Showdown
                 pinned commit
                      |
              Reference Adapter
                      |
              Differential Tests
                      |
                      v
+--------------------------------------------------+
|                  ROTOM SPARK                     |
|                                                  |
|  Regulation / Dex Data                           |
|          |                                       |
|          v                                       |
|     Battle State                                 |
|          |                                       |
|          v                                       |
|     Action Resolver                              |
|          |                                       |
|          v                                       |
|       Event System                               |
|          |                                       |
|          v                                       |
|     Mechanics Effects                            |
|                                                  |
|      Deterministic Battle RNG                    |
+--------------------------------------------------+
                      |
                Rust Public API
                      |
                Future Python API
                      |
                      v
                Pilot / Team AI
```

Showdown should be used as:

- a mechanics specification;
- a source for resolved Champions game data;
- a differential-testing reference.

It should **not** be called during normal Rotom Spark simulation.

---

# 4. Suggested Repository Layout

Use a Rust workspace from the start.

```text
rotom-spark/
|
|-- Cargo.toml
|-- Cargo.lock
|-- README.md
|-- AGENTS.md
|-- SHOWDOWN_VERSION
|
|-- crates/
|   |-- rotom-core/
|   |   `-- src/
|   |       |-- lib.rs
|   |       |-- battle/
|   |       |-- state/
|   |       |-- action/
|   |       |-- event/
|   |       |-- effect/
|   |       |-- rng/
|   |       `-- error/
|   |
|   |-- rotom-dex/
|   |   `-- src/
|   |       |-- species.rs
|   |       |-- moves.rs
|   |       |-- items.rs
|   |       |-- abilities.rs
|   |       |-- natures.rs
|   |       |-- types.rs
|   |       `-- champions.rs
|   |
|   |-- rotom-formats/
|   |   `-- src/
|   |       `-- reg_mc.rs
|   |
|   |-- rotom-protocol/
|   |
|   `-- rotom-python/        # future
|
|-- tools/
|   |-- showdown-reference/
|   |-- showdown-export/
|   |-- differential/
|   `-- corpus-generator/
|
|-- data/
|   |-- generated/
|   `-- fixtures/
|
|-- tests/
|   |-- mechanics/
|   |-- differential/
|   |-- regression/
|   `-- full_battles/
|
`-- vendor/
    `-- pokemon-showdown/
```

Keep these concerns separate:

- simulation engine;
- static game data;
- format/regulation rules;
- Showdown compatibility tooling;
- future AI integration.

---

# 5. Pin Pokémon Showdown

Do not develop against a moving `master` branch.

Create a file such as:

```text
SHOWDOWN_VERSION
```

containing:

```text
repo=https://github.com/smogon/pokemon-showdown
commit=<PINNED_SHA>
format=gen9championsvgc2026regmc
mod=champions
```

All generated data and differential fixtures should record the pinned commit.

Updating Showdown should be an explicit operation:

```text
old pinned Showdown
        |
engine matches
        |
update pinned SHA
        |
run differential suite
        |
inspect changes
        |
port desired behavior
        |
new engine version
```

---

# 6. Regulation M-C as a Format Layer

Do not hard-code Regulation M-C throughout the battle engine.

Create a format abstraction such as:

```rust
pub trait BattleFormat {
    fn id(&self) -> FormatId;
    fn game_type(&self) -> GameType;
    fn team_size(&self) -> usize;
    fn picked_team_size(&self) -> usize;
    fn validate_team(&self, team: &Team) -> Result<(), ValidationError>;
    fn configure_battle(&self, battle: &mut Battle);
}
```

Then implement:

```rust
pub struct ChampionsRegMC;
```

The format layer should define:

- legal Pokémon/forms;
- legal moves;
- legal abilities;
- legal items;
- level rules;
- doubles battle type;
- team preview;
- bring six / pick four;
- duplicate-species rules;
- duplicate-item rules;
- regulation-specific restrictions.

This allows future regulations to become new configurations rather than new battle engines.

---

# 7. Static Game Data

Use strongly typed data structures.

Example species data:

```rust
pub struct SpeciesData {
    pub id: SpeciesId,
    pub name: String,
    pub dex_number: u16,
    pub base_stats: Stats,
    pub types: [Option<Type>; 2],
    pub abilities: Vec<AbilityId>,
    pub forms: Vec<FormId>,
    pub flags: SpeciesFlags,
}
```

Example move data:

```rust
pub struct MoveData {
    pub id: MoveId,
    pub move_type: Type,
    pub category: MoveCategory,
    pub base_power: u16,
    pub accuracy: Accuracy,
    pub priority: i8,
    pub target: MoveTarget,
    pub pp: u8,
    pub flags: MoveFlags,
    pub effect: MoveEffectId,
}
```

Equivalent types should exist for:

- items;
- abilities;
- natures;
- types;
- forms;
- status conditions;
- weather;
- terrain;
- side conditions;
- volatile conditions.

Do not manually transcribe large quantities of Showdown data.

Instead create a Showdown exporter:

```text
Pinned Showdown Champions Dex
            |
    export_champions.ts
            |
            v
 normalized JSON / generated data
            |
            v
       typed Rust data
```

Export the **resolved Champions Dex**, not raw base-Gen-9 data plus hand-applied overrides.

---

# 8. Separate Data from Behavior

Do not make the static data structure itself implement every mechanic.

For example:

```rust
MoveData {
    id: MoveId::FakeOut,
    base_power: 40,
    effect: MoveEffectId::FakeOut,
}
```

Then implement its behavior separately:

```rust
fn fake_out(ctx: &mut MoveContext) {
    // mechanic implementation
}
```

Organize effects into modules such as:

```text
effects/
|-- abilities/
|-- items/
|-- moves/
|-- statuses/
|-- weather/
|-- terrain/
|-- side_conditions/
`-- volatiles/
```

Avoid one giant move/ability/item resolver if possible.

---

# 9. Battle State

Battle state is the most important long-term API because AI agents will inspect and clone it constantly.

Use plain structured state.

```rust
pub struct BattleState {
    pub turn: u32,
    pub sides: [SideState; 2],
    pub field: FieldState,
    pub queue: ActionQueue,
    pub phase: BattlePhase,
    pub rng: BattleRng,
    pub winner: Option<SideId>,
}
```

Side state:

```rust
pub struct SideState {
    pub team: TeamState,
    pub active: [Option<PokemonIndex>; 2],
    pub selected_team: Vec<PokemonIndex>,
    pub side_conditions: SideConditions,
}
```

Pokémon state should include at minimum:

```rust
pub struct PokemonState {
    pub species: SpeciesId,
    pub form: FormId,
    pub level: u8,
    pub stats: Stats,
    pub current_hp: u16,
    pub ability: AbilityId,
    pub item: Option<ItemId>,
    pub moves: [MoveSlot; 4],
    pub status: Option<MajorStatus>,
    pub status_state: StatusState,
    pub stat_stages: StatStages,
    pub volatiles: VolatileState,
    pub position: PokemonPosition,
    pub fainted: bool,
}
```

Do not mix immutable team definition data with mutable battle state.

Keep separate concepts:

```text
PokemonSet / TeamDefinition
           versus
PokemonState / BattleState
```

---

# 10. Team Representation

The team layer must preserve every choice needed by the eventual team-building AI.

```rust
pub struct PokemonSet {
    pub species: SpeciesId,
    pub form: FormId,
    pub level: u8,
    pub ability: AbilityId,
    pub item: Option<ItemId>,
    pub nature: NatureId,
    pub evs: StatAllocation,
    pub ivs: StatAllocation,
    pub moves: [MoveId; 4],
}
```

If Champions uses a different internal stat-allocation system, normalize it into a representation that can still exactly reproduce the Showdown Champions stats.

Stat generation should be treated as a Champions-specific mechanic and tested directly.

---

# 11. Complete State vs Player Observation

Keep the simulator state complete.

The engine should know all hidden information necessary to simulate the battle.

Later expose a projected player observation:

```rust
pub struct PlayerView {
    pub own_team: KnownTeam,
    pub opponent_public_team: PublicTeam,
    pub revealed_information: RevealedInfo,
    pub field: PublicFieldState,
    pub legal_actions: Vec<Action>,
}
```

Architecture:

```text
true simulator state
        |
observation projection
        |
        v
AI observation
```

Do not make the simulation itself forget hidden information.

---

# 12. Battle Phases

Represent battle phases explicitly.

```rust
pub enum BattlePhase {
    TeamPreview,
    TurnSelection,
    ResolvingTurn,
    ForcedSwitch,
    BattleEnd,
}
```

Typical flow:

```text
TeamPreview
   |
Both players select four and order them
   |
Initial leads enter
   |
TurnSelection
   |
Both players submit actions
   |
ResolvingTurn
   |
Process faints / forced replacements
   |
Next turn or BattleEnd
```

Illegal commands should be rejected based on the current phase.

---

# 13. Action Representation

Player actions should be typed values.

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

A doubles turn could be:

```rust
pub struct TurnChoice {
    pub actions: [PokemonAction; 2],
}
```

The engine should eventually expose:

```rust
battle.legal_actions(SideId::P1)
```

The engine owns action legality. Future neural networks should select from legal actions rather than constructing strings.

---

# 14. Legal Action Enumeration

Implement a first-class API such as:

```rust
fn legal_actions(&self, side: SideId) -> LegalActionSet
```

It must consider:

- fainted Pokémon;
- disabled moves;
- PP;
- move targeting;
- switch restrictions;
- trapped Pokémon;
- available bench Pokémon;
- forced-switch states;
- Mega Evolution availability;
- invalid combined doubles choices;
- any Champions-specific restrictions.

This will later become the AI policy action mask.

---

# 15. Deterministic RNG

All randomness must come from a battle-owned RNG.

Never use global or thread-local randomness inside battle mechanics.

```rust
pub struct BattleRng {
    // deterministic PRNG state
}
```

Every random mechanic should call through this interface:

```rust
rng.chance(...)
rng.range(...)
rng.sample(...)
```

Examples:

- accuracy;
- critical hits;
- damage rolls;
- secondary effects;
- paralysis;
- confusion;
- multi-hit count;
- speed ties;
- random targets;
- random move selection.

A battle should be reproducible from:

```text
team A
team B
team preview selections
action sequence
seed
engine version
```

---

# 16. RNG Tracing

Add an optional diagnostic trace.

```text
turn: 3
event: 42
reason: DamageRoll
range: 85..100
result: 93
```

This will make Showdown differential debugging much easier.

One extra or missing random draw can otherwise make every later event diverge.

---

# 17. Event-Driven Mechanics

Pokémon mechanics frequently intercept one another, so use an event model rather than one enormous sequential function.

Possible events include:

```rust
pub enum BattleEvent {
    BeforeTurn,
    BeforeMove,
    ModifyPriority,
    ModifySpeed,
    ModifyAttack,
    ModifyDefense,
    ModifyBasePower,
    ModifyDamage,
    TryHit,
    Hit,
    AfterDamage,
    AfterMove,
    SwitchIn,
    SwitchOut,
    Faint,
    Residual,
}
```

Do not blindly recreate Showdown's TypeScript architecture. Preserve the **semantics**, but use typed Rust structures and static dispatch where practical.

---

# 18. Effect Ordering

Pokémon mechanics depend heavily on exact ordering.

Create a generalized effect-ordering mechanism.

```rust
pub struct EffectOrder {
    pub order: i16,
    pub priority: i16,
    pub sub_order: i16,
}
```

When an event occurs:

```text
collect applicable effects
        |
assign ordering metadata
        |
sort
        |
execute
```

Applicable sources may include:

- abilities;
- items;
- weather;
- terrain;
- statuses;
- side conditions;
- volatile conditions;
- move effects.

Do not scatter ordering assumptions throughout unrelated mechanics code.

---

# 19. Action Queue

Construct an explicit action queue for each turn.

```rust
pub struct QueuedAction {
    pub side: SideId,
    pub pokemon: PokemonIndex,
    pub choice: PokemonAction,
    pub priority: i8,
    pub sub_priority: i8,
    pub speed: u16,
    pub tie_breaker: u32,
}
```

Typical ordering pipeline:

```text
player choices
      |
expand actions
      |
calculate move/switch priority
      |
apply priority modifiers
      |
calculate effective speed
      |
resolve speed ties via battle RNG
      |
ordered queue
      |
execute
```

The queue should be inspectable in tests.

---

# 20. Move Execution Pipeline

Do not make move execution one giant function.

Use an explicit pipeline such as:

```text
Validate move
      |
BeforeMove
      |
Consume PP
      |
Resolve target
      |
Accuracy
      |
TryHit
      |
Damage / primary effect
      |
Secondary effects
      |
AfterHit
      |
AfterMove
      |
Faint processing
```

The exact order must be derived from the pinned Showdown Champions behavior.

---

# 21. Damage Subsystem

Give damage calculation its own subsystem.

```rust
pub fn calculate_damage(
    battle: &BattleState,
    source: PokemonRef,
    target: PokemonRef,
    move_id: MoveId,
    roll: DamageRoll,
) -> DamageResult
```

Where practical, expose intermediate values:

```rust
pub struct DamageResult {
    pub base_damage: u32,
    pub attack_stat: u32,
    pub defense_stat: u32,
    pub stab_modifier: Modifier,
    pub type_modifier: Modifier,
    pub critical_modifier: Modifier,
    pub weather_modifier: Modifier,
    pub random_modifier: Modifier,
    pub final_damage: u16,
}
```

This will make mechanical debugging much easier.

---

# 22. Battle State Cloning

State cloning is essential for future AI search.

Support either:

```rust
impl Clone for BattleState
```

or a specialized method:

```rust
battle.fork()
```

Benchmark it early.

The future AI may branch from one state into many hypothetical continuations, so cloning cannot be prohibitively expensive.

---

# 23. Structured Battle Log

Do not use textual logs as authoritative state.

Battle state is structured Rust data.

A structured event log is useful for:

- debugging;
- replays;
- Showdown comparison;
- AI analysis.

Example:

```rust
pub enum BattleLogEvent {
    MoveUsed { /* ... */ },
    Damage { /* ... */ },
    StatusApplied { /* ... */ },
    AbilityActivated { /* ... */ },
    ItemConsumed { /* ... */ },
    PokemonFainted { /* ... */ },
}
```

The log should never drive the simulator.

---

# 24. Showdown Reference Adapter

Create a small Node/TypeScript tool under:

```text
tools/showdown-reference/
```

It should instantiate Showdown battles directly without requiring a web server.

Example input:

```json
{
  "format": "gen9championsvgc2026regmc",
  "seed": [1, 2, 3, 4],
  "p1_team": "...",
  "p2_team": "...",
  "commands": []
}
```

Normalize Showdown output into project-owned JSON rather than making Rust code understand every raw Showdown protocol detail.

Possible normalized output:

```json
{
  "events": [],
  "snapshots": [],
  "winner": null
}
```

---

# 25. Differential Testing

Development loop:

```text
generate fixture / battle
        |
run pinned Showdown
        |
run Rotom Spark
        |
normalize both states
        |
compare
```

Compare important state after each meaningful transition, not just the final winner.

Compare at minimum:

- HP;
- status;
- stat stages;
- item state;
- ability state;
- form;
- active positions;
- weather;
- terrain;
- side conditions;
- volatile conditions;
- PP;
- fainted state;
- legal choices;
- winner.

---

# 26. Three Differential-Test Levels

## Level 1 — Mechanic Fixtures

Small focused cases such as:

```text
Flamethrower -> damage calculation
Fake Out -> flinch
Protect -> block move
Intimidate -> attack reduction
Choice item -> move locking
Tailwind -> speed ordering
Trick Room -> inverted speed ordering
Mega Evolution -> form/stat transition
```

These should be easy to diagnose.

## Level 2 — Turn Fuzzing

Generate many legal states or partial legal battle histories.

Resolve one turn in both engines and compare normalized state.

## Level 3 — Full Battles

Generate legal Regulation M-C teams and use deterministic simple policies, for example:

- random legal action;
- highest base-power legal move;
- simple heuristic policy.

Give both simulators the same player decisions and compare the full trajectory.

---

# 27. Randomness During Differential Testing

Use two modes.

## Forced-Outcome Mode

Inject explicit outcomes where useful:

```text
accuracy = hit
critical = false
damage roll = 91
secondary effect = false
speed tie = P1
```

This allows deterministic mechanic comparison before PRNG parity is complete.

## Seed-Parity Mode

Later reproduce Showdown's PRNG semantics closely enough that the same seed and actions produce the same battle.

Seed parity is desirable, but it should not block early mechanics work.

---

# 28. Mechanics Implementation Order

Implement vertically rather than trying to finish the entire game at once.

## Milestone A — Battle Skeleton

Support:

- battle creation;
- team loading;
- team preview;
- pick four;
- starting leads;
- turn choices;
- switching;
- fainting;
- victory detection.

Moves may initially be restricted to a tiny test set.

## Milestone B — Fundamental Combat

Implement:

- Champions stats;
- type chart;
- damage formula;
- STAB;
- accuracy;
- priority;
- speed ordering;
- critical hits;
- physical/special/status categories;
- PP;
- targeting.

## Milestone C — Common State Mechanics

Implement:

- major statuses;
- stat stages;
- volatile effects;
- weather;
- terrain;
- side conditions;
- residual effects.

## Milestone D — Effects

Implement Regulation M-C legal:

- abilities;
- items;
- moves;
- forms;
- Mega Evolution;
- other Champions-specific mechanics.

Prioritize common and strategically important mechanics first.

## Milestone E — Exhaustive Legal Coverage

Every legal M-C mechanic must be either:

```text
SUPPORTED
```

or explicitly:

```text
UnsupportedMechanic
```

Never silently approximate.

---

# 29. Mechanics Coverage Registry

Maintain generated coverage data such as:

```text
data/mechanics-coverage.json
```

Example:

```json
{
  "moves": {
    "protect": "supported",
    "fakeout": "supported",
    "metronome": "unsupported"
  }
}
```

CI should report coverage for:

- moves;
- abilities;
- items;
- forms;
- relevant conditions.

Goal: 100% of legal Regulation M-C mechanics.

---

# 30. Team Validator

The Rust validator should enforce enough rules that all simulator inputs are valid Regulation M-C teams.

Validate:

- team size;
- species/forms;
- moves;
- abilities;
- items;
- level;
- stat allocations;
- move legality where required;
- species clause;
- item clause;
- regulation restrictions.

Initially, generated legality data from the pinned Showdown format can be used instead of reimplementing all historical Showdown validation logic.

---

# 31. Serialization and Replays

Use `serde` for important data structures.

```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct BattleState {
    // ...
}
```

Any failing differential case should be savable as a permanent fixture containing:

```text
engine version
Showdown SHA
format
teams
seed
action sequence
expected state
actual state
```

This should make every bug replayable.

---

# 32. Normalized Snapshots

Provide a normalized representation:

```rust
battle.snapshot()
```

Example:

```json
{
  "turn": 7,
  "weather": "rain",
  "p1": {
    "active": [
      {
        "species": "rillaboom",
        "hp": 117,
        "status": null,
        "boosts": {
          "atk": 0
        }
      }
    ]
  }
}
```

Differential tests should compare normalized snapshots rather than internal memory layouts.

---

# 33. CLI

Add a simple development binary:

```text
rotom
```

Useful commands:

```text
rotom validate-team team.txt

rotom battle \
  --p1 team1.txt \
  --p2 team2.txt \
  --seed 123

rotom replay regression.json

rotom differential regression.json

rotom coverage
```

The CLI is primarily for development and debugging.

---

# 34. Testing Philosophy

Every mechanics bug becomes a permanent regression test.

Use:

```text
tests/regression/
```

If a particular combination once behaved incorrectly, preserve it forever as a focused test.

Also add invariant/property tests such as:

- HP never exceeds max HP unless explicitly recalculated;
- HP never goes below zero;
- fainted Pokémon have zero HP;
- inactive Pokémon cannot execute normal actions;
- PP cannot become negative;
- one Pokémon cannot occupy two positions;
- battles stop after a winner exists;
- stat stages remain inside valid bounds.

Use `proptest` where useful.

---

# 35. Performance Benchmarks

Correctness comes first, but benchmark from the beginning.

Track:

- battle creations per second;
- turns per second;
- full battles per second;
- `BattleState` clone latency;
- `BattleState` size;
- legal-action generation latency;
- damage-calculation latency.

The most important long-term metric is likely:

```text
simulated state transitions per CPU core per second
```

---

# 36. Keep the Engine Synchronous

Do not put async logic inside a single battle.

One battle should behave approximately like:

```rust
battle.step(choice1, choice2)
```

Parallelism should happen outside the battle:

```text
thread 1 -> battles
thread 2 -> battles
thread 3 -> battles
...
```

This is ideal for future self-play.

Do not introduce a general-purpose ECS unless there is a proven need. Pokémon battles contain a small, fixed, structured set of entities, so typed battle state is preferable.

---

# 37. Public API Target

A target API might look like:

```rust
let config = BattleConfig {
    format: FormatId::ChampionsRegMC,
    seed,
    p1_team,
    p2_team,
};

let mut battle = Battle::new(config)?;

battle.submit_team_preview(
    SideId::P1,
    p1_selection,
)?;

battle.submit_team_preview(
    SideId::P2,
    p2_selection,
)?;

while !battle.is_finished() {
    let p1_actions = battle.legal_actions(SideId::P1);
    let p2_actions = battle.legal_actions(SideId::P2);

    let p1_choice = choose(...);
    let p2_choice = choose(...);

    battle.step(p1_choice, p2_choice)?;
}

println!("{:?}", battle.winner());
```

---

# 38. `step()` as the Core Simulation Primitive

Aim for a central API such as:

```rust
pub fn step(
    &mut self,
    p1: Choice,
    p2: Choice,
) -> Result<StepResult, BattleError>
```

Where:

```rust
pub struct StepResult {
    pub events: Vec<BattleLogEvent>,
    pub phase: BattlePhase,
    pub winner: Option<SideId>,
}
```

This maps naturally onto the future training environment.

---

# 39. State Hashing

Add a deterministic gameplay-state hash.

```rust
BattleState::hash()
```

Include gameplay-relevant state and ignore irrelevant debug metadata.

Future uses:

- MCTS transposition tables;
- state deduplication;
- training-data deduplication;
- cache lookup;
- regression validation.

---

# 40. No Global Mutable State

Static Dex and regulation data should be immutable.

All battle-specific mutable information must live inside the battle state.

This allows many battles to run safely in parallel.

---

# 41. Error Policy

Use explicit errors.

```rust
pub enum BattleError {
    IllegalChoice,
    UnsupportedMechanic,
    InvalidTeam,
    InvalidState,
    DataMismatch,
}
```

Unsupported behavior should fail loudly during development.

A wrong result is more dangerous than a failed test because incorrect simulations would eventually poison AI training data.

---

# 42. Phase 1 Definition of Done

Phase 1 is complete when:

1. `gen9championsvgc2026regmc` teams can be loaded.
2. Regulation M-C team legality is enforced.
3. Team preview and bring-six/pick-four work.
4. Complete doubles battles run locally without Showdown.
5. Every legal M-C species/form is represented.
6. Every legal M-C ability is implemented.
7. Every legal M-C item is implemented.
8. Every legal M-C move is implemented.
9. Mega Evolution behaves according to the pinned Champions implementation.
10. Major status, weather, terrain, side conditions, and relevant volatile mechanics work.
11. Battles are deterministic from a seed.
12. Battle state can be cloned.
13. Legal choices can be enumerated.
14. State can be serialized and restored.
15. Structured event logs are available.
16. Differential tests against pinned Showdown exist.
17. A large randomized differential suite passes.
18. Full battle trajectories match Showdown across a substantial generated corpus.
19. No runtime dependency on Showdown exists.
20. Performance benchmarks exist for important simulator operations.

---

# 43. Suggested Implementation Sequence

Do not ask Codex to implement the entire game in one pass.

Use small milestones.

## PR 1 — Workspace and Foundations

Implement:

- Rust workspace;
- crate boundaries;
- common IDs/enums;
- error types;
- serialization setup;
- CI;
- minimal CLI shell.

No real battle mechanics yet.

## PR 2 — Pin Showdown and Export Data

Implement:

- pinned Showdown checkout/submodule;
- `SHOWDOWN_VERSION`;
- Champions resolved-Dex exporter;
- generated normalized JSON;
- Rust loader/generator for static data.

## PR 3 — Regulation M-C and Team Parsing

Implement:

- Regulation M-C format definition;
- team representation;
- team parser;
- legality validation;
- unit tests.

## PR 4 — Battle State

Implement:

- `BattleState`;
- `SideState`;
- `PokemonState`;
- field state;
- phases;
- serialization;
- snapshots.

## PR 5 — Team Preview / Switching / Victory

Implement:

- bring six / pick four;
- ordering;
- starting leads;
- switching;
- faint state;
- replacement requirements;
- victory detection.

## PR 6 — Action Types and Legal Action Enumeration

Implement:

- typed moves/switches;
- target representation;
- legal-action generation;
- rejection of illegal actions.

## PR 7 — Deterministic RNG

Implement:

- battle-owned PRNG;
- deterministic seed handling;
- RNG trace;
- test reproducibility.

## PR 8 — Action Queue

Implement:

- priority;
- speed ordering;
- speed ties;
- queue inspection;
- initial tests.

## PR 9 — Base Combat

Implement:

- stats;
- type effectiveness;
- STAB;
- accuracy;
- base damage;
- PP;
- basic targeting;
- physical/special/status categories.

## PR 10 — Event / Effect Framework

Implement:

- battle events;
- effect registration/dispatch;
- effect ordering;
- initial generic hooks.

## PR 11 — Common Conditions

Implement:

- stat stages;
- major statuses;
- volatile conditions;
- residual processing.

## PR 12 — Field Effects

Implement:

- weather;
- terrain;
- side conditions;
- duration handling;
- ordering tests.

## PR 13+ — Moves / Abilities / Items

Implement in manageable batches.

Every batch should include:

- mechanic code;
- focused unit tests;
- Showdown differential fixtures;
- coverage-registry updates.

## Later PR — Mega Evolution and Special Champions Mechanics

Implement special transformation/form mechanics with dedicated tests.

## Later PR — Full Differential Harness

Expand the early Showdown adapter into:

- normalized snapshots;
- automatic state comparison;
- random fixtures;
- regression fixture generation;
- full-battle corpus comparison.

## Final Phase — Profiling and Optimization

Only once correctness is strong:

- measure hot paths;
- optimize battle cloning;
- reduce allocations;
- compact IDs/state;
- improve batch simulation throughput.

---

# 44. Build the Showdown Adapter Early

Although the complete fuzz/differential framework comes later, create a small Showdown adapter early.

As soon as a subsystem exists, compare it.

Examples:

```text
damage implemented -> compare damage
switching implemented -> compare switching
status implemented -> compare status
priority implemented -> compare priority
```

Do not build the entire simulator and only then begin validating it.

---

# 45. Do Not Implement These Yet

Phase 1 should not include:

- neural networks;
- PyTorch training;
- reinforcement learning;
- MCTS;
- opponent modeling;
- team-generation AI;
- metagame statistics;
- ladder bots;
- Pokémon Champions Android automation;
- Champions state extraction;
- Champions packet inspection;
- Champions RNG reverse engineering;
- GUI;
- web server;
- distributed training infrastructure.

These belong above or after the battle engine.

---

# 46. Future Work — Direct Pokémon Champions Validation

After the Showdown-based simulator is mature, compare selected mechanics against Pokémon Champions itself.

Architecture:

```text
             Pokemon Champions
                    |
          sampled empirical tests
                    |
                    v
           discrepancy analysis
              ^             ^
              |             |
        Rotom Engine     Showdown
```

The purpose will be to identify places where Showdown and the real game differ.

Showdown parity is the Phase 1 specification. It is not proof that Showdown is perfectly identical to Pokémon Champions.

---

# 47. Future Work — Python AI Layer

Add PyO3 bindings only after the Rust simulator is reliable.

Target API:

```python
battle = rotom.Battle(...)

obs1 = battle.observation(0)
actions1 = battle.legal_actions(0)

result = battle.step(action1, action2)

clone = battle.clone()
```

Avoid converting the entire state into Python dictionaries every turn.

Eventually expose packed numeric arrays / NumPy-compatible observations.

Architecture:

```text
Rust simulator
     |
     |-- native Rust search
     |
     `-- PyO3
          |
          v
        Python
          |
          v
        PyTorch
```

---

# 48. Future Work — General Pilot AI

The pilot should eventually operate arbitrary legal teams, not one fixed team.

Conceptually:

```text
Battle observation
        |
policy/value model
        |
probabilities over legal actions
```

The engine provides the legal-action mask.

Potential training progression:

```text
replays / imitation data
        |
self-play
        |
policy/value improvement
        |
strong general-purpose pilot
```

The exact model architecture should remain separate from the simulator.

---

# 49. Future Work — Team-Building AI

The team builder operates one level above the pilot.

Its search/action space includes:

- species;
- form;
- ability;
- item;
- nature;
- stat allocation;
- four moves.

Conceptually:

```text
Team Builder
     |
 candidate team
     |
     v
Strong Pilot
     |
     v
many simulated games
     |
     v
win rate / matchup score
     |
     `-----------------> Team Builder training signal
```

This keeps piloting and team construction as separate optimization problems while sharing the same simulator.

---

# 50. Long-Term Architecture

```text
+------------------------------------------------------+
|                    TEAM BUILDER                      |
|                                                      |
| species / moves / item / ability / nature / stats   |
+--------------------------+---------------------------+
                           |
                         team
                           |
                           v
+------------------------------------------------------+
|                    PILOT MODEL                       |
|                                                      |
| arbitrary battle observation -> action policy/value |
+--------------------------+---------------------------+
                           |
                        actions
                           |
                           v
+------------------------------------------------------+
|                   ROTOM SPARK                        |
|                                                      |
| deterministic Champions Reg M-C simulation          |
|                                                      |
| state + joint action + RNG -> next state             |
+--------------------------+---------------------------+
                           |
                        outcomes
                           |
            +--------------+--------------+
            |                             |
            v                             v
     pilot training                team-builder training
```

External validation systems remain outside this runtime loop:

```text
Pokemon Showdown -> development differential validation
Pokemon Champions -> future empirical validation
```

---

# 51. How Codex Should Tackle This Project

The most important practical rule is:

> **Do not ask Codex to "implement the battle engine." Ask it to complete one narrow milestone at a time with explicit acceptance tests.**

Large autonomous coding tasks tend to go wrong when the requested outcome is broad and difficult to verify. This project is especially vulnerable because mechanics bugs can remain hidden for a long time.

Use Codex as an iterative engineering agent rather than a one-shot code generator.

---

# 52. Add an `AGENTS.md`

Create an `AGENTS.md` in the repository root and keep the highest-level project rules there.

Recommended content:

```markdown
# Rotom Spark Engineering Rules

## Scope
Rotom Spark is a deterministic Rust simulator for Pokemon Champions Regulation M-C.

## Phase 1 Reference
The pinned Pokemon Showdown Champions Regulation M-C implementation defines expected Phase 1 behavior.

## Core Rules
1. Correctness before optimization.
2. Never silently approximate an unsupported mechanic.
3. Showdown is development/test tooling only and must not become a runtime dependency.
4. All randomness must pass through BattleRng.
5. Battle state must remain serializable and cloneable.
6. Do not add ML, networking, GUI, or Champions automation unless explicitly requested.
7. Every mechanics bug fix must include a regression test.
8. Prefer focused, typed Rust abstractions over reproducing Showdown's TypeScript design literally.
9. Keep changes scoped to the requested milestone.
10. Run formatting, linting, and relevant tests before completing a task.
```

Codex should read this every time it works on the repo.

---

# 53. Give Codex Small, Verifiable Tasks

A good Codex task should contain:

1. **Goal** — one milestone only.
2. **Files/areas allowed to change** where useful.
3. **Behavior required.**
4. **Tests required.**
5. **Explicit exclusions.**
6. **Definition of done.**

Example:

```text
Implement the initial BattleState data model described in
ROTOM_SPARK_IMPLEMENTATION_PLAN.md.

Scope:
- crates/rotom-core/src/state/
- minimal exports from rotom-core/src/lib.rs
- tests for serialization and cloning

Requirements:
- add BattleState, SideState, PokemonState, FieldState, BattlePhase
- all gameplay state must be serializable with serde
- BattleState must implement Clone
- do not implement move resolution yet
- do not add async, networking, Python, or Showdown integration

Acceptance criteria:
- cargo fmt --check passes
- cargo clippy passes for the workspace
- cargo test passes
- add tests proving serialize -> deserialize round-trip equality
- add a test proving cloning a state yields independent mutable copies

Before coding, inspect the existing repository and summarize the intended changes.
After coding, report changed files, tests run, and any unresolved design questions.
```

This is much safer than:

```text
Build the Pokemon battle engine.
```

---

# 54. Require Codex to Inspect Before Editing

For each milestone, begin prompts with something like:

```text
First inspect the repository, the implementation plan, and relevant existing tests.
Do not edit anything until you understand the current architecture.
```

Then ask it to summarize the intended implementation before modifying files.

This reduces duplicate abstractions and architecture drift.

---

# 55. One Milestone per Codex Session or PR

Avoid combining unrelated work such as:

```text
implement damage + weather + abilities + Python bindings
```

Instead use:

```text
Session 1: battle state
Session 2: team preview
Session 3: legal actions
Session 4: deterministic RNG
Session 5: priority/speed queue
Session 6: damage
...
```

Each milestone should end in a clean commit or PR that is independently reviewable.

---

# 56. Force Tests into Every Mechanics Task

Never ask Codex only to implement behavior.

Always require tests in the same task.

For example:

```text
Implement speed ordering.

Tests must cover:
- faster Pokemon acts first;
- priority outranks speed;
- Trick Room behavior when later implemented should have an extension point;
- exact speed ties consume BattleRng;
- identical seed produces identical tie result.
```

For individual mechanics, also require a Showdown differential fixture once the adapter exists.

---

# 57. Let Codex Build Vertical Slices

Prefer complete narrow flows over dozens of unconnected abstractions.

A useful early vertical slice is:

```text
load two tiny teams
        |
team preview
        |
select leads
        |
submit one simple damaging move
        |
resolve speed order
        |
calculate damage
        |
update HP
        |
produce snapshot
```

Once that works end-to-end, widen mechanic coverage.

This catches architecture problems earlier than building every data type first.

---

# 58. Ask Codex to Preserve a Running TODO / Coverage List

Maintain something like:

```text
IMPLEMENTATION_STATUS.md
```

or the generated mechanics coverage registry.

Codex should update it only when a feature has tests proving it works.

Example:

```text
[x] workspace
[x] battle state
[x] team preview
[x] deterministic RNG
[x] basic speed ordering
[x] basic damage
[ ] statuses
[ ] weather
[ ] terrain
[ ] abilities
[ ] items
[ ] complete move coverage
```

This prevents "implemented" from meaning "there is some code for it."

---

# 59. Use Codex for Showdown Research in Focused Bursts

Do not ask Codex to understand all of Showdown at once.

For a mechanic, ask it to trace one path.

Example:

```text
Research only how the pinned Showdown Champions implementation computes
move order for a normal doubles turn.

Identify:
- relevant files/functions;
- priority calculation;
- speed calculation;
- tie-breaking behavior;
- Champions-specific overrides.

Do not modify Rotom Spark yet.
Write findings to docs/research/action-ordering.md with source file/function references.
```

Then make the next task:

```text
Implement Rotom action ordering according to docs/research/action-ordering.md.
```

Separating research from implementation makes large mechanics much easier to audit.

---

# 60. Prefer Source References in Research Notes

Whenever Codex derives behavior from Showdown, require it to record:

- pinned commit SHA;
- source file;
- relevant class/function/hook;
- any Champions override involved;
- what behavior Rotom must reproduce.

For example:

```text
Showdown source:
- sim/battle.ts: <function>
- data/mods/champions/scripts.ts: <override>

Rotom behavior:
- effective speed is computed after X and before Y
- ties consume the battle PRNG
```

This is better than leaving mechanics knowledge only in Codex's temporary context.

---

# 61. Tell Codex Not to Over-Generalize Early

Codex may try to build elaborate abstractions before the requirements are understood.

Prompts should explicitly say:

```text
Prefer the simplest typed design that supports the current requirements.
Do not introduce plugin systems, ECS frameworks, async runtimes, dynamic scripting,
or generic effect DSLs unless the existing architecture demonstrably requires them.
```

The engine can be refactored once real mechanics reveal the necessary abstractions.

---

# 62. Have Codex Run the Full Quality Gate

Every coding task should end with at least:

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

Later add:

```text
npm / TypeScript tests for Showdown tools
rotom differential <fixture>
benchmark smoke tests
```

Do not accept a milestone merely because it compiles.

---

# 63. Ask Codex for a Change Report

At the end of every milestone, require a short report:

```text
1. Files changed
2. Behavior implemented
3. Tests added
4. Commands run
5. Known limitations
6. Follow-up work that is intentionally out of scope
```

This makes it much easier for you to review work without reading the entire diff immediately.

---

# 64. Recommended First Codex Tasks

A good initial sequence is:

## Task 1

```text
Read ROTOM_SPARK_IMPLEMENTATION_PLAN.md.
Create the Rust workspace and crate layout only.
Add AGENTS.md with the project engineering rules.
Add CI, rustfmt, clippy, serde dependencies where appropriate, and a minimal rotom CLI.
Do not implement battle mechanics.
Add smoke tests proving the workspace builds and the CLI starts.
```

## Task 2

```text
Pin Pokemon Showdown and build tools/showdown-export.
Export the resolved Champions Regulation M-C species, moves, items,
abilities, forms, and relevant format legality into normalized generated data.
Do not implement battle mechanics yet.
Add deterministic snapshot tests for the exported data.
```

## Task 3

```text
Implement the Regulation M-C Team and PokemonSet data model and validator.
Use generated legality data from the pinned Showdown export.
Add unit tests for valid and invalid teams.
Do not implement battle resolution.
```

## Task 4

```text
Implement BattleState, SideState, PokemonState, FieldState, and BattlePhase.
Add serde round-trip tests, Clone tests, and normalized snapshot generation.
Do not implement moves or action resolution.
```

## Task 5

```text
Implement team preview, bring-six/pick-four selection, lead ordering,
switching, faint state, forced replacement state, and basic victory detection.
Use only test fixtures that do not require real move effects.
```

## Task 6

```text
Implement typed player actions and legal-action enumeration.
Do not implement full move effects yet.
Test move targeting, switch legality, forced switches, and illegal choices.
```

## Task 7

```text
Implement BattleRng and deterministic RNG tracing.
Replace all simulator randomness with BattleRng.
Add reproducibility tests.
```

## Task 8

```text
Research and implement normal action ordering and speed ties.
Add focused Showdown reference fixtures before implementing complicated
priority-modifying abilities/items.
```

## Task 9

```text
Implement the first complete vertical combat slice:
normal damaging moves with stats, STAB, type effectiveness, accuracy,
PP, speed ordering, HP reduction, fainting, and snapshots.
Use a deliberately tiny supported move set at first.
Add Showdown differential tests for those moves.
```

After this point, widen mechanics gradually.

---

# 65. Recommended Working Rhythm

For each significant mechanic:

```text
1. Research Showdown behavior
2. Record source-backed mechanic notes
3. Write/define expected tests
4. Implement the narrow behavior
5. Run unit tests
6. Run differential tests
7. Save any discovered regression fixture
8. Update coverage/status
9. Commit
```

This loop is much safer than allowing Codex to make large speculative changes.

---

# 66. What You Should Review Personally

Even if Codex writes most of the code, manually review the highest-leverage architecture points:

- `BattleState` ownership boundaries;
- RNG design;
- event/effect ordering;
- action queue semantics;
- legal-action enumeration;
- state cloning strategy;
- Showdown normalization/differential logic;
- public `step()` API.

If these foundations are wrong, hundreds of later mechanics will inherit the mistake.

Individual move implementations are much safer to delegate once the framework is stable.

---

# 67. When to Let Codex Work More Autonomously

Early project phase:

```text
small tasks
strict scope
manual review
```

After the battle framework and differential harness are trustworthy:

```text
larger mechanic batches
Codex can research + implement + test
CI/differential suite acts as guardrail
```

A good later task might be:

```text
Implement the next 20 currently unsupported Regulation M-C abilities
listed in mechanics-coverage.json.
For each ability:
- research the pinned Showdown implementation;
- implement it using existing event hooks;
- add focused tests;
- add at least one differential fixture;
- update coverage only after tests pass.
Do not change core event architecture unless a mechanic cannot be expressed
with the current model; if that occurs, stop and document the architectural gap.
```

That is the point where Codex can become highly productive on bulk mechanic coverage.

---

# 68. Final Recommendation

Treat Codex as a strong implementation partner operating inside a carefully constructed verification loop.

The project should progress like this:

```text
small architecture milestone
        |
tests
        |
Showdown comparison
        |
manual review
        |
commit
        |
next milestone
```

Once the core engine, snapshots, deterministic RNG, and differential harness are stable, Codex can safely take on much larger batches of moves, abilities, and items because incorrect work will be exposed automatically.

The biggest mistake would be to give Codex one massive prompt asking it to recreate Pokémon Champions mechanics all at once.

The best strategy is to first build the machinery that makes Codex's future mistakes cheap to detect.
