#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const repositoryRoot = path.resolve(__dirname, '..', '..');
const {TeamValidator, Teams} = require(path.join(
	repositoryRoot,
	'vendor',
	'pokemon-showdown',
	'dist',
	'sim'
));

const inputPath = process.argv[2];
if (!inputPath) {
	console.error('usage: node tools/showdown-reference/validate-team.cjs <team.txt>');
	process.exit(2);
}

const team = Teams.import(fs.readFileSync(path.resolve(inputPath), 'utf8'));
if (!team) {
	console.error('Showdown could not parse the team export.');
	process.exit(1);
}

const problems = new TeamValidator('gen9championsvgc2026regmc').validateTeam(team);
if (problems) {
	console.error(JSON.stringify({valid: false, problems}, null, 2));
	process.exit(1);
}

console.log(JSON.stringify({valid: true, pokemon: team.length}));
