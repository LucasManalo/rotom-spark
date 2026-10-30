# Regulation M-C capability coverage and blocker graph

Pinned Showdown: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`
Format: `gen9championsvgc2026regmc` (mod `champions`)

> Generated from the reachable-mechanics manifest. Identity registration, executable capability coverage, and verification are separate claims.

## Identity coverage

| Kind | Reachable | Implemented | Partial | Unsupported |
| --- | ---: | ---: | ---: | ---: |
| ability | 215 | 120 | 95 | 0 |
| item | 166 | 47 | 119 | 0 |
| move | 510 | 223 | 0 | 287 |

## Executable capability coverage

| Total | Verified | Implemented | Partial | Unsupported |
| ---: | ---: | ---: | ---: | ---: |
| 62 | 5 | 17 | 20 | 20 |

`implemented` means the shared capability executes in its stated scope. `verified` additionally has qualifying pinned-reference evidence. Partial and unsupported capabilities continue to block dependent identities.

## Highest-leverage blockers

| Capability | Status | Blocked identities | Sole remaining blocker | Known gap |
| --- | --- | ---: | ---: | --- |
| `custom_callback` | unsupported | 181 | 112 | Resolved Showdown callbacks are recognized but not generically executable. |
| `special_behavior` | unsupported | 165 | 68 | The move requires semantics outside the currently supported declarative executor. |
| `form_change` | unsupported | 91 | 10 | Mega and other battle transformations are not kernel-mediated end-to-end. |
| `ability_transition` | unsupported | 81 | 0 | Ability changes are not represented by a shared lifecycle. |
| `mega_eligibility` | unsupported | 81 | 0 | Mega Stones are metadata-only. |
| `mega_timing` | unsupported | 81 | 0 | No canonical Mega action lifecycle exists. |
| `volatile` | partial | 52 | 3 | Only selected volatiles are executable; confusion, flinch, Taunt, Encore, Disable, Substitute, trapping, recharge, and invulnerability are incomplete. |
| `berry_lifecycle` | partial | 34 | 25 | Pre-existing status, damage/residual triggers, Substitute sensitivity, Unnerve, Klutz, Magic Room, Ripen, Cheek Pouch, Cud Chew, Harvest, and Symbiosis are not unified. |
| `field_or_side_condition` | partial | 14 | 1 | Hazards, delayed effects, screens with special removal, and several side conditions are incomplete. |
| `damage_event_lifecycle` | partial | 13 | 4 | Sturdy, Innards Out, Magic Guard, Liquid Ooze, and related effects need canonical event history. |
| `switch_control` | unsupported | 12 | 7 | Pivoting, phazing, item-driven switches, and ability-driven switches do not share an end-to-end lifecycle. |
| `contact_effects` | partial | 10 | 9 | Several abilities and items depend on unimplemented volatile/status/faint consequences. |
| `ability_damage_modifiers` | partial | 9 | 0 | Several modifier identities are registered but not interaction-complete. |
| `field_activation` | partial | 9 | 9 | Several setters exist, but entry and damage-triggered activation is not uniformly sequenced. |
| `stat_event_lifecycle` | partial | 9 | 8 | Intimidate and other reactive boosts/drops are not complete across interactions. |
| `item_lifecycle` | partial | 7 | 7 | Theft, exchange, restoration, suppression, and consumed-item history are incomplete. |
| `status_cure` | partial | 7 | 0 | Status berries, Heal Bell, Shed Skin, and related interactions are not unified. |
| `volatile_cure` | unsupported | 7 | 0 | Mental Herb-related volatiles do not use one canonical lifecycle. |
| `move_type_conversion` | partial | 5 | 5 | Aerilate-family and Champions conversion abilities are not interaction-complete. |
| `ability_lifecycle` | partial | 4 | 4 | Copying, replacement, suppression, and activation ordering are incomplete. |
| `forced_switch` | unsupported | 4 | 4 | Eject Button, Red Card, Emergency Exit, pivots, and phazing are not end-to-end. |
| `status_event_lifecycle` | partial | 3 | 3 | Synchronize, Leaf Guard, Purifying Salt, and related interactions are incomplete. |
| `calls_move` | unsupported | 2 | 0 | Copycat, Sleep Talk, and other callers are not executable. |
| `confusion` | unsupported | 2 | 0 | Confusion is not end-to-end. |
| `end_turn_effects` | partial | 2 | 1 | Speed Boost, Cud Chew, Wish, Perish Song, and similar effects are incomplete. |
| `flinch` | unsupported | 2 | 0 | Flinch is not represented by the canonical volatile lifecycle. |
| `switch_restriction` | unsupported | 2 | 2 | Shadow Tag, trapping volatiles, and Shed Shell are incomplete. |
| `accuracy_override` | partial | 1 | 1 | No Guard and move-specific accuracy overrides are not interaction-complete. |
| `action_order_modifier` | partial | 1 | 1 | Quick Claw-style bracket modification is not integrated with the canonical queue. |
| `choice_lock` | unsupported | 1 | 1 | Choice Scarf modifies Speed but does not enforce a complete choice lock. |
| `hazard_lifecycle` | unsupported | 1 | 1 | Spikes, Toxic Spikes, Sticky Web, Stealth Rock, and reactive hazard setters are incomplete. |
| `immunity_override` | partial | 1 | 1 | Exceptional immunity and suppression interactions remain incomplete. |
| `move_history` | partial | 1 | 1 | Several predicates and consecutive-use effects lack one authoritative history lifecycle. |
| `move_reflection` | unsupported | 1 | 1 | Magic Bounce is incomplete. |
| `pp_restore` | unsupported | 1 | 0 | Leppa Berry and move-driven PP restoration are incomplete. |
| `redirection_ability` | unsupported | 1 | 1 | Lightning Rod-style redirection is incomplete. |
| `residual_trapping` | unsupported | 1 | 1 | Binding moves and Binding Band are incomplete. |
| `stat_restore` | partial | 1 | 1 | White Herb and several stage-manipulation moves are incomplete. |
| `weather_suppression` | unsupported | 1 | 1 | Cloud Nine suppression is incomplete. |

## Dependency audit

- ability: 95 partial identities mapped; 0 currently unlocked.
- item: 119 partial identities mapped; 0 currently unlocked.
- move: 287 unsupported identities mapped; 0 currently unlocked.

Mega Stones are reported as a **metadata foundation**. They remain partial until `mega_eligibility`, `mega_timing`, `form_change`, and `ability_transition` are complete.

The complete per-identity graph, sub-capabilities, tests, fixtures, known gaps, and pinned source references are in `reg-mc-capability-dependencies.json`.

