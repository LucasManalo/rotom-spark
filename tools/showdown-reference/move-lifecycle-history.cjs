#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'move-lifecycle-history.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));
const filler = species => ({species, ability: 'swiftswim', moves: ['splash'], level: 50});

function run(source, target, seed) {
	const battle = new Battle({
		format: Dex.formats.get(version.format, true),
		seed,
		p1: {team: [source, filler('Magikarp'), filler('Feebas'), filler('Wishiwashi')]},
		p2: {team: [target, filler('Magikarp'), filler('Feebas'), filler('Wishiwashi')]},
	});
	battle.makeChoices('team 1234', 'team 1234');
	battle.randomChance = () => true;
	battle.randomizer = damage => damage;
	battle.makeChoices('move 1 1, move 1', 'move 1, move 1');
	return battle;
}

function history(battle) {
	const source = battle.p1.active[0];
	const target = battle.p2.active[0];
	const attackedBy = target.attackedBy.at(-1);
	return {
		source_last_move: source.lastMove?.id || null,
		source_move_this_turn: source.moveThisTurn || null,
		source_move_last_turn_result: source.moveLastTurnResult ?? null,
		source_last_damage: source.lastDamage,
		target_last_attacked_by: attackedBy ? {
			move: attackedBy.move,
			damage: attackedBy.damage,
			this_turn: attackedBy.thisTurn,
		} : null,
	};
}

const multiHit = run(
	{species: 'Cinccino', ability: 'skilllink', moves: ['tailslap'], level: 50},
	{species: 'Snorlax', ability: 'thickfat', moves: ['splash'], level: 50},
	'gen5,000000000000001d'
);
const priorityBlocked = run(
	{species: 'Medicham', ability: 'purepower', moves: ['aquajet'], level: 50},
	{species: 'Farigiraf', ability: 'armortail', moves: ['splash'], level: 50},
	'gen5,000000000000001e'
);
const protectedBattle = run(
	{species: 'Medicham', ability: 'purepower', moves: ['aquatail'], level: 50},
	{species: 'Snorlax', ability: 'thickfat', moves: ['protect'], level: 50},
	'gen5,000000000000001f'
);

const output = {
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'complete pinned Showdown doubles turns with controlled hit and damage outcomes',
		move_execution: true,
		full_transition_comparison: true,
	},
	cases: [
		{id: 'multi-hit-success-history', ...history(multiHit)},
		{id: 'priority-blocked-failure-history', ...history(priorityBlocked)},
		{id: 'protected-success-history', ...history(protectedBattle)},
	],
};
const contents = `${JSON.stringify(output, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Move lifecycle/history fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
