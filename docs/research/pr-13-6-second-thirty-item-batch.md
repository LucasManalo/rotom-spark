# PR 13.6.3 — second thirty-item batch

## Scope

This batch adds native identities and behavior families for 30 reachable Regulation M-C held
items:

- resistance Berries: Chilan, Tanga, Wacan, Yache;
- status/resource Berries: Aspear, Cheri, Chesto, Pecha, Rawst, Persim, Lum, Leppa, Oran,
  Sitrus;
- duration items: Damp Rock, Heat Rock, Icy Rock, Smooth Rock, Terrain Extender;
- terrain Seeds: Electric, Grassy, Misty, Psychic;
- passive/modifier items: Leftovers, Bright Powder, Wide Lens, Choice Scarf, Iron Ball,
  Light Ball, Focus Sash.

The Dex owns item/species identities. `rotom-core` owns typed callbacks and the narrow
field-duration/terrain activation integration. Item consumption, PP restoration, healing,
stat-stage changes, and Focus Sash removal all use the mutation kernel.

## Pinned Showdown basis

Behavior was derived from the pinned `vendor/pokemon-showdown/data/items.ts` callbacks:

- `onSourceModifyDamage` for resistance Berries;
- `onUpdate` / `onEat` for status, PP, and healing Berries;
- `onModifySpe`, `onModifyAtk`, `onModifySpA`, and accuracy callbacks for numeric modifiers;
- `onStart` / `onTerrainChange` and `boosts` for terrain Seeds;
- `onDamage` for Focus Sash and `onResidual` for Leftovers.

Weather and terrain duration items are declared by the corresponding field-setting logic in
Showdown; Rotom applies their eight-turn duration when the condition is created. The pinned
commit and format remain those recorded in `SHOWDOWN_VERSION`.

## Coverage limits

- Berry callbacks support move damage/status triggers, Unnerve, Ripen, Magic Room, and Klutz
  where the current typed event supplies the holder context. Cheek Pouch, Cud Chew, Harvest,
  and Symbiosis still fail explicitly before supported Berry consumption.
- Persim Berry is identity-mapped but remains partial because confusion state is not yet part
  of the supported core condition model. Lum Berry therefore covers major statuses but not
  confusion.
- Status and healing Berries remain partial until every Showdown `Update` trigger (including
  all residual/self-damage and pre-existing-status entry paths) is represented.
- Choice Scarf's Speed modifier is implemented; Choice lock is deferred, so its coverage is
  partial.
- Substitute-sensitive Berry timing and item-removal abilities remain deferred.

These limits are recorded as partial coverage rather than silent approximations.
