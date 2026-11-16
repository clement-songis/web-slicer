import { describe, expect, it } from 'vitest';
import { buildZip, crc32 } from './zip';
import { encodeBinaryStl, safeFileName, worldTriangles } from './stl';
import type { SceneObject } from '../scene/mesh';

const tri = (): SceneObject => ({
	id: 'a',
	mesh: {
		positions: new Float32Array([0, 0, 0, 1, 0, 0, 0, 1, 0]),
		normals: new Float32Array(9),
		indices: new Uint32Array([0, 1, 2])
	},
	position: [10, 20, 30]
});

describe('zip stored', () => {
	it('crc32 de référence', () => {
		expect(crc32(new TextEncoder().encode('123456789'))).toBe(0xcbf43926);
	});
	it('en-têtes et répertoire central', () => {
		const zip = buildZip([
			{ name: 'a.gcode', data: new TextEncoder().encode('G1 X1') },
			{ name: 'b.gcode', data: new TextEncoder().encode('G1 X2') }
		]);
		const v = new DataView(zip.buffer);
		expect(v.getUint32(0, true)).toBe(0x04034b50);
		const end = zip.length - 22;
		expect(v.getUint32(end, true)).toBe(0x06054b50);
		expect(v.getUint16(end + 10, true)).toBe(2);
	});
});

describe('STL binaire depuis la scène', () => {
	it('applique la position et calcule la normale', () => {
		const stl = encodeBinaryStl([worldTriangles(tri())]);
		const v = new DataView(stl.buffer);
		expect(stl.length).toBe(84 + 50);
		expect(new TextDecoder().decode(stl.slice(0, 5))).not.toBe('solid');
		expect(v.getUint32(80, true)).toBe(1);
		expect(v.getFloat32(84 + 8, true)).toBeCloseTo(1); // normale +Z
		expect([v.getFloat32(96, true), v.getFloat32(100, true), v.getFloat32(104, true)]).toEqual([
			10, 20, 30
		]);
	});
	it('rotation Z 90° et miroir conservent une normale sortante', () => {
		const object = {
			...tri(),
			position: [0, 0, 0] as [number, number, number],
			rotation: [0, 0, 90] as [number, number, number]
		};
		const p = worldTriangles(object).points;
		expect(p[3]).toBeCloseTo(0);
		expect(p[4]).toBeCloseTo(1);
		const mirrored = worldTriangles({ ...tri(), scale: [-1, 1, 1] });
		const stl = encodeBinaryStl([mirrored]);
		expect(new DataView(stl.buffer).getFloat32(84 + 8, true)).toBeCloseTo(1);
	});
	it('noms de fichiers sûrs', () => {
		expect(safeFileName('Cube/20.stl', 'stl')).toBe('Cube_20.stl');
		expect(safeFileName('', 'stl')).toBe('objet.stl');
	});
});
