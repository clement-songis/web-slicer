import { describe, expect, it } from 'vitest';
import { bedFromValues } from './bed';
import { importedPlates } from './import-plates';

const voron = bedFromValues({
	printable_area: ['0x0', '350x0', '350x350', '0x350'],
	printable_height: 340
});

describe('importedPlates (3MF multi-plateaux importé, audit 42 I1)', () => {
	it('un seul plateau : positions inchangées', () => {
		expect(
			importedPlates(
				[
					[175, 175],
					[20, 300]
				],
				voron
			)
		).toEqual([
			{ plate: 0, local: [175, 175] },
			{ plate: 0, local: [20, 300] }
		]);
	});
	it('riche.3mf : cylindre à X = 595 → plateau 2, X = 175', () => {
		const out = importedPlates(
			[
				[175, 175],
				[595, 175]
			],
			voron
		);
		expect(out[0]).toEqual({ plate: 0, local: [175, 175] });
		expect(out[1].plate).toBe(1);
		expect(out[1].local[0]).toBeCloseTo(175, 6);
		expect(out[1].local[1]).toBeCloseTo(175, 6);
	});
	it('grille 2 × 2 (plateau 3 devant, en −Y)', () => {
		const out = importedPlates(
			[
				[175, 175],
				[595, 175],
				[175, 175 - 420]
			],
			voron
		);
		expect(out.map((o) => o.plate)).toEqual([0, 1, 2]);
		expect(out[2].local[1]).toBeCloseTo(175, 6);
	});
	it('objet hors de toute grille : tout reste sur le premier plateau', () => {
		expect(
			importedPlates(
				[
					[175, 175],
					[-5000, 175]
				],
				voron
			).map((o) => o.plate)
		).toEqual([0, 0]);
	});
});
