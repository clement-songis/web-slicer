import { describe, expect, it } from 'vitest';
import {
	AXIS_ROTATION,
	axisOfRotation,
	buildCutRequest,
	centeredWorld,
	connectorAt,
	cutBlocker,
	defaultConnectorParams,
	defaultGroove,
	effectiveDraft,
	insideSection,
	normalizeConnector,
	offsetRange,
	planeBasis,
	planeQuaternion,
	pointToUv,
	sectionSegments,
	uvToPoint,
	wrapDeg,
	type CutDraft
} from './cut-plan';

// Cube de 20 mm centré (8 sommets, 12 triangles).
const P = new Float32Array([
	-10, -10, -10, 10, -10, -10, 10, 10, -10, -10, 10, -10, -10, -10, 10, 10, -10, 10, 10, 10, 10,
	-10, 10, 10
]);
const I = new Uint32Array([
	0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0, 4,
	3, 4, 7
]);
const plain = {
	position: [100, 100, 10] as [number, number, number],
	rotation: [0, 0, 0] as [number, number, number],
	scale: [1, 1, 1] as [number, number, number]
};
const draft = (over: Partial<CutDraft> = {}): CutDraft => ({
	mode: 'planar',
	rotation: [0, 0, 0],
	offset: 2.5,
	upper: { keep: true, placeOnCut: false, flip: false },
	lower: { keep: true, placeOnCut: false, flip: false },
	asParts: false,
	connectors: [],
	groove: null,
	...over
});
const close = (a: number[], b: number[]) => a.forEach((v, i) => expect(v).toBeCloseTo(b[i], 5));

describe('plan de coupe incliné (rotation_transform d’Orca Rz·Ry·Rx)', () => {
	it('plans droits : normales des axes', () => {
		close(planeBasis(AXIS_ROTATION.z).n, [0, 0, 1]);
		close(planeBasis(AXIS_ROTATION.x).n, [1, 0, 0]);
		close(planeBasis(AXIS_ROTATION.y).n, [0, 1, 0]);
		expect(axisOfRotation([0, 90, 0])).toBe('x');
		expect(axisOfRotation([30, 0, 0])).toBeNull();
	});
	it('rotation 30° autour de X : normale (0, −sin30, cos30)', () => {
		close(planeBasis([30, 0, 0]).n, [0, -0.5, Math.sqrt(3) / 2]);
	});
	it('ordre Orca : Rz après Ry après Rx', () => {
		const n = planeBasis([90, 0, 90]).n; // Rx(90)·Z = (0,−1,0) puis Rz(90) → (1,0,0)
		close(n, [1, 0, 0]);
	});
	it('quaternion cohérent avec la base (rotation de +Z)', () => {
		const [x, y, z, w] = planeQuaternion([20, -35, 50]);
		// q·Z·q⁻¹
		const n = [2 * (x * z + w * y), 2 * (y * z - w * x), 1 - 2 * (x * x + y * y)];
		close(n, planeBasis([20, -35, 50]).n);
	});
	it('wrapDeg ramène dans ]−180, 180]', () => {
		expect(wrapDeg(190)).toBe(-170);
		expect(wrapDeg(-180)).toBe(180);
	});
});

describe('section et connecteurs', () => {
	it('étendue le long de la normale et section carrée de 20 mm', () => {
		expect(offsetRange(P, [0, 0, 0], [0, 0, 1])).toEqual([-10, 10]);
		const b = planeBasis([0, 0, 0]);
		const s = sectionSegments(P, I, [0, 0, 2.5], b);
		expect(s.min).toEqual([-10, -10]);
		expect(s.max).toEqual([10, 10]);
		expect(insideSection(s.segments, 0, 0)).toBe(true);
		expect(insideSection(s.segments, 15, 0)).toBe(false);
	});
	it('(u, v) ↔ point du plan', () => {
		const b = planeBasis([30, 0, 0]);
		const p = uvToPoint([1, 2, 3], b, [4, -5]);
		close(pointToUv([1, 2, 3], b, p), [4, -5]);
	});
	it('règles du gizmo : goujon prisme, clip cercle et profondeur ≥ taille', () => {
		const d = normalizeConnector({ ...defaultConnectorParams(), type: 'dowel', style: 'frustum' });
		expect(d.style).toBe('prism');
		const s = normalizeConnector({
			...defaultConnectorParams(),
			type: 'snap',
			shape: 'square',
			size: 5,
			depth: 3
		});
		expect([s.shape, s.depth]).toEqual(['circle', 5]);
	});
	it('connectorAt : clic sur un connecteur existant', () => {
		const c = { id: 'a', u: 0, v: 0, ...defaultConnectorParams() };
		expect(connectorAt([c], 1, 1)?.id).toBe('a');
		expect(connectorAt([c], 5, 0)).toBeNull();
	});
	it('repère C : rotation/échelle de l’objet appliquées', () => {
		const p = centeredWorld([1, 0, 0], {
			position: [0, 0, 0],
			rotation: [0, 0, 90],
			scale: [2, 1, 1]
		});
		close([...p], [0, 2, 0]);
	});
});

describe('corps de POST /cut', () => {
	it('coupe plane droite : chemin historique (pas d’options nouvelles)', () => {
		const body = buildCutRequest(draft(), [0, 0, 0], plain, () => []);
		expect(body).toEqual({
			plane: { point: [0, 0, 2.5], normal: [0, 0, 1] },
			keep_upper: true,
			keep_lower: true
		});
	});
	it('plan incliné + matrice d’instance pour un objet tourné', () => {
		const body = buildCutRequest(
			draft({ rotation: [30, 0, 0], offset: 0 }),
			[0, 0, 0],
			{ ...plain, rotation: [0, 0, 45] },
			() => [1, 2]
		);
		expect(body.rotation_deg).toEqual([30, 0, 0]);
		expect(body.instance_matrix).toEqual([1, 2]);
		close(body.plane.normal!, [0, -0.5, Math.sqrt(3) / 2]);
	});
	it('connecteurs : positions 3D sur le plan, deux moitiés forcées, pas de pièces', () => {
		const c = {
			id: 'a',
			u: 3,
			v: -2,
			...defaultConnectorParams(),
			type: 'dowel' as const,
			shape: 'square' as const
		};
		const body = buildCutRequest(
			draft({
				connectors: [c],
				asParts: true,
				lower: { keep: false, placeOnCut: false, flip: false }
			}),
			[0, 0, 0],
			plain,
			() => []
		);
		expect(body.connectors?.[0]).toMatchObject({
			pos: [3, -2, 2.5],
			type: 'dowel',
			shape: 'square',
			style: 'prism',
			size_mm: 2.5,
			depth_mm: 3
		});
		expect(body.keep_lower).toBe(true);
		expect(body.keep_as_parts).toBeUndefined();
	});
	it('queue d’aronde : défauts du gizmo, exclusive des connecteurs', () => {
		const g = defaultGroove({ min: [-10, -10, -10], max: [10, 10, 10] });
		expect(g).toMatchObject({ depth: 1, width: 4, flapsAngle: 60, angle: 0, count: 1, gap: 10 });
		const c = { id: 'a', u: 0, v: 0, ...defaultConnectorParams() };
		const body = buildCutRequest(
			draft({ mode: 'groove', groove: g, connectors: [c] }),
			[0, 0, 0],
			plain,
			() => []
		);
		expect(body.groove).toMatchObject({ depth_mm: 1, width_mm: 4, flaps_angle_deg: 60 });
		expect(body.connectors).toBeUndefined();
	});
	it('placer sur la coupe / retourner exclusifs ; pièces sans placement', () => {
		const e = effectiveDraft(draft({ upper: { keep: true, placeOnCut: true, flip: true } }));
		expect(e.upper).toEqual({ keep: true, placeOnCut: true, flip: false });
		const body = buildCutRequest(
			draft({ asParts: true, upper: { keep: true, placeOnCut: true, flip: false } }),
			[0, 0, 0],
			plain,
			() => []
		);
		expect(body.keep_as_parts).toBe(true);
		expect(body.place_on_cut_upper).toBeUndefined();
	});
	it('blocages : aucune moitié, plan hors objet', () => {
		expect(
			cutBlocker(
				draft({
					upper: { keep: false, placeOnCut: false, flip: false },
					lower: { keep: false, placeOnCut: false, flip: false }
				}),
				[-10, 10]
			)
		).toMatch(/au moins/);
		expect(cutBlocker(draft({ offset: 12 }), [-10, 10])).toMatch(/traverse/);
		expect(cutBlocker(draft(), [-10, 10])).toBeNull();
	});
});
