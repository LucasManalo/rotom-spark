# Regulation M-C mechanics coverage

Pinned Showdown: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`
Format: `gen9championsvgc2026regmc` (mod `champions`)

This report is generated from `reg-mc-mechanics-manifest.json`. Presence means reachable, not implemented.

| Kind | Reachable | Implemented | Partial | Unsupported | Differential | Unit tested | Not tested |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ability | 215 | 0 | 0 | 215 | 0 | 0 | 215 |
| condition | 77 | 0 | 0 | 77 | 0 | 0 | 77 |
| form | 153 | 0 | 0 | 153 | 0 | 0 | 153 |
| item | 166 | 0 | 0 | 166 | 0 | 0 | 166 |
| move | 510 | 7 | 0 | 503 | 4 | 0 | 506 |
| species | 293 | 0 | 0 | 293 | 0 | 0 | 293 |

## Next unsupported reachable mechanics

- ability: `adaptability` — Adaptability (direct_legal_set)
- condition: `aquaring` — Aqua Ring (battle_generated)
- form: `abomasnowmega` — Abomasnow-Mega (battle_generated)
- item: `abomasite` — Abomasite (direct_legal_set)
- move: `accelerock` — Accelerock (direct_legal_set)
- species: `abomasnow` — Abomasnow (direct_legal_set)

## Unresolved reachability

- `ability_dynamic_effect/cursedbody`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/cutecharm`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/drizzle`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/drought`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/effectspore`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/electricsurge`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/electromorphosis`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/flamebody`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/flashfire`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/grassysurge`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/magicbounce`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/poisonpoint`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/poisontouch`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/psychicsurge`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/sandspit`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/sandstream`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/seedsower`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/snowwarning`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/static`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/synchronize`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/toxicdebris`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/unburden`: callback may generate a condition or move not exposed as structured ability metadata
- `item_dynamic_effect/choicescarf`: callback may generate a condition or move not exposed as structured item metadata
- `item_dynamic_effect/metronome`: callback may generate a condition or move not exposed as structured item metadata
- `item_dynamic_effect/normalgem`: callback may generate a condition or move not exposed as structured item metadata
- `move_dynamic_effect/alluringvoice`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/allyswitch`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/banefulbunker`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/beakblast`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/block`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/bounce`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/burningjealousy`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/ceaselessedge`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/chillyreception`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/copycat`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/counter`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/curse`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/detect`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/dig`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/direclaw`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/dive`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/electroshot`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/endure`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/fling`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/fly`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/focuspunch`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/futuresight`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/jawlock`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/kingsshield`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/lockon`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/meanlook`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/meteorbeam`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/mirrorcoat`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/perishsong`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/phantomforce`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/protect`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/psychup`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/quickguard`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/rest`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/skyattack`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/sleeptalk`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/solarbeam`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/solarblade`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/spikyshield`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/spiritshackle`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/stoneaxe`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/throatchop`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/toxicspikes`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/triattack`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/wideguard`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/yawn`: callback may generate a condition or move not exposed as structured move metadata

