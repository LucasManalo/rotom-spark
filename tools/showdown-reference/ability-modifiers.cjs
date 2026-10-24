#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'ability-modifiers.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));
const filler = {species: 'Magikarp', ability: 'swiftswim', moves: ['splash'], level: 50};

function battleFor(
	sourceAbility, move, targetAbility, allyAbility = 'swiftswim',
	sourceSpecies = 'Medicham', targetSpecies = 'Snorlax'
) {
	const set = (species, ability, moves) => ({species, ability, moves, level: 50, nature: 'Serious', evs: {}});
	const p1Team = [set(sourceSpecies, sourceAbility, [move]), set('Magikarp', allyAbility, ['splash']), filler, filler];
	const p2Team = [set(targetSpecies, targetAbility, ['recover']), filler, filler, filler];
	const battle = new Battle({
		format: Dex.formats.get(version.format, true),
		seed: 'gen5,0000000000000001',
		p1: {team: p1Team},
		p2: {team: p2Team},
	});
	battle.makeChoices('team 1234', 'team 1234');
	return battle;
}

function damage({
	id, sourceAbility, move, targetAbility, sourceStatus = '', targetStatus = '', critical = false,
	sourceHp = null, weather = '', terrain = '', allyAbility = 'swiftswim',
}) {
	const battle = battleFor(sourceAbility, move, targetAbility, allyAbility);
	const source = battle.p1.active[0];
	const target = battle.p2.active[0];
	if (sourceStatus) source.status = sourceStatus;
	if (targetStatus) target.status = targetStatus;
	if (sourceHp !== null) source.hp = sourceHp;
	if (weather) battle.field.setWeather(weather, source);
	if (terrain) battle.field.setTerrain(terrain, source);
	battle.randomizer = value => value;
	let activeMove = battle.dex.getActiveMove(move);
	activeMove = battle.runEvent('ModifyMove', source, target, activeMove, activeMove);
	battle.setActiveMove(activeMove, source, target);
	activeMove.willCrit = critical;
	return {
		id,
		ability: allyAbility !== 'swiftswim'
			? allyAbility
			: (sourceAbility !== 'telepathy' ? sourceAbility : targetAbility),
		operation: 'damage',
		source_ability: sourceAbility,
		target_ability: targetAbility,
		move,
		critical,
		source_status: sourceStatus || null,
		target_status: targetStatus || null,
		source_hp: sourceHp,
		weather: weather || null,
		terrain: terrain || null,
		ally_ability: allyAbility,
		damage: battle.actions.getDamage(source, target, activeMove),
	};
}

function speed({id, ability, status = '', weather = '', terrain = ''}) {
	const battle = battleFor(ability, 'tackle', 'immunity');
	const source = battle.p1.active[0];
	if (status) source.status = status;
	if (weather) battle.field.setWeather(weather, source);
	if (terrain) battle.field.setTerrain(terrain, source);
	return {
		id,
		ability,
		operation: 'speed',
		status: status || null,
		weather: weather || null,
		terrain: terrain || null,
		speed: source.getStat('spe'),
	};
}

function tryHit({id, sourceAbility = 'telepathy', targetAbility, allyAbility, move, allyTarget = false}) {
	const battle = battleFor(sourceAbility, move, targetAbility || 'immunity', allyAbility);
	const source = battle.p1.active[0];
	const target = allyTarget ? battle.p1.active[1] : battle.p2.active[0];
	let activeMove = battle.dex.getActiveMove(move);
	activeMove = battle.runEvent('ModifyMove', source, target, activeMove, activeMove);
	battle.setActiveMove(activeMove, source, target);
	return {
		id,
		ability: sourceAbility === 'moldbreaker' ? sourceAbility : (allyTarget ? allyAbility : targetAbility),
		operation: 'try_hit',
		source_ability: sourceAbility,
		target_ability: allyTarget ? allyAbility : targetAbility,
		move,
		ally_target: allyTarget,
		allowed: battle.runEvent('TryHit', target, source, activeMove) === true,
	};
}

function criticalRatio({id, ability}) {
	const battle = battleFor(ability, 'dragonclaw', 'immunity');
	const source = battle.p1.active[0];
	const target = battle.p2.active[0];
	const move = battle.dex.getActiveMove('dragonclaw');
	return {
		id,
		ability,
		operation: 'critical_ratio',
		critical_ratio: battle.runEvent('ModifyCritRatio', source, target, move, move.critRatio || 0),
	};
}

function priority({id, ability, move, targetSpecies = 'Snorlax', sourceHp = null}) {
	const battle = battleFor(ability, move, 'immunity', 'swiftswim', 'Medicham', targetSpecies);
	const source = battle.p1.active[0];
	const target = battle.p2.active[0];
	if (sourceHp !== null) source.hp = sourceHp;
	const activeMove = battle.dex.getActiveMove(move);
	const value = battle.runEvent('ModifyPriority', source, target, activeMove, activeMove.priority);
	return {
		id,
		ability,
		operation: 'priority',
		move,
		target_species: targetSpecies.toLowerCase(),
		source_hp: sourceHp,
		priority: value,
		prankster_boosted: !!activeMove.pranksterBoosted,
		dark_immune: !target.runStatusImmunity('prankster'),
	};
}

function statusImmunity({id, ability, status, move, ally = false}) {
	const battle = ally
		? battleFor('telepathy', move, 'immunity', ability)
		: battleFor('telepathy', move, ability);
	const source = ally ? battle.p2.active[0] : battle.p1.active[0];
	const target = ally ? battle.p1.active[0] : battle.p2.active[0];
	const activeMove = battle.dex.getActiveMove(move);
	battle.setActiveMove(activeMove, source, target);
	return {
		id,
		ability,
		operation: 'status_immunity',
		status,
		move,
		ally,
		allowed: target.setStatus(status, source, activeMove) === true,
	};
}

function weatherImmunity({id, ability, weather}) {
	const battle = battleFor(ability, 'tackle', 'immunity');
	const source = battle.p1.active[0];
	battle.field.setWeather(weather, source);
	return {
		id,
		ability,
		operation: 'weather_immunity',
		weather,
		immune: !source.runStatusImmunity('sandstorm'),
	};
}

const cases = [
	{id: 'huge-power-physical', sourceAbility: 'hugepower', move: 'dragonclaw', targetAbility: 'immunity'},
	{id: 'huge-power-special-unaffected', sourceAbility: 'hugepower', move: 'dragonpulse', targetAbility: 'immunity'},
	{id: 'pure-power-physical', sourceAbility: 'purepower', move: 'dragonclaw', targetAbility: 'immunity'},
	{id: 'guts-burn', sourceAbility: 'guts', move: 'dragonclaw', targetAbility: 'immunity', sourceStatus: 'brn'},
	{id: 'fur-coat-physical-defense', sourceAbility: 'telepathy', move: 'dragonclaw', targetAbility: 'furcoat'},
	{id: 'marvel-scale-statused', sourceAbility: 'telepathy', move: 'dragonclaw', targetAbility: 'marvelscale', targetStatus: 'psn'},
	{id: 'technician-threshold', sourceAbility: 'technician', move: 'tackle', targetAbility: 'immunity'},
	{id: 'multiscale-full-hp', sourceAbility: 'telepathy', move: 'dragonclaw', targetAbility: 'multiscale'},
	{id: 'sniper-critical', sourceAbility: 'sniper', move: 'dragonclaw', targetAbility: 'immunity', critical: true},
	{id: 'sniper-critical-blocked-by-battle-armor', sourceAbility: 'sniper', move: 'dragonclaw', targetAbility: 'battlearmor', critical: true},
	{id: 'battle-armor-critical-prevention', sourceAbility: 'telepathy', move: 'dragonclaw', targetAbility: 'battlearmor', critical: true},
	{id: 'shell-armor-critical-prevention', sourceAbility: 'telepathy', move: 'dragonclaw', targetAbility: 'shellarmor', critical: true},
	{id: 'mold-breaker-bypasses-battle-armor', sourceAbility: 'moldbreaker', move: 'dragonclaw', targetAbility: 'battlearmor', critical: true},
	{id: 'mold-breaker-bypasses-fur-coat', sourceAbility: 'moldbreaker', move: 'dragonclaw', targetAbility: 'furcoat'},
	{id: 'mold-breaker-bypasses-multiscale', sourceAbility: 'moldbreaker', move: 'dragonclaw', targetAbility: 'multiscale'},
].map(damage);

cases.push(...[
	{id: 'blaze-low-hp', sourceAbility: 'blaze', move: 'flamethrower', targetAbility: 'immunity', sourceHp: 1},
	{id: 'overgrow-low-hp', sourceAbility: 'overgrow', move: 'energyball', targetAbility: 'immunity', sourceHp: 1},
	{id: 'torrent-low-hp', sourceAbility: 'torrent', move: 'hydropump', targetAbility: 'immunity', sourceHp: 1},
	{id: 'swarm-low-hp', sourceAbility: 'swarm', move: 'bugbuzz', targetAbility: 'immunity', sourceHp: 1},
	{id: 'iron-fist-punch', sourceAbility: 'ironfist', move: 'drainpunch', targetAbility: 'immunity'},
	{id: 'strong-jaw-bite', sourceAbility: 'strongjaw', move: 'crunch', targetAbility: 'immunity'},
	{id: 'sharpness-slicing', sourceAbility: 'sharpness', move: 'nightslash', targetAbility: 'immunity'},
	{id: 'tough-claws-contact', sourceAbility: 'toughclaws', move: 'dragonclaw', targetAbility: 'immunity'},
	{id: 'reckless-recoil', sourceAbility: 'reckless', move: 'wildcharge', targetAbility: 'immunity'},
	{id: 'punk-rock-offense', sourceAbility: 'punkrock', move: 'bugbuzz', targetAbility: 'immunity'},
	{id: 'punk-rock-defense', sourceAbility: 'telepathy', move: 'bugbuzz', targetAbility: 'punkrock'},
	{id: 'sand-force-sand', sourceAbility: 'sandforce', move: 'earthpower', targetAbility: 'immunity', weather: 'sandstorm'},
	{id: 'steely-spirit-ally', sourceAbility: 'telepathy', move: 'flashcannon', targetAbility: 'immunity', allyAbility: 'steelyspirit'},
	{id: 'filter-super-effective', sourceAbility: 'telepathy', move: 'drainpunch', targetAbility: 'filter'},
	{id: 'solid-rock-super-effective', sourceAbility: 'telepathy', move: 'drainpunch', targetAbility: 'solidrock'},
	{id: 'fluffy-contact', sourceAbility: 'telepathy', move: 'dragonclaw', targetAbility: 'fluffy'},
	{id: 'fluffy-fire', sourceAbility: 'telepathy', move: 'flamethrower', targetAbility: 'fluffy'},
	{id: 'fluffy-fire-contact-cancels', sourceAbility: 'telepathy', move: 'firepunch', targetAbility: 'fluffy'},
	{id: 'thick-fat-fire', sourceAbility: 'telepathy', move: 'flamethrower', targetAbility: 'thickfat'},
	{id: 'grass-pelt-terrain', sourceAbility: 'telepathy', move: 'dragonclaw', targetAbility: 'grasspelt', terrain: 'grassyterrain'},
	{id: 'iron-fist-versus-filter', sourceAbility: 'ironfist', move: 'drainpunch', targetAbility: 'filter'},
	{id: 'tough-claws-versus-fluffy', sourceAbility: 'toughclaws', move: 'dragonclaw', targetAbility: 'fluffy'},
	{id: 'mold-breaker-bypasses-filter', sourceAbility: 'moldbreaker', move: 'drainpunch', targetAbility: 'filter'},
	{id: 'mold-breaker-bypasses-solid-rock', sourceAbility: 'moldbreaker', move: 'drainpunch', targetAbility: 'solidrock'},
	{id: 'mold-breaker-bypasses-fluffy', sourceAbility: 'moldbreaker', move: 'firepunch', targetAbility: 'fluffy'},
	{id: 'mold-breaker-bypasses-thick-fat', sourceAbility: 'moldbreaker', move: 'flamethrower', targetAbility: 'thickfat'},
	{id: 'mold-breaker-bypasses-grass-pelt', sourceAbility: 'moldbreaker', move: 'dragonclaw', targetAbility: 'grasspelt', terrain: 'grassyterrain'},
	{id: 'mold-breaker-bypasses-punk-rock-defense', sourceAbility: 'moldbreaker', move: 'bugbuzz', targetAbility: 'punkrock'},
	{id: 'mold-breaker-keeps-source-side-steely-spirit', sourceAbility: 'moldbreaker', move: 'flashcannon', targetAbility: 'filter', allyAbility: 'steelyspirit'},
].map(damage));

cases.push(
	speed({id: 'quick-feet-paralysis', ability: 'quickfeet', status: 'par'}),
	speed({id: 'chlorophyll-sun', ability: 'chlorophyll', weather: 'sunnyday'}),
	speed({id: 'swift-swim-rain', ability: 'swiftswim', weather: 'raindance'}),
	speed({id: 'slush-rush-snow', ability: 'slushrush', weather: 'snowscape'}),
	speed({id: 'surge-surfer-electric-terrain', ability: 'surgesurfer', terrain: 'electricterrain'}),
	speed({id: 'sand-rush-sandstorm', ability: 'sandrush', weather: 'sandstorm'}),
	weatherImmunity({id: 'sand-rush-weather-immunity', ability: 'sandrush', weather: 'sandstorm'}),
	priority({id: 'prankster-status-priority-dark-immunity', ability: 'prankster', move: 'thunderwave', targetSpecies: 'Umbreon'}),
	priority({id: 'gale-wings-full-hp', ability: 'galewings', move: 'aerialace'}),
	priority({id: 'gale-wings-chipped', ability: 'galewings', move: 'aerialace', sourceHp: 1}),
	criticalRatio({id: 'super-luck-critical-ratio', ability: 'superluck'}),
	tryHit({id: 'bulletproof-blocks-aura-sphere', targetAbility: 'bulletproof', move: 'aurasphere'}),
	tryHit({id: 'soundproof-blocks-hyper-voice', targetAbility: 'soundproof', move: 'hypervoice'}),
	tryHit({id: 'telepathy-blocks-ally-surf', allyAbility: 'telepathy', move: 'surf', allyTarget: true}),
	tryHit({id: 'mold-breaker-bypasses-bulletproof', sourceAbility: 'moldbreaker', targetAbility: 'bulletproof', move: 'aurasphere'}),
	tryHit({id: 'good-as-gold-blocks-status-move', targetAbility: 'goodasgold', move: 'thunderwave'}),
	statusImmunity({id: 'immunity-blocks-poison', ability: 'immunity', status: 'psn', move: 'sludgebomb'}),
	statusImmunity({id: 'insomnia-blocks-sleep', ability: 'insomnia', status: 'slp', move: 'hypnosis'}),
	statusImmunity({id: 'vital-spirit-blocks-sleep', ability: 'vitalspirit', status: 'slp', move: 'hypnosis'}),
	statusImmunity({id: 'limber-blocks-paralysis', ability: 'limber', status: 'par', move: 'thunderwave'}),
	statusImmunity({id: 'magma-armor-blocks-freeze', ability: 'magmaarmor', status: 'frz', move: 'icebeam'}),
	statusImmunity({id: 'sweet-veil-blocks-ally-sleep', ability: 'sweetveil', status: 'slp', move: 'hypnosis', ally: true}),
);

const contents = `${JSON.stringify({
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'pinned Battle.actions.getDamage, Pokemon.getStat/runImmunity, ModifyPriority/ModifyCritRatio, and TryHit with resolved Champions data',
		damage_roll_percent: 100,
		critical_hit: 'per_case',
		full_transition_comparison: false,
	},
	cases,
}, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Ability-modifier fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
