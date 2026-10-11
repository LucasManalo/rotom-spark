#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {Battle} = require(path.join(__dirname, '..', '..', 'vendor', 'pokemon-showdown', 'dist', 'sim'));

const repositoryRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repositoryRoot, 'data', 'fixtures', 'differential', 'event-order.json');
const version = Object.fromEntries(
	fs.readFileSync(path.join(repositoryRoot, 'SHOWDOWN_VERSION'), 'utf8')
		.trim()
		.split(/\r?\n/u)
		.map(line => line.split('=', 2)),
);

const cases = [
	{
		name: 'explicit-order-before-default',
		handlers: [
			{label: 1, order: null, priority: 9, speed: 300, sub_order: 1},
			{label: 2, order: 2, priority: 0, speed: 50, sub_order: 7},
			{label: 3, order: 1, priority: -1, speed: 1, sub_order: 8},
		],
	},
	{
		name: 'priority-before-speed',
		handlers: [
			{label: 1, order: null, priority: 0, speed: 300, sub_order: 7},
			{label: 2, order: null, priority: 1, speed: 1, sub_order: 7},
			{label: 3, order: null, priority: -1, speed: 500, sub_order: 7},
		],
	},
	{
		name: 'zero-order-is-default-order',
		handlers: [
			{label: 1, order: 0, priority: -1, speed: 300, sub_order: 7},
			{label: 2, order: 1, priority: -10, speed: 1, sub_order: 7},
			{label: 3, order: null, priority: 0, speed: 1, sub_order: 7},
		],
	},
	{
		name: 'speed-before-sub-order',
		handlers: [
			{label: 1, order: null, priority: 0, speed: 100, sub_order: 1},
			{label: 2, order: null, priority: 0, speed: 200, sub_order: 9},
			{label: 3, order: null, priority: 0, speed: 150, sub_order: 4},
		],
	},
	{
		name: 'sub-order-before-effect-order',
		handlers: [
			{label: 1, order: null, priority: 0, speed: 100, sub_order: 8},
			{label: 2, order: null, priority: 0, speed: 100, sub_order: 2},
			{label: 3, order: null, priority: 0, speed: 100, sub_order: 7},
		],
	},
	{
		name: 'effect-order-breaks-exact-ties',
		handlers: [
			{label: 1, order: null, priority: 0, speed: 100, sub_order: 7},
			{label: 2, order: null, priority: 0, speed: 100, sub_order: 7},
			{label: 3, order: null, priority: 0, speed: 100, sub_order: 7},
		],
	},
];

for (const testCase of cases) {
	const showdownHandlers = testCase.handlers.map((handler, effectOrder) => ({
		label: handler.label,
		order: handler.order === null ? false : handler.order,
		priority: handler.priority,
		speed: handler.speed,
		subOrder: handler.sub_order,
		effectOrder,
	}));
	testCase.expected = showdownHandlers
		.slice()
		.sort(Battle.prototype.comparePriority)
		.map(handler => handler.label);
}

const fixture = {
	showdown_commit: version.commit,
	format: version.format,
	source: 'sim/battle.ts Battle.comparePriority',
	cases,
};
const serialized = `${JSON.stringify(fixture, null, 2)}\n`;

if (process.argv.includes('--check')) {
	const current = fs.readFileSync(outputPath, 'utf8');
	if (current !== serialized) {
		throw new Error(`${path.relative(repositoryRoot, outputPath)} is stale`);
	}
} else {
	fs.writeFileSync(outputPath, serialized);
}
