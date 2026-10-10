#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'basic-damage.json');
const cases = [
	{
		name: 'dragon-pulse-neutral-stab',
		move: 'dragonpulse',
		source: {species: 'Goodra', ability: 'gooey', nature: 'Modest', evs: {spa: 32}, moves: ['dragonpulse', 'splash']},
		target: {species: 'Arcanine', ability: 'intimidate', nature: 'Serious', evs: {hp: 32}, moves: ['splash']},
	},
	{
		name: 'power-gem-quad-effective-stab',
		move: 'powergem',
		source: {species: 'Glimmora', ability: 'corrosion', nature: 'Modest', evs: {spa: 32}, moves: ['powergem', 'splash']},
		target: {species: 'Charizard', ability: 'blaze', nature: 'Serious', evs: {hp: 32}, moves: ['splash']},
	},
	{
		name: 'high-horsepower-super-effective-stab',
		move: 'highhorsepower',
		source: {species: 'Mudsdale', ability: 'owntempo', nature: 'Adamant', evs: {atk: 32}, moves: ['highhorsepower', 'splash']},
		target: {species: 'Arcanine', ability: 'flashfire', nature: 'Serious', evs: {hp: 32}, moves: ['splash']},
	},
];

function calculate(entry) {
	const format = Dex.formats.get('gen9championsvgc2026regmc', true);
	const filler = {species: 'Magikarp', ability: 'swiftswim', moves: ['splash'], level: 50};
	const fillTeam = lead => [lead, filler, filler, filler, filler, filler];
	const battle = new Battle({
		format,
		seed: 'gen5,0000000000000001',
		p1: {team: fillTeam({...entry.source, level: 50})},
		p2: {team: fillTeam({...entry.target, level: 50})},
	});
	battle.makeChoices('team 1234', 'team 1234');
	battle.randomizer = damage => damage;
	const source = battle.p1.active[0];
	const target = battle.p2.active[0];
	const move = battle.dex.getActiveMove(entry.move);
	move.willCrit = false;
	const damage = battle.actions.getDamage(source, target, move);
	const result = {
		name: entry.name,
		move: entry.move,
		source: {
			species: source.species.id,
			level: source.level,
			stats: source.storedStats,
			types: source.getTypes(),
		},
		target: {
			species: target.species.id,
			max_hp: target.maxhp,
			stats: target.storedStats,
			types: target.getTypes(),
		},
		damage,
	};
	battle.destroy();
	return result;
}

const output = {
	showdown_commit: 'b79ce06d06b8f0dc8de8bb348bc9cab5d931553f',
	format: 'gen9championsvgc2026regmc',
	random: {critical: false, damage_percent: 100},
	cases: cases.map(calculate),
};
const contents = `${JSON.stringify(output, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Basic damage fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.mkdirSync(path.dirname(outputPath), {recursive: true});
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
