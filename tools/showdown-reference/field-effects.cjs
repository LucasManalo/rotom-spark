#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Dex} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'field-effects.json');
const version = Object.fromEntries(
	fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
		.trim()
		.split(/\r?\n/u)
		.map(line => line.split('=', 2)),
);
const dex = Dex.mod(version.mod);
const conditions = dex.conditions;
const moves = dex.moves;
const relay = value => ({
	chainModify(modifier) { return modifier; },
	modify(_value, modifier) { return modifier; },
	debug() {},
});

function weatherDamage(condition, weather, type) {
	return condition.onWeatherModifyDamage.call(
		relay(),
		100,
		{effectiveWeather: () => weather},
		{effectiveWeather: () => weather},
		{id: 'fixture', type, category: 'Special'},
	);
}

function terrainPower(moveId, type, options = {}) {
	const condition = moves.get(moveId).condition;
	return condition.onBasePower.call(
		relay(),
		100,
		{isGrounded: () => true, isSemiInvulnerable: () => false},
		{isGrounded: () => true, isSemiInvulnerable: () => false},
		{id: options.id || 'fixture', type},
	);
}

function screen(moveId, category) {
	const condition = moves.get(moveId).condition;
	return condition.onAnyModifyDamage.call(
		{
			...relay(),
			effectState: {target: {hasAlly: () => true}},
			getCategory: () => category,
			activePerHalf: 2,
		},
		100,
		{},
		{getMoveHitData: () => ({crit: false}), side: {getSideCondition: () => false}},
		{infiltrates: false},
	);
}

const rain = conditions.get('raindance');
const sun = conditions.get('sunnyday');
const sand = conditions.get('sandstorm');
const snow = conditions.get('snowscape');
const fixture = {
	showdown_commit: version.commit,
	format: version.format,
	source: 'resolved champions conditions and move-condition callbacks',
	weather: {
		duration: rain.duration,
		rain_water: weatherDamage(rain, 'raindance', 'Water'),
		rain_fire: weatherDamage(rain, 'raindance', 'Fire'),
		sun_fire: weatherDamage(sun, 'sunnyday', 'Fire'),
		sun_water: weatherDamage(sun, 'sunnyday', 'Water'),
		sand_special_defense: sand.onModifySpD.call(relay(), 100, {
			hasType: type => type === 'Rock', effectiveWeather: () => 'sandstorm',
		}),
		sand_residual_order: sand.onFieldResidualOrder,
		snow_physical_defense: snow.onModifyDef.call(relay(), 100, {
			hasType: type => type === 'Ice', effectiveWeather: () => 'snowscape',
		}),
	},
	terrain: {
		duration: moves.get('electricterrain').condition.duration,
		electric_power: terrainPower('electricterrain', 'Electric'),
		grassy_power: terrainPower('grassyterrain', 'Grass'),
		grassy_earthquake: terrainPower('grassyterrain', 'Ground', {id: 'earthquake'}),
		psychic_power: terrainPower('psychicterrain', 'Psychic'),
		misty_dragon: terrainPower('mistyterrain', 'Dragon'),
		residual_order: moves.get('grassyterrain').condition.onFieldResidualOrder,
	},
	side_conditions: {
		reflect_duration: moves.get('reflect').condition.duration,
		reflect_doubles: screen('reflect', 'Physical'),
		light_screen_doubles: screen('lightscreen', 'Special'),
		aurora_veil_doubles: screen('auroraveil', 'Physical'),
		tailwind_duration: moves.get('tailwind').condition.duration,
		tailwind_speed: moves.get('tailwind').condition.onModifySpe.call(relay(), 100, {}),
		residual_order: moves.get('reflect').condition.onSideResidualOrder,
	},
	rooms: Object.fromEntries(['gravity', 'magicroom', 'trickroom', 'wonderroom'].map(id => {
		const condition = moves.get(id).condition;
		return [id, {
			duration: condition.duration,
			residual_order: condition.onFieldResidualOrder,
			residual_sub_order: condition.onFieldResidualSubOrder || 0,
		}];
	})),
};
const serialized = `${JSON.stringify(fixture, null, 2)}\n`;

if (process.argv.includes('--check')) {
	if (!fs.existsSync(outputPath) || fs.readFileSync(outputPath, 'utf8') !== serialized) {
		throw new Error(`${path.relative(repositoryRoot, outputPath)} is stale`);
	}
} else {
	fs.writeFileSync(outputPath, serialized);
}
