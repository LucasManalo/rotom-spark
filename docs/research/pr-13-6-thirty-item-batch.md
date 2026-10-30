# PR 13.6 thirty-item batch

Source revision: Pokémon Showdown `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`,
format `gen9championsvgc2026regmc`, mod `champions`.

This batch adds 30 directly reachable Regulation M-C held items. Plates were
explicitly excluded because the generated reachability manifest does not place
them in the format.

## Type-power items

The following 16 items inherit `onBasePowerPriority: 15` callbacks from
`vendor/pokemon-showdown/data/items.ts` and apply Showdown's fixed-point
`4915 / 4096` modifier only when the move type matches:

Black Belt, Black Glasses, Dragon Fang, Fairy Feather, Hard Stone, Magnet,
Metal Coat, Miracle Seed, Never-Melt Ice, Poison Barb, Sharp Beak, Silk Scarf,
Silver Powder, Soft Sand, Spell Tag, and Twisted Spoon.

Rotom represents the shared rule as `ItemBehavior::TypePower(PokemonType)`.
The item identity remains Dex-owned; the native callback is selected once when
immutable mechanics are assembled. Magic Room and Klutz suppress the source
item through the same typed event context used by the first item batch.

## Super-effective resistance Berries

The following 14 items inherit `onSourceModifyDamage` callbacks and consume to
apply a second fixed-point `1 / 2` modifier when the matching move is
super-effective:

Babiri, Charti, Chople, Coba, Colbur, Haban, Kasib, Kebia, Occa, Passho,
Payapa, Rindo, Roseli, and Shuca Berry.

The handler verifies the target holder and matching move type, requires a
positive type modifier, and removes the Berry through `StateKernel::set_item`
before changing the damage relay. Magic Room and Klutz prevent activation;
an opposing active Unnerve holder prevents eating; Ripen applies the pinned
priority-minus-one second halving, producing one-quarter damage. Public
transition atomicity protects consumption if a later callback fails.

Cheek Pouch, Cud Chew, Harvest, and Symbiosis still require their own ability
milestones and persistent tracking where applicable. A qualifying Berry
activation involving one of those deferred interactions returns
`UnsupportedMechanic` before consumption; it never silently drops the
cross-effect.

Pinned sources:

- item definitions: `vendor/pokemon-showdown/data/items.ts`, each named item;
- Ripen: `vendor/pokemon-showdown/data/abilities.ts`, `ripen`;
- Unnerve: `vendor/pokemon-showdown/data/abilities.ts`, `unnerve`;
- item consumption gate: `vendor/pokemon-showdown/sim/pokemon.ts`, `eatItem`.

Substitute interception is outside the currently supported move/state subset.
If Substitute is implemented later, Berry activation must remain after the
substitute-hit check shown in the pinned item callbacks.
