# Regulation M-C mechanics coverage

Pinned Showdown: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`
Format: `gen9championsvgc2026regmc` (mod `champions`)

This report is generated from `reg-mc-mechanics-manifest.json`. Presence means reachable, not implemented.
`Differential` counts move-level pinned Showdown calculation evidence. It does not mean that a complete Showdown battle transition was executed and compared.

| Kind | Reachable | Implemented | Partial | Unsupported | Differential | Unit tested | Not tested | Interaction verified |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ability | 215 | 0 | 0 | 215 | 0 | 0 | 215 | 0 |
| condition | 96 | 0 | 0 | 96 | 0 | 0 | 96 | 0 |
| form | 153 | 0 | 0 | 153 | 0 | 0 | 153 | 0 |
| item | 166 | 0 | 0 | 166 | 0 | 0 | 166 | 0 |
| move | 510 | 37 | 0 | 473 | 5 | 1 | 504 | 0 |
| species | 293 | 0 | 0 | 293 | 0 | 0 | 293 | 0 |

## Next unsupported reachable mechanics

- ability: `adaptability` — Adaptability (direct_legal_set)
- condition: `allyswitch` — Ally Switch (battle_generated)
- form: `abomasnowmega` — Abomasnow-Mega (battle_generated)
- item: `abomasite` — Abomasite (direct_legal_set)
- move: `acidarmor` — Acid Armor (direct_legal_set)
- species: `abomasnow` — Abomasnow (direct_legal_set)

## Unresolved reachability

- `ability_dynamic_effect/magicbounce`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/synchronize`: callback may generate a condition or move not exposed as structured ability metadata
- `move_dynamic_effect/direclaw`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/futuresight`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/psychup`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/triattack`: callback may generate a condition or move not exposed as structured move metadata

