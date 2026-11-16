import { describe, expect, it } from 'vitest';
import { isOpenEnum, percentText } from './types';
import { PARAMS } from '../../../generated/params';

describe('percentText (pré-remplissage coPercent)', () => {
	it('affiche un pourcentage résolu numérique au format Orca', () => {
		expect(percentText(15)).toBe('15%');
		expect(percentText(0)).toBe('0%');
		expect(percentText(12.5)).toBe('12.5%');
	});
	it('laisse une saisie texte telle quelle', () => {
		expect(percentText('51%')).toBe('51%');
		expect(percentText('51')).toBe('51');
	});
	it('rend vide une valeur absente', () => {
		expect(percentText(undefined)).toBe('');
		expect(percentText(null)).toBe('');
	});
});

describe('isOpenEnum (enums ouverts / sans liste)', () => {
	it('combo éditable Orca → champ texte', () => {
		expect(isOpenEnum(PARAMS.support_interface_top_layers)).toBe(true);
		expect(isOpenEnum(PARAMS.support_interface_bottom_layers)).toBe(true);
	});
	it('enum à liste partagée (enum_values = def_top_fill_pattern->enum_values) → select', () => {
		// L'audit résout désormais les listes partagées : plus de champ texte de repli.
		expect(PARAMS.bottom_surface_pattern.enumValues.length).toBeGreaterThan(0);
		expect(isOpenEnum(PARAMS.bottom_surface_pattern)).toBe(false);
	});
	it('enum fermé complet → select', () => {
		expect(isOpenEnum(PARAMS.top_surface_pattern)).toBe(false);
	});
});
