'use strict';

const CAPABILITIES = {
	standard_damage: verified('Canonical damage calculation and application', ['damage formula', 'kernel HP application'], 'No known gap in the ordinary damage path.', ['showdown:sim/battle-actions.ts:BattleActions.getDamage']),
	selected_single_target: verified('Selected single-target resolution', ['typed target validation', 'adjacency'], 'No known gap for ordinary selected targets.', ['showdown:sim/battle-actions.ts:BattleActions.getTarget']),
	accuracy: verified('Accuracy and evasion resolution', ['accuracy modifiers', 'deterministic RNG draw'], 'Exceptional always-hit and move-specific rules remain separate dependencies.', ['showdown:sim/battle-actions.ts:BattleActions.tryMoveHit']),
	fixed_damage: implemented('Fixed and level-based damage', ['fixed damage', 'level damage'], 'Interaction coverage is not differential-complete.', ['showdown:sim/battle-actions.ts:BattleActions.getDamage']),
	non_damage_primary: implemented('Ordinary non-damaging primary effects', ['primary operation dispatch'], 'Only declarative operations are implemented; special callbacks remain separate.', ['showdown:sim/battle-actions.ts:BattleActions.moveHit']),
	advanced_targeting: implemented('Non-selected and multi-target target resolution', ['target sets', 'ally/self targeting'], 'Several move-specific target rewrites remain separate.', ['showdown:sim/battle-actions.ts:BattleActions.getMoveTargets']),
	multi_hit: implemented('Declarative multi-hit damage', ['hit-count selection', 'per-hit damage'], 'Population Bomb-style accuracy and move-specific per-hit behavior remain separate.', ['showdown:sim/battle-actions.ts:BattleActions.hitStepMoveHitLoop']),
	secondary_effect: implemented('Declarative chance secondaries', ['chance roll', 'status and stage secondaries'], 'Volatile and custom secondaries depend on their own capabilities.', ['showdown:sim/battle-actions.ts:BattleActions.secondaries']),
	self_effect: implemented('Declarative self effects', ['self stat changes'], 'Custom self effects remain separate.', ['showdown:sim/battle-actions.ts:BattleActions.moveHit']),
	drain: implemented('Declarative drain', ['damage-based healing'], 'Liquid Ooze and exceptional drain rules remain separate.', ['showdown:sim/battle-actions.ts:BattleActions.moveHit']),
	recoil: implemented('Declarative recoil', ['damage-based recoil', 'fixed maximum-HP recoil'], 'Crash damage and exceptional recoil rules remain separate.', ['showdown:sim/battle-actions.ts:BattleActions.moveHit']),
	major_status: implemented('Major-status application', ['eligibility', 'status state'], 'Status cure, reflection, and move-specific status rules remain separate.', ['showdown:sim/pokemon.ts:Pokemon.setStatus']),
	volatile: partial('Shared volatile lifecycle', ['activation', 'duration', 'expiry', 'switch cleanup', 'prevention and cure'], 'Only selected volatiles are executable; confusion, flinch, Taunt, Encore, Disable, Substitute, trapping, recharge, and invulnerability are incomplete.', ['showdown:sim/pokemon.ts:Pokemon.addVolatile', 'showdown:sim/pokemon.ts:Pokemon.removeVolatile']),
	stat_stage: implemented('Stat-stage application', ['bounds', 'multi-stat changes'], 'Reflection, prevention, restoration, and event timing remain separate.', ['showdown:sim/battle.ts:Battle.boost']),
	healing: implemented('Ordinary direct healing', ['fractional healing', 'full-HP failure'], 'Weather-dependent and delayed healing remain separate.', ['showdown:sim/battle.ts:Battle.heal']),
	field_or_side_condition: partial('Field, side, and slot condition lifecycle', ['weather', 'terrain', 'side conditions', 'durations'], 'Hazards, delayed effects, screens with special removal, and several side conditions are incomplete.', ['showdown:sim/field.ts:Field.addPseudoWeather', 'showdown:sim/side.ts:Side.addSideCondition']),
	switch_control: unsupported('Mid-turn switch control', ['switch request', 'queue preservation', 'replacement choice', 'entry/exit sequencing'], 'Pivoting, phazing, item-driven switches, and ability-driven switches do not share an end-to-end lifecycle.', ['showdown:sim/battle-actions.ts:BattleActions.runSwitch', 'showdown:sim/battle.ts:Battle.checkFainted']),
	calls_move: unsupported('Move calling and copying', ['source selection', 'legality filters', 'history lookup'], 'Copycat, Sleep Talk, and other callers are not executable.', ['showdown:sim/battle-actions.ts:BattleActions.useMove']),
	custom_callback: unsupported('Move-specific callback orchestration', ['callback predicates', 'hook ordering', 'failure semantics'], 'Resolved Showdown callbacks are recognized but not generically executable.', ['showdown:sim/battle-actions.ts:BattleActions.runMove']),
	special_behavior: unsupported('Exceptional move behavior', ['intrinsic flags', 'special properties', 'history predicates'], 'The move requires semantics outside the currently supported declarative executor.', ['showdown:data/moves.ts:Moves']),
	target_set: implemented('Target-set construction', ['spread target collection'], 'Move-specific exclusions remain separate.', ['showdown:sim/battle-actions.ts:BattleActions.getMoveTargets']),
	spread_damage: implemented('Spread-damage modifier', ['target-count modifier'], 'No known generic gap.', ['showdown:sim/battle-actions.ts:BattleActions.getDamage']),
	protection: verified('Protect interception', ['protection check', 'repeat-use RNG'], 'Protect-family variants and contact retaliation remain separate.', ['showdown:data/moves.ts:Moves.protect']),
	redirection: verified('Follow Me and Rage Powder redirection', ['priority', 'eligibility'], 'Ability redirection and exceptional bypass rules remain separate.', ['showdown:sim/battle-actions.ts:BattleActions.getRedirectTarget']),
	retargeting: implemented('Fainted-target retargeting', ['replacement target selection'], 'Implemented and unit tested, but no isolated pinned differential fixture is recorded.', ['showdown:sim/battle-actions.ts:BattleActions.getRandomTarget']),
	per_target_hit: implemented('Per-target hit resolution', ['independent hit results'], 'Exceptional per-target callbacks remain separate.', ['showdown:sim/battle-actions.ts:BattleActions.trySpreadMoveHit']),
	conditional_power: implemented('Declarative conditional base power', ['Facade predicate'], 'Other variable-power predicates remain separate.', ['showdown:sim/battle-actions.ts:BattleActions.getDamage']),
	effective_stat_override: implemented('Effective offensive/defensive stat override', ['Psyshock-style defense selection'], 'Move-specific ignore rules remain separate.', ['showdown:sim/battle-actions.ts:BattleActions.getDamage']),

	berry_lifecycle: partial('Shared berry activation and consumption', ['trigger timing', 'eligibility', 'suppression', 'consumption', 'follow-up hooks'], 'Pre-existing status, damage/residual triggers, Substitute sensitivity, Unnerve, Klutz, Magic Room, Ripen, Cheek Pouch, Cud Chew, Harvest, and Symbiosis are not unified.', ['showdown:data/items.ts:Items.sitrusberry', 'showdown:sim/pokemon.ts:Pokemon.eatItem']),
	choice_lock: unsupported('Choice-item move locking', ['lock acquisition', 'legal-action filtering', 'switch cleanup'], 'Choice Scarf modifies Speed but does not enforce a complete choice lock.', ['showdown:data/items.ts:Items.choicescarf', 'showdown:data/conditions.ts:Conditions.choicelock']),
	forced_switch: unsupported('Forced-switch lifecycle', ['request', 'queue mutation', 'replacement', 'entry/exit hooks'], 'Eject Button, Red Card, Emergency Exit, pivots, and phazing are not end-to-end.', ['showdown:data/items.ts:Items.ejectbutton', 'showdown:data/items.ts:Items.redcard']),
	mega_eligibility: unsupported('Mega Evolution eligibility', ['species/stone compatibility', 'once-per-battle rule', 'request validation'], 'Mega Stones are metadata-only.', ['showdown:sim/battle-actions.ts:BattleActions.canMegaEvo']),
	mega_timing: unsupported('Mega Evolution action timing', ['choice validation', 'queue timing', 'order recalculation'], 'No canonical Mega action lifecycle exists.', ['showdown:sim/battle-actions.ts:BattleActions.runMegaEvo']),
	form_change: unsupported('Battle form transformation', ['species/form', 'typing', 'stats', 'HP preservation', 'persistence'], 'Mega and other battle transformations are not kernel-mediated end-to-end.', ['showdown:sim/pokemon.ts:Pokemon.formeChange']),
	ability_transition: unsupported('Ability replacement and activation transition', ['deactivation', 'replacement', 'start hooks', 'suppression'], 'Ability changes are not represented by a shared lifecycle.', ['showdown:sim/pokemon.ts:Pokemon.setAbility']),
	item_lifecycle: partial('Held-item mutation lifecycle', ['remove', 'consume', 'steal', 'exchange', 'restore', 'suppress'], 'Theft, exchange, restoration, suppression, and consumed-item history are incomplete.', ['showdown:sim/pokemon.ts:Pokemon.takeItem', 'showdown:sim/pokemon.ts:Pokemon.setItem']),
	ability_lifecycle: partial('Ability mutation lifecycle', ['copy', 'replace', 'suppress', 'ignore', 'entry', 'exit'], 'Copying, replacement, suppression, and activation ordering are incomplete.', ['showdown:sim/pokemon.ts:Pokemon.setAbility', 'showdown:sim/pokemon.ts:Pokemon.ignoringAbility']),
	volatile_cure: unsupported('Volatile prevention and cure', ['Mental Herb cure set', 'prevention', 'event timing'], 'Mental Herb-related volatiles do not use one canonical lifecycle.', ['showdown:data/items.ts:Items.mentalherb']),
	confusion: unsupported('Confusion lifecycle', ['application', 'duration', 'before-move check', 'self-hit', 'cure'], 'Confusion is not end-to-end.', ['showdown:data/conditions.ts:Conditions.confusion']),
	flinch: unsupported('Flinch lifecycle', ['application', 'before-move interruption', 'turn cleanup', 'immunity'], 'Flinch is not represented by the canonical volatile lifecycle.', ['showdown:data/conditions.ts:Conditions.flinch']),
	residual_trapping: unsupported('Partial-trapping lifecycle', ['duration', 'residual damage', 'switch restriction', 'Binding Band modifier'], 'Binding moves and Binding Band are incomplete.', ['showdown:data/conditions.ts:Conditions.partiallytrapped', 'showdown:data/items.ts:Items.bindingband']),
	move_history: partial('Canonical move and action history', ['last move', 'last successful move', 'last damage source', 'consecutive use', 'turn reset'], 'Several predicates and consecutive-use effects lack one authoritative history lifecycle.', ['showdown:sim/pokemon.ts:Pokemon.moveThisTurnResult', 'showdown:sim/pokemon.ts:Pokemon.lastMove']),
	action_order_modifier: partial('Dynamic action-order modifiers', ['random priority activation', 'same-bracket ordering', 'diagnostic RNG'], 'Quick Claw-style bracket modification is not integrated with the canonical queue.', ['showdown:data/items.ts:Items.quickclaw']),
	accuracy_override: partial('Exceptional accuracy overrides', ['source and target scope', 'semi-invulnerable targets', 'always-hit semantics'], 'No Guard and move-specific accuracy overrides are not interaction-complete.', ['showdown:data/abilities.ts:Abilities.noguard']),
	status_cure: partial('Status cure lifecycle', ['major-status cure', 'entry/update timing', 'ability and item hooks'], 'Status berries, Heal Bell, Shed Skin, and related interactions are not unified.', ['showdown:sim/pokemon.ts:Pokemon.cureStatus']),
	pp_restore: unsupported('PP restoration lifecycle', ['slot selection', 'bounds', 'consumed item interaction'], 'Leppa Berry and move-driven PP restoration are incomplete.', ['showdown:data/items.ts:Items.leppaberry']),
	stat_restore: partial('Stat restoration and inversion lifecycle', ['negative-stage reset', 'item trigger timing', 'multi-stat update'], 'White Herb and several stage-manipulation moves are incomplete.', ['showdown:data/items.ts:Items.whiteherb']),
	switch_restriction: unsupported('Switch restriction and bypass', ['trapping predicates', 'legal-action mask', 'Shed Shell bypass'], 'Shadow Tag, trapping volatiles, and Shed Shell are incomplete.', ['showdown:sim/pokemon.ts:Pokemon.trapped', 'showdown:data/items.ts:Items.shedshell']),
	contact_effects: partial('Contact-triggered effects', ['contact qualification', 'post-hit ordering', 'faint interaction'], 'Several abilities and items depend on unimplemented volatile/status/faint consequences.', ['showdown:sim/battle-actions.ts:BattleActions.moveHit']),
	weather_suppression: unsupported('Weather suppression', ['effective-weather queries', 'entry/exit recomputation'], 'Cloud Nine suppression is incomplete.', ['showdown:data/abilities.ts:Abilities.cloudnine']),
	field_activation: partial('Ability-driven weather and terrain activation', ['entry hook', 'replacement', 'duration', 'ordering'], 'Several setters exist, but entry and damage-triggered activation is not uniformly sequenced.', ['showdown:data/abilities.ts:Abilities.drizzle', 'showdown:data/abilities.ts:Abilities.electricsurge']),
	stat_event_lifecycle: partial('Reactive stat-stage event lifecycle', ['prevention', 'reflection', 'source attribution', 'ordering'], 'Intimidate and other reactive boosts/drops are not complete across interactions.', ['showdown:sim/battle.ts:Battle.boost']),
	move_type_conversion: partial('Move type and power conversion abilities', ['type conversion', 'power modifier', 'move identity queries'], 'Aerilate-family and Champions conversion abilities are not interaction-complete.', ['showdown:data/abilities.ts:Abilities.aerilate']),
	damage_event_lifecycle: partial('Exceptional damage event lifecycle', ['damage-source attribution', 'survival', 'faint consequences', 'indirect damage'], 'Sturdy, Innards Out, Magic Guard, Liquid Ooze, and related effects need canonical event history.', ['showdown:sim/battle.ts:Battle.damage']),
	move_reflection: unsupported('Status-move reflection', ['reflectability', 'target/source swap', 'loop prevention'], 'Magic Bounce is incomplete.', ['showdown:data/abilities.ts:Abilities.magicbounce']),
	immunity_override: partial('Ability-based immunity override', ['Mold Breaker-style ignore', 'Scrappy exceptions', 'hook scoping'], 'Exceptional immunity and suppression interactions remain incomplete.', ['showdown:sim/pokemon.ts:Pokemon.ignoringAbility']),
	status_event_lifecycle: partial('Reactive status lifecycle', ['prevention', 'reflection', 'weather predicates', 'source attribution'], 'Synchronize, Leaf Guard, Purifying Salt, and related interactions are incomplete.', ['showdown:sim/pokemon.ts:Pokemon.setStatus']),
	end_turn_effects: partial('End-turn and delayed effects', ['residual ordering', 'duration', 'delayed activation'], 'Speed Boost, Cud Chew, Wish, Perish Song, and similar effects are incomplete.', ['showdown:sim/battle.ts:Battle.residualEvent']),
	hazard_lifecycle: unsupported('Entry-hazard lifecycle', ['placement', 'layers', 'switch-in application', 'removal'], 'Spikes, Toxic Spikes, Sticky Web, Stealth Rock, and reactive hazard setters are incomplete.', ['showdown:sim/side.ts:Side.addSideCondition']),
	redirection_ability: unsupported('Ability-based redirection', ['target rewrite', 'immunity/boost consequence', 'ordering'], 'Lightning Rod-style redirection is incomplete.', ['showdown:data/abilities.ts:Abilities.lightningrod']),
	ability_damage_modifiers: partial('Exceptional ability damage modifiers', ['predicate evaluation', 'hook ordering', 'source/target scoping'], 'Several modifier identities are registered but not interaction-complete.', ['showdown:sim/battle.ts:Battle.runEvent']),
	item_effect_modifiers: partial('Exceptional item effect modifiers', ['trigger predicates', 'consumption/removal', 'ordering'], 'Identity metadata or one modifier does not cover the complete held-item behavior.', ['showdown:data/items.ts:Items']),
};

function capability(status, title, subCapabilities, knownGap, referenceEvidence) {
	return {status, title, sub_capabilities: subCapabilities, lifecycle_and_ordering: subCapabilities, known_gaps: [knownGap], tests: [], differential_fixtures: [], pinned_reference_evidence: referenceEvidence};
}
function unsupported(...args) { return capability('unsupported', ...args); }
function partial(...args) { return capability('partial', ...args); }
function implemented(...args) { return capability('implemented', ...args); }
function verified(...args) { return capability('verified', ...args); }

const ABILITY_GROUPS = [
	[['aerilate', 'dragonize', 'eelevate', 'pixilate', 'refrigerate'], ['move_type_conversion']],
	[['aftermath', 'cutecharm', 'effectspore', 'flamebody', 'poisonpoint', 'poisontouch', 'roughskin', 'static', 'stench'], ['contact_effects']],
	[['aromaveil', 'flowerveil', 'innerfocus', 'oblivious', 'owntempo', 'tangledfeet'], ['volatile', 'volatile_cure']],
	[['cheekpouch', 'cudchew', 'gluttony', 'harvest', 'ripen', 'unnerve'], ['berry_lifecycle']],
	[['cudchew', 'speedboost'], ['end_turn_effects']],
	[['cloudnine'], ['weather_suppression']],
	[['curiousmedicine', 'guarddog', 'intimidate', 'mirrorarmor', 'opportunist', 'rattled', 'spicyspray', 'supersweetsyrup'], ['stat_event_lifecycle']],
	[['cursedbody'], ['volatile']],
	[['damp'], ['damage_event_lifecycle']],
	[['disguise', 'forecast', 'hungerswitch', 'illusion', 'imposter', 'libero', 'mimicry', 'protean', 'stancechange', 'zerotohero'], ['form_change']],
	[['drizzle', 'drought', 'electricsurge', 'grassysurge', 'psychicsurge', 'sandspit', 'sandstream', 'seedsower', 'snowwarning'], ['field_activation']],
	[['electromorphosis'], ['volatile']],
	[['emergencyexit'], ['forced_switch']],
	[['flashfire', 'heatproof', 'heavymetal', 'lightmetal', 'liquidooze', 'parentalbond', 'rivalry', 'stakeout', 'sturdy'], ['ability_damage_modifiers', 'damage_event_lifecycle']],
	[['infiltrator'], ['volatile']],
	[['innardsout', 'magicguard', 'poisonheal'], ['damage_event_lifecycle']],
	[['klutz', 'magician', 'pickpocket', 'pickup', 'stickyhold', 'symbiosis', 'unburden'], ['item_lifecycle']],
	[['leafguard', 'purifyingsalt', 'synchronize'], ['status_event_lifecycle']],
	[['lightningrod'], ['redirection_ability']],
	[['magicbounce'], ['move_reflection']],
	[['mummy', 'receiver', 'trace', 'wanderingspirit'], ['ability_lifecycle']],
	[['noguard'], ['accuracy_override']],
	[['scrappy'], ['immunity_override']],
	[['screencleaner'], ['field_or_side_condition']],
	[['shadowtag'], ['switch_restriction']],
	[['steadfast'], ['flinch', 'stat_event_lifecycle']],
	[['suctioncups'], ['forced_switch']],
	[['toxicdebris'], ['hazard_lifecycle']],
];

const MEGA_STONES = [
	'abomasite', 'absolite', 'absolitez', 'aerodactylite', 'aggronite', 'alakazite', 'altarianite',
	'ampharosite', 'audinite', 'banettite', 'barbaracite', 'baxcalibrite', 'beedrillite',
	'blastoisinite', 'blazikenite', 'cameruptite', 'chandelurite', 'charizarditex', 'charizarditey',
	'chesnaughtite', 'chimechite', 'clefablite', 'crabominite', 'delphoxite', 'dragalgite',
	'dragoninite', 'drampanite', 'eelektrossite', 'emboarite', 'excadrite', 'falinksite',
	'feraligite', 'floettite', 'froslassite', 'galladite', 'garchompite', 'garchompitez',
	'gardevoirite', 'gengarite', 'glalitite', 'glimmoranite', 'golisopite', 'golurkite',
	'greninjite', 'gyaradosite', 'hawluchanite', 'heracronite', 'houndoominite', 'kangaskhanite',
	'lopunnite', 'lucarionite', 'lucarionitez', 'malamarite', 'manectite', 'mawilite',
	'medichamite', 'meganiumite', 'meowsticite', 'metagrossite', 'pidgeotite', 'pinsirite',
	'pyroarite', 'raichunitex', 'raichunitey', 'sablenite', 'salamencite', 'sceptilite',
	'scizorite', 'scolipite', 'scovillainite', 'scraftinite', 'sharpedonite', 'skarmorite',
	'slowbronite', 'staraptite', 'starminite', 'steelixite', 'swampertite', 'tyranitarite',
	'venusaurite', 'victreebelite',
];
const RESIST_BERRIES = ['babiriberry', 'chartiberry', 'chilanberry', 'chopleberry', 'cobaberry', 'colburberry', 'habanberry', 'kasibberry', 'kebiaberry', 'occaberry', 'passhoberry', 'payapaberry', 'rindoberry', 'roseliberry', 'shucaberry', 'tangaberry', 'wacanberry', 'yacheberry'];
const STATUS_BERRIES = ['aspearberry', 'cheriberry', 'chestoberry', 'lumberry', 'pechaberry', 'persimberry', 'rawstberry'];

const ITEM_GROUPS = [
	[MEGA_STONES, ['mega_eligibility', 'mega_timing', 'form_change', 'ability_transition']],
	[[...RESIST_BERRIES, 'oranberry', 'sitrusberry'], ['berry_lifecycle']],
	[STATUS_BERRIES, ['berry_lifecycle', 'status_cure']],
	[['persimberry', 'lumberry'], ['confusion']],
	[['leppaberry'], ['berry_lifecycle', 'pp_restore']],
	[['bindingband'], ['residual_trapping']],
	[['choicescarf'], ['choice_lock']],
	[['ejectbutton', 'redcard'], ['forced_switch']],
	[['kingsrock'], ['contact_effects', 'flinch']],
	[['mentalherb'], ['volatile', 'volatile_cure']],
	[['metronome'], ['move_history']],
	[['quickclaw'], ['action_order_modifier']],
	[['shedshell'], ['switch_restriction']],
	[['whiteherb'], ['stat_restore']],
];

const VERIFIED_EVIDENCE = {
	standard_damage: {
		tests: ['unit:crates/rotom-dex/tests/combat.rs#canonical_step_matches_preselected_physical_special_effective_immune_and_critical_cases'],
		differential_fixtures: ['differential:data/fixtures/differential/canonical-step.json#dragon-claw-physical-step'],
	},
	selected_single_target: {
		tests: ['unit:crates/rotom-core/src/combat.rs#selected_targets_must_match_the_authoritative_move_definition'],
		differential_fixtures: ['differential:data/fixtures/differential/doubles-targeting.json#follow-me-redirection'],
	},
	accuracy: {
		tests: ['unit:crates/rotom-dex/tests/combat.rs#pinned_move_rng_sequences_match_showdown_draw_count_and_order'],
		differential_fixtures: ['differential:data/fixtures/differential/basic-damage.json#aqua-jet-low-damage-quarter-resisted'],
	},
	protection: {
		tests: ['unit:crates/rotom-dex/tests/combat.rs#follow_me_redirects_single_target_moves_and_repeat_protect_uses_the_battle_rng'],
		differential_fixtures: ['differential:data/fixtures/differential/doubles-targeting.json#protect-repeat'],
	},
	redirection: {
		tests: ['unit:crates/rotom-dex/tests/combat.rs#follow_me_redirects_single_target_moves_and_repeat_protect_uses_the_battle_rng'],
		differential_fixtures: ['differential:data/fixtures/differential/doubles-targeting.json#follow-me-redirection'],
	},
};

for (const [id, evidence] of Object.entries(VERIFIED_EVIDENCE)) Object.assign(CAPABILITIES[id], evidence);

function dependenciesFromGroups(kind, groups) {
	const result = new Map();
	for (const [ids, capabilities] of groups) {
		for (const id of ids) {
			const key = `${kind}:${id}`;
			result.set(key, [...new Set([...(result.get(key) || []), ...capabilities])].sort());
		}
	}
	return result;
}

const IDENTITY_DEPENDENCIES = new Map([
	...dependenciesFromGroups('ability', ABILITY_GROUPS),
	...dependenciesFromGroups('item', ITEM_GROUPS),
]);

module.exports = {CAPABILITIES, IDENTITY_DEPENDENCIES, MEGA_STONES};
