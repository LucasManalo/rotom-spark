#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'ability-modifiers.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));
const filler = {species: 'Magikarp', ability: 'swiftswim', moves: ['splash'], level: 50};

function damage({id, sourceAbility, move, targetAbility, sourceStatus = '', targetStatus = ''}) {
	const set = (species, ability, moves) => ({species, ability, moves, level: 50, nature: 'Serious', evs: {}});
	const pad = lead => [lead, filler, filler, filler];
	const battle = new Battle({
		format: Dex.formats.get(version.format, true),
		seed: 'gen5,0000000000000001',
		p1: {team: pad(set('Medicham', sourceAbility, [move]))},
		p2: {team: pad(set('Snorlax', targetAbility, ['recover']))},
	});
	battle.makeChoices('team 1234', 'team 1234');
	const source = battle.p1.active[0];
	const target = battle.p2.active[0];
	if (sourceStatus) source.status = sourceStatus;
	if (targetStatus) target.status = targetStatus;
	battle.randomizer = value => value;
	const activeMove = battle.dex.getActiveMove(move);
	activeMove.willCrit = false;
	return {
		id,
		ability: sourceAbility !== 'telepathy' ? sourceAbility : targetAbility,
		move,
		source_status: sourceStatus || null,
		target_status: targetStatus || null,
		damage: battle.actions.getDamage(source, target, activeMove),
	};
}

const cases = [
	{id: 'huge-power-physical', sourceAbility: 'hugepower', move: 'dragonclaw', targetAbility: 'immunity'},
	{id: 'huge-power-special-unaffected', sourceAbility: 'hugepower', move: 'dragonpulse', targetAbility: 'immunity'},
	{id: 'pure-power-physical', sourceAbility: 'purepower', move: 'dragonclaw', targetAbility: 'immunity'},
	{id: 'guts-burn', sourceAbility: 'guts', move: 'dragonclaw', targetAbility: 'immunity', sourceStatus: 'brn'},
	{id: 'fur-coat-physical-defense', sourceAbility: 'telepathy', move: 'dragonclaw', targetAbility: 'furcoat'},
	{id: 'marvel-scale-statused', sourceAbility: 'telepathy', move: 'dragonclaw', targetAbility: 'marvelscale', targetStatus: 'psn'},
	{id: 'technician-threshold', sourceAbility: 'technician', move: 'tackle', targetAbility: 'immunity'},
	{id: 'multiscale-full-hp', sourceAbility: 'telepathy', move: 'dragonclaw', targetAbility: 'multiscale'},
].map(damage);

const contents = `${JSON.stringify({
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'pinned Battle.actions.getDamage with resolved Champions data',
		damage_roll_percent: 100,
		critical_hit: false,
		full_transition_comparison: false,
	},
	cases,
}, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Ability-modifier fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
