#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'sequenced-effects.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));

const filler = () => ({species: 'Magikarp', ability: 'swiftswim', moves: ['splash'], level: 50});
const set = (species, moves, ability) => ({species, ability, moves, level: 50, nature: 'Serious', evs: {}});
const pad = team => [...team, ...Array.from({length: 4 - team.length}, filler)];

function makeBattle(move) {
	const battle = new Battle({
		format: Dex.formats.get(version.format, true), seed: 'gen5,0000000000000001',
		p1: {team: pad([set('Goodra', [move], 'sapsipper'), filler()])},
		p2: {team: pad([set('Blastoise', ['splash'], 'torrent'), filler()])},
	});
	battle.makeChoices('team 1234', 'team 1234');
	battle.randomizer = damage => damage;
	return battle;
}

function use(move, configure = () => {}) {
	const battle = makeBattle(move);
	const source = battle.p1.active[0];
	const target = battle.p2.active[0];
	configure({battle, source, target});
	const sourceHp = source.hp;
	const targetHp = target.hp;
	const start = battle.log.length;
	const result = battle.actions.useMove(move, source, {target});
	const moveLog = battle.log.slice(start);
	const hitCount = moveLog.findLast(line => line.startsWith('|-hitcount|'));
	return {
		result: Boolean(result),
		source: {hp_before: sourceHp, hp_after: source.hp, status: source.status || null, boosts: {...source.boosts}},
		target: {hp_before: targetHp, hp_after: target.hp, status: target.status || null, boosts: {...target.boosts}},
		damage_events: moveLog.filter(line => line.startsWith('|-damage|')).length,
		hit_count: hitCount ? Number(hitCount.split('|').at(-1)) : null,
	};
}

const neverCrit = ({battle}) => {
	battle.randomChance = (numerator, denominator) => numerator === denominator;
};
const alwaysHitNeverCrit = ({battle}) => {
	battle.randomChance = (numerator, denominator) => !(numerator === 1 && denominator === 24);
};

const facadeClear = use('facade', neverCrit);
const facadeBurned = use('facade', context => {
	context.source.setStatus('brn', context.source);
	neverCrit(context);
});
const flamethrower = use('flamethrower', ({battle}) => {
	battle.random = () => 0;
});
const bulletSeed = use('bulletseed', ({battle}) => {
	battle.randomChance = (numerator, denominator) => numerator === denominator;
	battle.sample = values => values[0];
});
const populationBomb = use('populationbomb', ({battle, target}) => {
	target.baseMaxhp = 999;
	target.maxhp = 999;
	target.hp = 999;
	battle.random = () => 0;
});
const seismicToss = use('seismictoss', neverCrit);
const closeCombat = use('closecombat', neverCrit);
const chloroblast = use('chloroblast', alwaysHitNeverCrit);
const gigaDrain = use('gigadrain', context => {
	context.source.hp = Math.floor(context.source.maxhp / 2);
	context.target.hp = 10;
	neverCrit(context);
});
const doubleEdge = use('doubleedge', context => {
	context.target.hp = 10;
	neverCrit(context);
});

const output = {
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'pinned Battle.actions.useMove with resolved Champions data',
		move_execution: true,
		full_transition_comparison: false,
		rng_comparison: 'Controlled semantic outcomes; raw PRNG seed/state parity is not claimed.',
	},
	cases: {
		facade: {
			clear_damage: facadeClear.target.hp_before - facadeClear.target.hp_after,
			burned_damage: facadeBurned.target.hp_before - facadeBurned.target.hp_after,
		},
		flamethrower: {status: flamethrower.target.status},
		bullet_seed: {hits: bulletSeed.hit_count},
		population_bomb: {hits: populationBomb.hit_count},
		seismic_toss: {damage: seismicToss.target.hp_before - seismicToss.target.hp_after},
		close_combat: {source_boosts: closeCombat.source.boosts},
		chloroblast: {self_damage: chloroblast.source.hp_before - chloroblast.source.hp_after},
		giga_drain: {
			damage: gigaDrain.target.hp_before - gigaDrain.target.hp_after,
			healing: gigaDrain.source.hp_after - gigaDrain.source.hp_before,
		},
		double_edge: {
			damage: doubleEdge.target.hp_before - doubleEdge.target.hp_after,
			recoil: doubleEdge.source.hp_before - doubleEdge.source.hp_after,
		},
	},
};
const contents = `${JSON.stringify(output, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Sequenced-effects fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.mkdirSync(path.dirname(outputPath), {recursive: true});
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
