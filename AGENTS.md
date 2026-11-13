# Rotom Spark v2 rules

Read docs/v2/ARCHITECTURE.md and ENGINEERING_RULES.md before implementing.

- One authoritative cloneable, serializable battle state.
- Only the kernel mutates production state and enforces representation invariants.
- Mechanics evaluates rules; one reusable typed effect interpreter executes effects.
- Moves are primarily data. No ordinary move-ID-specific executor branching.
- No speculative event bus, capability registry, or historical runtime compatibility shims.
- One explicit deterministic RNG stream; errors preserve caller state and RNG. Diagnostics never affect outcomes.
- Verify behavior against pinned Champions Showdown sources and focused fixtures. Showdown is development-only; unsupported mechanics fail explicitly.
- Keep immutable Dex/format data separate from state. Dependencies point toward state/kernel, never orchestration.
- Make small tested changes. Run cargo fmt --check, cargo check --workspace, cargo test --workspace and report actual results.

Only rotom-engine is active. Dex/formats source is excluded pending independent v2 contracts. Do not copy historical runtime types to enable it.
