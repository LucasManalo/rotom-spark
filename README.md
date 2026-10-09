# Rotom Spark

Rotom Spark is a standalone, deterministic Rust battle simulator targeting
Pokémon Champions Regulation M-C. During Phase 1, behavior will be validated
against a pinned Pokémon Showdown Champions implementation.

The repository currently contains the workspace foundations, normalized static
data from a pinned Pokémon Showdown revision, Regulation M-C team parsing and
validation, and the serializable core battle-state model. Battle resolution is
being added incrementally; team preview, basic switching, faint resolution,
forced replacements, and victory detection are available.

## Workspace

- `rotom-core`: shared engine IDs, errors, and authoritative battle state.
- `rotom-dex`: static game data boundary.
- `rotom-formats`: format rules and legality boundary.
- `rotom-protocol`: normalized battle snapshots and protocol boundary.
- `rotom-cli`: development command-line shell, installed as `rotom`.

## Development

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

See `AGENTS.md` and `ROTOM_SPARK_IMPLEMENTATION_PLAN.md` before making changes.

## Regenerating Champions data

Pokémon Showdown is a development-only Git submodule. It is not needed when
using the checked-in generated data through `rotom-dex`.

```text
git submodule update --init --recursive
npm ci --prefix vendor/pokemon-showdown
npm run build --prefix vendor/pokemon-showdown
node tools/showdown-export/export-champions.cjs
node tools/showdown-export/export-champions.cjs --check
```

