# PR 13.6.5 — remaining Mega Stone foundation

## Scope

This batch registers the remaining 71 Mega Stones reachable in Champions Regulation M-C.
Together with the 10 stones registered in PR 13.6.4, all 81 reachable Mega Stone identities now
resolve to the typed `ItemBehavior::MegaStone` family.

The production registration is derived from each generated `ItemData::mega_stone` mapping rather
than a second hand-maintained item allowlist. The generated data remains the identity and
species/form-mapping source; `rotom-core` continues to own the typed behavior marker.

## Pinned Showdown basis

`data/generated/champions-reg-mc.json` is exported from the resolved Champions Dex at the pinned
Showdown commit. Every registered item has a non-empty `mega_stone` mapping and a legal item user.
The focused Dex test enumerates the expected 71 identities, verifies that their generated metadata
is present, and verifies both handler registration and behavior queries.

## Explicit limitation

This is foundation only. `ItemBehavior::MegaStone` has no transformation callback. Mega action
queue expansion and resolution remain explicitly unsupported and transactional, so selecting a
Mega action cannot partially alter form, typing, stats, ability, item state, or RNG. Coverage marks
all 81 stones as partial until canonical Mega Evolution resolution and differential fixtures exist.

No battle-state mutation path is added by this batch.
