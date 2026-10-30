# PR 13.6.4 — third thirty-item batch

## Scope

This batch registers the final 20 unsupported non-Mega items reachable in Regulation M-C and
the first 10 Mega-stone identities:

- Air Balloon, Big Root, Binding Band, Eject Button, Focus Band, King's Rock, Leek, Life Orb,
  Light Clay, Mental Herb, Metronome, Normal Gem, Quick Claw, Red Card, Rocky Helmet, Scope
  Lens, Shed Shell, Shell Bell, White Herb, and Zoom Lens;
- Abomasite, Absolite, Absolite Z, Aerodactylite, Aggronite, Alakazite, Altarianite,
  Ampharosite, Audinite, and Banettite.

The Dex owns identities and species restrictions. Numeric modifiers and item reactions use the
typed event registry. Healing, recoil, stat restoration, and item consumption use `EffectState`
or `StateKernel`. Big Root and Light Clay integrate at the existing drain and side-condition
creation points because those mechanics need the resolved damage or condition duration.

## Pinned Showdown basis

The implementation follows the pinned `vendor/pokemon-showdown/data/items.ts` callbacks:

- Air Balloon `onDamagingHit`;
- Big Root `onTryHeal` with `[5324, 4096]`;
- Focus Band `onDamage` with a one-in-ten item RNG draw;
- Leek and Scope Lens `onModifyCritRatio`;
- Life Orb `onModifyDamage` and `onAfterMoveSecondarySelf`;
- Normal Gem primary-hit consumption and the modern 1.3x Gem modifier;
- Quick Claw `onFractionalPriority`;
- Rocky Helmet `onDamagingHit`;
- Shell Bell `onAfterMoveSecondarySelf`;
- White Herb negative-stage clearing;
- Zoom Lens `onSourceModifyAccuracy` with `[4915, 4096]`.

Light Clay and Binding Band are implemented by their corresponding screen/binding mechanics in
Showdown. The selected Mega stones use the pinned `megaStone` metadata; Mega transformation
resolution remains outside this item batch.

## Explicit partial coverage

- Binding Band awaits binding-residual state.
- Eject Button and Red Card await deterministic mid-turn forced-switch settlement.
- King's Rock awaits the flinch volatile lifecycle.
- Mental Herb awaits the full Attract/Taunt/Encore/Torment/Disable/Heal Block condition set.
- Metronome awaits per-holder consecutive-move state.
- Quick Claw implements item RNG and ordering but defers the Mycelium Might status-move exception.
- Shed Shell awaits mechanics-aware switch legality; `BattleState::legal_actions` deliberately
  has no Dex dependency today.
- White Herb handles event-driven stage drops but not a pre-existing negative stage at initial
  battle entry.
- The ten Mega stones are identity-mapped only. Mega actions remain explicitly unsupported and
  atomic; no transformation is silently approximated.

Coverage records these cases as partial.
