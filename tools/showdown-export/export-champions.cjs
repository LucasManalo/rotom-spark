#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {execFileSync} = require('node:child_process');

const repositoryRoot = path.resolve(__dirname, '..', '..');
const showdownRoot = path.join(repositoryRoot, 'vendor', 'pokemon-showdown');
const outputPath = path.join(repositoryRoot, 'data', 'generated', 'champions-reg-mc.json');

function fail(message) {
	console.error(message);
	process.exit(1);
}

function readVersionFile() {
	const versionPath = path.join(repositoryRoot, 'SHOWDOWN_VERSION');
	const entries = Object.create(null);

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

function compareStrings(left, right) {
	return left < right ? -1 : left > right ? 1 : 0;
}

function sorted(values) {
	return [...values].sort((left, right) => compareStrings(left.id, right.id));
}

function optional(value) {
	return value === undefined || value === '' ? null : value;
}

function sortedStrings(values) {
	return [...(values || [])].sort(compareStrings);
}

function sortedStringOrStrings(value) {
	if (!value) return [];
	return sortedStrings(Array.isArray(value) ? value : [value]);
}

function sortedObjectEntries(value) {
	return Object.entries(value || {})
		.sort(([left], [right]) => compareStrings(left, right))
		.map(([key, entryValue]) => ({key, value: entryValue}));
}

function normalizeMegaStone(item) {
	if (!item.megaStone) return [];
	if (typeof item.megaStone === 'object') return sortedObjectEntries(item.megaStone);
	return [{key: item.megaEvolves || '', value: item.megaStone}];
}

function normalizeAccuracy(accuracy) {
	return accuracy === true ? {kind: 'always'} : {kind: 'percent', value: accuracy};
}

function normalizeFraction(value) {
	return value ? {numerator: value[0], denominator: value[1]} : null;
}

function normalizeMultiHit(value) {
	if (value === undefined) return null;
	return Array.isArray(value) ? {min: value[0], max: value[1]} : {min: value, max: value};
}

function normalizeDamage(value) {
	if (value === undefined) return null;
	return typeof value === 'number' ? {kind: 'fixed', value} : {kind: value};
}

function formatRuleEntries(ruleTable) {
	return [...ruleTable.entries()]
		.map(([rule, source]) => ({rule, source}))
		.sort((left, right) => compareStrings(left.rule, right.rule));
}

function formatValueRules(ruleTable) {
	return [...ruleTable.valueRules.entries()]
		.map(([rule, value]) => ({rule, value}))
		.sort((left, right) => compareStrings(left.rule, right.rule));
}

function normalizeComplexBans(bans) {
	return bans
		.map(([rule, source, limit, entries]) => ({
			rule,
			source,
			limit: Number.isFinite(limit) ? limit : null,
			entries: [...entries],
		}))
		.sort((left, right) => compareStrings(left.rule, right.rule));
}

function exportSpecies(dex, validator) {
	return sorted(dex.species.all().filter(species => species.exists)).map(species => {
		const set = {name: species.name, species: species.name, moves: [], ability: ''};
		const banReason = validator.checkSpecies(set, species, species, {});
		const sourcesByMove = new Map();
		for (const learnset of dex.species.getFullLearnset(species.id)) {
			for (const [move, sources] of Object.entries(learnset.learnset)) {
				const collected = sourcesByMove.get(move) || [];
				collected.push(...sources.filter(source => source.startsWith('9')));
				sourcesByMove.set(move, collected);
			}
		}
		const eventOnlyMoves = [...sourcesByMove]
			.filter(([, sources]) => sources.length && sources.every(source => source.startsWith('9S')))
			.map(([move]) => move)
			.sort(compareStrings);

		return {
			id: species.id,
			name: species.name,
			num: species.num,
			gen: species.gen,
			base_species: species.baseSpecies,
			forme: optional(species.forme),
			types: [...species.types],
			base_stats: {
				hp: species.baseStats.hp,
				atk: species.baseStats.atk,
				def: species.baseStats.def,
				spa: species.baseStats.spa,
				spd: species.baseStats.spd,
				spe: species.baseStats.spe,
			},
			abilities: sortedObjectEntries(species.abilities),
			weight_kg: species.weightkg,
			height_m: species.heightm,
			gender: optional(species.gender),
			gender_ratio: {male: species.genderRatio.M, female: species.genderRatio.F},
			prevo: optional(species.prevo),
			evos: sortedStrings(species.evos),
			other_formes: sortedStrings(species.otherFormes),
			required_item: optional(species.requiredItem),
			required_move: optional(species.requiredMove),
			battle_only: sortedStringOrStrings(species.battleOnly),
			changes_from: optional(species.changesFrom),
			is_mega: Boolean(species.isMega),
			is_primal: Boolean(species.isPrimal),
			is_nonstandard: species.isNonstandard || null,
			tags: sortedStrings(species.tags),
			tier: species.tier,
			move_pool: [...dex.species.getMovePool(species.id)].sort(compareStrings),
			event_only_moves: eventOnlyMoves,
			legal: banReason == null,
			ban_reason: banReason || null,
		};
	});
}

function exportForms(species) {
	return species
		.filter(entry => entry.forme !== null)
		.map(entry => ({
			id: entry.id,
			name: entry.name,
			base_species: entry.base_species,
			forme: entry.forme,
			required_item: entry.required_item,
			required_move: entry.required_move,
			battle_only: entry.battle_only,
			changes_from: entry.changes_from,
			is_mega: entry.is_mega,
			is_primal: entry.is_primal,
			legal: entry.legal,
			ban_reason: entry.ban_reason,
		}));
}

function exportMoves(dex, validator) {
	return Object.keys(dex.data.Moves).sort(compareStrings).map(id => {
		const move = dex.moves.get(id);
		const banReason = validator.checkMove({name: move.name}, move, {});
		return {
			id,
			name: move.name,
			num: move.num,
			gen: move.gen,
			type: move.type,
			category: move.category,
			base_power: move.basePower,
			accuracy: normalizeAccuracy(move.accuracy),
			pp: move.pp,
			priority: move.priority,
			target: move.target,
			flags: sortedStrings(Object.keys(move.flags || {}).filter(flag => move.flags[flag])),
			crit_ratio: move.critRatio || 1,
			multi_hit: normalizeMultiHit(move.multihit),
			drain: normalizeFraction(move.drain),
			recoil: normalizeFraction(move.recoil),
			damage: normalizeDamage(move.damage),
			status: optional(move.status),
			volatile_status: optional(move.volatileStatus),
			is_nonstandard: move.isNonstandard || null,
			legal: banReason == null,
			ban_reason: banReason || null,
		};
	});
}

function exportItems(dex, validator) {
	return sorted(dex.items.all().filter(item => item.exists)).map(item => {
		const banReason = validator.checkItem({name: item.name}, item, {});
		return {
			id: item.id,
			name: item.name,
			num: item.num,
			gen: item.gen,
			mega_stone: normalizeMegaStone(item),
			mega_evolves: optional(item.megaEvolves),
			item_user: sortedStrings(item.itemUser),
			fling: item.fling ? {
				base_power: item.fling.basePower,
				status: optional(item.fling.status),
				volatile_status: optional(item.fling.volatileStatus),
			} : null,
			natural_gift: item.naturalGift ? {
				base_power: item.naturalGift.basePower,
				type: item.naturalGift.type,
			} : null,
			boosts: sortedObjectEntries(item.boosts),
			flags: [
				item.isBerry && 'berry',
				item.isChoice && 'choice',
				item.isGem && 'gem',
				item.isPokeball && 'pokeball',
			].filter(Boolean),
			is_nonstandard: item.isNonstandard || null,
			legal: banReason == null,
			ban_reason: banReason || null,
		};
	});
}

function exportAbilities(dex, validator) {
	return sorted(dex.abilities.all().filter(ability => ability.exists)).map(ability => {
		const banReason = validator.checkAbility({name: ability.name, species: ''}, ability, {});
		return {
			id: ability.id,
			name: ability.name,
			num: ability.num,
			gen: ability.gen,
			rating: ability.rating,
			is_nonstandard: ability.isNonstandard || null,
			legal: banReason == null,
			ban_reason: banReason || null,
		};
	});
}

function exportNatures(dex, validator) {
	return sorted(dex.natures.all().filter(nature => nature.exists)).map(nature => {
		const banReason = validator.checkNature({name: nature.name}, nature, {});
		return {
			id: nature.id,
			name: nature.name,
			plus: optional(nature.plus),
			minus: optional(nature.minus),
			legal: banReason == null,
			ban_reason: banReason || null,
		};
	});
}

function buildExport(version, Showdown) {
	const {Dex, TeamValidator} = Showdown;
	const format = Dex.formats.get(version.format);
	if (!format.exists) fail(`Pinned Showdown does not contain format ${version.format}`);
	if (format.mod !== version.mod) fail(`Expected format mod ${version.mod}, found ${format.mod}`);

	const dex = Dex.forFormat(format);
	if (dex.currentMod !== version.mod) {
		fail(`Expected resolved Dex mod ${version.mod}, found ${dex.currentMod}`);
	}

	const validator = new TeamValidator(format.id);
	const ruleTable = validator.ruleTable;
	const species = exportSpecies(dex, validator);

	return {
		schema_version: 2,
		source: {repo: version.repo, commit: version.commit},
		format: {
			id: format.id,
			name: format.name,
			mod: format.mod,
			game_type: format.gameType,
			ruleset: [...format.ruleset],
			banlist: [...format.banlist],
			restricted: [...format.restricted],
			unbanlist: [...format.unbanlist],
			min_team_size: ruleTable.minTeamSize,
			max_team_size: ruleTable.maxTeamSize,
			picked_team_size: ruleTable.pickedTeamSize,
			max_move_count: ruleTable.maxMoveCount,
			min_level: ruleTable.minLevel,
			max_level: ruleTable.maxLevel,
			default_level: ruleTable.defaultLevel,
			adjust_level: ruleTable.adjustLevel,
			ev_limit: ruleTable.evLimit,
			rule_table: formatRuleEntries(ruleTable),
			value_rules: formatValueRules(ruleTable),
			complex_bans: normalizeComplexBans(ruleTable.complexBans),
			complex_team_bans: normalizeComplexBans(ruleTable.complexTeamBans),
		},
		species,
		forms: exportForms(species),
		moves: exportMoves(dex, validator),
		items: exportItems(dex, validator),
		abilities: exportAbilities(dex, validator),
		natures: exportNatures(dex, validator),
	};
}

function main() {
	const version = readVersionFile();
	const actualCommit = execFileSync('git', ['-C', showdownRoot, 'rev-parse', 'HEAD'], {
		encoding: 'utf8',
	}).trim();
	if (actualCommit !== version.commit) {
		fail(`Pinned Showdown checkout is ${actualCommit}, expected ${version.commit}`);
	}

	const contents = `${JSON.stringify(buildExport(version, loadShowdown()), null, 2)}\n`;
	if (process.argv.includes('--check')) {
		if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
			fail('Generated Champions data is stale. Run the exporter and commit its output.');
		}
		console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
		return;
	}

	fs.mkdirSync(path.dirname(outputPath), {recursive: true});
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}

main();

