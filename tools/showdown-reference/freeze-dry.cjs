#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'freeze-dry.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));
const filler = species => ({species, ability: 'swiftswim', moves: ['splash'], level: 50});

function run(targetSpecies, seed) {
	const battle = new Battle({
		format: Dex.formats.get(version.format, true),
		seed,
		p1: {team: [
			{species: 'Glaceon', ability: 'snowcloak', moves: ['freezedry'], level: 50},
			filler('Magikarp'), filler('Feebas'), filler('Wishiwashi'),
		]},
		p2: {team: [
			{species: targetSpecies, ability: 'immunity', moves: ['splash'], level: 50},
			filler('Magikarp'), filler('Feebas'), filler('Wishiwashi'),
		]},
	});
	battle.makeChoices('team 1234', 'team 1234');
	const target = battle.p2.active[0];
	const move = battle.dex.getActiveMove('freezedry');
	const typeModifiers = target.getTypes().map(type => {
		const ordinary = battle.dex.getEffectiveness(move.type, type);
		const overridden = move.onEffectiveness?.call(battle, ordinary, target, type, move);
		return {type, ordinary, freeze_dry: overridden ?? ordinary};
	});
	const hpBefore = target.hp;
	battle.makeChoices('move freezedry 1, move splash', 'move splash, move splash');
	return {
		id: `freeze-dry-${targetSpecies.toLowerCase()}`,
		move: 'freezedry',
		target_species: targetSpecies.toLowerCase(),
		target_types: typeModifiers,
		combined_type_modifier: typeModifiers.reduce((sum, entry) => sum + entry.freeze_dry, 0),
		hp_loss: hpBefore - target.hp,
	};
}

const output = {
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'pinned Freeze-Dry onEffectiveness callback plus complete Showdown doubles turns',
		move_execution: true,
		reference_derived_assertions: true,
		full_state_differential_comparison: false,
	},
	cases: [
		run('Vaporeon', 'gen5,0000000000000021'),
		run('Kingdra', 'gen5,0000000000000022'),
		run('Lapras', 'gen5,0000000000000023'),
		run('Snorlax', 'gen5,0000000000000024'),
	],
};
const contents = `${JSON.stringify(output, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Freeze-Dry fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
