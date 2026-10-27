# Regulation M-C mechanics coverage

Pinned Showdown: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`
Format: `gen9championsvgc2026regmc` (mod `champions`)

This report is generated from `reg-mc-mechanics-manifest.json`. Presence means reachable, not implemented.
`Differential` counts isolated pinned Showdown mechanic evidence. It does not mean that a complete Showdown battle transition or cross-mechanic interaction was executed and compared.

| Kind | Reachable | Implemented | Partial | Unsupported | Differential | Unit tested | Not tested | Interaction verified |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ability | 215 | 118 | 19 | 78 | 51 | 57 | 107 | 72 |
| condition | 96 | 0 | 0 | 96 | 0 | 0 | 96 | 0 |
| form | 153 | 0 | 0 | 153 | 0 | 0 | 153 | 0 |
| item | 166 | 0 | 0 | 166 | 0 | 0 | 166 | 0 |
| move | 510 | 223 | 0 | 287 | 23 | 200 | 287 | 15 |
| species | 293 | 0 | 0 | 293 | 0 | 0 | 293 | 0 |

## Next unsupported reachable mechanics

- ability: `aftermath` — Aftermath (direct_legal_set)
- condition: `allyswitch` — Ally Switch (battle_generated)
- form: `abomasnowmega` — Abomasnow-Mega (battle_generated)
- item: `abomasite` — Abomasite (direct_legal_set)
- move: `acrobatics` — Acrobatics (direct_legal_set)
- species: `abomasnow` — Abomasnow (direct_legal_set)

## Unresolved reachability

- `ability_dynamic_effect/magicbounce`: callback may generate a condition or move not exposed as structured ability metadata
- `ability_dynamic_effect/synchronize`: callback may generate a condition or move not exposed as structured ability metadata
- `move_dynamic_effect/direclaw`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/futuresight`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/psychup`: callback may generate a condition or move not exposed as structured move metadata
- `move_dynamic_effect/triattack`: callback may generate a condition or move not exposed as structured move metadata

