// Parité G-code sur un corpus modèles × profils (gelé). Références : tools/make_refs.py.
import { test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { harnessSlice, ORACLE, readJsonRef, readRef, type FlatConfig } from '../lib/app';
import { expectSameGcode } from '../lib/gcode';

type GoldenCase = { id: string; model: string; process: string };
const cases = JSON.parse(readFileSync(join(ORACLE, 'golden', 'cases.json'), 'utf8')) as GoldenCase[];

for (const c of cases) {
	test(`${c.model} × ${c.process} @F:${c.id}`, async ({ request }) => {
		const name = c.id.slice('golden.'.length);
		const ref = readRef(`golden/${name}.gcode`);
		const cfg = readJsonRef<FlatConfig>(`golden/${name}.config.json`);
		test.skip(!ref || !cfg, 'référence Orca absente : lancer tests/oracle/tools/make_refs.py');
		expectSameGcode(await harnessSlice(request, c.model, cfg!), ref!);
	});
}
