// Tests du presse-papier de scène (T108).
import { describe, expect, test } from 'vitest';
import { copyObjects, pastedPosition, PASTE_OFFSET } from './clipboard';
import type { SceneMesh, SceneObject } from '../scene/mesh';

const mesh: SceneMesh = {
	positions: new Float32Array([0, 0, 0]),
	normals: new Float32Array([0, 0, 0]),
	indices: new Uint32Array([0])
};

describe('clipboard', () => {
	test('copyObjects : ne garde que les objets sélectionnés', () => {
		const objs: SceneObject[] = [
			{ id: 'a', mesh },
			{ id: 'b', mesh },
			{ id: 'c', mesh }
		];
		const copied = copyObjects(objs, new Set(['a', 'c']));
		expect(copied.map((o) => o.id)).toEqual(['a', 'c']);
		// Copie superficielle : nouvel objet, même maillage.
		expect(copied[0]).not.toBe(objs[0]);
		expect(copied[0].mesh).toBe(mesh);
	});

	test('copyObjects : sélection vide → tableau vide', () => {
		expect(copyObjects([{ id: 'a', mesh }], new Set())).toEqual([]);
	});

	test('pastedPosition : décale X/Y, préserve Z', () => {
		expect(pastedPosition([1, 2, 3])).toEqual([1 + PASTE_OFFSET, 2 + PASTE_OFFSET, 3]);
		expect(pastedPosition(undefined)).toEqual([PASTE_OFFSET, PASTE_OFFSET, 0]);
	});
});
