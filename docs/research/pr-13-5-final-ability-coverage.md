# PR 13.5.12 final reachable ability coverage

The behavioral reference remains the unchanged `SHOWDOWN_VERSION` pin,
`b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`. The reachable set and behavior
definitions come from the resolved Champions Dex and
`vendor/pokemon-showdown/data/abilities.ts`, including the Champions override
for Spicy Spray in `data/mods/champions/abilities.ts`.

This batch assigns a typed native behavior family to the final 48 reachable
abilities that previously had no implementation registration. It deliberately
separates mechanics that can execute on the current state model from mechanics
that require the next item, form, type, volatile, or switch-tracking slice.

## Executable mechanics added

- Aftermath, Rough Skin, Innards Out, and Spicy Spray use ordered
  `AfterDamage` callbacks and kernel HP/status mutations.
- Flash Fire provides its suppressible Fire immunity. Its persistent Fire
  boost remains deferred because the Flash Fire volatile is not represented.
- Liquid Ooze reverses supported move drain through the shared drain path.
- Electric Surge, Grassy Surge, and Psychic Surge reuse a typed terrain-setter
  descriptor on ordinary switch-in.
- Sand Spit and Seed Sower use typed reactive field descriptors after confirmed
  damage.

Pinned constants and conditions were checked directly: Aftermath deals one
quarter of the attacker's maximum HP only after contact causes the holder to
faint; Rough Skin deals one eighth after contact; Innards Out deals the actual
HP removed by the finishing hit; Spicy Spray burns the attacker after any
damaging hit; terrain and reactive weather use the standard five-turn field
duration; and Liquid Ooze converts drain healing into equal HP loss.

## Explicitly deferred families

The remaining registrations are intentionally `partial`, not empty claims:

- Item lifecycle: Cheek Pouch, Cud Chew, Gluttony, Harvest, Pickup, Ripen,
  Sticky Hold, Symbiosis, and Unnerve. These are queued for the item milestone.
- Form/identity state: Forecast, Hunger Switch, Illusion, Imposter, Stance
  Change, and Zero to Hero.
- Mutable typing: Libero, Mimicry, and Protean.
- Volatile mechanics: Aroma Veil, Cursed Body, Cute Charm, Electromorphosis,
  Steadfast, Stench, and Tangled Feet.
- Switching and trapping: Emergency Exit, Shadow Tag, and Suction Cups.
- Missing state provenance: Heavy Metal, Light Metal, Rivalry, and Stakeout.
- Specialized sequencing: Disguise, Magic Bounce, Parental Bond, Poison Heal,
  and Toxic Debris.

The executable abilities also remain partial where adjacent mechanics are not
yet complete: Damp suppression of Aftermath, immediate attacker faint
settlement from contact retaliation, Flash Fire's persistent boost, initial
lead and forced-replacement terrain activation, Liquid Ooze interactions with
Leech Seed and Strength Sap, status immunity for Spicy Spray, and Toxic Debris
layering. No differential-equivalence claim is made for this batch.
