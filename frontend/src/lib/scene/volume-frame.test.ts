import { describe, expect, it } from 'vitest';
import * as THREE from 'three';
import type { Transform } from './transform';
import type { SceneVolume } from './volumes';
import {
	quaternionFromEulerDeg,
	volumeFromWorld,
	volumeWorldTransform,
	worldToObjectTransform
} from './volume-frame';

const vol = (over: Partial<SceneVolume> = {}): SceneVolume => ({
	id: 'v',
	type: 'negative',
	name: 'Volume négatif 1',
	shape: 'box',
	size: [10, 6, 4],
	offset: [3, -2, 5],
	settings: {},
	...over
});
const obj = (position: number[], rotation: number[], scale: number[]): Transform => ({
	position: position as [number, number, number],
	rotation: rotation as [number, number, number],
	scale: scale as [number, number, number]
});
const close = (a: readonly number[], b: readonly number[], digits = 6) =>
	a.forEach((x, i) => expect(x).toBeCloseTo(b[i], digits));

/** Pose monde de référence calculée par Three.js (volume enfant de l'objet). */
function threeWorld(o: Transform, v: SceneVolume) {
	const parent = new THREE.Object3D();
	parent.position.set(...o.position);
	parent.rotation.set(...(o.rotation.map((a) => (a * Math.PI) / 180) as [number, number, number]));
	parent.scale.set(...o.scale);
	const child = new THREE.Object3D();
	child.position.set(...v.offset);
	const r = v.rotation ?? [0, 0, 0];
	child.rotation.set(...(r.map((a) => (a * Math.PI) / 180) as [number, number, number]));
	parent.add(child);
	parent.updateMatrixWorld(true);
	const p = new THREE.Vector3();
	const q = new THREE.Quaternion();
	child.matrixWorld.decompose(p, q, new THREE.Vector3());
	return { position: p.toArray(), quaternion: q };
}

const objects = [
	obj([0, 0, 0], [0, 0, 0], [1, 1, 1]),
	obj([120, 80, 10], [0, 0, 90], [1, 1, 1]),
	obj([50, -20, 3], [30, -45, 120], [2, 2, 2]),
	obj([10, 10, 10], [0, 0, 35], [1.5, 0.5, 2])
];

describe('volumeWorldTransform (repère objet → monde)', () => {
	it('objet neutre : la pose monde est l’offset', () => {
		const w = volumeWorldTransform(objects[0], vol({ rotation: [0, 0, 20] }));
		close(w.position, [3, -2, 5]);
		close(w.rotation, [0, 0, 20]);
		expect(w.scale).toEqual([1, 1, 1]);
	});

	it.each(objects.map((o, i) => [i, o] as const))('concorde avec Three.js (objet %i)', (_i, o) => {
		const v = vol({ rotation: [10, 0, -25] });
		const ref = threeWorld(o, v);
		const w = volumeWorldTransform(o, v);
		close(w.position, ref.position, 5);
		if (o.scale.every((s) => s === o.scale[0])) {
			// Rotation comparable seulement sans cisaillement (échelle uniforme).
			const q = new THREE.Quaternion(...quaternionFromEulerDeg(w.rotation));
			expect(Math.abs(q.dot(ref.quaternion))).toBeCloseTo(1, 6);
		}
	});
});

describe('worldToObjectTransform / volumeFromWorld (monde → repère objet)', () => {
	it.each(objects.map((o, i) => [i, o] as const))('aller-retour sans geste (objet %i)', (_i, o) => {
		const v = vol({ rotation: [0, 0, 30] });
		const back = volumeFromWorld(v, o, volumeWorldTransform(o, v));
		close(back.offset, v.offset, 3);
		close(back.size, v.size, 3);
		if (o.scale.every((s) => s === o.scale[0])) close(back.rotation ?? [0, 0, 0], [0, 0, 30], 3);
	});

	it('translation monde → offset objet (objet tourné de 90° et mis à l’échelle ×2)', () => {
		const o = obj([100, 100, 0], [0, 0, 90], [2, 2, 2]);
		const v = vol({ offset: [0, 0, 0] });
		const w = volumeWorldTransform(o, v);
		// +10 mm en X monde = −10 mm en Y objet (rotation 90°), divisés par l’échelle.
		const moved = volumeFromWorld(v, o, {
			...w,
			position: [w.position[0] + 10, w.position[1], w.position[2]]
		});
		close(moved.offset, [0, -5, 0]);
		expect(moved.rotation).toBeUndefined();
	});

	it('rotation monde → rotation propre du volume', () => {
		const o = obj([0, 0, 0], [0, 0, 45], [1, 1, 1]);
		const v = vol();
		const w = volumeWorldTransform(o, v);
		const turned = volumeFromWorld(v, o, { ...w, rotation: [0, 0, w.rotation[2] + 30] });
		close(turned.rotation ?? [0, 0, 0], [0, 0, 30], 4);
		close(turned.offset, v.offset, 3);
	});

	it('échelle du geste → taille du pavé sur les axes propres du volume', () => {
		const o = obj([5, 5, 5], [20, 0, 60], [1, 1, 1]);
		const v = vol({ rotation: [0, 0, 15] });
		const w = volumeWorldTransform(o, v);
		const scaled = volumeFromWorld(v, o, { ...w, scale: [2, 1, 0.5] });
		close(scaled.size, [20, 6, 2]);
		close(scaled.offset, v.offset, 3);
	});

	it('le bruit flottant n’introduit ni rotation ni décalage parasites', () => {
		const o = obj([1.1, 2.2, 3.3], [33, -12, 71], [1, 1, 1]);
		const t = worldToObjectTransform(o, volumeWorldTransform(o, vol({ offset: [0, 0, 0] })));
		expect(t.position).toEqual([0, 0, 0]);
		expect(t.rotation).toEqual([0, 0, 0]);
	});
});
