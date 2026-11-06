#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'hp-dependent-power.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));
const filler = {species: 'Magikarp', ability: 'swiftswim', moves: ['splash'], level: 50};

function run(id, moveName, hpNumerator, hpDenominator, seed) {
	const set = (species, ability, moves) => ({species, ability, moves, level: 50, nature: 'Serious', evs: {}});
	const battle = new Battle({
		format: Dex.formats.get(version.format, true),
		seed,
		p1: {team: [set('Mew', 'synchronize', [moveName]), filler, filler, filler]},
		p2: {team: [set('Snorlax', 'immunity', ['splash']), filler, filler, filler]},
	});
	battle.makeChoices('team 1234', 'team 1234');
	const source = battle.p1.active[0];
	const target = battle.p2.active[0];
	source.hp = Math.max(Math.floor(source.maxhp * hpNumerator / hpDenominator), 1);
	let move = battle.dex.getActiveMove(moveName);
	move = battle.runEvent('ModifyMove', source, target, move, move);
	const callbackBasePower = move.basePowerCallback.call(battle, source, target, move);
	const resolvedBasePower = Math.max(Math.floor(callbackBasePower), 1);
	battle.randomizer = value => value;
	const hpBefore = target.hp;
	const choice = ['eruption', 'waterspout'].includes(moveName)
		? `move ${moveName}, move splash`
		: `move ${moveName} 1, move splash`;
	battle.makeChoices(choice, 'move splash, move splash');
	return {
		id,
		move: moveName,
		source_hp: source.hp,
		source_max_hp: source.maxhp,
		resolved_base_power: resolvedBasePower,
		callback_base_power: callbackBasePower,
		hp_loss: hpBefore - target.hp,
		execution_path: 'complete Showdown doubles turn after direct callback inspection',
	};
}

const contents = `${JSON.stringify({
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'pinned HP-dependent base-power callbacks plus complete Showdown doubles turns',
		damage_roll_percent: 100,
		move_execution: true,
		reference_derived_assertions: true,
		full_state_differential_comparison: false,
	},
	cases: [
		run('eruption-half-hp', 'eruption', 1, 2, 'gen5,0000000000000051'),
		run('water-spout-one-hp', 'waterspout', 1, 999, 'gen5,0000000000000052'),
		run('flail-full-hp', 'flail', 1, 1, 'gen5,0000000000000053'),
		run('flail-ratio-16', 'flail', 1, 3, 'gen5,0000000000000054'),
		run('reversal-ratio-4', 'reversal', 1, 10, 'gen5,0000000000000055'),
		run('reversal-ratio-1', 'reversal', 1, 48, 'gen5,0000000000000056'),
	],
}, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('HP-dependent-power fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
