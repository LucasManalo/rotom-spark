#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'common-conditions.json');
const version = Object.fromEntries(
	fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
		.trim()
		.split(/\r?\n/u)
		.map(line => line.split('=', 2)),
);
const conditions = Dex.mod(version.mod).conditions;

const paralysis = conditions.get('par');
const paralysisChance = [];
paralysis.onBeforeMove.call({
	randomChance(numerator, denominator) {
		paralysisChance.push(numerator, denominator);
		return false;
	},
});
const paralysisSpeed = paralysis.onModifySpe.call(
	{finalModify: value => value},
	101,
	{hasAbility: () => false},
);

const sleep = conditions.get('slp');
const sleepState = {};
let sleepDurations;
sleep.onStart.call(
	{
		add() {},
		effectState: sleepState,
		sample(values) {
			sleepDurations = values.slice();
			return values[0];
		},
	},
	{removeVolatile: () => false},
);

const freeze = conditions.get('frz');
const freezeState = {};
const frozen = {
	baseSpecies: {baseSpecies: 'Test'},
	species: {name: 'Test'},
	statusState: freezeState,
	cures: 0,
	cureStatus() {
		this.cures++;
	},
};
freeze.onStart.call({add() {}, effectState: freezeState}, frozen);
const freezeChances = [];
let automaticThawAttempt = null;
for (let attempt = 1; attempt <= 3; attempt++) {
	freeze.onBeforeMove.call(
		{
			add() {},
			randomChance(numerator, denominator) {
				freezeChances.push([numerator, denominator]);
				return false;
			},
		},
		frozen,
		null,
		{flags: {}, id: 'tackle'},
	);
	if (frozen.cures) {
		automaticThawAttempt = attempt;
		break;
	}
}

function residual(conditionId, times) {
	const condition = conditions.get(conditionId);
	const effectState = {};
	if (condition.onStart) {
		condition.onStart.call({add() {}, effectState}, {});
	}
	const damage = [];
	const context = {
		effectState,
		damage(value) {
			damage.push(value);
		},
		clampIntRange(value, minimum) {
			return Math.max(Math.trunc(value), minimum);
		},
	};
	for (let index = 0; index < times; index++) {
		condition.onResidual.call(context, {baseMaxhp: 160});
	}
	return {order: condition.onResidualOrder, damage};
}

const fixture = {
	showdown_commit: version.commit,
	format: version.format,
	source: 'resolved champions conditions callbacks',
	paralysis: {
		speed_from_101: paralysisSpeed,
		before_move_chance: paralysisChance,
	},
	sleep: {
		durations: sleepDurations,
		initialized_turns: sleepState.time,
	},
	freeze: {
		initialized_turns: freezeState.startTime,
		early_thaw_chance: freezeChances[0],
		automatic_thaw_attempt: automaticThawAttempt,
	},
	residual: {
		max_hp: 160,
		poison: residual('psn', 1),
		toxic: residual('tox', 2),
		burn: residual('brn', 1),
	},
};
const serialized = `${JSON.stringify(fixture, null, 2)}\n`;

if (process.argv.includes('--check')) {
	const current = fs.readFileSync(outputPath, 'utf8');
	if (current !== serialized) {
		throw new Error(`${path.relative(repositoryRoot, outputPath)} is stale`);
	}
} else {
	fs.writeFileSync(outputPath, serialized);
}
