#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'move-phazing.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));
const filler = species => ({species, ability: 'swiftswim', moves: ['splash'], level: 50});

function run(move, targetAbility, seed) {
	const battle = new Battle({
		format: Dex.formats.get(version.format, true),
		seed,
		p1: {team: [
			{species: 'Aerodactyl', ability: 'noguard', moves: [move], level: 50},
			filler('Magikarp'), filler('Feebas'), filler('Wishiwashi'),
		]},
		p2: {team: [
			{species: 'Blissey', ability: targetAbility, moves: ['splash'], level: 50},
			filler('Magikarp'), filler('Feebas'), filler('Wishiwashi'),
		]},
	});
	battle.makeChoices('team 1234', 'team 1234');
	const original = battle.p2.active[0];
	battle.makeChoices(`move ${move} 1, move splash`, 'move splash, move splash');
	return {
		id: `${move}-${targetAbility}`,
		move,
		target_ability: targetAbility,
		original_active: original.isActive,
		active_species: battle.p2.active[0].species.id,
		forced_switch: battle.p2.active[0] !== original,
		request_state: battle.requestState,
	};
}

const output = {
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'complete pinned Showdown doubles turns using move forceSwitch and DragOut prevention',
		move_execution: true,
		reference_derived_assertions: true,
		full_state_differential_comparison: false,
	},
	cases: [
		run('circlethrow', 'naturalcure', 'gen5,0000000000000011'),
		run('dragontail', 'naturalcure', 'gen5,0000000000000012'),
		run('roar', 'naturalcure', 'gen5,0000000000000013'),
		run('whirlwind', 'naturalcure', 'gen5,0000000000000014'),
		run('roar', 'suctioncups', 'gen5,0000000000000015'),
	],
};
const contents = `${JSON.stringify(output, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Move-phazing fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
