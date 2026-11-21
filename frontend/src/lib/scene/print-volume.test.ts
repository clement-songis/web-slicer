import { describe, expect, it } from 'vitest';
import { bedFromValues } from './bed';
import type { SceneMesh } from './mesh';
import { insideBed, objectVolumeState, outsideStates, printVolumeBox } from './print-volume';

/** Cube de côté `size` centré sur l'origine (comme les maillages de la scène). */
function cube(size = 20): SceneMesh {
	const h = size / 2;
	const positions: number[] = [];
	for (const x of [-h, h])
		for (const y of [-h, h]) for (const z of [-h, h]) positions.push(x, y, z);
	return {
		positions: new Float32Array(positions),
		normals: new Float32Array(0),
		indices: new Uint32Array([0, 1, 2, 1, 3, 2, 4, 6, 5, 5, 6, 7])
	};
}

const voron = bedFromValues({
	printable_area: ['0x0', '350x0', '350x350', '0x350'],
	printable_height: 340
});
const at = (x: number, y = 175, z = 10, extra: Record<string, unknown> = {}) => ({
	id: 'o',
	mesh: cube(),
	position: [x, y, z] as [number, number, number],
	...extra
});

describe('objectVolumeState (Orca check_outside_state)', () => {
	it('dans le plateau, bord compris', () => {
		expect(objectVolumeState(at(175), voron)).toBe('inside');
		expect(objectVolumeState(at(10), voron)).toBe('inside');
		expect(objectVolumeState(at(340), voron)).toBe('inside');
	});
	it('à cheval sur le bord (X = 345, audit 42 I1) : partly', () => {
		expect(objectVolumeState(at(345), voron)).toBe('partly');
		expect(objectVolumeState(at(175, -5), voron)).toBe('partly');
	});
	it('entièrement dehors (X = 500 ; cylindre du plateau 2 à X = 595) : outside', () => {
		expect(objectVolumeState(at(500), voron)).toBe('outside');
		expect(objectVolumeState(at(595), voron)).toBe('outside');
	});
	it('plus haut que printable_height : partly', () => {
		expect(objectVolumeState(at(175, 175, 335), voron)).toBe('partly');
	});
	it('partie enfoncée sous le plateau : permise (objet « sinking » d’Orca)', () => {
		expect(objectVolumeState(at(175, 175, 5), voron)).toBe('inside');
	});
	it('rotation et échelle prises en compte', () => {
		// Cube de 20 tourné de 45° : demi-diagonale 14,14 → déborde à X = 340.
		expect(objectVolumeState(at(340, 175, 10, { rotation: [0, 0, 45] }), voron)).toBe('partly');
		expect(objectVolumeState(at(335, 175, 20, { scale: [2, 2, 2] }), voron)).toBe('partly');
	});
	it('objet plus grand que le plateau, aucun sommet dessus : partly (pas dehors)', () => {
		expect(objectVolumeState(at(175, 175, 10, { scale: [40, 40, 1] }), voron)).toBe('partly');
	});
	it('plateau circulaire (delta) : contour polygonal', () => {
		const pts: string[] = [];
		for (let i = 0; i < 64; i++) {
			const a = (i / 64) * 2 * Math.PI;
			pts.push(`${(100 * Math.cos(a)).toFixed(3)}x${(100 * Math.sin(a)).toFixed(3)}`);
		}
		const delta = bedFromValues({ printable_area: pts, printable_height: 300 });
		expect(insideBed(0, 0, delta)).toBe(true);
		expect(insideBed(95, 95, delta)).toBe(false);
		expect(objectVolumeState(at(0, 0), delta)).toBe('inside');
		expect(objectVolumeState(at(85, 85), delta)).toBe('partly');
	});
	it('outsideStates : seulement les objets dehors ou à cheval', () => {
		const objs = [
			{ ...at(175), id: 'a' },
			{ ...at(345), id: 'b' },
			{ ...at(500), id: 'c' }
		];
		expect(outsideStates(objs, voron)).toEqual({ b: 'partly', c: 'outside' });
	});
	it('printVolumeBox : boîte du plateau, sans borne basse', () => {
		const box = printVolumeBox(voron);
		expect(box.max).toEqual([350, 350, 340]);
		expect(box.min[0]).toBe(0);
		expect(box.min[2]).toBeLessThan(-1e6);
	});
});
