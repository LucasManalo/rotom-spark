# PR 15.1 — shared Berry lifecycle

Pinned Showdown commit: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`  
Format: `gen9championsvgc2026regmc` (mod `champions`)

## Pinned behavior

- `sim/pokemon.ts:Pokemon.eatItem` runs `UseItem`/`TryEatItem`, the item's
  `Eat` callback, ability `EatItem` callbacks, records `lastItem`, removes the
  held item, and finally runs `AfterUseItem`.
- `data/abilities.ts:cheekpouch.onEatItem` heals one third of base maximum HP.
- `data/abilities.ts:cudchew.onEatItem/onResidual` records the eaten Berry and
  repeats its `Eat` and `EatItem` callbacks on the second residual boundary.
- `data/abilities.ts:harvest.onResidual` restores the last consumed Berry when
  the holder is itemless; sun guarantees activation and other weather uses a
  one-half ability RNG check.
- `data/abilities.ts:symbiosis.onAllyAfterUseItem` transfers the ally's
  removable held item after consumption.
- `data/abilities.ts:ripen` doubles healing, PP restoration, and resistance
  reduction; `data/abilities.ts:unnerve` rejects foe Berry consumption.
- `data/items.ts` Berry `onUpdate`, `onEat`, and resistance callbacks define
  the per-item trigger and effect. Resistance callbacks explicitly skip a hit
  intercepted by Substitute.

The generated fixture at `data/fixtures/differential/berry-lifecycle.json`
executes the pinned `Pokemon.eatItem`, `EatItem`, `AfterUseItem`, and
`Residual` paths for Sitrus Berry with Cheek Pouch, Cud Chew, Harvest, and
Symbiosis.

## Rotom implementation

Berry effects remain typed `ItemBehavior` handlers. Shared lifecycle work is
kernel-mediated:

1. Check Magic Room/Klutz and opposing Unnerve before consumption.
2. Atomically remove the matching Berry and record `last_consumed_berry`.
3. Schedule Cud Chew state when applicable.
4. Apply the item's typed effect, including Ripen's modifier.
5. Apply Cheek Pouch and then Symbiosis follow-ups.
6. At residual order, resolve update-triggered healing after residual damage,
   delayed Cud Chew reuse, and Harvest restoration.

The same update path handles a status already present when a holder enters a
decision turn and HP thresholds crossed by residual damage. Authoritative
snapshots include consumption and delayed-reuse state so serialization and
replay do not reconstruct it from diagnostics.

## Coverage decisions

Promoted to implemented: Aspear, Cheri, Chesto, Pecha, Rawst, Leppa, Oran,
and Sitrus Berry. Their supported effects, update timing, suppression,
consumption history, Ripen modifier, and reachable follow-up hooks execute
through the shared lifecycle.

Still partial:

- Persim Berry, and Lum Berry's confusion branch: blocked by the shared
  confusion volatile lifecycle planned for PR 15.3.
- The 18 resistance Berries: their damage reduction works, but the
  Substitute-interception branch cannot be promoted before Substitute damage
  routing exists.
- Cheek Pouch, Cud Chew, Harvest, Ripen, and Symbiosis remain partial ability
  identities where later unsupported Berry effects or item-transfer rules are
  still reachable. Unnerve and Klutz behavior used by this lifecycle is tested,
  but their complete ability identities retain their non-Berry blockers.
- Gluttony does not affect the currently reachable implemented Berry families;
  its pinch-Berry threshold behavior remains deferred.

No Berry handler writes battle state directly; item, HP, PP, status, and
history changes pass through `StateKernel`.
