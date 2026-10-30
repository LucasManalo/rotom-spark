# PR 13.6 first five held items

Source revision: Pokémon Showdown `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`,
format `gen9championsvgc2026regmc`, mod `champions`.

This first item batch deliberately covers five complete, stateless damage
modifiers that fit the existing typed move lifecycle. The resolved Champions
mod does not override them, so behavior is inherited from
`vendor/pokemon-showdown/data/items.ts`:

| Item | Pinned hook | Condition | Fixed-point modifier |
| --- | --- | --- | --- |
| Muscle Band | `onBasePower`, priority 16 | Physical move | `4505 / 4096` |
| Wise Glasses | `onBasePower`, priority 16 | Special move | `4505 / 4096` |
| Expert Belt | `onModifyDamage` | positive target type modifier | `4915 / 4096` |
| Charcoal | `onBasePower`, priority 15 | Fire move | `4915 / 4096` |
| Mystic Water | `onBasePower`, priority 15 | Water move | `4915 / 4096` |

Rotom maps stable generated Dex identities to native `ItemBehavior` handlers.
Handlers are holder-scoped, modify only ephemeral event relays, and never write
battle state directly. The move executor supplies whether the source item is
inactive under Magic Room or Klutz; suppressed handlers leave the relay
unchanged. All five execute through canonical `step()` tests with the same
Showdown fixed-point rounding used by existing ability modifiers.

Consumables, choice locking, defensive stat items, and stateful item effects
remain unsupported and are intentionally outside this batch.
