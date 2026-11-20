import { describe, expect, test } from 'vitest';
import { remapExtruderAfterMerge } from './presets';

describe('« Fusionner avec » un filament (delete_filament(from, to) d’Orca)', () => {
	test('les objets du slot retiré passent au filament cible, les suivants descendent', () => {
		// 4 filaments ; le 2 (slot 1) est fusionné dans le 4 (slot 3) → le 4 devient 3.
		expect([0, 1, 2, 3, 4].map((e) => remapExtruderAfterMerge(e, 1, 3))).toEqual([0, 1, 3, 2, 3]);
		// Fusion du 3 (slot 2) dans le 1 (slot 0).
		expect([0, 1, 2, 3, 4].map((e) => remapExtruderAfterMerge(e, 2, 0))).toEqual([0, 1, 2, 1, 3]);
	});
});
