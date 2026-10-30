# Showdown resolved-data exporter

This development-only tool loads the exact Pokémon Showdown revision recorded
in `SHOWDOWN_VERSION`, resolves the configured format through Showdown's Dex,
and writes normalized JSON for Rust. Showdown is not used by the simulator at
runtime.

From the repository root:

```text
git submodule update --init --recursive
npm ci --prefix vendor/pokemon-showdown
npm run build --prefix vendor/pokemon-showdown
node tools/showdown-export/export-champions.cjs
```

Verify that the checked-in generated file is current:

```text
node tools/showdown-export/export-champions.cjs --check
```

PR 13.0 adds a separate reachability and coverage artifact. It enumerates
complete validator-approved starting sets, then closes over held items,
battle-only forms, abilities, and structured generated conditions:

```text
node tools/showdown-export/generate-mechanics-manifest.cjs
node tools/showdown-export/generate-mechanics-manifest.cjs --check
node tools/showdown-export/generate-mechanics-manifest.cjs --next=move
node tools/showdown-export/generate-mechanics-manifest.cjs --next-research
node --test tools/showdown-export/mechanics-manifest.test.cjs
```

PR 15.0 derives a separate capability dependency graph from that manifest. It
covers every partial ability/item and unsupported move, and deliberately keeps
identity coverage separate from shared executable-capability coverage:

```text
node tools/showdown-export/generate-capability-report.cjs
node tools/showdown-export/generate-capability-report.cjs --check
node --test tools/showdown-export/capability-report.test.cjs
```

The generated JSON is the machine-readable blocker graph. The generated
Markdown is its headline audit; neither file promotes an identity merely
because one callback or metadata field is registered.

`data/mechanics-coverage.json` contains reviewed implementation and verification
claims. Move implementation status is derived from the complete operation and
capability requirements exported from the resolved move. The current PR 13.4
set includes reusable primary operations plus conditional power, effective
stat overrides, chance secondaries, self effects, fixed/level and multi-hit
damage, drain, and recoil. Reviewed claims may add stronger verification, but cannot mark a
move implemented when its runtime requirements are unsupported. Other entries
default to `unsupported` / `not_tested`. Dynamic callbacks whose reachability
closure cannot be proven from structured data are reported under
`unresolved_reachability` for source-backed follow-up.

Format-level `legal` fields reflect the resolved Showdown ban and tag rules.
The export also includes natures and each species' resolved move pool for the
Regulation M-C team validator. Event-only moves are marked separately because
their source-combination rules are not silently approximated by Rotom Spark.

