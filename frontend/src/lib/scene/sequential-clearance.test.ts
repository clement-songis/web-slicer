// Phase 27 — contrôle objet par objet avant tranchage (Orca sequential_print_clearance_valid).
import { describe, expect, it } from 'vitest';
import { clearanceItem, sequentialClearance, type ClearanceItem } from './sequential-clearance';
import type { SceneMesh } from './mesh';

const P = {
	radius: 40,
	heightToRod: 40,
	heightToLid: 120,
	printableHeight: 250,
	nozzleHeight: 2.5
};
const box = (id: string, x: number, y: number, w = 20, d = 20, h = 20): ClearanceItem => ({
	id,
	x,
	y,
	width: w,
	depth: d,
	height: h
});

describe('sequentialClearance — distance horizontale', () => {
	it('signale l’objet imprimé ensuite quand l’écart est inférieur au rayon', () => {
		// Écart bord à bord : 30 − 20 = 10 mm < 40 mm.
		expect(sequentialClearance([box('a', 0, 0), box('b', 30, 0)], P).close).toEqual(['b']);
	});

	it('accepte un écart supérieur au rayon de dégagement', () => {
		expect(sequentialClearance([box('a', 0, 0), box('b', 70, 0)], P).close).toEqual([]);
	});

	it('coins arrondis : en diagonale, distance euclidienne entre boîtes', () => {
		// dx = dy = 30 − 20 = 10 → 14,1 mm < 40 ; dx = dy = 50 → 42,4 mm ≥ 40.
		expect(sequentialClearance([box('a', 0, 0), box('b', 30, 30)], P).close).toEqual(['b']);
		expect(sequentialClearance([box('a', 0, 0), box('b', 50, 50)], P).close).toEqual([]);
	});

	it('objets tous plus bas que la buse : seul le diamètre extérieur de buse compte', () => {
		const flat = [box('a', 0, 0, 20, 20, 1), box('b', 25, 0, 20, 20, 1)];
		expect(sequentialClearance(flat, P).close).toEqual([]);
	});
});

describe('sequentialClearance — hauteur', () => {
	it('un objet suivi dans sa bande Y ne doit pas dépasser la hauteur au portique', () => {
		const r = sequentialClearance([box('a', 0, 0, 20, 20, 50), box('b', 100, 0)], P);
		expect(r.tall).toEqual(['a']);
	});

	it('hors de la bande Y des suivants : limite au capot ; le dernier : hauteur imprimable', () => {
		const r = sequentialClearance([box('a', 0, 0, 20, 20, 50), box('b', 0, 100, 20, 20, 200)], P);
		expect(r.tall).toEqual([]);
		const r2 = sequentialClearance([box('a', 0, 0, 20, 20, 130), box('b', 0, 100)], P);
		expect(r2.tall).toEqual(['a']);
	});
});

describe('clearanceItem', () => {
	const cube = (sx: number, sy: number, sz: number): SceneMesh => {
		const pts: number[] = [];
		for (const x of [-sx / 2, sx / 2])
			for (const y of [-sy / 2, sy / 2]) for (const z of [-sz / 2, sz / 2]) pts.push(x, y, z);
		return {
			positions: new Float32Array(pts),
			normals: new Float32Array(),
			indices: new Uint32Array()
		};
	};

	it('rotation Z de 90° : largeur et profondeur échangées ; échelle appliquée', () => {
		const it = clearanceItem(
			{
				id: 'o',
				mesh: cube(40, 10, 5),
				position: [100, 50, 2.5],
				rotation: [0, 0, 90],
				scale: [1, 2, 3]
			},
			[0, 0]
		);
		expect(it.width).toBeCloseTo(20, 4);
		expect(it.depth).toBeCloseTo(40, 4);
		expect(it.height).toBeCloseTo(15, 4);
		expect([it.x, it.y]).toEqual([100, 50]);
	});
});
