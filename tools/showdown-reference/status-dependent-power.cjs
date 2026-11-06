#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'status-dependent-power.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));
const filler = {species: 'Magikarp', ability: 'swiftswim', moves: ['splash'], level: 50};

function run(id, moveName, status, seed) {
	const set = (species, ability, moves) => ({species, ability, moves, level: 50, nature: 'Serious', evs: {}});
	const battle = new Battle({
		format: Dex.formats.get(version.format, true),
		seed,
		p1: {team: [set('Mew', 'synchronize', [moveName]), filler, filler, filler]},
		p2: {team: [set('Milotic', 'competitive', ['splash']), filler, filler, filler]},
	});
	battle.makeChoices('team 1234', 'team 1234');
	const source = battle.p1.active[0];
	const target = battle.p2.active[0];
	if (status) target.setStatus(status, source);
	let move = battle.dex.getActiveMove(moveName);
	move = battle.runEvent('ModifyMove', source, target, move, move);
	battle.setActiveMove(move, source, target);
	let resolvedBasePower = move.basePower;
	if (move.basePowerCallback) {
		resolvedBasePower = move.basePowerCallback.call(battle, source, target, move);
	} else if (move.onBasePower) {
		resolvedBasePower = battle.runEvent('BasePower', source, target, move, move.basePower, true);
	}
	battle.randomizer = value => value;
	const damage = battle.actions.getDamage(source, target, move);
	const hpBefore = target.hp;
	battle.makeChoices(`move ${moveName} 1, move splash`, 'move splash, move splash');
	return {
		id,
		move: moveName,
		target_status: status || null,
		resolved_base_power: resolvedBasePower,
		damage,
		hp_loss: hpBefore - target.hp,
		execution_path: 'complete Showdown doubles turn after direct callback inspection',
	};
}

const contents = `${JSON.stringify({
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'pinned status-dependent base-power callbacks plus complete Showdown doubles turns',
		damage_roll_percent: 100,
		move_execution: true,
		reference_derived_assertions: true,
		full_state_differential_comparison: false,
	},
	cases: [
		run('hex-healthy', 'hex', null, 'gen5,0000000000000041'),
		run('hex-burn', 'hex', 'brn', 'gen5,0000000000000042'),
		run('infernal-parade-paralysis', 'infernalparade', 'par', 'gen5,0000000000000043'),
		run('venoshock-burn', 'venoshock', 'brn', 'gen5,0000000000000044'),
		run('venoshock-poison', 'venoshock', 'psn', 'gen5,0000000000000045'),
		run('barb-barrage-toxic', 'barbbarrage', 'tox', 'gen5,0000000000000046'),
	],
}, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Status-dependent-power fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
