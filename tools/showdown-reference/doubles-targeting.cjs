#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'doubles-targeting.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));
const dex = Dex.forFormat(version.format);
const protect = dex.moves.get('protect');
const followMe = dex.moves.get('followme');
const ragePowder = dex.moves.get('ragepowder');
const stall = dex.conditions.get('stall');

const filler = () => ({species: 'Magikarp', ability: 'swiftswim', moves: ['splash'], level: 50});
const set = (species, moves, ability = 'pressure', item, extra = {}) => ({
	species, ability, item, moves, level: 50, ...extra,
});
const pad = team => [...team, ...Array.from({length: 4 - team.length}, filler)];

function battleWith(p1, p2, seed = 'gen5,0000000000000001') {
	const battle = new Battle({
		format: Dex.formats.get(version.format, true), seed, p1: {team: pad(p1)}, p2: {team: pad(p2)},
	});
	battle.makeChoices('team 1234', 'team 1234');
	return battle;
}

function activeState(battle) {
	return [battle.p1, battle.p2].flatMap(side => side.active.map((pokemon, slot) => pokemon && ({
		side: side.id, slot, ident: pokemon.fullname, species: pokemon.species.id,
		hp: pokemon.hp, max_hp: pokemon.maxhp, fainted: pokemon.fainted,
		volatiles: Object.keys(pokemon.volatiles).sort(),
	})));
}

function targetRef(pokemon) {
	if (!pokemon) return null;
	return {side: pokemon.side.id, slot: pokemon.position, species: pokemon.species.id};
}

function relevantLog(battle, start) {
	return battle.log.slice(start).filter(line => /\|(move|-damage|-immune|-activate|-miss|faint|switch)\|/u.test(line));
}

function use(battle, move, source, target) {
	const before = activeState(battle);
	const start = battle.log.length;
	const activeMove = battle.dex.getActiveMove(move);
	const moveId = activeMove.id;
	const execution = {
		move: moveId, source: targetRef(source), selected_target: targetRef(target),
		resolved_targets: [], stages: [],
	};
	const originals = new Map();
	const wrap = (name, observe) => {
		const original = battle.actions[name];
		originals.set(name, original);
		battle.actions[name] = function (...args) {
			const result = original.apply(this, args);
			observe(args, result);
			return result;
		};
	};
	wrap('trySpreadMoveHit', ([targets]) => {
		execution.resolved_targets = targets.map(targetRef);
	});
	for (const [name, stage] of [
		['hitStepInvulnerabilityEvent', 'invulnerability'],
		['hitStepTryHitEvent', 'try_hit'],
		['hitStepTypeImmunity', 'type_immunity'],
		['hitStepTryImmunity', 'move_immunity'],
		['hitStepAccuracy', 'accuracy'],
		['hitStepBreakProtect', 'break_protect'],
		['hitStepMoveHitLoop', 'damage'],
	]) {
		wrap(name, ([targets], results) => {
			execution.stages.push({
				stage, targets: targets.map(targetRef),
				results: results === undefined ? null : results.map(result =>
					result === undefined ? null : result
				),
			});
		});
	}
	const originalWillAct = battle.queue.willAct;
	if (moveId === 'protect') battle.queue.willAct = () => true;
	const result = battle.actions.useMove(move, source, {target});
	battle.queue.willAct = originalWillAct;
	for (const [name, original] of originals) battle.actions[name] = original;
	battle.faintMessages();
	const after = activeState(battle);
	execution.outcomes = execution.resolved_targets.map(resolved => {
		const initial = before.find(state => state.side === resolved.side && state.slot === resolved.slot);
		const final = after.find(state => state.side === resolved.side && state.slot === resolved.slot);
		const failureStage = execution.stages.find(stage => {
			const index = stage.targets.findIndex(candidate =>
				candidate.side === resolved.side && candidate.slot === resolved.slot
			);
			return index >= 0 && stage.results !== null && !stage.results[index];
		})?.stage ?? null;
		return {
			target: resolved, result: failureStage ?? 'hit',
			applied_hp_loss: initial && final ? initial.hp - final.hp : null,
			final_hp: final?.hp ?? 0,
			fainted: final?.fainted ?? true,
		};
	});
	return {result: Boolean(result), before, after, execution, log: relevantLog(battle, start)};
}

function controlledDamage(battle) {
	const draws = [];
	battle.randomChance = (numerator, denominator) => {
		const reason = denominator === 100 ? 'accuracy' : 'critical_hit';
		draws.push({reason, operation: 'chance', numerator, denominator, result: denominator === 100});
		return denominator === 100;
	};
	battle.randomizer = damage => {
		draws.push({reason: 'damage_roll', operation: 'damage_randomizer', percent: 100});
		return damage;
	};
	return draws;
}

function spreadCases() {
	const allAdjacent = battleWith(
		[set('Goodra', ['surf'], 'pressure'), filler()],
		[set('Arcanine', ['splash'], 'pressure'), set('Blastoise', ['splash'], 'pressure')],
	);
	const allAdjacentDraws = controlledDamage(allAdjacent);
	const allAdjacentResult = use(allAdjacent, 'surf', allAdjacent.p1.active[0], allAdjacent.p2.active[0]);

	const surf = battleWith([set('Goodra', ['surf'], 'pressure'), filler()], [
		set('Arcanine', ['protect'], 'pressure'), set('Blastoise', ['splash'], 'pressure'),
	]);
	use(surf, 'protect', surf.p2.active[0], surf.p2.active[0]);
	const surfDraws = controlledDamage(surf);
	const surfResult = use(surf, 'surf', surf.p1.active[0], surf.p2.active[0]);

	const two = battleWith([set('Goodra', ['dazzlinggleam'], 'pressure'), filler()], [
		set('Arcanine', ['splash'], 'pressure'), set('Blastoise', ['splash'], 'pressure'),
	]);
	const twoDraws = controlledDamage(two);
	const twoResult = use(two, 'dazzlinggleam', two.p1.active[0], two.p2.active[0]);

	const one = battleWith([set('Goodra', ['dazzlinggleam'], 'pressure'), filler()], [
		set('Arcanine', ['splash'], 'pressure'), set('Blastoise', ['splash'], 'pressure'),
	]);
	one.p2.active[1].hp = 0;
	one.p2.active[1].fainted = true;
	const oneDraws = controlledDamage(one);
	const oneResult = use(one, 'dazzlinggleam', one.p1.active[0], one.p2.active[0]);

	const guarded = battleWith([set('Goodra', ['dazzlinggleam'], 'pressure'), filler()], [
		set('Arcanine', ['protect'], 'pressure'), set('Blastoise', ['splash'], 'pressure'),
	]);
	use(guarded, 'protect', guarded.p2.active[0], guarded.p2.active[0]);
	const guardedDraws = controlledDamage(guarded);
	const guardedResult = use(guarded, 'dazzlinggleam', guarded.p1.active[0], guarded.p2.active[0]);

	const immune = battleWith([set('Garchomp', ['earthquake'], 'pressure'), filler()], [
		set('Charizard', ['splash'], 'pressure'), set('Arcanine', ['splash'], 'pressure'),
	]);
	const immuneDraws = controlledDamage(immune);
	const immuneResult = use(immune, 'earthquake', immune.p1.active[0], immune.p2.active[0]);

	return {
		surf_all_adjacent: {...allAdjacentResult, rng_operations: allAdjacentDraws},
		surf_mixed_protected: {...surfResult, rng_operations: surfDraws},
		two_valid_targets: {...twoResult, rng_operations: twoDraws},
		one_remaining_target: {...oneResult, rng_operations: oneDraws},
		mixed_protected: {...guardedResult, rng_operations: guardedDraws},
		mixed_type_immune: {...immuneResult, rng_operations: immuneDraws},
	};
}

function targetCases() {
	const fainted = battleWith([set('Goodra', ['aquatail'], 'pressure'), filler()], [
		set('Arcanine', ['splash'], 'pressure'), set('Blastoise', ['splash'], 'pressure'),
	]);
	const original = fainted.p2.active[0].fullname;
	fainted.p2.active[0].hp = 0;
	fainted.p2.active[0].fainted = true;
	controlledDamage(fainted);
	const retargeted = use(fainted, 'aquatail', fainted.p1.active[0], fainted.p2.active[0]);

	const switched = battleWith([set('Goodra', ['aquatail'], 'pressure'), filler()], [
		set('Arcanine', ['splash'], 'pressure'), set('Blastoise', ['splash'], 'pressure'),
		set('Clefairy', ['splash'], 'pressure'),
	]);
	const switchedOut = switched.p2.active[0].fullname;
	switched.actions.switchIn(switched.p2.pokemon[2], 0);
	controlledDamage(switched);
	const switchedResult = use(switched, 'aquatail', switched.p1.active[0], switched.p2.active[0]);

	const missingAlly = battleWith(
		[set('Goodra', ['helpinghand'], 'pressure'), set('Arcanine', ['splash'], 'pressure')],
		[set('Blastoise', ['splash'], 'pressure'), filler()],
	);
	missingAlly.p1.active[1].hp = 0;
	missingAlly.p1.active[1].fainted = true;
	const missingAllyResult = use(missingAlly, 'helpinghand', missingAlly.p1.active[0], missingAlly.p1.active[1]);
	const missingAllyDamage = battleWith(
		[set('Goodra', ['dragonpulse'], 'pressure'), set('Arcanine', ['splash'], 'pressure')],
		[set('Blastoise', ['splash'], 'pressure'), filler()],
	);
	missingAllyDamage.p1.active[1].hp = 0;
	missingAllyDamage.p1.active[1].fainted = true;
	const missingAllyDamageDraws = controlledDamage(missingAllyDamage);
	const missingAllyDamageResult = use(
		missingAllyDamage, 'dragonpulse', missingAllyDamage.p1.active[0], missingAllyDamage.p1.active[1]
	);
	return {
		selected_foe_fainted: {original_target: original, ...retargeted},
		selected_slot_switched: {original_target: switchedOut, resolved_target: switched.p2.active[0].fullname, ...switchedResult},
		missing_ally: missingAllyResult,
		missing_ally_damage: {...missingAllyDamageResult, rng_operations: missingAllyDamageDraws},
	};
}

function protectionCases() {
	const battle = battleWith([set('Goodra', ['protect'], 'pressure'), filler()], [
		set('Arcanine', ['splash'], 'pressure'), filler(),
	]);
	const denominators = [];
	const rngOperations = [];
	battle.randomChance = (numerator, denominator) => {
		denominators.push([numerator, denominator]);
		const result = denominator === 3;
		rngOperations.push({reason: 'protect', operation: 'chance', numerator, denominator, result});
		return result;
	};
	const user = battle.p1.active[0];
	const first = use(battle, 'protect', user, user);
	user.removeVolatile('protect');
	const repeatSuccess = use(battle, 'protect', user, user);
	user.removeVolatile('protect');
	const repeatFailure = use(battle, 'protect', user, user);
	user.removeVolatile('stall');
	user.removeVolatile('protect');
	const reset = use(battle, 'protect', user, user);

	const feintBattle = battleWith([set('Goodra', ['feint'], 'pressure'), filler()], [
		set('Arcanine', ['protect'], 'pressure'), filler(),
	]);
	use(feintBattle, 'protect', feintBattle.p2.active[0], feintBattle.p2.active[0]);
	controlledDamage(feintBattle);
	const feint = use(feintBattle, 'feint', feintBattle.p1.active[0], feintBattle.p2.active[0]);
	return {
		first, repeat_success: repeatSuccess, repeat_failure: repeatFailure, reset,
		denominators, rng_operations: rngOperations, feint,
	};
}

function redirectionCases() {
	const execute = (redirectMove, sourceSpecies = 'Goodra', sourceAbility = 'pressure', sourceItem) => {
		const battle = battleWith([set(sourceSpecies, ['aquatail'], sourceAbility, sourceItem), filler()], [
			set('Clefairy', [redirectMove], 'pressure'), set('Blastoise', ['splash'], 'pressure'),
		]);
		use(battle, redirectMove, battle.p2.active[0], battle.p2.active[0]);
		controlledDamage(battle);
		return use(battle, 'aquatail', battle.p1.active[0], battle.p2.active[1]);
	};

	const conflict = battleWith([set('Goodra', ['aquatail'], 'pressure'), filler()], [
		set('Clefairy', ['followme'], 'pressure'), set('Clefairy', ['followme'], 'pressure'),
	]);
	use(conflict, 'followme', conflict.p2.active[1], conflict.p2.active[1]);
	use(conflict, 'followme', conflict.p2.active[0], conflict.p2.active[0]);
	controlledDamage(conflict);
	const conflictResult = use(conflict, 'aquatail', conflict.p1.active[0], conflict.p2.active[0]);
	const speedConflict = battleWith([set('Goodra', ['aquatail'], 'pressure'), filler()], [
		set('Clefairy', ['followme'], 'pressure', undefined, {evs: {spe: 0}}),
		set('Clefairy', ['followme'], 'pressure', undefined, {evs: {spe: 252}}),
	]);
	use(speedConflict, 'followme', speedConflict.p2.active[0], speedConflict.p2.active[0]);
	use(speedConflict, 'followme', speedConflict.p2.active[1], speedConflict.p2.active[1]);
	controlledDamage(speedConflict);
	const speedConflictResult = use(
		speedConflict, 'aquatail', speedConflict.p1.active[0], speedConflict.p2.active[0]
	);
	const entryConflict = battleWith([set('Goodra', ['aquatail'], 'pressure'), filler()], [
		set('Clefairy', ['followme'], 'pressure'), set('Clefairy', ['followme'], 'pressure'),
		set('Clefairy', ['followme'], 'pressure'),
	]);
	entryConflict.actions.switchIn(entryConflict.p2.pokemon[2], 0);
	use(entryConflict, 'followme', entryConflict.p2.active[0], entryConflict.p2.active[0]);
	use(entryConflict, 'followme', entryConflict.p2.active[1], entryConflict.p2.active[1]);
	controlledDamage(entryConflict);
	const entryConflictResult = use(
		entryConflict, 'aquatail', entryConflict.p1.active[0], entryConflict.p2.active[0]
	);

	const spread = battleWith([set('Goodra', ['dazzlinggleam'], 'pressure'), filler()], [
		set('Clefairy', ['followme'], 'pressure'), set('Blastoise', ['splash'], 'pressure'),
	]);
	use(spread, 'followme', spread.p2.active[0], spread.p2.active[0]);
	controlledDamage(spread);
	const spreadResult = use(spread, 'dazzlinggleam', spread.p1.active[0], spread.p2.active[1]);

	const unavailable = battleWith(
		[set('Goodra', ['aquatail'], 'pressure'), set('Goodra', ['aquatail'], 'pressure')],
		[set('Clefairy', ['followme'], 'pressure'), set('Blastoise', ['splash'], 'pressure')],
	);
	use(unavailable, 'followme', unavailable.p2.active[0], unavailable.p2.active[0]);
	unavailable.p2.active[0].hp = 1;
	controlledDamage(unavailable);
	const first = use(unavailable, 'aquatail', unavailable.p1.active[0], unavailable.p2.active[1]);
	const second = use(unavailable, 'aquatail', unavailable.p1.active[1], unavailable.p2.active[1]);
	const switched = battleWith([set('Goodra', ['aquatail'], 'pressure'), filler()], [
		set('Clefairy', ['followme'], 'pressure'), set('Blastoise', ['splash'], 'pressure'),
		set('Arcanine', ['splash'], 'pressure'),
	]);
	use(switched, 'followme', switched.p2.active[0], switched.p2.active[0]);
	switched.actions.switchIn(switched.p2.pokemon[2], 0);
	controlledDamage(switched);
	const switchedResult = use(switched, 'aquatail', switched.p1.active[0], switched.p2.active[1]);

	return {
		follow_me: execute('followme'), rage_powder: execute('ragepowder'),
		grass_ignores_rage_powder: execute('ragepowder', 'Amoonguss', 'effectspore'),
		overcoat_ignores_rage_powder: execute('ragepowder', 'Goodra', 'overcoat'),
		safety_goggles_ignores_rage_powder: execute('ragepowder', 'Goodra', 'pressure', 'safetygoggles'),
		conflict_active_entry_order_wins_equal_speed: conflictResult,
		conflict_faster_redirector_wins: speedConflictResult,
		conflict_newer_left_loses_equal_speed: entryConflictResult,
		spread_ignores_redirection: spreadResult,
		redirector_unavailable: {first, second}, redirector_switched_out: switchedResult,
	};
}

function psychicTerrainCase() {
	const battle = battleWith([set('Goodra', ['dazzlinggleam'], 'pressure'), filler()], [
		set('Arcanine', ['protect'], 'pressure'), set('Charizard', ['protect'], 'pressure'),
	]);
	battle.field.setTerrain('psychicterrain', battle.p1.active[0]);
	use(battle, 'protect', battle.p2.active[1], battle.p2.active[1]);
	const move = battle.dex.getActiveMove('dazzlinggleam');
	move.priority = 1;
	const draws = controlledDamage(battle);
	return {...use(battle, move, battle.p1.active[0], battle.p2.active[0]), rng_operations: draws, synthetic_priority: true};
}

function perTargetCases() {
	const accuracy = battleWith([set('Charizard', ['aircutter'], 'pressure'), filler()], [
		set('Arcanine', ['splash'], 'pressure'), set('Blastoise', ['splash'], 'pressure'),
	]);
	let accuracyIndex = 0;
	const operations = [];
	accuracy.randomChance = (numerator, denominator) => {
		const result = denominator === 100 ? accuracyIndex++ === 0 : false;
		operations.push({
			reason: denominator === 100 ? 'accuracy' : 'critical_hit',
			operation: 'chance', numerator, denominator, result,
		});
		return result;
	};
	accuracy.randomizer = damage => {
		operations.push({reason: 'damage_roll', operation: 'damage_randomizer', percent: 100});
		return damage;
	};
	const result = use(accuracy, 'aircutter', accuracy.p1.active[0], accuracy.p2.active[0]);
	const faint = battleWith([set('Charizard', ['aircutter'], 'pressure'), filler()], [
		set('Arcanine', ['splash'], 'pressure'), set('Blastoise', ['splash'], 'pressure'),
	]);
	faint.p2.active[0].hp = 1;
	const faintDraws = controlledDamage(faint);
	const faintResult = use(faint, 'aircutter', faint.p1.active[0], faint.p2.active[0]);
	return {
		independent_accuracy: {...result, rng_operations: operations},
		target_level_faint: {...faintResult, rng_operations: faintDraws},
		psychic_terrain_mixed_targets: psychicTerrainCase(),
	};
}

const denominators = [];
for (const counter of [3, 9, 27]) {
	stall.onStallMove.call({
		effectState: {counter}, debug() {},
		randomChance(numerator, denominator) { denominators.push([numerator, denominator]); return true; },
	}, {volatiles: {stall: {}}});
}

const fixture = {
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		execution: 'Pinned Showdown BattleActions.useMove executes each recorded move against live Battle/Pokemon state.',
		classification: {
			formula_differential: ['spread.surf_all_adjacent', 'spread.two_valid_targets', 'spread.one_remaining_target', 'spread.mixed_protected', 'spread.mixed_type_immune'],
			individual_mechanic: ['target_resolution', 'protection', 'redirection', 'per_target_execution'],
			full_transition: [],
			interaction: ['spread.mixed_protected', 'redirection.conflict_active_entry_order_wins_equal_speed', 'redirection.conflict_faster_redirector_wins', 'redirection.conflict_newer_left_loses_equal_speed', 'redirection.redirector_unavailable', 'redirection.redirector_switched_out'],
		},
		rng_comparison: 'Controlled equivalent outcomes are used for accuracy, critical-hit, damage-roll, and Protect branches; no raw PRNG or seed parity is claimed.',
	},
	source: {
		target_sets: 'sim/pokemon.ts:Pokemon.getMoveTargets',
		retargeting: 'sim/battle.ts:Battle.getTarget/getRandomTarget',
		hit_order: 'sim/battle-actions.ts:BattleActions.trySpreadMoveHit',
		spread_modifier: 'data/mods/champions/scripts.ts:BattleActions.modifyDamage',
		priority_immunity: 'data/moves.ts:psychicterrain.condition.onTryHit',
		redirect_order: 'sim/battle.ts:Battle.compareRedirectOrder/resolvePriority',
		guards_and_redirection: 'data/moves.ts:protect/followme/ragepowder and data/conditions.ts:stall',
	},
	target_order: {all_adjacent: ['adjacent_allies', 'adjacent_foes'], all_adjacent_foes: ['adjacent_foes']},
	per_target_hit_order: ['try_hit', 'type_immunity', 'move_immunity', 'accuracy', 'break_protect', 'damage'],
	spread: {doubles_modifier: [3, 4], determined_before_target_failures: true},
	protect: {
		priority: protect.priority, try_hit_priority: protect.condition.onTryHitPriority,
		duration: protect.condition.duration, stall_duration: stall.duration,
		repeat_chances: denominators, counter_multiplier: 3,
	},
	psychic_terrain: {try_hit_priority: dex.moves.get('psychicterrain').condition.onTryHitPriority, per_target: true},
	redirection: {
		follow_me_priority: followMe.condition.onFoeRedirectTargetPriority,
		rage_powder_priority: ragePowder.condition.onFoeRedirectTargetPriority,
		duration: followMe.condition.duration, rage_powder_is_powder: Boolean(ragePowder.flags.powder),
		spread_moves_redirected: false,
		conflict_order: ['event_priority_desc', 'effective_speed_desc', 'holder_ability_state_creation_order_asc'],
	},
	cases: [
		{name: 'surf-spread-protection', move: 'surf', execution_path: 'executed_cases.spread.surf_mixed_protected'},
		{name: 'protect-repeat', move: 'protect', execution_path: 'executed_cases.protection'},
		{name: 'follow-me-redirection', move: 'followme', execution_path: 'executed_cases.redirection.follow_me'},
		{name: 'rage-powder-redirection', move: 'ragepowder', execution_path: 'executed_cases.redirection.rage_powder'},
		{name: 'feint-break-protect', move: 'feint', execution_path: 'executed_cases.protection.feint'},
	],
	executed_cases: {
		spread: spreadCases(), target_resolution: targetCases(), protection: protectionCases(),
		redirection: redirectionCases(), per_target_execution: perTargetCases(),
	},
};
const serialized = `${JSON.stringify(fixture, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== serialized) {
		throw new Error(`${path.relative(repositoryRoot, outputPath)} is stale`);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.mkdirSync(path.dirname(outputPath), {recursive: true});
	fs.writeFileSync(outputPath, serialized);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
