// Logique de scène (T050) : géométrie du plateau, cadrage caméra, sélection.
import { describe, expect, it } from 'bun:test';

import { bedFromValues, gridDivisions, isRectangular, parseAreaPoints } from './bed';
import { fitDistance, frameBed, viewPose, viewUp } from './camera';
import { decodeMesh } from './mesh';
import { applyPick, isSelected } from './selection';

/** Encode un maillage « WSMh » (miroir du backend) pour tester le décodage. */
function encodeWsmh(positions: number[], normals: number[], indices: number[]): ArrayBuffer {
	const vc = positions.length / 3;
	const buf = new ArrayBuffer(16 + positions.length * 4 + normals.length * 4 + indices.length * 4);
	const view = new DataView(buf);
	view.setUint32(0, 0x574d_5368, false); // "WSMh"
	view.setUint32(4, 1, true);
	view.setUint32(8, vc, true);
	view.setUint32(12, indices.length, true);
	let off = 16;
	for (const p of positions) {
		view.setFloat32(off, p, true);
		off += 4;
	}
	for (const n of normals) {
		view.setFloat32(off, n, true);
		off += 4;
	}
	for (const i of indices) {
		view.setUint32(off, i, true);
		off += 4;
	}
	return buf;
}

describe('plateau', () => {
	it('parse le contour « XxY » et détecte un rectangle', () => {
		const poly = parseAreaPoints(['0x0', '340x0', '340x320', '0x320']);
		expect(poly).toHaveLength(4);
		expect(isRectangular(poly)).toBe(true);
		expect(isRectangular(poly.slice(0, 3))).toBe(false);
	});

	it('construit le plateau depuis les valeurs du preset', () => {
		const bed = bedFromValues({
			printable_area: ['0x0', '340x0', '340x320', '0x320'],
			printable_height: 325,
			bed_custom_texture: 'tex.png'
		});
		expect(bed.width).toBe(340);
		expect(bed.depth).toBe(320);
		expect(bed.height).toBe(325);
		expect(bed.center).toEqual({ x: 170, y: 160 });
		expect(bed.rectangular).toBe(true);
		expect(bed.customTexture).toBe('tex.png');
		expect(bed.customModel).toBeNull();
	});

	it('retombe sur un plateau par défaut si le contour manque', () => {
		const bed = bedFromValues({});
		expect(bed.width).toBe(256);
		expect(bed.height).toBe(256);
		expect(bed.polygon).toHaveLength(4);
	});

	it('calcule les divisions de grille', () => {
		expect(gridDivisions(340, 10)).toBe(34);
		expect(gridDivisions(5, 10)).toBe(1); // au moins une division
	});
});

describe('caméra', () => {
	it('la distance croît avec le rayon', () => {
		expect(fitDistance(100)).toBeGreaterThan(fitDistance(50));
	});

	it('cadre le plateau : cible au centre à mi-hauteur', () => {
		const bed = bedFromValues({
			printable_area: ['0x0', '200x0', '200x200', '0x200'],
			printable_height: 200
		});
		const pose = frameBed(bed);
		expect(pose.target).toEqual([100, 100, 100]);
		// Caméra reculée (distance non nulle du centre).
		const dx = pose.position[0] - 100;
		const dy = pose.position[1] - 100;
		const dz = pose.position[2] - 100;
		expect(Math.hypot(dx, dy, dz)).toBeGreaterThan(100);
		// Vue de dessus/avant : Z caméra au-dessus de la cible.
		expect(pose.position[2]).toBeGreaterThan(100);
	});

	it('vues nommées : cible au centre, direction attendue', () => {
		const bed = bedFromValues({
			printable_area: ['0x0', '200x0', '200x200', '0x200'],
			printable_height: 200
		});
		const target: [number, number, number] = [100, 100, 100];
		// Dessus : caméra au-dessus, X/Y au centre.
		const top = viewPose(bed, 'top');
		expect(top.target).toEqual(target);
		expect(top.position[0]).toBeCloseTo(100);
		expect(top.position[1]).toBeCloseTo(100);
		expect(top.position[2]).toBeGreaterThan(100);
		expect(viewUp('top')).toEqual([0, 1, 0]);
		// Face : caméra en Y négatif.
		expect(viewPose(bed, 'front').position[1]).toBeLessThan(100);
		// Droite : caméra en X positif.
		expect(viewPose(bed, 'right').position[0]).toBeGreaterThan(100);
		// Gauche : caméra en X négatif.
		expect(viewPose(bed, 'left').position[0]).toBeLessThan(100);
		// Haut par défaut = Z+.
		expect(viewUp('front')).toEqual([0, 0, 1]);
	});
});

describe('sélection', () => {
	it('clic simple remplace la sélection', () => {
		const sel = applyPick(new Set(['a']), 'b', false);
		expect([...sel]).toEqual(['b']);
	});

	it('clic dans le vide vide la sélection (sans modificateur)', () => {
		expect(applyPick(new Set(['a', 'b']), null, false).size).toBe(0);
	});

	it('additif bascule l’objet touché', () => {
		expect([...applyPick(new Set(['a']), 'b', true)].sort()).toEqual(['a', 'b']);
		expect([...applyPick(new Set(['a', 'b']), 'b', true)]).toEqual(['a']);
	});

	it('additif dans le vide conserve la sélection', () => {
		expect(applyPick(new Set(['a']), null, true).size).toBe(1);
	});

	it('isSelected reflète l’appartenance', () => {
		expect(isSelected(new Set(['a']), 'a')).toBe(true);
		expect(isSelected(new Set(['a']), 'b')).toBe(false);
	});
});

describe('décodage maillage WSMh', () => {
	const positions = [0, 0, 0, 1, 0, 0, 0, 1, 0];
	const normals = [0, 0, 1, 0, 0, 1, 0, 0, 1];
	const indices = [0, 1, 2];

	it('décode positions/normales/indices', () => {
		const mesh = decodeMesh(encodeWsmh(positions, normals, indices));
		expect([...mesh.positions]).toEqual(positions);
		expect([...mesh.normals]).toEqual(normals);
		expect([...mesh.indices]).toEqual(indices);
	});

	it('rejette un en-tête invalide et un tampon tronqué', () => {
		expect(() => decodeMesh(new ArrayBuffer(8))).toThrow();
		const buf = encodeWsmh(positions, normals, indices);
		expect(() => decodeMesh(buf.slice(0, buf.byteLength - 4))).toThrow();
	});
});
