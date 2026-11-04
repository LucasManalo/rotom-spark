#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'guaranteed-critical.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));
const filler = {species: 'Magikarp', ability: 'swiftswim', moves: ['splash'], level: 50};

function guaranteedCritical(id, move, targetAbility = 'immunity') {
	const set = (species, ability, moves) => ({species, ability, moves, level: 50, nature: 'Serious', evs: {}});
	const battle = new Battle({
		format: Dex.formats.get(version.format, true),
		seed: 'gen5,0000000000000001',
		p1: {team: [set('Medicham', 'telepathy', [move]), filler, filler, filler]},
		p2: {team: [set('Snorlax', targetAbility, ['recover']), filler, filler, filler]},
	});
	battle.makeChoices('team 1234', 'team 1234');
	const source = battle.p1.active[0];
	const target = battle.p2.active[0];
	let activeMove = battle.dex.getActiveMove(move);
	activeMove = battle.runEvent('ModifyMove', source, target, activeMove, activeMove);
	battle.setActiveMove(activeMove, source, target);
	const declaredWillCrit = activeMove.willCrit === true;
	const critical = declaredWillCrit && battle.runEvent('CriticalHit', target, null, activeMove) === true;
	activeMove.willCrit = critical;
	battle.randomizer = value => value;
	return {
		id,
		move,
		target_ability: targetAbility,
		declared_will_crit: declaredWillCrit,
		critical_after_prevention: critical,
		damage: battle.actions.getDamage(source, target, activeMove),
	};
}

const contents = `${JSON.stringify({
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'pinned resolved move willCrit, CriticalHit event, and Battle.actions.getDamage',
		damage_roll_percent: 100,
		move_execution: true,
		full_state_differential_comparison: false,
	},
	cases: [
		guaranteedCritical('flower-trick-guaranteed-critical', 'flowertrick'),
		guaranteedCritical('frost-breath-guaranteed-critical', 'frostbreath'),
		guaranteedCritical('storm-throw-guaranteed-critical', 'stormthrow'),
		guaranteedCritical('flower-trick-battle-armor', 'flowertrick', 'battlearmor'),
	],
}, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Guaranteed-critical fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
