#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const repositoryRoot = path.resolve(__dirname, '..', '..');
const damageFixturePath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'basic-damage.json');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'canonical-step.json');
const damageFixture = JSON.parse(fs.readFileSync(damageFixturePath, 'utf8'));

const selections = [
	['dragon-claw-physical-step', 'dragon-claw-neutral-stab', 19, 99],
	['dragon-pulse-special-step', 'dragon-pulse-neutral-stab', 19, 96],
	['power-gem-effective-faint-step', 'power-gem-quad-effective-stab', 19, 96],
	['high-horsepower-effective-faint-step', 'high-horsepower-super-effective-stab', 19, 96],
	['high-horsepower-immunity-step', 'high-horsepower-immunity', 19, 96],
	['dragon-pulse-critical-step', 'dragon-pulse-critical', 12, 89],
];

const cases = selections.map(([name, damageCaseName, rotomSeed, damagePercent]) => {
	const damageCase = damageFixture.cases.find(candidate => candidate.name === damageCaseName);
	if (!damageCase) throw new Error(`Missing pinned damage case ${damageCaseName}`);
	const roll = damageCase.rolls.find(candidate => candidate.percent === damagePercent);
	if (!roll) throw new Error(`Missing ${damagePercent}% roll for ${damageCaseName}`);
	const targetHp = damageCase.target.stats.hp;
	return {
		name,
		move: damageCase.move,
		damage_case: damageCaseName,
		rotom_seed: rotomSeed,
		controlled_outcomes: {
			accuracy: 'hit',
			critical_hit: damageCase.critical,
			damage_percent: damagePercent,
		},
		expected: {
			damage: roll.damage,
			applied_hp_loss: Math.min(targetHp, roll.damage),
			target_hp: Math.max(0, targetHp - roll.damage),
			pp_consumed: 1,
			fainted: roll.damage >= targetHp && roll.damage > 0,
			replacement_scheduled: roll.damage >= targetHp && roll.damage > 0,
		},
	};
});

const output = {
	showdown_commit: damageFixture.showdown_commit,
	format: damageFixture.format,
	methodology: {
		damage_source: 'tools/showdown-reference/basic-damage.cjs using Battle.actions.getDamage',
		integration_scope: 'controlled pinned damage outcomes applied through the Rotom Spark public step(); HP, PP, fainting, replacement, stable-state, and serialization consequences are asserted in Rust',
		full_transition_comparison: 'not performed; this fixture does not execute or compare an independent complete Showdown battle transition',
		rng_comparison: 'controlled outcomes; no seed-level parity claim',
		rotom_rng: 'SplitMix64',
		showdown_rng: 'Pokemon Showdown PRNG',
		move_draw_order: ['accuracy', 'critical_hit', 'damage_roll'],
	},
	cases,
};
const contents = `${JSON.stringify(output, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Canonical step fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.mkdirSync(path.dirname(outputPath), {recursive: true});
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
