#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'volatile-lifecycle.json');
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
	result.randomChance = (numerator, denominator) => !(numerator === 1 && denominator === 24);
	result.random = () => 0;
	result.randomizer = damage => damage;
	return result;
}

function usedMove(log, side, move) {
	return log.some(line => line.startsWith(`|move|${side}`) && line.includes(`|${move}|`));
}

function flinchCase(ability) {
	const current = battle(
		{species: 'Togekiss', ability: 'hustle', moves: ['airslash'], level: 50},
		{species: 'Blissey', ability, moves: ['swordsdance'], level: 50},
		'gen5,0000000000000011'
	);
	const target = current.p2.active[0];
	const start = current.log.length;
	current.makeChoices('move airslash 1, move splash', 'move swordsdance, move splash');
	return {
		ability,
		target_moved: usedMove(current.log.slice(start), 'p2a:', 'Swords Dance'),
		flinch_remaining: Boolean(target.volatiles.flinch),
		speed_stage: target.boosts.spe,
	};
}

const rock = battle(
	{species: 'Metagross', ability: 'clearbody', item: 'kingsrock', moves: ['tackle'], level: 50},
	{species: 'Blissey', ability: 'naturalcure', moves: ['swordsdance'], level: 50},
	'gen5,0000000000000012'
);
const rockStart = rock.log.length;
rock.makeChoices('move tackle 1, move splash', 'move swordsdance, move splash');

const herb = battle(
	{species: 'Aerodactyl', ability: 'sturdy', moves: ['taunt'], level: 50},
	{species: 'Blissey', ability: 'naturalcure', item: 'mentalherb', moves: ['splash'], level: 50},
	'gen5,0000000000000013'
);
const herbTarget = herb.p2.active[0];
const herbStart = herb.log.length;
herb.makeChoices('move taunt 1, move splash', 'move splash, move splash');

const output = {
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'complete pinned Showdown doubles turns with forced successful secondary rolls',
		move_execution: true,
		full_transition_comparison: true,
	},
	cases: [
		{id: 'air-slash-steadfast', ...flinchCase('steadfast')},
		{id: 'air-slash-inner-focus', ...flinchCase('innerfocus')},
		{
			id: 'kings-rock-flinch',
			target_moved: usedMove(rock.log.slice(rockStart), 'p2a:', 'Swords Dance'),
			item_retained: rock.p1.active[0].item === 'kingsrock',
		},
		{
			id: 'mental-herb-taunt',
			target_moved: usedMove(herb.log.slice(herbStart), 'p2a:', 'Splash'),
			item: herbTarget.item || null,
			taunt_remaining: Boolean(herbTarget.volatiles.taunt),
		},
	],
};
const contents = `${JSON.stringify(output, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Volatile lifecycle fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
