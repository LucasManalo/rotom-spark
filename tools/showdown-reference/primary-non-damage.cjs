#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle, Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'primary-non-damage.json');
const version = Object.fromEntries(fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
	.trim().split(/\r?\n/u).map(line => line.split('=', 2)));

const filler = () => ({species: 'Magikarp', ability: 'swiftswim', moves: ['splash'], level: 50});
const set = (species, moves, ability = 'pressure') => ({species, ability, moves, level: 50});
const pad = team => [...team, ...Array.from({length: 4 - team.length}, filler)];

function battleWith(sourceMoves, targetSpecies = 'Blastoise', seed = 'gen5,0000000000000001') {
	const battle = new Battle({
		format: Dex.formats.get(version.format, true), seed,
		p1: {team: pad([set('Goodra', sourceMoves, 'sapsipper'), filler()])},
		p2: {team: pad([set(targetSpecies, ['splash']), filler()])},
	});
	battle.makeChoices('team 1234', 'team 1234');
	return battle;
}

function state(battle) {
	const pokemon = battle.p2.active[0];
	const source = battle.p1.active[0];
	const conditionState = value => value ? {id: value.id, duration: value.duration ?? null} : null;
	return {
		target: {
			hp: pokemon.hp, max_hp: pokemon.maxhp, status: pokemon.status || null,
			sleep_turns: pokemon.statusState.time ?? null, boosts: {...pokemon.boosts},
			volatiles: Object.fromEntries(Object.entries(pokemon.volatiles)
				.map(([id, value]) => [id, conditionState(value)])),
		},
		source: {
			hp: source.hp, max_hp: source.maxhp, status: source.status || null,
			boosts: {...source.boosts},
			volatiles: Object.fromEntries(Object.entries(source.volatiles)
				.map(([id, value]) => [id, conditionState(value)])),
		},
		field: {
			weather: battle.field.weather || null,
			weather_duration: battle.field.weatherState.duration ?? null,
			terrain: battle.field.terrain || null,
			terrain_duration: battle.field.terrainState.duration ?? null,
			pseudo_weather: Object.fromEntries(Object.entries(battle.field.pseudoWeather)
				.map(([id, value]) => [id, conditionState(value)])),
		},
		sides: [battle.p1, battle.p2].map(side => ({
			id: side.id,
			conditions: Object.fromEntries(Object.entries(side.sideConditions)
				.map(([id, value]) => [id, conditionState(value)])),
		})),
	};
}

function use(battle, move, source = battle.p1.active[0], target = battle.p2.active[0]) {
	const start = battle.log.length;
	const originalWillAct = battle.queue.willAct;
	if (move === 'protect') battle.queue.willAct = () => true;
	const result = battle.actions.useMove(move, source, {target});
	battle.queue.willAct = originalWillAct;
	return {
		result: Boolean(result), state: state(battle),
		log: battle.log.slice(start).filter(line => /\|(move|-status|-start|-fieldstart|-fieldend|-sidestart|-sideend|-fail|-immune|-miss)\|/u.test(line)),
	};
}

function statusCases() {
	const successBattle = battleWith(['willowisp']);
	const success = use(successBattle, 'willowisp');
	const fireBattle = battleWith(['willowisp'], 'Charizard');
	const fireImmune = use(fireBattle, 'willowisp');
	const existingBattle = battleWith(['willowisp']);
	existingBattle.p2.active[0].setStatus('par', existingBattle.p1.active[0]);
	const alreadyStatused = use(existingBattle, 'willowisp');
	const terrainBattle = battleWith(['willowisp']);
	terrainBattle.field.setTerrain('mistyterrain', terrainBattle.p1.active[0]);
	const terrainPrevented = use(terrainBattle, 'willowisp');
	const missBattle = battleWith(['willowisp']);
	missBattle.randomChance = () => false;
	const missed = use(missBattle, 'willowisp');
	return {success, fire_immune: fireImmune, already_statused: alreadyStatused, terrain_prevented: terrainPrevented, missed};
}

function thunderWaveCases() {
	const electricBattle = battleWith(['thunderwave'], 'Pikachu');
	const groundBattle = battleWith(['thunderwave'], 'Garchomp');
	return {electric_immune: use(electricBattle, 'thunderwave'), ground_immune: use(groundBattle, 'thunderwave')};
}

function sleepCases() {
	const battle = battleWith(['hypnosis'], 'Blastoise', 'gen5,0000000000000011');
	const rng = [];
	battle.randomChance = (numerator, denominator) => {
		rng.push({reason: 'accuracy', operation: {kind: 'chance', numerator, denominator}, result: true});
		return true;
	};
	const originalSample = battle.sample.bind(battle);
	battle.sample = values => {
		const result = originalSample(values);
		rng.push({reason: 'sleep_duration', operation: {kind: 'sample', values}, result});
		return result;
	};
	const hypnosis = use(battle, 'hypnosis');
	hypnosis.rng = rng;
	const terrain = battleWith(['hypnosis']);
	terrain.field.setTerrain('electricterrain', terrain.p1.active[0]);
	terrain.randomChance = () => true;
	return {hypnosis, electric_terrain_prevented: use(terrain, 'hypnosis')};
}

function statCases() {
	const swordsDance = {};
	for (const initial of [0, 5, 6]) {
		const battle = battleWith(['swordsdance']);
		battle.p1.active[0].boosts.atk = initial;
		swordsDance[`from_${initial}`] = use(battle, 'swordsdance', battle.p1.active[0], battle.p1.active[0]);
	}
	const growl = {};
	for (const initial of [0, -5, -6]) {
		const battle = battleWith(['growl']);
		battle.p2.active[0].boosts.atk = initial;
		growl[`from_${initial}`] = use(battle, 'growl');
	}
	const combinedBattle = battleWith(['nobleroar']);
	return {swords_dance: swordsDance, growl, combined: use(combinedBattle, 'nobleroar')};
}

function recoverCases() {
	const normalBattle = battleWith(['recover']);
	const normalSource = normalBattle.p1.active[0];
	normalSource.hp = Math.floor(normalSource.maxhp / 4);
	const normalBefore = normalSource.hp;
	const normal = use(normalBattle, 'recover', normalSource, normalSource);
	normal.healed = normal.state.source.hp - normalBefore;

	const oddBattle = battleWith(['recover']);
	const oddSource = oddBattle.p1.active[0];
	oddSource.baseMaxhp = 155;
	oddSource.maxhp = 155;
	oddSource.hp = 50;
	const odd = use(oddBattle, 'recover', oddSource, oddSource);
	odd.healed = odd.state.source.hp - 50;

	const fullBattle = battleWith(['recover']);
	const full = use(fullBattle, 'recover', fullBattle.p1.active[0], fullBattle.p1.active[0]);
	return {normal, odd_max_hp: odd, full_hp: full};
}

function fieldCases() {
	const weather = battleWith(['sunnyday', 'raindance']);
	const sunny = use(weather, 'sunnyday', weather.p1.active[0], weather.p1.active[0]);
	const sunnyRepeated = use(weather, 'sunnyday', weather.p1.active[0], weather.p1.active[0]);
	const replaced = use(weather, 'raindance', weather.p1.active[0], weather.p1.active[0]);
	const terrain = battleWith(['electricterrain']);
	const terrainInitial = use(terrain, 'electricterrain', terrain.p1.active[0], terrain.p1.active[0]);
	const terrainRepeated = use(terrain, 'electricterrain', terrain.p1.active[0], terrain.p1.active[0]);
	return {sunny_day: {initial: sunny, repeated: sunnyRepeated, replacement: replaced}, electric_terrain: {initial: terrainInitial, repeated: terrainRepeated}};
}

function sideCases() {
	const result = {};
	for (const move of ['reflect', 'lightscreen', 'tailwind']) {
		const battle = battleWith([move]);
		result[move] = {
			initial: use(battle, move, battle.p1.active[0], battle.p1.active[0]),
			repeated: use(battle, move, battle.p1.active[0], battle.p1.active[0]),
		};
	}
	return result;
}

function trickRoomCases() {
	const battle = battleWith(['trickroom']);
	const initial = use(battle, 'trickroom', battle.p1.active[0], battle.p1.active[0]);
	initial.active = Boolean(battle.field.pseudoWeather.trickroom);
	const repeated = use(battle, 'trickroom', battle.p1.active[0], battle.p1.active[0]);
	repeated.active = Boolean(battle.field.pseudoWeather.trickroom);
	return {initial, repeated};
}

function magnetRiseCases() {
	const battle = battleWith(['magnetrise']);
	const initial = use(battle, 'magnetrise', battle.p1.active[0], battle.p1.active[0]);
	const repeated = use(battle, 'magnetrise', battle.p1.active[0], battle.p1.active[0]);
	const gravity = battleWith(['magnetrise']);
	gravity.field.addPseudoWeather('gravity', gravity.p1.active[0]);
	const blocked = use(gravity, 'magnetrise', gravity.p1.active[0], gravity.p1.active[0]);
	return {initial, repeated, gravity_blocked: blocked};
}

function protectCase() {
	const battle = battleWith(['willowisp']);
	battle.p2.active[0].set.moves = ['protect'];
	use(battle, 'protect', battle.p2.active[0], battle.p2.active[0]);
	return use(battle, 'willowisp');
}

const observations = {
	will_o_wisp: statusCases(), thunder_wave: thunderWaveCases(), sleep: sleepCases(),
	stat_stages: statCases(), recover: recoverCases(), field_conditions: fieldCases(),
	side_conditions: sideCases(), trick_room: trickRoomCases(), magnet_rise: magnetRiseCases(),
	protect: protectCase(),
};

const output = {
	showdown_commit: version.commit,
	format: version.format,
	methodology: {
		source: 'pinned Battle.actions.useMove with resolved Champions data',
		move_execution: true,
		full_transition_comparison: false,
		rng_comparison: 'semantic operation and result comparison; PRNG seed/state parity is not claimed',
	},
	cases: [
		{name: 'will-o-wisp-matrix', move: 'willowisp', expected: {
			success_status: observations.will_o_wisp.success.state.target.status,
			fire_immune: !observations.will_o_wisp.fire_immune.result,
			already_statused: observations.will_o_wisp.already_statused.state.target.status,
			terrain_prevented: !observations.will_o_wisp.terrain_prevented.result,
			missed: !observations.will_o_wisp.missed.result,
		}},
		{name: 'thunder-wave-immunities', move: 'thunderwave', expected: {
			electric_immune: !observations.thunder_wave.electric_immune.result,
			ground_immune: !observations.thunder_wave.ground_immune.result,
		}},
		{name: 'hypnosis-sleep-rng', move: 'hypnosis', expected: {
			status: observations.sleep.hypnosis.state.target.status,
			sleep_turns: observations.sleep.hypnosis.state.target.sleep_turns,
			electric_terrain_prevented: !observations.sleep.electric_terrain_prevented.result,
			rng: observations.sleep.hypnosis.rng,
		}},
		{name: 'swords-dance-limits', move: 'swordsdance', expected: Object.fromEntries(
			Object.entries(observations.stat_stages.swords_dance).map(([name, value]) =>
				[name, value.state.source.boosts.atk]
			)
		)},
		{name: 'growl-limits', move: 'growl', expected: Object.fromEntries(
			Object.entries(observations.stat_stages.growl).map(([name, value]) =>
				[name, value.state.target.boosts.atk]
			)
		)},
		{name: 'noble-roar-combined', move: 'nobleroar', expected: {
			atk: observations.stat_stages.combined.state.target.boosts.atk,
			spa: observations.stat_stages.combined.state.target.boosts.spa,
		}},
		{name: 'recover-rounding', move: 'recover', expected: {
			normal_healed: observations.recover.normal.healed,
			odd_max_hp_healed: observations.recover.odd_max_hp.healed,
			full_hp_failed: !observations.recover.full_hp.result,
		}},
		{name: 'sunny-day-reapplication', move: 'sunnyday', expected: {
			initial_duration: observations.field_conditions.sunny_day.initial.state.field.weather_duration,
			repeated_failed: !observations.field_conditions.sunny_day.repeated.result,
			replacement: observations.field_conditions.sunny_day.replacement.state.field.weather,
		}},
		{name: 'electric-terrain-reapplication', move: 'electricterrain', expected: {
			initial_duration: observations.field_conditions.electric_terrain.initial.state.field.terrain_duration,
			repeated_failed: !observations.field_conditions.electric_terrain.repeated.result,
		}},
		...['reflect', 'lightscreen', 'tailwind'].map(move => ({
			name: `${move.replace('lightscreen', 'light-screen')}-reapplication`, move,
			expected: {
				initial_duration: observations.side_conditions[move].initial.state.sides[0].conditions[move].duration,
				repeated_failed: !observations.side_conditions[move].repeated.result,
			},
		})),
		{name: 'trick-room-toggle', move: 'trickroom', expected: {
			initial_active: observations.trick_room.initial.active,
			repeated_active: observations.trick_room.repeated.active,
		}},
		{name: 'magnet-rise-reapplication', move: 'magnetrise', expected: {
			initial_duration: observations.magnet_rise.initial.state.source.volatiles.magnetrise.duration,
			repeated_failed: !observations.magnet_rise.repeated.result,
			gravity_blocked: !observations.magnet_rise.gravity_blocked.result,
		}},
		{name: 'protect-blocks-status', move: 'protect', expected: {
			blocked: !observations.protect.result,
		}},
	],
	observations,
};
const contents = `${JSON.stringify(output, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== contents) {
		console.error('Primary non-damage fixture is stale. Run this script without --check.');
		process.exit(1);
	}
	console.log(`Verified ${path.relative(repositoryRoot, outputPath)}`);
} else {
	fs.mkdirSync(path.dirname(outputPath), {recursive: true});
	fs.writeFileSync(outputPath, contents);
	console.log(`Wrote ${path.relative(repositoryRoot, outputPath)}`);
}
