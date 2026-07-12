// Fraîcheur du registre généré `params.ts` (T040) : reconstruit l'ensemble
// attendu depuis audit/parameters.json et le compare aux constantes générées.
// Échoue si l'audit a changé sans régénération (`scripts/codegen.sh`).
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { PARAM_COUNT, PARAMS, type ParamType } from '../../generated/params';

const AUDIT = resolve(import.meta.dirname, '../../../../audit/parameters.json');
const audit = JSON.parse(readFileSync(AUDIT, 'utf-8')) as {
	count: number;
	parameters: Record<string, { type: string; mode: string }>;
};

const PARAM_TYPES = new Set<ParamType>([
	'coBool',
	'coBools',
	'coBoolsNullable',
	'coEnum',
	'coEnums',
	'coEnumsNullable',
	'coFloat',
	'coFloats',
	'coFloatsNullable',
	'coFloatOrPercent',
	'coFloatsOrPercents',
	'coInt',
	'coInts',
	'coPercent',
	'coPercents',
	'coPercentsNullable',
	'coPoint',
	'coPoints',
	'coPointsGroups',
	'coString',
	'coStrings'
]);

describe('registre params généré', () => {
	it('couvre exactement les 858 paramètres de l’audit', () => {
		expect(PARAM_COUNT).toBe(audit.count);
		expect(Object.keys(PARAMS).length).toBe(audit.count);
	});

	it('a les mêmes clés que l’audit (aucune dérive)', () => {
		const generated = Object.keys(PARAMS).sort();
		const expected = Object.keys(audit.parameters).sort();
		expect(generated).toEqual(expected);
	});

	it('reprend fidèlement type et mode de chaque paramètre', () => {
		for (const [key, def] of Object.entries(PARAMS)) {
			const src = audit.parameters[key];
			expect(def.type as string).toBe(src.type);
			expect(def.mode as string).toBe(src.mode);
		}
	});

	it('n’utilise que des types de config connus', () => {
		for (const def of Object.values(PARAMS)) {
			expect(PARAM_TYPES.has(def.type)).toBe(true);
		}
	});
});
