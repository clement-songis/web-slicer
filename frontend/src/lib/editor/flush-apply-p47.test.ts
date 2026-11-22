// Audit 45 MM-a — matrice affichée = matrice du tranchage ; « Appliquer » ne
// réécrit que les cellules modifiées.
import { describe, expect, test } from 'vitest';
import { flushApplyMatrix, usableFlushMatrix } from './auto-flush';

describe('usableFlushMatrix', () => {
	test('matrice N×N valide reprise telle quelle', () => {
		expect(usableFlushMatrix([0, 1, 2, 0], 2)).toEqual([0, 1, 2, 0]);
	});
	test('absente, mal dimensionnée ou invalide → null (calcul automatique au tranchage)', () => {
		expect(usableFlushMatrix(undefined, 2)).toBeNull();
		expect(usableFlushMatrix([0, 1, 2, 0], 3)).toBeNull();
		expect(usableFlushMatrix([0, -1, 2, 0], 2)).toBeNull();
		expect(usableFlushMatrix([0, Number.NaN, 2, 0], 2)).toBeNull();
	});
});

describe('flushApplyMatrix', () => {
	const base = [0, 298, 418, 433, 0, 532, 228, 312, 0];
	test('rien de modifié → undefined (matrice ou calcul automatique conservés)', () => {
		expect(flushApplyMatrix(base, [...base], 3)).toBeUndefined();
	});
	test('une cellule modifiée : seules ses valeurs changent, les autres restent celles du tranchage', () => {
		const draft = [...base];
		draft[1] = 500;
		expect(flushApplyMatrix(base, draft, 3)).toEqual([0, 500, 418, 433, 0, 532, 228, 312, 0]);
	});
	test('base inconnue ou brouillon mal dimensionné', () => {
		expect(flushApplyMatrix(null, [0, 1, 2, 0], 2)).toEqual([0, 1, 2, 0]);
		expect(flushApplyMatrix(base, [0, 1], 3)).toBeUndefined();
	});
});
