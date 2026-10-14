'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');
const {generatedContents, nextResearch} = require('./generate-mechanics-manifest.cjs');

const repositoryRoot = path.resolve(__dirname, '..', '..');
const manifest = JSON.parse(fs.readFileSync(
	path.join(repositoryRoot, 'data', 'generated', 'reg-mc-mechanics-manifest.json'),
	'utf8'
));

function entry(kind, id) {
	return manifest.entries.find(candidate => candidate.kind === kind && candidate.id === id);
}

test('manifest is pinned, sorted, and internally unique', () => {
	assert.equal(manifest.schema_version, 1);
	assert.equal(manifest.source.commit, 'b79ce06d06b8f0dc8de8bb348bc9cab5d931553f');
	assert.equal(manifest.format.id, 'gen9championsvgc2026regmc');
	const keys = manifest.entries.map(candidate => `${candidate.kind}:${candidate.id}`);
	assert.deepEqual(keys, [...keys].sort());
	assert.equal(new Set(keys).size, keys.length);
	assert.equal(manifest.summary.total, manifest.entries.length);
	assert.deepEqual(
		Object.fromEntries(Object.entries(manifest.summary.by_kind).map(([kind, value]) => [kind, value.total])),
		{ability: 215, condition: 96, form: 153, item: 166, move: 510, species: 293}
	);
	assert.equal(manifest.team_wide_constraints.complex_team_bans, 0);
	assert.ok(manifest.unresolved_reachability.every(candidate => candidate.reachability === 'needs_research'));
});

test('direct legality comes from complete validated starting sets', () => {
	assert.equal(entry('species', 'charizard').reachability, 'direct_legal_set');
	assert.equal(entry('move', 'protect').reachability, 'direct_legal_set');
	assert.equal(entry('ability', 'blaze').reachability, 'direct_legal_set');
	assert.equal(entry('item', 'charizarditex').reachability, 'direct_legal_set');
	assert.equal(entry('species', 'mew'), undefined);
	assert.equal(entry('move', 'pound'), undefined);
	assert.equal(manifest.entries.some(candidate => candidate.kind === 'species' && candidate.id.endsWith('gmax')), false);
});

test('battle-only forms require a reachable source and transformation item', () => {
	const mega = entry('form', 'charizardmegax');
	assert.equal(mega.reachability, 'battle_generated');
	assert.deepEqual(mega.evidence, ['item/charizarditex', 'species/charizard']);
	assert.equal(entry('form', 'aegislashblade').reachability, 'battle_generated');
	assert.equal(entry('form', 'mewtwomegax'), undefined);
});

test('coverage claims do not confuse implementation with verification', () => {
	assert.equal(entry('move', 'aquajet').implementation, 'implemented');
	assert.equal(entry('move', 'aquajet').verification, 'differential_verified');
	assert.equal(entry('move', 'aquatail').implementation, 'implemented');
	assert.equal(entry('move', 'aquatail').verification, 'not_tested');
	assert.equal(entry('move', 'protect').implementation, 'unsupported');
	assert.equal(entry('move', 'protect').verification, 'not_tested');
});

test('move support is derived from complete executor capabilities', () => {
	const dragonClaw = entry('move', 'dragonclaw');
	assert.equal(dragonClaw.operation, 'standard_damage');
	assert.deepEqual(
		dragonClaw.required_capabilities,
		['standard_damage', 'selected_single_target', 'accuracy']
	);
	assert.equal(dragonClaw.implementation, 'implemented');
	assert.equal(dragonClaw.owner_pr, 'PR 13.1');

	for (const [id, capability] of [
		['flamethrower', 'secondary_effect'],
		['facade', 'custom_callback'],
		['surf', 'advanced_targeting'],
		['bulletseed', 'multi_hit'],
		['gigadrain', 'drain'],
		['electroshot', 'custom_callback'],
	]) {
		const move = entry('move', id);
		assert.ok(move.required_capabilities.includes(capability), `${id} should require ${capability}`);
		assert.equal(move.implementation, 'unsupported');
	}
});

test('structured generated conditions retain their provenance', () => {
	const protect = entry('condition', 'protect');
	assert.equal(protect.reachability, 'battle_generated');
	assert.ok(protect.evidence.includes('move/protect:volatileStatus'));
});

test('Transform and Imposter close over identities already reachable in battle', () => {
	for (const [id, trigger] of [['transform', 'move/transform'], ['imposter', 'ability/imposter']]) {
		const closure = manifest.reachability_closure.find(candidate => candidate.kind === 'state_copy' && candidate.id === id);
		assert.equal(closure.trigger, trigger);
		assert.ok(closure.copies.includes('species_or_form'));
		assert.ok(closure.copies.includes('move_slots'));
		assert.ok(closure.copies.includes('ability'));
		assert.ok(closure.evidence.includes('showdown:sim/pokemon.ts:Pokemon.transformInto'));
	}
});

test('literal callback-generated conditions retain callback provenance', () => {
	const rain = entry('condition', 'raindance');
	assert.ok(rain.evidence.includes('ability/drizzle:callback.setWeather'));
	assert.equal(
		manifest.unresolved_reachability.some(candidate => candidate.kind === 'ability_dynamic_effect' && candidate.id === 'drizzle'),
		false
	);
});

test('callback-generated forms retain their source ability', () => {
	const blade = entry('form', 'aegislashblade');
	assert.ok(blade.evidence.includes('ability/stancechange:callback.formeChange'));
});

test('unresolved reachability has a deterministic next-research query', () => {
	const next = nextResearch(manifest);
	assert.equal(next, manifest.unresolved_reachability[0]);
	assert.equal(next.reachability, 'needs_research');
});

test('two complete generations are byte-identical', {timeout: 120_000}, () => {
	const first = generatedContents({verifyCheckout: false});
	const second = generatedContents({verifyCheckout: false});
	assert.equal(first.manifestContents, second.manifestContents);
	assert.equal(first.reportContents, second.reportContents);
});
