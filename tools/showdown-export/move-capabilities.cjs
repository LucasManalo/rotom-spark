'use strict';

const INTRINSIC_MOVE_FLAGS = new Set([
	'cantusetwice',
	'charge',
	'defrost',
	'futuremove',
	'gravity',
	'heal',
	'minimize',
	'pledgecombo',
	'powder',
	'recharge',
]);

const SPECIAL_MOVE_PROPERTIES = [
	'alwaysHit',
	'basePowerModifier',
	'breaksProtect',
	'chloroblastRecoil',
	'critModifier',
	'forceSTAB',
	'hasCrashDamage',
	'hasSheerForceBoost',
	'ignoreAbility',
	'ignoreAccuracy',
	'ignoreDefensive',
	'ignoreEvasion',
	'ignoreImmunity',
	'ignoreNegativeOffensive',
	'ignoreOffensive',
	'ignorePositiveDefensive',
	'ignorePositiveEvasion',
	'mindBlownRecoil',
	'multiaccuracy',
	'multihitType',
	'noDamageVariance',
	'ohko',
	'overrideDefensivePokemon',
	'overrideDefensiveStat',
	'overrideOffensivePokemon',
	'overrideOffensiveStat',
	'selfdestruct',
	'sleepUsable',
	'smartTarget',
	'spreadModifier',
	'stallingMove',
	'stealsBoosts',
	'struggleRecoil',
	'thawsTarget',
	'tracksTarget',
	'willCrit',
];

const CAPABILITY_ORDER = [
	'standard_damage',
	'selected_single_target',
	'accuracy',
	'fixed_damage',
	'non_damage_primary',
	'advanced_targeting',
	'multi_hit',
	'secondary_effect',
	'self_effect',
	'drain',
	'recoil',
	'major_status',
	'volatile',
	'stat_stage',
	'healing',
	'field_or_side_condition',
	'switch_control',
	'calls_move',
	'custom_callback',
	'special_behavior',
	'target_set',
	'spread_damage',
	'protection',
	'redirection',
	'retargeting',
	'per_target_hit',
];

const PR_13_1_CAPABILITIES = new Set([
	'standard_damage',
	'selected_single_target',
	'accuracy',
]);
const PR_13_2_CAPABILITIES = new Set([
	...PR_13_1_CAPABILITIES,
	'advanced_targeting',
	'non_damage_primary',
	'volatile',
	'target_set',
	'spread_damage',
	'protection',
	'redirection',
	'retargeting',
	'per_target_hit',
]);
const PR_13_3_CAPABILITIES = new Set([
	...PR_13_2_CAPABILITIES, 'major_status', 'stat_stage', 'healing', 'field_or_side_condition',
]);

function sortedStrings(values) {
	return [...(values || [])].sort((left, right) => left < right ? -1 : left > right ? 1 : 0);
}

function moveBehavior(move) {
	const effectKinds = [];
	if (move.secondary || move.secondaries?.length) effectKinds.push('secondary_effect');
	if (move.self || move.selfBoost) effectKinds.push('self_effect');
	if (move.boosts) effectKinds.push('stat_stage');
	if (move.heal) effectKinds.push('healing');
	if (move.weather || move.terrain || move.pseudoWeather || move.sideCondition || move.slotCondition) {
		effectKinds.push('field_or_side_condition');
	}
	if (move.callsMove) effectKinds.push('calls_move');
	if (move.selfSwitch || move.forceSwitch) effectKinds.push('switch_control');
	return {
		effect_kinds: effectKinds,
		callbacks: sortedStrings(Object.keys(move).filter(key => typeof move[key] === 'function')),
		intrinsic_flags: sortedStrings(
			Object.keys(move.flags || {}).filter(flag => move.flags[flag] && INTRINSIC_MOVE_FLAGS.has(flag))
		),
		special_properties: SPECIAL_MOVE_PROPERTIES.filter(property => Boolean(move[property])),
	};
}

function moveExecutionRequirements(move) {
	if (move.id === 'protect') {
		return {operation: 'non_damage', required_capabilities: ['non_damage_primary', 'volatile', 'protection']};
	}
	if (['followme', 'ragepowder'].includes(move.id)) {
		return {operation: 'non_damage', required_capabilities: ['non_damage_primary', 'volatile', 'redirection']};
	}
	const behavior = moveBehavior(move);
	const capabilities = new Set(['accuracy']);
	let operation;
	if (typeof move.damage === 'number') {
		operation = 'fixed_damage';
		capabilities.add('fixed_damage');
	} else if (move.damage === 'level') {
		operation = 'level_damage';
		capabilities.add('fixed_damage');
	} else if (move.category !== 'Status' && move.basePower > 0) {
		operation = 'standard_damage';
		capabilities.add('standard_damage');
	} else {
		operation = 'non_damage';
		capabilities.add('non_damage_primary');
	}
	if (['normal', 'any', 'adjacentFoe'].includes(move.target)) {
		capabilities.add('selected_single_target');
	} else {
		capabilities.add('advanced_targeting');
	}
	if (move.category !== 'Status' && move.basePower > 0) {
		capabilities.add('per_target_hit');
		capabilities.add('protection');
		capabilities.add('retargeting');
		if (['allAdjacent', 'allAdjacentFoes'].includes(move.target)) {
			capabilities.add('target_set');
			capabilities.add('spread_damage');
		} else if (['normal', 'any', 'adjacentFoe'].includes(move.target)) {
			capabilities.add('redirection');
		}
	}
	if (move.multihit) capabilities.add('multi_hit');
	if (move.drain) capabilities.add('drain');
	if (move.recoil) capabilities.add('recoil');
	if (move.status) capabilities.add('major_status');
	if (move.volatileStatus) {
		capabilities.add('volatile');
		if (move.volatileStatus !== 'magnetrise') capabilities.add('special_behavior');
	}
	const unsupportedCondition =
		(move.sideCondition && !['reflect', 'lightscreen', 'tailwind'].includes(move.sideCondition)) ||
		(move.pseudoWeather && !['trickroom', 'gravity', 'magicroom', 'wonderroom'].includes(move.pseudoWeather));
	for (const effectKind of behavior.effect_kinds) capabilities.add(effectKind);
	if (behavior.callbacks.length) capabilities.add('custom_callback');
	if (move.id === 'toxic' || unsupportedCondition ||
		behavior.intrinsic_flags.some(flag => flag !== 'heal' || !move.heal) ||
		behavior.special_properties.some(property => !['breaksProtect', 'ignoreImmunity'].includes(property))) {
		capabilities.add('special_behavior');
	}
	return {
		operation,
		required_capabilities: CAPABILITY_ORDER.filter(capability => capabilities.has(capability)),
	};
}

function isPr131Supported(move) {
	return moveExecutionRequirements(move).required_capabilities
		.every(capability => PR_13_1_CAPABILITIES.has(capability));
}

function isPr132Supported(move) {
	return moveExecutionRequirements(move).required_capabilities
		.every(capability => PR_13_2_CAPABILITIES.has(capability));
}

function isPr133Supported(move) {
	return moveExecutionRequirements(move).required_capabilities
		.every(capability => PR_13_3_CAPABILITIES.has(capability));
}

module.exports = {isPr131Supported, isPr132Supported, isPr133Supported, moveBehavior, moveExecutionRequirements};
