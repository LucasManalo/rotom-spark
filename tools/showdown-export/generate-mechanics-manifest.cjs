#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {execFileSync} = require('node:child_process');
const {isPr131Supported, moveExecutionRequirements} = require('./move-capabilities.cjs');

const repositoryRoot = path.resolve(__dirname, '..', '..');
const showdownRoot = path.join(repositoryRoot, 'vendor', 'pokemon-showdown');
const manifestPath = path.join(repositoryRoot, 'data', 'generated', 'reg-mc-mechanics-manifest.json');
const reportPath = path.join(repositoryRoot, 'data', 'generated', 'reg-mc-mechanics-coverage.md');
const coveragePath = path.join(repositoryRoot, 'data', 'mechanics-coverage.json');
const CONDITION_KEYS = new Map([
	['status', 'major_status'],
	['volatileStatus', 'volatile'],
	['sideCondition', 'side_condition'],
	['slotCondition', 'slot_condition'],
	['weather', 'weather'],
	['terrain', 'terrain'],
	['pseudoWeather', 'field_condition'],
]);
const CALLBACK_CONDITION_METHODS = new Map([
	['setStatus', 'major_status'],
	['trySetStatus', 'major_status'],
	['addVolatile', 'volatile'],
	['addSideCondition', 'side_condition'],
	['addSlotCondition', 'slot_condition'],
	['setWeather', 'weather'],
	['setTerrain', 'terrain'],
	['addPseudoWeather', 'field_condition'],
]);
const IMPLEMENTATION_VALUES = new Set(['unsupported', 'partial', 'implemented']);
const VERIFICATION_VALUES = new Set(['not_tested', 'unit_tested', 'differential_verified']);

function fail(message) {
	throw new Error(message);
}

function compareStrings(left, right) {
	return left < right ? -1 : left > right ? 1 : 0;
}

function uniqueSorted(values) {
	return [...new Set(values)].sort(compareStrings);
}

function readJson(filePath) {
	return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function readVersionFile() {
	const entries = Object.create(null);
	const versionPath = path.join(repositoryRoot, 'SHOWDOWN_VERSION');
	for (const line of fs.readFileSync(versionPath, 'utf8').split(/\r?\n/)) {
		if (!line) continue;
		const separator = line.indexOf('=');
		if (separator < 1) fail(`Invalid SHOWDOWN_VERSION line: ${line}`);
		entries[line.slice(0, separator)] = line.slice(separator + 1);
	}
	for (const key of ['repo', 'commit', 'format', 'mod']) {
		if (!entries[key]) fail(`SHOWDOWN_VERSION is missing ${key}`);
	}
	return entries;
}

function verifyCheckout(version) {
	const actualCommit = execFileSync('git', ['-C', showdownRoot, 'rev-parse', 'HEAD'], {
		encoding: 'utf8',
	}).trim();
	if (actualCommit !== version.commit) {
		fail(`Pinned Showdown checkout is ${actualCommit}, expected ${version.commit}`);
	}
}

function loadShowdown() {
	const entrypoint = path.join(showdownRoot, 'dist', 'sim');
	try {
		return require(entrypoint);
	} catch (error) {
		if (error && error.code === 'MODULE_NOT_FOUND') {
			fail(
				'Pinned Showdown is not built. Run `npm ci` and `npm run build` in ' +
				'`vendor/pokemon-showdown` first.'
			);
		}
		throw error;
	}
}

function validationSet(species, ability, moves, item = '') {
	return {
		name: species.baseSpecies,
		species: species.name,
		ability,
		item,
		moves: uniqueSorted([species.requiredMove, ...moves].filter(Boolean)).slice(0, 4),
		nature: 'Hardy',
		level: 50,
		evs: {hp: 1, atk: 0, def: 0, spa: 0, spd: 0, spe: 0},
		ivs: {hp: 31, atk: 31, def: 31, spa: 31, spd: 31, spe: 31},
	};
}

function validatedSet(validator, species, ability, moves, item = '') {
	const set = validationSet(species, ability, moves, item);
	const problems = validator.validateSet(set, {});
	return problems === null || problems.length === 0 ? set : null;
}

function possibleAbilities(species) {
	return Object.entries(species.abilities)
		.filter(([, name]) => name)
		.sort(([left], [right]) => compareStrings(left, right));
}

function isStartingForm(species) {
	return !species.battleOnly && !species.isMega && !species.isPrimal;
}

function directSetInventory(dex, validator) {
	const directSpecies = new Map();
	const moveEvidence = new Map();
	const abilityEvidence = new Map();

	for (const species of dex.species.all().filter(entry => entry.exists).sort((a, b) => compareStrings(a.id, b.id))) {
		if (!isStartingForm(species)) continue;
		const item = species.requiredItem || '';
		const abilities = possibleAbilities(species);
		const movePool = [...dex.species.getMovePool(species.id)].sort(compareStrings);
		const validCombinations = [];
		for (const moveId of movePool) {
			const move = dex.moves.get(moveId);
			if (!move.exists) continue;
			for (const [slot, ability] of abilities) {
				const validated = validatedSet(validator, species, ability, [move.id], item);
				if (!validated || dex.species.get(validated.species).id !== species.id) continue;
				validCombinations.push({move: move.id, ability, slot});
				const evidence = moveEvidence.get(move.id) || [];
				evidence.push(`species/${species.id}`);
				moveEvidence.set(move.id, evidence);
				break;
			}
		}
		if (!validCombinations.length) continue;

		const seed = validCombinations[0];
		directSpecies.set(species.id, {species, seed, item});
		for (const [slot, ability] of abilities) {
			const validated = validatedSet(validator, species, ability, [seed.move], item);
			if (!validated || dex.species.get(validated.species).id !== species.id) continue;
			const abilityId = dex.abilities.get(ability).id;
			const evidence = abilityEvidence.get(abilityId) || [];
			evidence.push(`species/${species.id}/ability-slot/${slot}`);
			abilityEvidence.set(abilityId, evidence);
		}
	}

	for (const evidence of moveEvidence.values()) evidence.splice(0, evidence.length, ...uniqueSorted(evidence));
	for (const evidence of abilityEvidence.values()) evidence.splice(0, evidence.length, ...uniqueSorted(evidence));
	return {directSpecies, moveEvidence, abilityEvidence};
}

function legalItemInventory(dex, validator, directSpecies) {
	const evidenceByItem = new Map();
	const baselines = [...directSpecies.values()];
	for (const item of dex.items.all().filter(entry => entry.exists).sort((a, b) => compareStrings(a.id, b.id))) {
		const preferred = [];
		for (const baseline of baselines) {
			const {species} = baseline;
			if (item.itemUser?.includes(species.name) || item.megaEvolves === species.name || species.requiredItem === item.name) {
				preferred.push(baseline);
			}
		}
		const candidates = [...preferred, ...baselines.filter(entry => !preferred.includes(entry))];
		for (const baseline of candidates) {
			const {species, seed} = baseline;
			const validated = validatedSet(validator, species, seed.ability, [seed.move], item.name);
			if (!validated || dex.species.get(validated.species).id !== species.id) continue;
			evidenceByItem.set(item.id, [`species/${species.id}`]);
			break;
		}
	}
	return evidenceByItem;
}

function battleGeneratedForms(dex, directSpecies, legalItems) {
	const generated = new Map();
	for (const species of dex.species.all().filter(entry => entry.exists).sort((a, b) => compareStrings(a.id, b.id))) {
		const sources = Array.isArray(species.battleOnly)
			? species.battleOnly
			: species.battleOnly ? [species.battleOnly] : [];
		const source = sources.map(name => dex.species.get(name)).find(entry => directSpecies.has(entry.id));
		if (!source) continue;
		if (species.requiredItem && !legalItems.has(dex.items.get(species.requiredItem).id)) continue;
		const evidence = [`species/${source.id}`];
		if (species.requiredItem) evidence.push(`item/${dex.items.get(species.requiredItem).id}`);
		if (species.id === 'aegislashblade') evidence.push('ability/stancechange:callback.formeChange');
		generated.set(species.id, uniqueSorted(evidence));
	}
	return generated;
}

function addConditionReference(target, conditionType, id, evidence) {
	if (typeof id !== 'string' || !id) return;
	const normalized = id.toLowerCase().replace(/[^a-z0-9]+/g, '');
	if (!normalized) return;
	const key = `${conditionType}:${normalized}`;
	const current = target.get(key) || {id: normalized, condition_type: conditionType, evidence: []};
	current.evidence.push(evidence);
	target.set(key, current);
}

function collectConditionReferences(value, evidencePrefix, target, seen = new Set(), pathParts = []) {
	if (!value || typeof value !== 'object' || seen.has(value)) return;
	seen.add(value);
	for (const [key, child] of Object.entries(value)) {
		const childPath = [...pathParts, key];
		if (CONDITION_KEYS.has(key)) {
			addConditionReference(target, CONDITION_KEYS.get(key), child, `${evidencePrefix}:${childPath.join('.')}`);
		}
		if (typeof child === 'object') collectConditionReferences(child, evidencePrefix, target, seen, childPath);
	}
}

function collectCallbackConditionReferences(value, evidencePrefix, target, seen = new Set()) {
	if (!value || (typeof value !== 'object' && typeof value !== 'function') || seen.has(value)) {
		return {calls: 0, extracted: 0};
	}
	seen.add(value);
	if (typeof value === 'function') {
		const source = value.toString();
		let calls = 0;
		let extracted = 0;
		for (const [method, conditionType] of CALLBACK_CONDITION_METHODS) {
			calls += [...source.matchAll(new RegExp(`\\b${method}\\s*\\(`, 'g'))].length;
			const literalPattern = new RegExp(`\\b${method}\\s*\\(\\s*(['"])([a-zA-Z0-9 -]+)\\1`, 'g');
			for (const match of source.matchAll(literalPattern)) {
				addConditionReference(target, conditionType, match[2], `${evidencePrefix}:callback.${method}`);
				extracted += 1;
			}
		}
		if (/\buseMove\s*\(/.test(source)) calls += 1;
		return {calls, extracted};
	}
	return Object.values(value).reduce((total, child) => {
		const result = collectCallbackConditionReferences(child, evidencePrefix, target, seen);
		return {calls: total.calls + result.calls, extracted: total.extracted + result.extracted};
	}, {calls: 0, extracted: 0});
}

function moveFamilies(move) {
	const families = [];
	if (move.category === 'Status') families.push('status_move');
	else families.push(move.basePower ? 'damage' : 'special_damage');
	if (move.multihit) families.push('multi_hit');
	if (move.recoil) families.push('recoil');
	if (move.drain) families.push('drain');
	if (move.secondary || move.secondaries) families.push('secondary_effect');
	if (move.self || move.selfBoost) families.push('self_effect');
	if (move.boosts) families.push('stat_change');
	if (move.heal) families.push('healing');
	if (move.status) families.push('major_status');
	if (move.volatileStatus) families.push('volatile');
	if (move.weather || move.terrain || move.pseudoWeather || move.sideCondition || move.slotCondition) {
		families.push('field_or_side_condition');
	}
	if (move.callsMove) families.push('calls_move');
	if (move.selfSwitch || move.forceSwitch) families.push('switch_control');
	if (move.onTry || move.onTryHit || move.onHit || move.basePowerCallback || move.damageCallback) {
		families.push('custom_callback');
	}
	return uniqueSorted(families);
}

function defaultCoverage() {
	return {implementation: 'unsupported', verification: 'not_tested', owner_pr: null};
}

function coverageClaims() {
	const source = readJson(coveragePath);
	if (source.schema_version !== 1 || !Array.isArray(source.claims)) fail('Invalid mechanics coverage registry schema');
	const claims = new Map();
	const fixture = readJson(path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'basic-damage.json'));
	const differentialMoves = new Set(fixture.cases.map(entry => entry.move));
	for (const claim of source.claims) {
		const key = `${claim.kind}:${claim.id}`;
		if (claims.has(key)) fail(`Duplicate coverage claim for ${key}`);
		if (!IMPLEMENTATION_VALUES.has(claim.implementation)) fail(`Invalid implementation for ${key}`);
		if (!VERIFICATION_VALUES.has(claim.verification)) fail(`Invalid verification for ${key}`);
		if (claim.implementation === 'unsupported' && claim.verification !== 'not_tested') {
			fail(`Unsupported claim ${key} cannot be marked verified`);
		}
		if (claim.implementation !== 'unsupported' && (!claim.owner_pr || !claim.evidence?.length)) {
			fail(`Supported claim ${key} requires an owner and evidence`);
		}
		for (const evidence of claim.evidence || []) {
			const separator = evidence.indexOf(':');
			if (separator < 1) fail(`Invalid evidence ${evidence} for ${key}`);
			const filePath = path.join(repositoryRoot, evidence.slice(separator + 1));
			if (!fs.existsSync(filePath)) fail(`Missing evidence ${evidence} for ${key}`);
		}
		if (claim.verification === 'differential_verified' && (claim.kind !== 'move' || !differentialMoves.has(claim.id))) {
			fail(`Differential claim ${key} is absent from the pinned fixture`);
		}
		claims.set(key, claim);
	}
	return claims;
}

function entryWithCoverage(base, claims) {
	const {pr_13_1_supported: pr131Supported = false, ...entry} = base;
	const key = `${base.kind}:${base.id}`;
	const claim = claims.get(key);
	const automatic = pr131Supported ? {
		implementation: 'implemented',
		verification: 'not_tested',
		owner_pr: 'PR 13.1',
	} : defaultCoverage();
	const coverage = claim ? {
		implementation: claim.implementation,
		verification: claim.verification,
		owner_pr: claim.owner_pr,
	} : automatic;
	return {
		...entry,
		evidence: uniqueSorted([
			...(base.evidence || []),
			...(pr131Supported ? [
				'file:crates/rotom-core/src/combat.rs',
				'file:crates/rotom-dex/src/combat.rs',
			] : []),
			...(claim?.evidence || []),
		]),
		...coverage,
	};
}

function summarize(entries, unresolved) {
	const summary = {total: entries.length, unresolved_reachability: unresolved.length, by_kind: {}};
	for (const entry of entries) {
		const kind = summary.by_kind[entry.kind] || {
			total: 0,
			implementation: {unsupported: 0, partial: 0, implemented: 0},
			verification: {not_tested: 0, unit_tested: 0, differential_verified: 0},
			reachability: {direct_legal_set: 0, battle_generated: 0, needs_research: 0},
		};
		kind.total += 1;
		kind.implementation[entry.implementation] += 1;
		kind.verification[entry.verification] += 1;
		kind.reachability[entry.reachability] += 1;
		summary.by_kind[entry.kind] = kind;
	}
	return summary;
}

function buildManifest(version, Showdown) {
	const {Dex, TeamValidator} = Showdown;
	const format = Dex.formats.get(version.format);
	if (!format.exists) fail(`Pinned Showdown does not contain format ${version.format}`);
	if (format.mod !== version.mod) fail(`Expected format mod ${version.mod}, found ${format.mod}`);
	const dex = Dex.forFormat(format);
	const validator = new TeamValidator(format.id);
	const claims = coverageClaims();
	const {directSpecies, moveEvidence, abilityEvidence} = directSetInventory(dex, validator);
	const itemEvidence = legalItemInventory(dex, validator, directSpecies);
	const generatedForms = battleGeneratedForms(dex, directSpecies, itemEvidence);
	const conditions = new Map();
	const unresolved = [];
	const reachabilityClosure = [];
	const entries = [];

	for (const [id, baseline] of directSpecies) {
		entries.push(entryWithCoverage({
			id,
			name: baseline.species.name,
			kind: 'species',
			reachability: 'direct_legal_set',
			evidence: [`format/${format.id}`, `validator/species/${id}`],
			effect_families: ['starting_species'],
		}, claims));
		if (baseline.species.forme) {
			entries.push(entryWithCoverage({
				id,
				name: baseline.species.name,
				kind: 'form',
				reachability: 'direct_legal_set',
				evidence: [`species/${id}`, `validator/species/${id}`],
				effect_families: ['starting_form'],
			}, claims));
		}
	}

	for (const [id, evidence] of moveEvidence) {
		const move = dex.moves.get(id);
		const requirements = moveExecutionRequirements(move);
		entries.push(entryWithCoverage({
			id,
			name: move.name,
			kind: 'move',
			reachability: 'direct_legal_set',
			evidence,
			effect_families: moveFamilies(move),
			operation: requirements.operation,
			required_capabilities: requirements.required_capabilities,
			pr_13_1_supported: isPr131Supported(move),
		}, claims));
		collectConditionReferences(move, `move/${id}`, conditions);
		const callbackReferences = collectCallbackConditionReferences(move, `move/${id}`, conditions);
		if (move.callsMove && ['copycat', 'sleeptalk'].includes(id)) {
			reachabilityClosure.push({
				id,
				kind: 'move_call',
				resolution: 'calls only moves already reachable from legal battle sets or battle history',
				evidence: [`move/${id}`],
			});
		} else if (move.callsMove) {
			unresolved.push({
				id,
				kind: 'move_reachability_closure',
				reachability: 'needs_research',
				reason: 'callsMove behavior requires source-backed closure research',
				evidence: [`move/${id}`],
			});
		}
		if (callbackReferences.calls > callbackReferences.extracted && !['copycat', 'sleeptalk'].includes(id)) {
			unresolved.push({
				id,
				kind: 'move_dynamic_effect',
				reachability: 'needs_research',
				reason: 'callback may generate a condition or move not exposed as structured move metadata',
				evidence: [`move/${id}`],
			});
		}
	}

	for (const [id, evidence] of abilityEvidence) {
		const ability = dex.abilities.get(id);
		entries.push(entryWithCoverage({
			id,
			name: ability.name,
			kind: 'ability',
			reachability: 'direct_legal_set',
			evidence,
			effect_families: ['ability_behavior'],
		}, claims));
		collectConditionReferences(ability, `ability/${id}`, conditions);
		const callbackReferences = collectCallbackConditionReferences(ability, `ability/${id}`, conditions);
		if (callbackReferences.calls > callbackReferences.extracted) {
			unresolved.push({
				id,
				kind: 'ability_dynamic_effect',
				reachability: 'needs_research',
				reason: 'callback may generate a condition or move not exposed as structured ability metadata',
				evidence: [`ability/${id}`],
			});
		}
	}

	for (const [id, evidence] of itemEvidence) {
		const item = dex.items.get(id);
		entries.push(entryWithCoverage({
			id,
			name: item.name,
			kind: 'item',
			reachability: 'direct_legal_set',
			evidence,
			effect_families: ['held_item_behavior'],
		}, claims));
		collectConditionReferences(item, `item/${id}`, conditions);
		const callbackReferences = collectCallbackConditionReferences(item, `item/${id}`, conditions);
		if (callbackReferences.calls > callbackReferences.extracted) {
			unresolved.push({
				id,
				kind: 'item_dynamic_effect',
				reachability: 'needs_research',
				reason: 'callback may generate a condition or move not exposed as structured item metadata',
				evidence: [`item/${id}`],
			});
		}
	}

	for (const [id, evidence] of generatedForms) {
		const species = dex.species.get(id);
		entries.push(entryWithCoverage({
			id,
			name: species.name,
			kind: 'form',
			reachability: 'battle_generated',
			evidence,
			effect_families: ['battle_transformation'],
		}, claims));
		for (const [slot, abilityName] of possibleAbilities(species)) {
			const ability = dex.abilities.get(abilityName);
			if (abilityEvidence.has(ability.id)) continue;
			const abilityKey = `ability:${ability.id}`;
			if (entries.some(entry => `${entry.kind}:${entry.id}` === abilityKey)) continue;
			entries.push(entryWithCoverage({
				id: ability.id,
				name: ability.name,
				kind: 'ability',
				reachability: 'battle_generated',
				evidence: [`form/${id}/ability-slot/${slot}`],
				effect_families: ['ability_behavior'],
			}, claims));
		}
	}

	for (const condition of conditions.values()) {
		entries.push(entryWithCoverage({
			id: condition.id,
			name: dex.conditions.get(condition.id).name || condition.id,
			kind: 'condition',
			condition_type: condition.condition_type,
			reachability: 'battle_generated',
			evidence: uniqueSorted(condition.evidence),
			effect_families: [condition.condition_type],
		}, claims));
	}

	for (const closure of [
		{trigger: 'move/transform', id: 'transform'},
		{trigger: 'ability/imposter', id: 'imposter'},
	]) {
		const [kind, id] = closure.trigger.split('/');
		if (!entries.some(entry => entry.kind === kind && entry.id === id)) continue;
		reachabilityClosure.push({
			id: closure.id,
			kind: 'state_copy',
			trigger: closure.trigger,
			resolution: 'copies only state identities already reachable on another active Pokemon',
			copies: ['ability', 'move_slots', 'non_hp_stats', 'selected_volatiles', 'species_or_form', 'stat_stages', 'types'],
			evidence: [
				closure.trigger === 'move/transform'
					? 'showdown:data/moves.ts:transform.onHit'
					: 'showdown:data/abilities.ts:imposter.onSwitchIn',
				'showdown:sim/pokemon.ts:Pokemon.transformInto',
			],
		});
	}

	entries.sort((left, right) => compareStrings(left.kind, right.kind) || compareStrings(left.id, right.id));
	unresolved.sort((left, right) => compareStrings(left.kind, right.kind) || compareStrings(left.id, right.id));
	for (const [key] of claims) {
		if (!entries.some(entry => `${entry.kind}:${entry.id}` === key)) fail(`Coverage claim ${key} is not reachable`);
	}
	for (const claim of claims.values()) {
		if (claim.kind !== 'move' || claim.implementation !== 'implemented') continue;
		const move = dex.moves.get(claim.id);
		if (!isPr131Supported(move)) {
			fail(`Move ${claim.id} is claimed implemented but requires unsupported capabilities`);
		}
	}
	return {
		schema_version: 1,
		source: {repo: version.repo, commit: version.commit},
		format: {id: format.id, name: format.name, mod: format.mod},
		team_wide_constraints: {
			species_clause: validator.ruleTable.has('speciesclause'),
			item_clause: validator.ruleTable.has('itemclause'),
			complex_team_bans: validator.ruleTable.complexTeamBans.length,
			reachability_impact: 'No complex team bans; uniqueness clauses do not remove individual set reachability',
		},
		methodology: {
			starting_sets: 'TeamValidator.validateSet over each resolved starting form, ability slot, and learnset move',
			items: 'TeamValidator.validateSet against at least one validated starting set',
			battle_forms: 'resolved battleOnly form with a reachable source and required held item',
			conditions: 'structured and literal callback condition references reachable from direct moves, abilities, and items',
			team_constraints: 'existence is validated per set; Species Clause and Item Clause do not remove individual reachability',
		},
		entries,
		reachability_closure: reachabilityClosure.sort((left, right) => compareStrings(left.kind, right.kind) || compareStrings(left.id, right.id)),
		unresolved_reachability: unresolved,
		summary: summarize(entries, unresolved),
	};
}

function reportFor(manifest) {
	const lines = [
		'# Regulation M-C mechanics coverage',
		'',
		`Pinned Showdown: \`${manifest.source.commit}\``,
		`Format: \`${manifest.format.id}\` (mod \`${manifest.format.mod}\`)`,
		'',
		'This report is generated from `reg-mc-mechanics-manifest.json`. Presence means reachable, not implemented.',
		'',
		'| Kind | Reachable | Implemented | Partial | Unsupported | Differential | Unit tested | Not tested |',
		'| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |',
	];
	for (const [kind, values] of Object.entries(manifest.summary.by_kind).sort(([a], [b]) => compareStrings(a, b))) {
		lines.push(`| ${kind} | ${values.total} | ${values.implementation.implemented} | ${values.implementation.partial} | ${values.implementation.unsupported} | ${values.verification.differential_verified} | ${values.verification.unit_tested} | ${values.verification.not_tested} |`);
	}
	lines.push('', '## Next unsupported reachable mechanics', '');
	for (const kind of Object.keys(manifest.summary.by_kind).sort(compareStrings)) {
		const next = manifest.entries.find(entry => entry.kind === kind && entry.implementation === 'unsupported');
		if (next) lines.push(`- ${kind}: \`${next.id}\` — ${next.name} (${next.reachability})`);
	}
	lines.push('', '## Unresolved reachability', '');
	if (!manifest.unresolved_reachability.length) lines.push('- None');
	for (const entry of manifest.unresolved_reachability) lines.push(`- \`${entry.kind}/${entry.id}\`: ${entry.reason}`);
	lines.push('');
	return `${lines.join('\n')}\n`;
}

function nextResearch(manifest) {
	return manifest.unresolved_reachability[0] || null;
}

function generatedContents(options = {}) {
	const version = readVersionFile();
	if (options.verifyCheckout !== false) verifyCheckout(version);
	const manifest = buildManifest(version, loadShowdown());
	return {
		manifest,
		manifestContents: `${JSON.stringify(manifest, null, 2)}\n`,
		reportContents: reportFor(manifest),
	};
}

function main() {
	try {
		const generated = generatedContents();
		if (process.argv.includes('--next-research')) {
			const next = nextResearch(generated.manifest);
			console.log(next ? `${next.kind}/${next.id}\t${next.reason}` : 'none');
			return;
		}
		if (process.argv.some(argument => argument === '--next' || argument.startsWith('--next='))) {
			const option = process.argv.find(argument => argument.startsWith('--next='));
			const kind = option?.slice('--next='.length);
			const next = generated.manifest.entries.find(entry =>
				entry.implementation === 'unsupported' && (!kind || entry.kind === kind)
			);
			console.log(next ? `${next.kind}/${next.id}\t${next.name}\t${next.reachability}` : 'none');
			return;
		}
		if (process.argv.includes('--check')) {
			for (const [filePath, expected] of [[manifestPath, generated.manifestContents], [reportPath, generated.reportContents]]) {
				if (!fs.existsSync(filePath) || fs.readFileSync(filePath, 'utf8') !== expected) {
					fail(`Generated file is stale: ${path.relative(repositoryRoot, filePath)}`);
				}
			}
			console.log('Verified Regulation M-C mechanics manifest and coverage report');
			return;
		}
		fs.mkdirSync(path.dirname(manifestPath), {recursive: true});
		fs.writeFileSync(manifestPath, generated.manifestContents);
		fs.writeFileSync(reportPath, generated.reportContents);
		console.log(`Wrote ${path.relative(repositoryRoot, manifestPath)}`);
		console.log(`Wrote ${path.relative(repositoryRoot, reportPath)}`);
	} catch (error) {
		console.error(error.message);
		process.exitCode = 1;
	}
}

module.exports = {
	battleGeneratedForms,
	buildManifest,
	directSetInventory,
	generatedContents,
	nextResearch,
	reportFor,
};

if (require.main === module) main();
