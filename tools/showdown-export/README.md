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

Format-level `legal` fields reflect the resolved Showdown ban and tag rules.
The export also includes natures and each species' resolved move pool for the
Regulation M-C team validator. Event-only moves are marked separately because
their source-combination rules are not silently approximated by Rotom Spark.

