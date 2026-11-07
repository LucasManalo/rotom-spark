#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'stat-dependent-power.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));
const filler = {species: 'Magikarp', ability: 'swiftswim', moves: ['splash'], level: 50};

function run(id, moveName, boosts, targetAbility, seed) {
	const set = (species, ability, moves) => ({species, ability, moves, level: 50, nature: 'Serious', evs: {}});
	const battle = new Battle({
		format: Dex.formats.get(version.format, true),
		seed,
		p1: {team: [set('Mew', 'synchronize', [moveName]), filler, filler, filler]},
		p2: {team: [set('Snorlax', targetAbility, ['splash']), filler, filler, filler]},
	});
	battle.makeChoices('team 1234', 'team 1234');
	const source = battle.p1.active[0];
	const target = battle.p2.active[0];
	Object.assign(source.boosts, boosts);
	let move = battle.dex.getActiveMove(moveName);
	move = battle.runEvent('ModifyMove', source, target, move, move);
	const resolvedBasePower = move.basePowerCallback.call(battle, source, target, move);
	battle.randomizer = value => value;
	const hpBefore = target.hp;
	battle.makeChoices(`move ${moveName} 1, move splash`, 'move splash, move splash');
	return {
		id,
		move: moveName,
		boosts,
		target_ability: targetAbility,
		resolved_base_power: resolvedBasePower,
		hp_loss: hpBefore - target.hp,
		execution_path: 'complete Showdown doubles turn after direct callback inspection',
	};
}

const contents = `${JSON.stringify({
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'pinned positiveBoosts base-power callbacks plus complete Showdown doubles turns',
		damage_roll_percent: 100,
		move_execution: true,
		reference_derived_assertions: true,
		full_state_differential_comparison: false,
	},
	cases: [
		run('stored-power-unboosted', 'storedpower', {}, 'immunity', 'gen5,0000000000000061'),
		run('stored-power-mixed-stages', 'storedpower', {spa: 2, spe: 1, def: -3}, 'immunity', 'gen5,0000000000000062'),
		run('stored-power-unaware', 'storedpower', {spa: 2, accuracy: 1}, 'unaware', 'gen5,0000000000000063'),
		run('power-trip-mixed-stages', 'powertrip', {atk: 2, spe: 1, evasion: 1, def: -2}, 'immunity', 'gen5,0000000000000064'),
		run('power-trip-max-stages', 'powertrip', {atk: 6, def: 6, spa: 6, spd: 6, spe: 6, accuracy: 6, evasion: 6}, 'immunity', 'gen5,0000000000000065'),
	],
}, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Stat-dependent-power fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
