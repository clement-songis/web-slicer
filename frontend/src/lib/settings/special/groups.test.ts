// Pages spéciales (T044) : parité Annexe B ↔ registre et logique de surcharge.
import { describe, expect, it } from 'vitest';

import { PARAMS } from '../../../generated/params';
import {
	defaultFor,
	FILAMENT_MULTIMATERIAL,
	FILAMENT_OVERRIDES,
	isOverrideActive,
	MACHINE_GCODE,
	type SpecialGroup
} from './groups';

const allKeys = (groups: SpecialGroup[]) => groups.flatMap((g) => g.options);

describe('parité Annexe B ↔ registre', () => {
	it('toutes les clés des pages spéciales existent dans PARAMS', () => {
		const keys = [
			...allKeys(FILAMENT_OVERRIDES),
			...allKeys(MACHINE_GCODE),
			...allKeys(FILAMENT_MULTIMATERIAL)
		];
		for (const key of keys) {
			expect(PARAMS[key], `clé « ${key} » absente du registre`).toBeDefined();
		}
	});

	it('les blocs Machine G-code portent chacun un champ multiligne', () => {
		for (const group of MACHINE_GCODE) {
			expect(group.options).toHaveLength(1);
			const def = PARAMS[group.options[0]];
			expect(def.type === 'coString' || def.type === 'coStrings').toBe(true);
			expect(group.options[0]).toContain('gcode');
		}
	});

	it('couvre les 12 blocs G-code et les 2 groupes de surcharges', () => {
		expect(MACHINE_GCODE).toHaveLength(12);
		expect(FILAMENT_OVERRIDES.map((g) => g.title)).toEqual(['Retraction', 'Ironing']);
	});
});

describe('isOverrideActive', () => {
	it('N/A (null/undefined) = inactif, toute autre valeur = actif', () => {
		expect(isOverrideActive(null)).toBe(false);
		expect(isOverrideActive(undefined)).toBe(false);
		expect(isOverrideActive(0)).toBe(true);
		expect(isOverrideActive(false)).toBe(true);
		expect(isOverrideActive('')).toBe(true);
	});
});

describe('defaultFor', () => {
	it('amorce selon le widget quand le registre n’a pas de défaut', () => {
		// filament_wipe : coBoolsNullable, défaut null → false.
		expect(defaultFor(PARAMS['filament_wipe'])).toBe(false);
		// filament_retraction_length : coFloatsNullable, défaut null → 0.
		expect(defaultFor(PARAMS['filament_retraction_length'])).toBe(0);
		// filament_z_hop_types : coEnumsNullable → première valeur d'enum.
		const zhop = PARAMS['filament_z_hop_types'];
		expect(defaultFor(zhop)).toBe(zhop.enumValues[0] ?? '');
	});

	it('préfère le défaut du registre s’il existe', () => {
		const def = PARAMS['layer_height']; // défaut numérique connu
		expect(defaultFor(def)).toBe(def.default);
	});
});
