#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'doubles-targeting.json');
const version = Object.fromEntries(
	fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
		.trim()
		.split(/\r?\n/u)
		.map(line => line.split('=', 2)),
);
const dex = Dex.forFormat(version.format);
const protect = dex.moves.get('protect');
const followMe = dex.moves.get('followme');
const ragePowder = dex.moves.get('ragepowder');
const stall = dex.conditions.get('stall');

const denominators = [];
for (const counter of [3, 9, 27]) {
	stall.onStallMove.call({
		effectState: {counter},
		debug() {},
		randomChance(numerator, denominator) {
			denominators.push([numerator, denominator]);
			return true;
		},
	}, {volatiles: {stall: {}}});
}

const fixture = {
	showdown_commit: version.commit,
	format: version.format,
	source: {
		target_sets: 'sim/pokemon.ts:Pokemon.getMoveTargets',
		retargeting: 'sim/battle.ts:Battle.getTarget/getRandomTarget',
		hit_order: 'sim/battle-actions.ts:BattleActions.trySpreadMoveHit',
		spread_modifier: 'data/mods/champions/scripts.ts:BattleActions.modifyDamage',
		guards_and_redirection: 'data/moves.ts:protect/followme/ragepowder and data/conditions.ts:stall',
	},
	target_order: {
		all_adjacent: ['adjacent_allies', 'adjacent_foes'],
		all_adjacent_foes: ['adjacent_foes'],
	},
	per_target_hit_order: ['try_hit', 'type_immunity', 'move_immunity', 'accuracy', 'damage'],
	spread: {doubles_modifier: [3, 4], determined_before_target_failures: true},
	protect: {
		priority: protect.priority,
		duration: protect.condition.duration,
		stall_duration: stall.duration,
		repeat_chances: denominators,
		counter_multiplier: 3,
	},
	redirection: {
		follow_me_priority: followMe.condition.onFoeRedirectTargetPriority,
		rage_powder_priority: ragePowder.condition.onFoeRedirectTargetPriority,
		duration: followMe.condition.duration,
		rage_powder_is_powder: Boolean(ragePowder.flags.powder),
		spread_moves_redirected: false,
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
