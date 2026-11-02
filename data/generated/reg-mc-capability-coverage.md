# Regulation M-C capability coverage and blocker graph

Pinned Showdown: `b79ce06d06b8f0dc8de8bb348bc9cab5d931553f`
Format: `gen9championsvgc2026regmc` (mod `champions`)

> Generated from the reachable-mechanics manifest. Identity registration, executable capability coverage, and verification are separate claims.

## Identity coverage

| Kind | Reachable | Implemented | Partial | Unsupported |
| --- | ---: | ---: | ---: | ---: |
| ability | 215 | 123 | 92 | 0 |
| item | 166 | 141 | 25 | 0 |
| move | 510 | 241 | 0 | 269 |

## Executable capability coverage

| Total | Verified | Implemented | Partial | Unsupported |
| ---: | ---: | ---: | ---: | ---: |
| 71 | 12 | 21 | 21 | 17 |

`implemented` means the shared capability executes in its stated scope. `verified` additionally has qualifying pinned-reference evidence. Partial and unsupported capabilities continue to block dependent identities.

## Highest-leverage blockers

| Capability | Status | Blocked identities | Sole remaining blocker | Known gap |
| --- | --- | ---: | ---: | --- |
| `custom_callback` | unsupported | 178 | 109 | Resolved Showdown callbacks are recognized but not generically executable. |
| `special_behavior` | unsupported | 148 | 47 | The move requires semantics outside the currently supported declarative executor. |
| `volatile` | partial | 63 | 3 | Flinch and selected existing conditions use the shared lifecycle; confusion, Taunt, Encore, Disable, Substitute, trapping, recharge, and invulnerability remain incomplete. |
| `substitute_damage_routing` | unsupported | 18 | 18 | Resistance Berries cannot be promoted until damage and item activation distinguish Substitute damage from direct HP damage. |
| `field_or_side_condition` | partial | 14 | 1 | Hazards, delayed effects, screens with special removal, and several side conditions are incomplete. |
| `damage_event_lifecycle` | partial | 13 | 4 | Sturdy, Innards Out, Magic Guard, Liquid Ooze, and related effects need canonical event history. |
| `switch_control` | unsupported | 12 | 7 | Pivoting, phazing, item-driven switches, and ability-driven switches do not share an end-to-end lifecycle. |
| `form_change` | unsupported | 10 | 10 | Mega Evolution uses its verified dedicated capability; Stance Change, Hunger Switch, Zero to Hero, Transform, and other forms remain PR 18 scope. |
| `ability_damage_modifiers` | partial | 9 | 0 | Several modifier identities are registered but not interaction-complete. |
| `contact_effects` | partial | 9 | 9 | Several abilities and items depend on unimplemented volatile/status/faint consequences. |
| `field_activation` | partial | 9 | 9 | Several setters exist, but entry and damage-triggered activation is not uniformly sequenced. |
| `stat_event_lifecycle` | partial | 8 | 8 | Intimidate and other reactive boosts/drops are not complete across interactions. |
| `item_lifecycle_consumers` | partial | 7 | 7 | Pickup, Magician, Pickpocket, Symbiosis, and future removal moves still require identity-specific timing coverage. |
| `berry_effect_coverage` | partial | 5 | 4 | Confusion-curing and pinch-stat berries remain incomplete, so berry-dependent abilities are not interaction-complete for every reachable berry. |
| `move_type_conversion` | partial | 5 | 5 | Aerilate-family and Champions conversion abilities are not interaction-complete. |
| `volatile_cure` | partial | 5 | 0 | Mental Herb and flinch prevention are implemented, but confusion cures and other ability-driven volatile rules remain incomplete. |
| `ability_lifecycle_restrictions` | partial | 4 | 4 | Trace/Receiver filters, immutable ability flags, Ability Shield, and Neutralizing Gas remain identity-specific blockers. |
| `status_event_lifecycle` | partial | 3 | 3 | Synchronize, Leaf Guard, Purifying Salt, and related interactions are incomplete. |
| `calls_move` | unsupported | 2 | 0 | Copycat, Sleep Talk, and other callers are not executable. |
| `confusion` | unsupported | 2 | 0 | Confusion is not end-to-end. |
| `end_turn_effects` | partial | 2 | 1 | Speed Boost, Cud Chew, Wish, Perish Song, and similar effects are incomplete. |
| `status_cure` | partial | 2 | 0 | Status berries, Heal Bell, Shed Skin, and related interactions are not unified. |
| `switch_restriction` | unsupported | 2 | 2 | Shadow Tag, trapping volatiles, and Shed Shell are incomplete. |
| `accuracy_override` | partial | 1 | 1 | No Guard and move-specific accuracy overrides are not interaction-complete. |
| `action_order_modifier` | partial | 1 | 1 | Quick Claw-style bracket modification is not integrated with the canonical queue. |
| `choice_lock` | unsupported | 1 | 1 | Choice Scarf modifies Speed but does not enforce a complete choice lock. |
| `emergency_exit_trigger` | unsupported | 1 | 1 | Emergency Exit has not yet been connected to the shared forced-switch lifecycle. |
| `hazard_lifecycle` | unsupported | 1 | 1 | Spikes, Toxic Spikes, Sticky Web, Stealth Rock, and reactive hazard setters are incomplete. |
| `immunity_override` | partial | 1 | 1 | Exceptional immunity and suppression interactions remain incomplete. |
| `move_reflection` | unsupported | 1 | 1 | Magic Bounce is incomplete. |
| `pinch_berry_thresholds` | unsupported | 1 | 1 | Stat-boosting pinch Berries and Gluttony threshold changes are not executable. |
| `redirection_ability` | unsupported | 1 | 1 | Lightning Rod-style redirection is incomplete. |
| `residual_trapping` | unsupported | 1 | 1 | Binding moves and Binding Band are incomplete. |
| `stat_restore` | partial | 1 | 1 | White Herb and several stage-manipulation moves are incomplete. |
| `weather_suppression` | unsupported | 1 | 1 | Cloud Nine suppression is incomplete. |

## Recently unlocked identities

- `berry_lifecycle`: `item:aspearberry`, `item:cheriberry`, `item:chestoberry`, `item:leppaberry`, `item:oranberry`, `item:pechaberry`, `item:rawstberry`, `item:sitrusberry`
- `flinch`: `ability:innerfocus`, `ability:steadfast`, `item:kingsrock`
- `forced_switch`: `ability:suctioncups`, `item:ejectbutton`, `item:redcard`
- `mega_eligibility`: `item:abomasite`, `item:absolite`, `item:absolitez`, `item:aerodactylite`, `item:aggronite`, `item:alakazite`, `item:altarianite`, `item:ampharosite`, `item:audinite`, `item:banettite`, `item:barbaracite`, `item:baxcalibrite`, `item:beedrillite`, `item:blastoisinite`, `item:blazikenite`, `item:cameruptite`, `item:chandelurite`, `item:charizarditex`, `item:charizarditey`, `item:chesnaughtite`, `item:chimechite`, `item:clefablite`, `item:crabominite`, `item:delphoxite`, `item:dragalgite`, `item:dragoninite`, `item:drampanite`, `item:eelektrossite`, `item:emboarite`, `item:excadrite`, `item:falinksite`, `item:feraligite`, `item:floettite`, `item:froslassite`, `item:galladite`, `item:garchompite`, `item:garchompitez`, `item:gardevoirite`, `item:gengarite`, `item:glalitite`, `item:glimmoranite`, `item:golisopite`, `item:golurkite`, `item:greninjite`, `item:gyaradosite`, `item:hawluchanite`, `item:heracronite`, `item:houndoominite`, `item:kangaskhanite`, `item:lopunnite`, `item:lucarionite`, `item:lucarionitez`, `item:malamarite`, `item:manectite`, `item:mawilite`, `item:medichamite`, `item:meganiumite`, `item:meowsticite`, `item:metagrossite`, `item:pidgeotite`, `item:pinsirite`, `item:pyroarite`, `item:raichunitex`, `item:raichunitey`, `item:sablenite`, `item:salamencite`, `item:sceptilite`, `item:scizorite`, `item:scolipite`, `item:scovillainite`, `item:scraftinite`, `item:sharpedonite`, `item:skarmorite`, `item:slowbronite`, `item:staraptite`, `item:starminite`, `item:steelixite`, `item:swampertite`, `item:tyranitarite`, `item:venusaurite`, `item:victreebelite`
- `mental_herb_cure`: `item:mentalherb`
- `move_history`: `item:metronome`

## Dependency audit

- ability: 92 partial identities mapped; 0 currently unlocked.
- item: 25 partial identities mapped; 0 currently unlocked.
- move: 269 unsupported identities mapped; 0 currently unlocked.

Mega Stones are reported as a **metadata foundation**. They remain partial until `mega_eligibility`, `mega_timing`, `form_change`, and `ability_transition` are complete.

The complete per-identity graph, sub-capabilities, tests, fixtures, known gaps, and pinned source references are in `reg-mc-capability-dependencies.json`.

