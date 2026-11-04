#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'forced-switch-lifecycle.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));
const filler = species => ({species, ability: 'swiftswim', moves: ['splash'], level: 50});

function createBattle(sourceAbility, targetItem, seed) {
	return new Battle({
		format: Dex.formats.get(version.format, true),
		seed,
		p1: {team: [
			{species: 'Aerodactyl', ability: sourceAbility, moves: ['tackle'], level: 50},
			filler('Magikarp'), filler('Feebas'), filler('Wishiwashi'),
		]},
		p2: {team: [
			{species: 'Blissey', ability: 'naturalcure', item: targetItem, moves: ['splash'], level: 50},
			filler('Magikarp'), filler('Feebas'), filler('Wishiwashi'),
		]},
	});
}

function select(battle) {
	battle.makeChoices('team 1234', 'team 1234');
}

function attack(battle) {
	battle.makeChoices('move tackle 1, move splash', 'move splash, move splash');
}

const ejectBattle = createBattle('sturdy', 'ejectbutton', 'gen5,0000000000000001');
select(ejectBattle);
const ejectHolder = ejectBattle.p2.active[0];
attack(ejectBattle);
const ejectPending = {
	request_state: ejectBattle.requestState,
	holder_item: ejectHolder.item || null,
	holder_switch_flag: Boolean(ejectHolder.switchFlag),
	queued_choices: ejectBattle.queue.list.map(action => action.choice),
};
ejectBattle.makeChoices('', 'switch 3');
const ejectSettled = {
	request_state: ejectBattle.requestState,
	incoming_species: ejectBattle.p2.active[0].species.id,
	outgoing_active: ejectHolder.isActive,
};

const redBattle = createBattle('sturdy', 'redcard', 'gen5,0000000000000002');
select(redBattle);
const redSource = redBattle.p1.pokemon[0];
const redHolder = redBattle.p2.pokemon[0];
attack(redBattle);
const redCard = {
	holder_item: redHolder.item || null,
	source_active: redSource.isActive,
	incoming_species: redBattle.p1.active[0].species.id,
	request_state: redBattle.requestState,
};

const cupsBattle = createBattle('suctioncups', 'redcard', 'gen5,0000000000000003');
select(cupsBattle);
const cupsSource = cupsBattle.p1.pokemon[0];
const cupsHolder = cupsBattle.p2.pokemon[0];
attack(cupsBattle);
const suctionCups = {
	holder_item: cupsHolder.item || null,
	source_active: cupsSource.isActive,
	active_species: cupsBattle.p1.active[0].species.id,
};

const output = {
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'complete pinned Showdown doubles turns using Eject Button and Red Card',
		move_execution: true,
		reference_derived_assertions: true,
		full_state_differential_comparison: false,
	},
	cases: [
		{id: 'eject-button-pauses-queue', pending: ejectPending, settled: ejectSettled},
		{id: 'red-card-random-drag', ...redCard},
		{id: 'red-card-suction-cups', ...suctionCups},
	],
};
const contents = `${JSON.stringify(output, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Forced-switch lifecycle fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
