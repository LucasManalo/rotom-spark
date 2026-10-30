#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {CAPABILITIES, IDENTITY_DEPENDENCIES} = require('./capability-dependencies.cjs');

const repositoryRoot = path.resolve(__dirname, '..', '..');
const manifestPath = path.join(repositoryRoot, 'data', 'generated', 'reg-mc-mechanics-manifest.json');
const outputPath = path.join(repositoryRoot, 'data', 'generated', 'reg-mc-capability-dependencies.json');
const reportPath = path.join(repositoryRoot, 'data', 'generated', 'reg-mc-capability-coverage.md');
const TARGET_STATUS = new Map([['ability', 'partial'], ['item', 'partial'], ['move', 'unsupported']]);
const COMPLETE_CAPABILITY_STATUSES = new Set(['implemented', 'verified']);

function fail(message) { throw new Error(message); }
function compareStrings(left, right) { return left < right ? -1 : left > right ? 1 : 0; }
function uniqueSorted(values) { return [...new Set(values)].sort(compareStrings); }

function identityReference(entry) {
	const collection = entry.kind === 'ability' ? 'abilities' : entry.kind === 'item' ? 'items' : 'moves';
	const exportName = entry.kind === 'ability' ? 'Abilities' : entry.kind === 'item' ? 'Items' : 'Moves';
	return `showdown:data/${collection}.ts:${exportName}.${entry.id}`;
}

function relevantEntries(manifest) {
	return manifest.entries.filter(entry => TARGET_STATUS.get(entry.kind) === entry.implementation);
}

function buildCapabilityReport(manifest) {
	const targets = relevantEntries(manifest);
	const dependencies = [];
	for (const entry of targets) {
		const capabilityIds = entry.kind === 'move'
			? (entry.required_capabilities || []).filter(id => !COMPLETE_CAPABILITY_STATUSES.has(CAPABILITIES[id]?.status))
			: IDENTITY_DEPENDENCIES.get(`${entry.kind}:${entry.id}`) || [];
		if (!capabilityIds.length) fail(`No capability dependency for ${entry.kind}:${entry.id}`);
		for (const id of capabilityIds) if (!CAPABILITIES[id]) fail(`Unknown capability ${id} for ${entry.kind}:${entry.id}`);
		const blockers = capabilityIds.filter(id => !COMPLETE_CAPABILITY_STATUSES.has(CAPABILITIES[id].status));
		if (!blockers.length) fail(`Target identity ${entry.kind}:${entry.id} has no outstanding blocker`);
		dependencies.push({
			kind: entry.kind,
			id: entry.id,
			name: entry.name,
			identity_status: entry.implementation,
			capabilities: uniqueSorted(capabilityIds),
			outstanding_blockers: uniqueSorted(blockers),
			unlocked: false,
			tests: entry.evidence.filter(value => value.startsWith('unit:') || value.startsWith('interaction_unit:')),
			differential_fixtures: entry.evidence.filter(value => value.startsWith('differential:') || value.startsWith('interaction_differential:')),
			pinned_reference_evidence: [identityReference(entry)],
		});
	}

	dependencies.sort((left, right) => compareStrings(left.kind, right.kind) || compareStrings(left.id, right.id));
	const capabilities = Object.entries(CAPABILITIES).map(([id, definition]) => {
		const blocked = dependencies.filter(entry => entry.outstanding_blockers.includes(id));
		const solelyBlocked = blocked.filter(entry => entry.outstanding_blockers.length === 1);
		return {
			id,
			...definition,
			blocked_identities: blocked.map(entry => `${entry.kind}:${entry.id}`),
			unlocked_identities: [],
			would_unlock_if_completed: solelyBlocked.map(entry => `${entry.kind}:${entry.id}`),
		};
	}).sort((left, right) => compareStrings(left.id, right.id));

	const identityCoverage = {};
	for (const kind of ['ability', 'item', 'move']) {
		const summary = manifest.summary.by_kind[kind];
		identityCoverage[kind] = {reachable: summary.total, ...summary.implementation};
	}
	const capabilityCoverage = {total: capabilities.length, unsupported: 0, partial: 0, implemented: 0, verified: 0};
	for (const entry of capabilities) capabilityCoverage[entry.status] += 1;
	const targetCounts = Object.fromEntries(['ability', 'item', 'move'].map(kind => [
		kind, dependencies.filter(entry => entry.kind === kind).length,
	]));
	return {
		schema_version: 1,
		source: manifest.source,
		format: manifest.format,
		methodology: {
			identity_scope: 'Every partial ability/item and unsupported move in the generated reachable-mechanics manifest.',
			move_dependencies: 'Derived from resolved move execution requirements; only capabilities not currently implemented or verified are blockers.',
			ability_item_dependencies: 'Reviewed capability groups validated against the complete generated target set.',
			unlocked_rule: 'An identity is unlocked only when every dependency is implemented or verified; current partial/unsupported identities therefore remain locked.',
		},
		summary: {identity_coverage: identityCoverage, executable_capability_coverage: capabilityCoverage, dependency_targets: targetCounts},
		capabilities,
		identity_dependencies: dependencies,
	};
}

function reportFor(audit) {
	const lines = [
		'# Regulation M-C capability coverage and blocker graph', '',
		`Pinned Showdown: \`${audit.source.commit}\``,
		`Format: \`${audit.format.id}\` (mod \`${audit.format.mod}\`)`, '',
		'> Generated from the reachable-mechanics manifest. Identity registration, executable capability coverage, and verification are separate claims.', '',
		'## Identity coverage', '',
		'| Kind | Reachable | Implemented | Partial | Unsupported |',
		'| --- | ---: | ---: | ---: | ---: |',
	];
	for (const kind of ['ability', 'item', 'move']) {
		const value = audit.summary.identity_coverage[kind];
		lines.push(`| ${kind} | ${value.reachable} | ${value.implemented} | ${value.partial} | ${value.unsupported} |`);
	}
	const capability = audit.summary.executable_capability_coverage;
	lines.push(
		'', '## Executable capability coverage', '',
		'| Total | Verified | Implemented | Partial | Unsupported |',
		'| ---: | ---: | ---: | ---: | ---: |',
		`| ${capability.total} | ${capability.verified} | ${capability.implemented} | ${capability.partial} | ${capability.unsupported} |`,
		'',
		'`implemented` means the shared capability executes in its stated scope. `verified` additionally has qualifying pinned-reference evidence. Partial and unsupported capabilities continue to block dependent identities.',
		'', '## Highest-leverage blockers', '',
		'| Capability | Status | Blocked identities | Sole remaining blocker | Known gap |',
		'| --- | --- | ---: | ---: | --- |'
	);
	for (const entry of [...audit.capabilities].filter(value => value.blocked_identities.length)
		.sort((left, right) => right.blocked_identities.length - left.blocked_identities.length || compareStrings(left.id, right.id))) {
		lines.push(`| \`${entry.id}\` | ${entry.status} | ${entry.blocked_identities.length} | ${entry.would_unlock_if_completed.length} | ${entry.known_gaps.join(' ')} |`);
	}
	lines.push('', '## Dependency audit', '');
	for (const kind of ['ability', 'item', 'move']) {
		const entries = audit.identity_dependencies.filter(entry => entry.kind === kind);
		lines.push(`- ${kind}: ${entries.length} ${kind === 'move' ? 'unsupported' : 'partial'} identities mapped; ${entries.filter(entry => entry.unlocked).length} currently unlocked.`);
	}
	lines.push(
		'',
		'Mega Stones are reported as a **metadata foundation**. They remain partial until `mega_eligibility`, `mega_timing`, `form_change`, and `ability_transition` are complete.',
		'',
		'The complete per-identity graph, sub-capabilities, tests, fixtures, known gaps, and pinned source references are in `reg-mc-capability-dependencies.json`.',
		''
	);
	return `${lines.join('\n')}\n`;
}

function generatedContents() {
	const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
	const audit = buildCapabilityReport(manifest);
	return {audit, json: `${JSON.stringify(audit, null, 2)}\n`, markdown: reportFor(audit)};
}

function main() {
	try {
		const generated = generatedContents();
		if (process.argv.includes('--check')) {
			for (const [filePath, expected] of [[outputPath, generated.json], [reportPath, generated.markdown]]) {
				if (!fs.existsSync(filePath) || fs.readFileSync(filePath, 'utf8') !== expected) fail(`Generated file is stale: ${path.relative(repositoryRoot, filePath)}`);
			}
			console.log('Verified Regulation M-C capability dependency graph and report');
			return;
		}
		fs.writeFileSync(outputPath, generated.json);
		fs.writeFileSync(reportPath, generated.markdown);
		console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
		console.log(`Wrote ${path.relative(repositoryRoot, reportPath)}`);
	} catch (error) {
		console.error(error.message);
		process.exitCode = 1;
	}
}

module.exports = {buildCapabilityReport, generatedContents, reportFor};
if (require.main === module) main();
