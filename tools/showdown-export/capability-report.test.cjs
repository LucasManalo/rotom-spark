'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');
const {generatedContents} = require('./generate-capability-report.cjs');
const {rustTestExists} = require('./generate-mechanics-manifest.cjs');
const {MEGA_STONES} = require('./capability-dependencies.cjs');

const repositoryRoot = path.resolve(__dirname, '..', '..');

function identity(audit, kind, id) {
	return audit.identity_dependencies.find(entry => entry.kind === kind && entry.id === id);
}

test('dependency graph covers every partial ability and item and every unsupported move', () => {
	const {audit} = generatedContents();
	const derived = Object.fromEntries(['ability', 'item', 'move'].map(kind => [
		kind,
		audit.identity_dependencies.filter(entry => entry.kind === kind).length,
	]));
	assert.deepEqual(audit.summary.dependency_targets, derived);
	assert.equal(Object.values(derived).reduce((sum, count) => sum + count, 0), audit.identity_dependencies.length);
	assert.ok(audit.identity_dependencies.every(entry => entry.outstanding_blockers.length > 0));
	assert.ok(audit.identity_dependencies.every(entry => entry.unlocked === false));
});

test('headline separates identity labels from executable capability coverage', () => {
	const {audit, markdown} = generatedContents();
	for (const coverage of Object.values(audit.summary.identity_coverage)) {
		assert.equal(coverage.reachable, coverage.unsupported + coverage.partial + coverage.implemented);
	}
	assert.match(markdown, /## Identity coverage/);
	assert.match(markdown, /## Executable capability coverage/);
	assert.match(markdown, /Mega Stones are executable through the verified dedicated/);
	assert.match(markdown, /Generic `form_change` remains unsupported/);
	assert.doesNotMatch(markdown, /metadata foundation/);
});

test('roadmap examples expose their actual remaining blockers', () => {
	const {audit} = generatedContents();
	assert.deepEqual(identity(audit, 'item', 'choicescarf').outstanding_blockers, ['choice_lock']);
	assert.ok(identity(audit, 'item', 'persimberry').outstanding_blockers.includes('confusion'));
	assert.equal(identity(audit, 'item', 'ejectbutton'), undefined);
	assert.equal(identity(audit, 'item', 'kingsrock'), undefined);
	assert.equal(identity(audit, 'item', 'mentalherb'), undefined);
	assert.equal(identity(audit, 'item', 'metronome'), undefined);
	assert.equal(identity(audit, 'item', 'charizarditex'), undefined);
	const berryLifecycle = audit.capabilities.find(entry => entry.id === 'berry_lifecycle');
	assert.equal(berryLifecycle.status, 'implemented');
	assert.deepEqual(berryLifecycle.unlocked_identities, [
		'item:aspearberry', 'item:cheriberry', 'item:chestoberry', 'item:leppaberry',
		'item:oranberry', 'item:pechaberry', 'item:rawstberry', 'item:sitrusberry',
	]);
	const forcedSwitch = audit.capabilities.find(entry => entry.id === 'forced_switch');
	assert.equal(forcedSwitch.status, 'implemented');
	assert.deepEqual(forcedSwitch.unlocked_identities, [
		'ability:suctioncups', 'item:ejectbutton', 'item:redcard',
	]);
	const flinch = audit.capabilities.find(entry => entry.id === 'flinch');
	assert.equal(flinch.status, 'implemented');
	assert.deepEqual(flinch.unlocked_identities, [
		'ability:innerfocus', 'ability:steadfast', 'item:kingsrock',
	]);
	const mentalHerb = audit.capabilities.find(entry => entry.id === 'mental_herb_cure');
	assert.equal(mentalHerb.status, 'implemented');
	assert.deepEqual(mentalHerb.unlocked_identities, ['item:mentalherb']);
	const moveHistory = audit.capabilities.find(entry => entry.id === 'move_history');
	assert.equal(moveHistory.status, 'verified');
	assert.deepEqual(moveHistory.unlocked_identities, ['item:metronome']);
	for (const id of ['mega_eligibility', 'mega_timing', 'mega_form_change', 'mega_ability_transition']) {
		assert.equal(audit.capabilities.find(entry => entry.id === id).status, 'verified');
	}
	assert.deepEqual(
		audit.capabilities.find(entry => entry.id === 'mega_eligibility').unlocked_identities,
		MEGA_STONES.map(id => `item:${id}`)
	);
});

test('every dependency and capability carries pinned-reference evidence', () => {
	const {audit} = generatedContents();
	assert.ok(audit.identity_dependencies.every(entry =>
		entry.pinned_reference_evidence.every(value => value.startsWith('showdown:'))
	));
	assert.ok(audit.capabilities.every(entry => entry.pinned_reference_evidence.length > 0));
	assert.ok(audit.capabilities.every(entry => ['unsupported', 'partial', 'implemented', 'verified'].includes(entry.status)));
	assert.ok(audit.capabilities.filter(entry => entry.status === 'verified').every(entry =>
		entry.tests.length > 0 && entry.differential_fixtures.length > 0
	));
	for (const capability of audit.capabilities) {
		for (const evidence of capability.tests) {
			const match = /^unit:([^#]+)#(.+)$/.exec(evidence);
			assert.ok(match, `invalid unit evidence ${evidence}`);
			assert.ok(rustTestExists(fs.readFileSync(path.join(repositoryRoot, match[1]), 'utf8'), match[2]));
		}
		for (const evidence of capability.differential_fixtures) {
			const match = /^differential:([^#]+)#(.+)$/.exec(evidence);
			assert.ok(match, `invalid differential evidence ${evidence}`);
			const fixture = JSON.parse(fs.readFileSync(path.join(repositoryRoot, match[1]), 'utf8'));
			assert.ok(fixture.cases.some(entry => entry.name === match[2] || entry.id === match[2]));
		}
	}
});

test('generation is byte deterministic', () => {
	const first = generatedContents();
	const second = generatedContents();
	assert.equal(first.json, second.json);
	assert.equal(first.markdown, second.markdown);
});
