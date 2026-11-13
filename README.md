# Rotom Spark v2

A clean Rust foundation for a deterministic Champions Regulation M-C battle-state transition engine.

Only `crates/rotom-engine` is active, with no battle implementation yet. Dex and format source is preserved but excluded pending v2 contracts. See docs/v2/ARCHITECTURE.md, ENGINEERING_RULES.md, and MIGRATION.md.

## Verification

```text
cargo fmt --check
cargo check --workspace
cargo test --workspace
```

Pinned Showdown information, generated Champions data, team inputs, differential fixtures, reference scripts, and static-data export tooling remain available. Showdown is a development oracle only.

