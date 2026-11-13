# v2 cleanup

The historical runtime is preserved on master at 10d552cacb7660dc89e13e33a141f0cf84d8d2fe. Cleanup took place on v2/foundation without switching branches or pushing.

Only rotom-engine is active. rotom-dex and rotom-formats retain source but are excluded because their models, combat adapters, parsers, and validation import removed v1 types. Their historical manifests are not independently buildable. Redesign data boundaries before reactivating them; do not add compatibility shims.

Generated Champions data, differential JSON, team fixtures, pinned Showdown source, and reference scripts are preserved. Adopt behavioral fixtures after v2 defines state contracts.

The exporter still requires tools/showdown-export/move-capabilities.cjs to reproduce checked-in metadata. It is retained as export infrastructure only, not a v2 registry. Decouple it in a dedicated data change.

Legacy runtime, CLI, protocol, tracking reports/scripts/tests, and historical planning/research documentation were removed. Git retains their history. Uncommitted v2 foundation files were preserved and updated.
