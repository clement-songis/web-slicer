// Sérialisation des dialogs spéciaux (T045) : round-trips forme de plateau,
// matrice de purge, tableaux de températures.
import { describe, expect, it } from 'bun:test';

import { PARAMS } from '../../../generated/params';
import {
	BED_SHAPE_KEYS,
	bedExtents,
	flattenMatrix,
	FLUSH_KEYS,
	matrixSize,
	parseNumbers,
	parsePoints,
	PLATE_TYPES,
	rectangularBed,
	serializeNumbers,
	serializePoints,
	toMatrix
} from './dialogs';

describe('points (printable_area)', () => {
	it('parse le format Orca « XxY »', () => {
		expect(parsePoints(['0x0', '340x0', '340x320', '0x320'])).toEqual([
			{ x: 0, y: 0 },
			{ x: 340, y: 0 },
			{ x: 340, y: 320 },
			{ x: 0, y: 320 }
		]);
	});

	it('round-trip points ↔ chaînes', () => {
		const orca = ['0x0', '256x0', '256x256', '0x256'];
		expect(serializePoints(parsePoints(orca))).toEqual(orca);
	});

	it('tolère CSV et liste vide', () => {
		expect(parsePoints('10x10,20x20')).toEqual([
			{ x: 10, y: 10 },
			{ x: 20, y: 20 }
		]);
		expect(parsePoints([])).toEqual([]);
		expect(parsePoints(null)).toEqual([]);
	});

	it('construit un plateau rectangulaire et retrouve ses dimensions', () => {
		const bed = rectangularBed(340, 320);
		expect(serializePoints(bed)).toEqual(['0x0', '340x0', '340x320', '0x320']);
		expect(bedExtents(bed)).toEqual({ width: 340, depth: 320 });
	});
});

describe('nombres (temps plaque, coInts/coFloats)', () => {
	it('round-trip tableau de nombres ↔ chaînes Orca', () => {
		expect(parseNumbers(['35', '40'])).toEqual([35, 40]);
		expect(serializeNumbers([35, 40])).toEqual(['35', '40']);
		expect(parseNumbers([35])).toEqual([35]); // déjà numérique
	});

	it('ignore les entrées non numériques', () => {
		expect(parseNumbers(['35', '', 'x'])).toEqual([35]);
	});
});

describe('matrice de purge (flush_volumes_matrix)', () => {
	it('déduit la taille NxN d’une forme aplatie', () => {
		expect(matrixSize(new Array(4).fill(0))).toBe(2);
		expect(matrixSize(new Array(16).fill(0))).toBe(4);
	});

	it('round-trip aplati ↔ matrice', () => {
		const flat = [0, 280, 140, 0];
		const m = toMatrix(flat);
		expect(m).toEqual([
			[0, 280],
			[140, 0]
		]);
		expect(flattenMatrix(m)).toEqual(flat);
	});

	it('reconstruit une 3x3 ligne par ligne', () => {
		const flat = [0, 1, 2, 3, 4, 5, 6, 7, 8];
		expect(toMatrix(flat)).toEqual([
			[0, 1, 2],
			[3, 4, 5],
			[6, 7, 8]
		]);
	});
});

describe('parité clés dialogs ↔ registre', () => {
	it('toutes les clés des dialogs existent dans PARAMS', () => {
		const keys = [
			...Object.values(BED_SHAPE_KEYS),
			...Object.values(FLUSH_KEYS),
			...PLATE_TYPES.flatMap((p) => [p.tempKey, p.initialKey])
		];
		for (const key of keys) {
			expect(PARAMS[key], `clé « ${key} » absente du registre`).toBeDefined();
		}
	});

	it('couvre les 6 types de plaque', () => {
		expect(PLATE_TYPES).toHaveLength(6);
	});
});
