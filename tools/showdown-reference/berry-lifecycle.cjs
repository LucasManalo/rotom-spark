#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'berry-lifecycle.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));
const filler = () => ({species: 'Magikarp', ability: 'swiftswim', moves: ['splash'], level: 50});
const set = (species, ability, item = '') => ({species, ability, item, moves: ['splash'], level: 50});

function battleFor(holderAbility, holderItem, allyAbility = 'swiftswim', allyItem = '') {
	const battle = new Battle({
		format: Dex.formats.get(version.format, true),
		seed: 'gen5,0000000000000001',
		p1: {team: [filler(), filler(), filler(), filler()]},
		p2: {team: [
			set('Metagross', holderAbility, holderItem),
			set('Magikarp', allyAbility, allyItem),
			filler(), filler(),
		]},
	});
	battle.makeChoices('team 1234', 'team 1234');
	return battle;
}

function eatCase(id, ability, item, hp) {
	const battle = battleFor(ability, item);
	const holder = battle.p2.active[0];
	holder.hp = hp;
	const before = holder.hp;
	const consumed = holder.eatItem(true);
	return {
		id,
		ability,
		item,
		consumed,
		hp_before: before,
		hp_after: holder.hp,
		item_after: holder.item || null,
		last_item: holder.lastItem || null,
	};
}

const cheekPouch = eatCase('sitrus-cheek-pouch', 'cheekpouch', 'sitrusberry', 50);

const cudBattle = battleFor('cudchew', 'sitrusberry');
const cudHolder = cudBattle.p2.active[0];
cudHolder.hp = 50;
cudBattle.queue.list.push({choice: 'move'});
cudHolder.eatItem(true);
cudBattle.queue.list.length = 0;
const cudScheduled = {
	berry: cudHolder.abilityState.berry?.id || null,
	counter: cudHolder.abilityState.counter,
};
cudBattle.fieldEvent('Residual');
const cudAfterOne = {hp: cudHolder.hp, counter: cudHolder.abilityState.counter};
cudHolder.hp = 40;
cudBattle.fieldEvent('Residual');
const cudAfterTwo = {
	hp: cudHolder.hp,
	berry: cudHolder.abilityState.berry?.id || null,
	counter: cudHolder.abilityState.counter ?? null,
};

const harvestBattle = battleFor('harvest', 'sitrusberry');
const harvestHolder = harvestBattle.p2.active[0];
harvestHolder.hp = 50;
harvestHolder.eatItem(true);
harvestBattle.field.setWeather('sunnyday', harvestHolder);
harvestBattle.fieldEvent('Residual');

const symbiosisBattle = battleFor('clearbody', 'sitrusberry', 'symbiosis', 'leftovers');
const symbiosisHolder = symbiosisBattle.p2.active[0];
const symbiosisAlly = symbiosisBattle.p2.active[1];
symbiosisHolder.hp = 50;
symbiosisHolder.eatItem(true);

const output = {
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'pinned Pokemon.eatItem plus resolved EatItem, AfterUseItem, and Residual callbacks',
		move_execution: false,
		full_transition_comparison: false,
	},
	cases: [
		cheekPouch,
		{
			id: 'sitrus-cud-chew', ability: 'cudchew', item: 'sitrusberry',
			scheduled: cudScheduled, after_one_residual: cudAfterOne, after_two_residuals: cudAfterTwo,
		},
		{
			id: 'sitrus-harvest-sun', ability: 'harvest', item: 'sitrusberry',
			item_after_residual: harvestHolder.item || null,
			last_item_after_residual: harvestHolder.lastItem || null,
		},
		{
			id: 'sitrus-symbiosis', ability: 'symbiosis', item: 'sitrusberry',
			holder_item_after: symbiosisHolder.item || null,
			ally_item_after: symbiosisAlly.item || null,
		},
	],
};
const contents = `${JSON.stringify(output, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Berry-lifecycle fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
