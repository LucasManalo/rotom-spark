'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const test = require('node:test');
const {
	generatedContents,
	nextResearch,
	rustTestExists,
	validateCoverageClaims,
} = require('./generate-mechanics-manifest.cjs');

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
	assert.equal(entry('move', 'aquatail').verification, 'unit_tested');
	assert.ok(entry('move', 'aquatail').evidence.includes(
		'unit:crates/rotom-dex/tests/combat.rs#queued_turn_consumes_pp_and_updates_hp'
	));
	assert.equal(entry('move', 'aquatail').interaction_verification, 'not_tested');
	assert.equal(entry('move', 'protect').implementation, 'implemented');
	assert.equal(entry('move', 'protect').verification, 'differential_verified');
	assert.equal(entry('move', 'protect').interaction_verification, 'differential_verified');
});

test('ability batches keep isolated and interaction verification separate', () => {
	for (const id of [
		'battlearmor', 'blaze', 'bulletproof', 'chlorophyll', 'filter', 'fluffy', 'furcoat',
		'galewings', 'goodasgold', 'grasspelt', 'guts', 'hugepower', 'immunity', 'insomnia',
		'ironfist', 'limber', 'magmaarmor', 'marvelscale', 'moldbreaker', 'multiscale',
		'overgrow', 'prankster', 'punkrock', 'purepower', 'quickfeet', 'reckless', 'sandforce',
		'sandrush', 'sharpness', 'shellarmor', 'slushrush', 'sniper', 'solidrock', 'soundproof',
		'steelyspirit', 'strongjaw', 'superluck', 'surgesurfer', 'swarm', 'sweetveil',
		'swiftswim', 'technician', 'telepathy', 'thickfat', 'torrent', 'toughclaws', 'vitalspirit',
		'compoundeyes', 'hustle', 'megalauncher', 'merciless',
	]) {
		const ability = entry('ability', id);
		assert.equal(ability.implementation, 'implemented');
		assert.equal(ability.verification, 'differential_verified');
		assert.equal(ability.interaction_verification, 'unit_tested');
		assert.equal(
			ability.evidence.some(evidence => evidence.startsWith('interaction_differential:')),
			false
		);
	}
	for (const id of [
		'eartheater', 'friendguard', 'hospitality', 'hydration', 'motordrive', 'naturalcure',
		'overcoat', 'regenerator', 'thermalexchange', 'voltabsorb', 'waterabsorb', 'waterbubble',
		'levitate', 'sapsipper',
	]) {
		const ability = entry('ability', id);
		assert.equal(ability.implementation, 'implemented');
		assert.equal(ability.verification, 'unit_tested');
		assert.equal(ability.interaction_verification, 'unit_tested');
	}
	for (const id of [
		'angerpoint', 'anticipation', 'forewarn', 'frisk', 'gooey', 'healer', 'icebody',
		'justified', 'moxie', 'raindish', 'runaway', 'shedskin', 'stamina', 'weakarmor',
		'auraguard', 'sandveil', 'snowcloak',
	]) {
		const ability = entry('ability', id);
		assert.equal(ability.implementation, 'implemented');
		assert.equal(ability.verification, 'unit_tested');
		assert.equal(ability.interaction_verification, 'not_tested');
	}
	assert.deepEqual(
		manifest.summary.by_kind.ability.interaction_verification,
		{not_tested: 141, unit_tested: 74, differential_verified: 0}
	);
});

test('move support is derived from complete executor capabilities', () => {
	const dragonClaw = entry('move', 'dragonclaw');
	assert.equal(dragonClaw.operation, 'standard_damage');
	assert.deepEqual(
		dragonClaw.required_capabilities,
		[
			'standard_damage', 'selected_single_target', 'accuracy', 'protection',
			'redirection', 'retargeting', 'per_target_hit',
		]
	);
	assert.equal(dragonClaw.implementation, 'implemented');
	assert.equal(dragonClaw.owner_pr, 'PR 13.1');
	assert.equal(manifest.summary.by_kind.move.implementation.implemented, 223);
	assert.deepEqual(
		manifest.summary.by_kind.move.verification,
		{not_tested: 287, unit_tested: 200, differential_verified: 23}
	);
	const implementedMoves = manifest.entries.filter(candidate =>
		candidate.kind === 'move' && candidate.implementation === 'implemented'
	);
	assert.equal(implementedMoves.filter(candidate => candidate.verification === 'not_tested').length, 0);
	assert.ok(entry('move', 'powerwhip').evidence.includes(
		'unit:crates/rotom-dex/tests/combat.rs#mechanics_manifest_drives_every_move_capability_and_execution_smoke_case'
	));
	assert.deepEqual(
		manifest.summary.by_kind.move.interaction_verification,
		{not_tested: 495, unit_tested: 1, differential_verified: 14}
	);
	assert.equal(entry('move', 'magnetrise').implementation, 'implemented');
	assert.equal(entry('move', 'magnetrise').verification, 'differential_verified');
	assert.deepEqual(entry('move', 'surf').required_capabilities, [
		'standard_damage', 'accuracy', 'advanced_targeting', 'target_set',
		'spread_damage', 'protection', 'retargeting', 'per_target_hit',
	]);
	assert.equal(entry('move', 'surf').implementation, 'implemented');
	assert.equal(entry('move', 'surf').owner_pr, 'PR 13.2');

	for (const [id, capability] of [
		['flamethrower', 'secondary_effect'],
		['facade', 'conditional_power'],
		['bulletseed', 'multi_hit'],
		['gigadrain', 'drain'],
	]) {
		const move = entry('move', id);
		assert.ok(move.required_capabilities.includes(capability), `${id} should require ${capability}`);
		assert.equal(move.implementation, 'implemented');
		assert.equal(move.owner_pr, 'PR 13.4');
	}
	const electroShot = entry('move', 'electroshot');
	assert.ok(electroShot.required_capabilities.includes('custom_callback'));
	assert.equal(electroShot.implementation, 'unsupported');
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

test('coverage evidence requires real tests and move-specific fixture cases', () => {
	const root = fs.mkdtempSync(path.join(os.tmpdir(), 'rotom-evidence-'));
	try {
		fs.mkdirSync(path.join(root, 'tests'), {recursive: true});
		fs.mkdirSync(path.join(root, 'fixtures'), {recursive: true});
		fs.writeFileSync(path.join(root, 'tests', 'combat.rs'), [
			'fn ordinary_helper() {}',
			'#[test]',
			'fn verifies_dragon_claw() {}',
		].join('\n'));
		fs.writeFileSync(path.join(root, 'fixtures', 'damage.json'), JSON.stringify({
			methodology: {move_execution: false},
			cases: [
				{name: 'dragon-claw-neutral', move: 'dragonclaw'},
				{name: 'dragon-pulse-neutral', move: 'dragonpulse'},
			],
		}));

		assert.equal(rustTestExists('#[test]\nfn actual_test() {}', 'actual_test'), true);
		assert.equal(rustTestExists('fn actual_test() {}', 'actual_test'), false);

		const claim = overrides => ({
			id: 'dragonclaw',
			kind: 'move',
			implementation: 'implemented',
			verification: 'unit_tested',
			owner_pr: 'test',
			evidence: ['file:tests/combat.rs', 'unit:tests/combat.rs#verifies_dragon_claw'],
			...overrides,
		});
		const registry = claims => ({schema_version: 1, claims});

		assert.equal(validateCoverageClaims(registry([claim({})]), root).size, 1);
		assert.throws(
			() => validateCoverageClaims(registry([claim({evidence: ['unit:tests/missing.rs#test']} )]), root),
			/Missing evidence/
		);
		assert.throws(
			() => validateCoverageClaims(registry([claim({evidence: ['unit:tests/combat.rs#missing_test']} )]), root),
			/does not name an existing test/
		);
		assert.throws(
			() => validateCoverageClaims(registry([claim({evidence: ['unit:tests/combat.rs#ordinary_helper']} )]), root),
			/does not name an existing test/
		);
		assert.throws(
			() => validateCoverageClaims(registry([claim({
				verification: 'differential_verified',
				evidence: ['differential:fixtures/damage.json#missing-case'],
			})]), root),
			/does not name an existing fixture case/
		);
		assert.throws(
			() => validateCoverageClaims(registry([claim({
				verification: 'differential_verified',
				evidence: ['differential:fixtures/damage.json#dragon-pulse-neutral'],
			})]), root),
			/does not reference claimed move/
		);
		assert.throws(
			() => validateCoverageClaims(registry([claim({verification: 'unit_tested', evidence: ['file:tests/combat.rs']})]), root),
			/no anchored unit-test evidence/
		);
		assert.throws(
			() => validateCoverageClaims(registry([claim({verification: 'not_tested'})]), root),
			/has verification evidence/
		);
		assert.throws(
			() => validateCoverageClaims(registry([claim({
				implementation: 'unsupported',
				verification: 'unit_tested',
			})]), root),
			/cannot be marked verified/
		);
		assert.throws(
			() => validateCoverageClaims(registry([claim({
				implementation: 'unsupported',
				verification: 'not_tested',
				interaction_verification: 'unit_tested',
			})]), root),
			/cannot be marked interaction verified/
		);
		assert.throws(
			() => validateCoverageClaims(registry([claim({
				evidence: [
					'unit:tests/combat.rs#verifies_dragon_claw',
					'unit:tests/combat.rs#verifies_dragon_claw',
				],
			})]), root),
			/Duplicate evidence/
		);
		assert.throws(
			() => validateCoverageClaims(registry([claim({
				interaction_verification: 'unit_tested',
			})]), root),
			/no anchored interaction evidence/
		);
		assert.throws(
			() => validateCoverageClaims(registry([claim({
				interaction_verification: 'differential_verified',
				evidence: [
					'unit:tests/combat.rs#verifies_dragon_claw',
					'interaction_differential:fixtures/damage.json#dragon-claw-neutral',
				],
			})]), root),
			/only an isolated calculation/
		);
		assert.equal(validateCoverageClaims(registry([claim({
			verification: 'not_tested',
			interaction_verification: 'unit_tested',
			evidence: [
				'file:tests/combat.rs',
				'interaction_unit:tests/combat.rs#verifies_dragon_claw',
			],
		})]), root).size, 1);
		assert.throws(
			() => validateCoverageClaims(registry([claim({
				interaction_verification: 'not_tested',
				evidence: [
					'unit:tests/combat.rs#verifies_dragon_claw',
					'interaction_unit:tests/combat.rs#verifies_dragon_claw',
				],
			})]), root),
			/has interaction evidence/
		);
	} finally {
		fs.rmSync(root, {recursive: true, force: true});
	}
});
