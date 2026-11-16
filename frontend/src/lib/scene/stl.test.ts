import { describe, it, expect } from 'vitest';
import { sceneMeshToStlBytes, sceneMeshToStlFile } from './stl';
import type { SceneMesh } from './mesh';

// Cube de 20 mm centré sur l'origine (12 triangles), pour les tests.
function cube20(): SceneMesh {
	const h = 10;
	const positions = new Float32Array([
		-h,
		-h,
		-h,
		h,
		-h,
		-h,
		h,
		h,
		-h,
		-h,
		h,
		-h,
		-h,
		-h,
		h,
		h,
		-h,
		h,
		h,
		h,
		h,
		-h,
		h,
		h
	]);
	const indices = new Uint32Array([
		0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0,
		4, 3, 4, 7
	]);
	return { positions, normals: new Float32Array(positions.length), indices };
}

// Un seul triangle dans le plan z=0 (normale +Z attendue).
const triangle: SceneMesh = {
	positions: new Float32Array([0, 0, 0, 1, 0, 0, 0, 1, 0]),
	normals: new Float32Array(9),
	indices: new Uint32Array([0, 1, 2])
};

describe('sceneMeshToStlBytes', () => {
	it('émet un STL binaire de la bonne taille (84 + 50/triangle)', () => {
		const bytes = sceneMeshToStlBytes(triangle);
		expect(bytes.byteLength).toBe(84 + 50);
		const view = new DataView(bytes.buffer);
		expect(view.getUint32(80, true)).toBe(1);
	});

	it('calcule la normale de face (produit vectoriel unitaire)', () => {
		const bytes = sceneMeshToStlBytes(triangle);
		const view = new DataView(bytes.buffer);
		expect(view.getFloat32(84, true)).toBeCloseTo(0);
		expect(view.getFloat32(88, true)).toBeCloseTo(0);
		expect(view.getFloat32(92, true)).toBeCloseTo(1);
		// Premier sommet réécrit tel quel.
		expect(view.getFloat32(96, true)).toBeCloseTo(0);
		expect(view.getFloat32(100, true)).toBeCloseTo(0);
	});

	it('encode chaque triangle d’un maillage (cube = 12 triangles)', () => {
		const mesh = cube20();
		const bytes = sceneMeshToStlBytes(mesh);
		const view = new DataView(bytes.buffer);
		expect(view.getUint32(80, true)).toBe(mesh.indices.length / 3);
		expect(bytes.byteLength).toBe(84 + (mesh.indices.length / 3) * 50);
	});
});

describe('sceneMeshToStlFile', () => {
	it('produit un File nommé .stl', () => {
		const file = sceneMeshToStlFile(triangle, 'Cube');
		expect(file.name).toBe('Cube.stl');
		expect(file.size).toBe(84 + 50);
		// Un nom déjà suffixé n'est pas redoublé.
		expect(sceneMeshToStlFile(triangle, 'part.stl').name).toBe('part.stl');
	});
});
