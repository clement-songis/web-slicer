// Audit 45 SEQ-a — « A » / « Tout arranger » avec un plateau réglé « par objet ».
import { describe, expect, test } from 'vitest';
import { arrangeAllScopes, realPlateSequence } from './arrange';

describe('realPlateSequence (PartPlate::get_real_print_seq)', () => {
	test('surcharge du plateau, sinon traitement, sinon par couche', () => {
		expect(realPlateSequence('by object', 'by layer')).toBe('by object');
		expect(realPlateSequence(undefined, 'by object')).toBe('by object');
		expect(realPlateSequence('', 'by object')).toBe('by object');
		expect(realPlateSequence('by default', undefined)).toBe('by layer');
	});
});

describe('arrangeAllScopes', () => {
	test('séquence du plateau = séquence globale → arrangement global seul (Orca)', () => {
		expect(arrangeAllScopes(undefined, 'by layer')).toEqual(['all']);
		expect(arrangeAllScopes('by object', 'by object')).toEqual(['all']);
	});
	test('plateau « par objet » sous un traitement « par couche » → plateau arrangé avec sa séquence', () => {
		expect(arrangeAllScopes('by object', 'by layer')).toEqual(['all', 'plate']);
		expect(arrangeAllScopes('by layer', 'by object')).toEqual(['all', 'plate']);
	});
});
