#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'mega-evolution-lifecycle.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));
const filler = species => ({species, ability: 'sturdy', moves: ['swordsdance'], level: 50});

function createBattle(source, opponent, seed) {
	const battle = new Battle({
		format: Dex.formats.get(version.format, true),
		seed,
		p1: {team: [source, filler('Shuckle'), filler('Shuckle'), filler('Shuckle')]},
		p2: {team: [opponent, filler('Shuckle'), filler('Shuckle'), filler('Shuckle')]},
	});
	battle.makeChoices('team 1234', 'team 1234');
	return battle;
}

function moveOrder(log) {
	return log
		.filter(line => line.startsWith('|move|'))
		.map(line => line.split('|')[2]);
}

const charizardBattle = createBattle(
	{species: 'Charizard', ability: 'blaze', item: 'charizarditex', moves: ['swordsdance'], level: 50},
	{species: 'Granbull', ability: 'intimidate', moves: ['swordsdance'], level: 50},
	'gen5,0000000000000014'
);
const charizard = charizardBattle.p1.active[0];
charizard.sethp(charizard.maxhp - 17);
const charizardBefore = {hp: charizard.hp, max_hp: charizard.maxhp};
const charizardStart = charizardBattle.log.length;
charizardBattle.makeChoices('move swordsdance mega, move swordsdance', 'move swordsdance, move swordsdance');
const charizardCase = {
	before: charizardBefore,
	after: {
		species: charizard.species.id,
		ability: charizard.ability,
		item: charizard.item,
		hp: charizard.hp,
		max_hp: charizard.maxhp,
	},
	move_order: moveOrder(charizardBattle.log.slice(charizardStart)),
};

const abomasnowBattle = createBattle(
	{species: 'Abomasnow', ability: 'snowwarning', item: 'abomasite', moves: ['swordsdance'], level: 50},
	{species: 'Granbull', ability: 'intimidate', moves: ['swordsdance'], level: 50},
	'gen5,0000000000000015'
);
const abomasnow = abomasnowBattle.p1.active[0];
const abomasnowStart = abomasnowBattle.log.length;
abomasnowBattle.makeChoices('move swordsdance mega, move swordsdance', 'move swordsdance, move swordsdance');
const abomasnowCase = {
	species: abomasnow.species.id,
	ability: abomasnow.ability,
	weather: abomasnowBattle.field.weather || null,
	move_order: moveOrder(abomasnowBattle.log.slice(abomasnowStart)),
};

const incompatibleBattle = createBattle(
	{species: 'Charizard', ability: 'blaze', item: 'gyaradosite', moves: ['swordsdance'], level: 50},
	{species: 'Granbull', ability: 'intimidate', moves: ['swordsdance'], level: 50},
	'gen5,0000000000000016'
);
const incompatible = incompatibleBattle.p1.active[0];

const output = {
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'complete pinned Showdown doubles turns using move-plus-Mega requests',
		move_execution: true,
		reference_derived_assertions: true,
		full_state_differential_comparison: false,
	},
	cases: [
		{id: 'charizard-mega-x-damage-and-item', ...charizardCase},
		{id: 'abomasnow-mega-speed-and-ability-start', ...abomasnowCase},
		{
			id: 'incompatible-stone',
			can_mega: incompatibleBattle.actions.canMegaEvo(incompatible),
			species: incompatible.species.id,
			item: incompatible.item,
		},
	],
};
const contents = `${JSON.stringify(output, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Mega Evolution lifecycle fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
