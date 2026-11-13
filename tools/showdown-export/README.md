# Champions static-data export

Run `node tools/showdown-export/export-champions.cjs --check` against a built
pinned Showdown checkout to verify `data/generated/champions-reg-mc.json`.
Omit `--check` to regenerate it intentionally. The move-capabilities helper is
retained because the exporter requires it for existing normalized metadata;
it does not establish v2 runtime abstractions. See docs/v2/MIGRATION.md.

