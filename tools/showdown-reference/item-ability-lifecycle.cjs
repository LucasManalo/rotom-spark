#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'item-ability-lifecycle.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));
const filler = species => ({species, ability: 'swiftswim', moves: ['splash'], level: 50});

function battle(source, target, seed) {
	const result = new Battle({
		format: Dex.formats.get(version.format, true),
		seed,
		p1: {team: [source, filler('Magikarp'), filler('Feebas'), filler('Wishiwashi')]},
		p2: {team: [target, filler('Magikarp'), filler('Feebas'), filler('Wishiwashi')]},
	});
	result.makeChoices('team 1234', 'team 1234');
	result.randomChance = () => true;
	result.randomizer = damage => damage;
	return result;
}

function run(source, target, _move, seed) {
	const current = battle(source, target, seed);
	current.makeChoices('move 1 1, move 1', 'move 1, move 1');
	return current;
}

const knockOff = run(
	{species: 'Pangoro', ability: 'ironfist', moves: ['knockoff'], level: 50},
	{species: 'Hawlucha', ability: 'unburden', item: 'leftovers', moves: ['splash'], level: 50},
	'gen5,0000000000000017'
);
const stickyKnockOff = run(
	{species: 'Pangoro', ability: 'ironfist', moves: ['knockoff'], level: 50},
	{species: 'Muk', ability: 'stickyhold', item: 'leftovers', moves: ['splash'], level: 50},
	'gen5,0000000000000018'
);
const trick = run(
	{species: 'Reuniclus', ability: 'magicguard', item: 'muscleband', moves: ['trick'], level: 50},
	{species: 'Blastoise', ability: 'torrent', item: 'leftovers', moves: ['splash'], level: 50},
	'gen5,0000000000000019'
);
const stickyTrick = run(
	{species: 'Reuniclus', ability: 'magicguard', item: 'muscleband', moves: ['trick'], level: 50},
	{species: 'Muk', ability: 'stickyhold', item: 'leftovers', moves: ['splash'], level: 50},
	'gen5,000000000000001a'
);
const mummy = run(
	{species: 'Hawlucha', ability: 'limber', moves: ['aerialace'], level: 50},
	{species: 'Cofagrigus', ability: 'mummy', moves: ['splash'], level: 50},
	'gen5,000000000000001b'
);
const wandering = run(
	{species: 'Hawlucha', ability: 'limber', moves: ['aerialace'], level: 50},
	{species: 'Runerigus', ability: 'wanderingspirit', moves: ['splash'], level: 50},
	'gen5,000000000000001c'
);

const output = {
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'complete pinned Showdown doubles turns with deterministic successful hits',
		move_execution: true,
		reference_derived_assertions: true,
		full_state_differential_comparison: false,
	},
	cases: [
		{
			id: 'knock-off-unburden',
			item: knockOff.p2.active[0].item || null,
			last_item: knockOff.p2.active[0].lastItem || null,
			unburden: Boolean(knockOff.p2.active[0].volatiles.unburden),
		},
		{id: 'knock-off-sticky-hold', item: stickyKnockOff.p2.active[0].item || null},
		{
			id: 'trick-exchange',
			source_item: trick.p1.active[0].item || null,
			target_item: trick.p2.active[0].item || null,
		},
		{
			id: 'trick-sticky-hold',
			source_item: stickyTrick.p1.active[0].item || null,
			target_item: stickyTrick.p2.active[0].item || null,
		},
		{id: 'mummy-replacement', source_ability: mummy.p1.active[0].ability},
		{
			id: 'wandering-spirit-swap',
			source_ability: wandering.p1.active[0].ability,
			target_ability: wandering.p2.active[0].ability,
		},
	],
};
const contents = `${JSON.stringify(output, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Item/ability lifecycle fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
