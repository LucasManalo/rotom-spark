#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'basic-damage.json');
const noPoints = {hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0};
const points = values => ({...noPoints, ...values});
const cases = [
	{
		name: 'dragon-pulse-neutral-stab',
		move: 'dragonpulse',
		source: {species: 'Goodra', ability: 'gooey', nature: 'Modest', statPoints: points({spa: 32})},
		target: {species: 'Arcanine', ability: 'flashfire', nature: 'Serious', statPoints: points({hp: 32})},
	},
	{
		name: 'dragon-claw-neutral-stab',
		move: 'dragonclaw',
		source: {species: 'Garchomp', ability: 'roughskin', nature: 'Adamant', statPoints: points({atk: 32})},
		target: {species: 'Arcanine', ability: 'flashfire', nature: 'Serious', statPoints: points({hp: 32})},
	},
	{
		name: 'power-gem-quad-effective-stab',
		move: 'powergem',
		source: {species: 'Glimmora', ability: 'corrosion', nature: 'Modest', statPoints: points({spa: 32})},
		target: {species: 'Charizard', ability: 'blaze', nature: 'Serious', statPoints: points({hp: 32})},
	},
	{
		name: 'high-horsepower-super-effective-stab',
		move: 'highhorsepower',
		source: {species: 'Mudsdale', ability: 'owntempo', nature: 'Adamant', statPoints: points({atk: 32})},
		target: {species: 'Arcanine', ability: 'flashfire', nature: 'Serious', statPoints: points({hp: 32})},
	},
	{
		name: 'dragon-pulse-critical',
		move: 'dragonpulse',
		critical: true,
		source: {species: 'Goodra', ability: 'gooey', nature: 'Modest', statPoints: points({spa: 32})},
		target: {species: 'Arcanine', ability: 'flashfire', nature: 'Serious', statPoints: points({hp: 32})},
	},
	{
		name: 'dragon-pulse-positive-offense-stage',
		move: 'dragonpulse',
		source: {
			species: 'Goodra', ability: 'gooey', nature: 'Modest', statPoints: points({spa: 32}), stages: {spa: 1},
		},
		target: {species: 'Arcanine', ability: 'flashfire', nature: 'Serious', statPoints: points({hp: 32})},
	},
	{
		name: 'dragon-pulse-negative-offense-stage',
		move: 'dragonpulse',
		source: {
			species: 'Goodra', ability: 'gooey', nature: 'Modest', statPoints: points({spa: 32}), stages: {spa: -2},
		},
		target: {species: 'Arcanine', ability: 'flashfire', nature: 'Serious', statPoints: points({hp: 32})},
	},
	{
		name: 'dragon-pulse-positive-defense-stage',
		move: 'dragonpulse',
		source: {species: 'Goodra', ability: 'gooey', nature: 'Modest', statPoints: points({spa: 32})},
		target: {
			species: 'Arcanine', ability: 'flashfire', nature: 'Serious', statPoints: points({hp: 32}), stages: {spd: 2},
		},
	},
	{
		name: 'dragon-pulse-negative-defense-stage',
		move: 'dragonpulse',
		source: {species: 'Goodra', ability: 'gooey', nature: 'Modest', statPoints: points({spa: 32})},
		target: {
			species: 'Arcanine', ability: 'flashfire', nature: 'Serious', statPoints: points({hp: 32}), stages: {spd: -1},
		},
	},
	{
		name: 'critical-ignores-negative-offense-and-positive-defense',
		move: 'dragonpulse',
		critical: true,
		source: {
			species: 'Goodra', ability: 'gooey', nature: 'Modest', statPoints: points({spa: 32}), stages: {spa: -3},
		},
		target: {
			species: 'Arcanine', ability: 'flashfire', nature: 'Serious', statPoints: points({hp: 32}), stages: {spd: 4},
		},
	},
	{
		name: 'critical-keeps-positive-offense-and-negative-defense',
		move: 'dragonpulse',
		critical: true,
		source: {
			species: 'Goodra', ability: 'gooey', nature: 'Modest', statPoints: points({spa: 32}), stages: {spa: 2},
		},
		target: {
			species: 'Arcanine', ability: 'flashfire', nature: 'Serious', statPoints: points({hp: 32}), stages: {spd: -2},
		},
	},
	{
		name: 'aqua-jet-low-damage-quarter-resisted',
		move: 'aquajet',
		source: {species: 'Happiny', ability: 'naturalcure', nature: 'Serious', statPoints: points({})},
		target: {species: 'Palkia', ability: 'pressure', nature: 'Serious', statPoints: points({spd: 32})},
	},
	{
		name: 'high-horsepower-immunity',
		move: 'highhorsepower',
		source: {species: 'Mudsdale', ability: 'owntempo', nature: 'Adamant', statPoints: points({atk: 32})},
		target: {species: 'Charizard', ability: 'blaze', nature: 'Serious', statPoints: points({hp: 32})},
	},
];

function showdownSet(input, move) {
	return {
		species: input.species,
		ability: input.ability,
		nature: input.nature,
		evs: input.statPoints,
		moves: [move, 'splash'],
		level: 50,
	};
}

function fixturePokemon(input, pokemon) {
	return {
		species: pokemon.species.id,
		ability: input.ability,
		nature: input.nature.toLowerCase(),
		stat_points: input.statPoints,
		stages: {...noPoints, accuracy: 0, evasion: 0, ...(input.stages || {})},
		level: pokemon.level,
		stats: {hp: pokemon.maxhp, ...pokemon.storedStats},
		types: pokemon.getTypes(),
	};
}

function calculate(entry) {
	const format = Dex.formats.get('gen9championsvgc2026regmc', true);
	const filler = {species: 'Magikarp', ability: 'swiftswim', moves: ['splash'], level: 50};
	const fillTeam = lead => [lead, filler, filler, filler, filler, filler];
	const battle = new Battle({
		format,
		seed: 'gen5,0000000000000001',
		p1: {team: fillTeam(showdownSet(entry.source, entry.move))},
		p2: {team: fillTeam(showdownSet(entry.target, 'splash'))},
	});
	battle.makeChoices('team 1234', 'team 1234');
	const source = battle.p1.active[0];
	const target = battle.p2.active[0];
	Object.assign(source.boosts, entry.source.stages || {});
	Object.assign(target.boosts, entry.target.stages || {});
	const rolls = [];
	for (let percent = 85; percent <= 100; percent++) {
		battle.randomizer = damage => Math.floor(Math.floor(damage * percent) / 100);
		const move = battle.dex.getActiveMove(entry.move);
		move.willCrit = Boolean(entry.critical);
		const damage = battle.actions.getDamage(source, target, move);
		rolls.push({percent, damage: damage === false ? 0 : damage});
	}
	const result = {
		name: entry.name,
		move: entry.move,
		critical: Boolean(entry.critical),
		source: fixturePokemon(entry.source, source),
		target: fixturePokemon(entry.target, target),
		rolls,
	};
	battle.destroy();
	return result;
}

const output = {
	showdown_commit: 'b79ce06d06b8f0dc8de8bb348bc9cab5d931553f',
	format: 'gen9championsvgc2026regmc',
	random: {damage_percent_min: 85, damage_percent_max: 100},
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
