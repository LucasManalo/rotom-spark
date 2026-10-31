'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');
const {generatedContents} = require('./generate-capability-report.cjs');
const {rustTestExists} = require('./generate-mechanics-manifest.cjs');

const repositoryRoot = path.resolve(__dirname, '..', '..');

function identity(audit, kind, id) {
	return audit.identity_dependencies.find(entry => entry.kind === kind && entry.id === id);
}

test('dependency graph covers every partial ability and item and every unsupported move', () => {
	const {audit} = generatedContents();
	assert.deepEqual(audit.summary.dependency_targets, {ability: 95, item: 111, move: 287});
	assert.equal(audit.identity_dependencies.length, 493);
	assert.ok(audit.identity_dependencies.every(entry => entry.outstanding_blockers.length > 0));
	assert.ok(audit.identity_dependencies.every(entry => entry.unlocked === false));
});

test('headline separates identity labels from executable capability coverage', () => {
	const {audit, markdown} = generatedContents();
	assert.deepEqual(audit.summary.identity_coverage.ability, {reachable: 215, unsupported: 0, partial: 95, implemented: 120});
	assert.deepEqual(audit.summary.identity_coverage.item, {reachable: 166, unsupported: 0, partial: 111, implemented: 55});
	assert.deepEqual(audit.summary.identity_coverage.move, {reachable: 510, unsupported: 287, partial: 0, implemented: 223});
	assert.match(markdown, /## Identity coverage/);
	assert.match(markdown, /## Executable capability coverage/);
	assert.match(markdown, /metadata foundation/);
});

test('roadmap examples expose their actual remaining blockers', () => {
	const {audit} = generatedContents();
	assert.deepEqual(identity(audit, 'item', 'choicescarf').outstanding_blockers, ['choice_lock']);
	assert.ok(identity(audit, 'item', 'persimberry').outstanding_blockers.includes('confusion'));
	assert.deepEqual(identity(audit, 'item', 'ejectbutton').outstanding_blockers, ['forced_switch']);
	assert.deepEqual(identity(audit, 'item', 'charizarditex').outstanding_blockers, [
		'ability_transition', 'form_change', 'mega_eligibility', 'mega_timing',
	]);
	const berryLifecycle = audit.capabilities.find(entry => entry.id === 'berry_lifecycle');
	assert.equal(berryLifecycle.status, 'implemented');
	assert.deepEqual(berryLifecycle.unlocked_identities, [
		'item:aspearberry', 'item:cheriberry', 'item:chestoberry', 'item:leppaberry',
		'item:oranberry', 'item:pechaberry', 'item:rawstberry', 'item:sitrusberry',
	]);
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
