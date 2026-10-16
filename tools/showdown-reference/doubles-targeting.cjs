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
const set = (species, moves, ability = 'pressure', item) => ({species, ability, item, moves, level: 50});
const pad = team => [...team, ...Array.from({length: 4 - team.length}, filler)];

function battleWith(p1, p2, seed = 'gen5,0000000000000001') {
	const battle = new Battle({
		format: Dex.formats.get(version.format, true), seed, p1: {team: pad(p1)}, p2: {team: pad(p2)},
	});
	battle.makeChoices('team 1234', 'team 1234');
	return battle;
}

function activeState(battle) {
	return [battle.p1, battle.p2].flatMap(side => side.active.map(pokemon => pokemon && ({
		ident: pokemon.fullname, species: pokemon.species.id, hp: pokemon.hp,
		max_hp: pokemon.maxhp, fainted: pokemon.fainted,
	})));
}

function relevantLog(battle, start) {
	return battle.log.slice(start).filter(line => /\|(move|-damage|-immune|-activate|-miss|faint|switch)\|/u.test(line));
}

function use(battle, move, source, target) {
	const before = activeState(battle);
	const start = battle.log.length;
	const moveId = typeof move === 'string' ? battle.dex.moves.get(move).id : move.id;
	const originalWillAct = battle.queue.willAct;
	if (moveId === 'protect') battle.queue.willAct = () => true;
	const result = battle.actions.useMove(move, source, {target});
	battle.queue.willAct = originalWillAct;
	battle.faintMessages();
	return {result: Boolean(result), before, after: activeState(battle), log: relevantLog(battle, start)};
}

function controlledDamage(battle) {
	const draws = [];
	battle.randomChance = (numerator, denominator) => {
		draws.push({operation: 'chance', numerator, denominator, result: denominator === 100});
		return denominator === 100;
	};
	battle.randomizer = damage => {
		draws.push({operation: 'damage_randomizer', percent: 100});
		return damage;
	};
	return draws;
}

function spreadCases() {
	const surf = battleWith([set('Goodra', ['surf'], 'gooey'), filler()], [
		set('Arcanine', ['protect'], 'intimidate'), set('Blastoise', ['splash'], 'torrent'),
	]);
	use(surf, 'protect', surf.p2.active[0], surf.p2.active[0]);
	const surfDraws = controlledDamage(surf);
	const surfResult = use(surf, 'surf', surf.p1.active[0], surf.p2.active[0]);

	const two = battleWith([set('Goodra', ['dazzlinggleam'], 'gooey'), filler()], [
		set('Arcanine', ['splash'], 'intimidate'), set('Blastoise', ['splash'], 'torrent'),
	]);
	const twoDraws = controlledDamage(two);
	const twoResult = use(two, 'dazzlinggleam', two.p1.active[0], two.p2.active[0]);

	const one = battleWith([set('Goodra', ['dazzlinggleam'], 'gooey'), filler()], [
		set('Arcanine', ['splash'], 'intimidate'), set('Blastoise', ['splash'], 'torrent'),
	]);
	one.p2.active[1].hp = 0;
	one.p2.active[1].fainted = true;
	const oneDraws = controlledDamage(one);
	const oneResult = use(one, 'dazzlinggleam', one.p1.active[0], one.p2.active[0]);

	const guarded = battleWith([set('Goodra', ['dazzlinggleam'], 'gooey'), filler()], [
		set('Arcanine', ['protect'], 'intimidate'), set('Blastoise', ['splash'], 'torrent'),
	]);
	use(guarded, 'protect', guarded.p2.active[0], guarded.p2.active[0]);
	const guardedDraws = controlledDamage(guarded);
	const guardedResult = use(guarded, 'dazzlinggleam', guarded.p1.active[0], guarded.p2.active[0]);

	const immune = battleWith([set('Garchomp', ['earthquake'], 'roughskin'), filler()], [
		set('Charizard', ['splash'], 'blaze'), set('Arcanine', ['splash'], 'intimidate'),
	]);
	const immuneDraws = controlledDamage(immune);
	const immuneResult = use(immune, 'earthquake', immune.p1.active[0], immune.p2.active[0]);

	return {
		surf_mixed_protected: {...surfResult, rng_operations: surfDraws},
		two_valid_targets: {...twoResult, rng_operations: twoDraws},
		one_remaining_target: {...oneResult, rng_operations: oneDraws},
		mixed_protected: {...guardedResult, rng_operations: guardedDraws},
		mixed_type_immune: {...immuneResult, rng_operations: immuneDraws},
	};
}

function targetCases() {
	const fainted = battleWith([set('Goodra', ['aquatail'], 'gooey'), filler()], [
		set('Arcanine', ['splash'], 'intimidate'), set('Blastoise', ['splash'], 'torrent'),
	]);
	const original = fainted.p2.active[0].fullname;
	fainted.p2.active[0].hp = 0;
	fainted.p2.active[0].fainted = true;
	controlledDamage(fainted);
	const retargeted = use(fainted, 'aquatail', fainted.p1.active[0], fainted.p2.active[0]);

	const switched = battleWith([set('Goodra', ['aquatail'], 'gooey'), filler()], [
		set('Arcanine', ['splash'], 'intimidate'), set('Blastoise', ['splash'], 'torrent'),
		set('Clefairy', ['splash'], 'friendguard'),
	]);
	const switchedOut = switched.p2.active[0].fullname;
	switched.actions.switchIn(switched.p2.pokemon[2], 0);
	controlledDamage(switched);
	const switchedResult = use(switched, 'aquatail', switched.p1.active[0], switched.p2.active[0]);

	const missingAlly = battleWith(
		[set('Goodra', ['helpinghand'], 'gooey'), set('Arcanine', ['splash'], 'intimidate')],
		[set('Blastoise', ['splash'], 'torrent'), filler()],
	);
	missingAlly.p1.active[1].hp = 0;
	missingAlly.p1.active[1].fainted = true;
	const missingAllyResult = use(missingAlly, 'helpinghand', missingAlly.p1.active[0], missingAlly.p1.active[1]);
	return {
		selected_foe_fainted: {original_target: original, ...retargeted},
		selected_slot_switched: {original_target: switchedOut, resolved_target: switched.p2.active[0].fullname, ...switchedResult},
		missing_ally: missingAllyResult,
	};
}

function protectionCases() {
	const battle = battleWith([set('Goodra', ['protect'], 'gooey'), filler()], [
		set('Arcanine', ['splash'], 'intimidate'), filler(),
	]);
	const denominators = [];
	battle.randomChance = (numerator, denominator) => {
		denominators.push([numerator, denominator]);
		return denominator === 3;
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

	const feintBattle = battleWith([set('Goodra', ['feint'], 'gooey'), filler()], [
		set('Arcanine', ['protect'], 'intimidate'), filler(),
	]);
	use(feintBattle, 'protect', feintBattle.p2.active[0], feintBattle.p2.active[0]);
	controlledDamage(feintBattle);
	const feint = use(feintBattle, 'feint', feintBattle.p1.active[0], feintBattle.p2.active[0]);
	return {first, repeat_success: repeatSuccess, repeat_failure: repeatFailure, reset, denominators, feint};
}

function redirectionCases() {
	const execute = (redirectMove, sourceSpecies = 'Goodra', sourceAbility = 'gooey', sourceItem) => {
		const battle = battleWith([set(sourceSpecies, ['aquatail'], sourceAbility, sourceItem), filler()], [
			set('Clefairy', [redirectMove], 'friendguard'), set('Blastoise', ['splash'], 'torrent'),
		]);
		use(battle, redirectMove, battle.p2.active[0], battle.p2.active[0]);
		controlledDamage(battle);
		return use(battle, 'aquatail', battle.p1.active[0], battle.p2.active[1]);
	};

	const conflict = battleWith([set('Goodra', ['aquatail'], 'gooey'), filler()], [
		set('Clefairy', ['followme'], 'friendguard'), set('Clefairy', ['followme'], 'friendguard'),
	]);
	use(conflict, 'followme', conflict.p2.active[1], conflict.p2.active[1]);
	use(conflict, 'followme', conflict.p2.active[0], conflict.p2.active[0]);
	controlledDamage(conflict);
	const conflictResult = use(conflict, 'aquatail', conflict.p1.active[0], conflict.p2.active[0]);

	const spread = battleWith([set('Goodra', ['dazzlinggleam'], 'gooey'), filler()], [
		set('Clefairy', ['followme'], 'friendguard'), set('Blastoise', ['splash'], 'torrent'),
	]);
	use(spread, 'followme', spread.p2.active[0], spread.p2.active[0]);
	controlledDamage(spread);
	const spreadResult = use(spread, 'dazzlinggleam', spread.p1.active[0], spread.p2.active[1]);

	const unavailable = battleWith(
		[set('Goodra', ['aquatail'], 'gooey'), set('Goodra', ['aquatail'], 'gooey')],
		[set('Clefairy', ['followme'], 'friendguard'), set('Blastoise', ['splash'], 'torrent')],
	);
	use(unavailable, 'followme', unavailable.p2.active[0], unavailable.p2.active[0]);
	unavailable.p2.active[0].hp = 1;
	controlledDamage(unavailable);
	const first = use(unavailable, 'aquatail', unavailable.p1.active[0], unavailable.p2.active[1]);
	const second = use(unavailable, 'aquatail', unavailable.p1.active[1], unavailable.p2.active[1]);

	return {
		follow_me: execute('followme'), rage_powder: execute('ragepowder'),
		grass_ignores_rage_powder: execute('ragepowder', 'Amoonguss', 'effectspore'),
		overcoat_ignores_rage_powder: execute('ragepowder', 'Goodra', 'overcoat'),
		safety_goggles_ignores_rage_powder: execute('ragepowder', 'Goodra', 'gooey', 'safetygoggles'),
		conflict_active_entry_order_wins_equal_speed: conflictResult,
		spread_ignores_redirection: spreadResult,
		redirector_unavailable: {first, second},
	};
}

function psychicTerrainCase() {
	const battle = battleWith([set('Goodra', ['dazzlinggleam'], 'gooey'), filler()], [
		set('Arcanine', ['protect'], 'intimidate'), set('Charizard', ['protect'], 'blaze'),
	]);
	battle.field.setTerrain('psychicterrain', battle.p1.active[0]);
	use(battle, 'protect', battle.p2.active[1], battle.p2.active[1]);
	const move = battle.dex.getActiveMove('dazzlinggleam');
	move.priority = 1;
	const draws = controlledDamage(battle);
	return {...use(battle, move, battle.p1.active[0], battle.p2.active[0]), rng_operations: draws, synthetic_priority: true};
}

function perTargetCases() {
	const accuracy = battleWith([set('Garchomp', ['rockslide'], 'roughskin'), filler()], [
		set('Arcanine', ['splash'], 'intimidate'), set('Blastoise', ['splash'], 'torrent'),
	]);
	let accuracyIndex = 0;
	const operations = [];
	accuracy.randomChance = (numerator, denominator) => {
		const result = denominator === 100 ? accuracyIndex++ === 0 : false;
		operations.push({operation: 'chance', numerator, denominator, result});
		return result;
	};
	accuracy.randomizer = damage => {
		operations.push({operation: 'damage_randomizer', percent: 100});
		return damage;
	};
	const result = use(accuracy, 'rockslide', accuracy.p1.active[0], accuracy.p2.active[0]);
	return {independent_accuracy: {...result, rng_operations: operations}, psychic_terrain_mixed_targets: psychicTerrainCase()};
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
			formula_differential: ['spread.two_valid_targets', 'spread.one_remaining_target', 'spread.mixed_protected', 'spread.mixed_type_immune'],
			individual_mechanic: ['target_resolution', 'protection', 'redirection', 'per_target_execution'],
			full_transition: [],
			interaction: ['spread.mixed_protected', 'redirection.conflict_active_entry_order_wins_equal_speed', 'redirection.redirector_unavailable'],
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
