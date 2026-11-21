// Phase 44b : synchronisation peinture / oreilles de bordure / profil de couche
// entre les instances d'un même groupe (ModelObject d'Orca).
import { describe, expect, it } from 'vitest';
import { mapPlatePoints, planMemberSync, rememberMembers } from './instances';

describe('planMemberSync', () => {
	it('mémorise les nouveaux membres sans rien recopier', () => {
		const memory = new Map<string, string>();
		const groups = new Map([['g', ['a', 'b']]]);
		expect(planMemberSync(groups, (id) => (id === 'a' ? 'x' : 'y'), memory)).toEqual([]);
		expect(memory.get('a')).toBe('x');
		expect(memory.get('b')).toBe('y');
	});

	it('le membre modifié fait foi, puis la mémoire suit la recopie', () => {
		const data: Record<string, string> = { a: 'p0', b: 'p0', c: 'p0' };
		const sig = (id: string) => data[id];
		const memory = new Map<string, string>();
		const groups = new Map([['g', ['a', 'b', 'c']]]);
		planMemberSync(groups, sig, memory);
		data.b = 'p1';
		const plan = planMemberSync(groups, sig, memory);
		expect(plan).toEqual([{ source: 'b', targets: ['a', 'c'] }]);
		for (const id of plan[0].targets) data[id] = data.b;
		rememberMembers(['a', 'b', 'c'], sig, memory);
		expect(planMemberSync(groups, sig, memory)).toEqual([]);
	});

	it('oublie les membres retirés des groupes', () => {
		const memory = new Map([['gone', 'x']]);
		planMemberSync(new Map([['g', ['a', 'b']]]), () => 's', memory);
		expect(memory.has('gone')).toBe(false);
	});
});

describe('mapPlatePoints (oreilles dans le repère de l’objet)', () => {
	const id = { position: [0, 0, 0], rotation: [0, 0, 0], scale: [1, 1, 1] } as const;
	it('translation : suit le décalage de l’instance', () => {
		const from = { ...id, position: [10, 20, 5] as [number, number, number] };
		const to = { ...id, position: [30, 25, 5] as [number, number, number] };
		expect(mapPlatePoints([[12, 18]], from as never, to as never)).toEqual([[32, 23]]);
	});

	it('rotation Z de 90° et échelle de l’instance', () => {
		const from = { position: [0, 0, 0], rotation: [0, 0, 0], scale: [1, 1, 1] };
		const to = { position: [100, 100, 0], rotation: [0, 0, 90], scale: [2, 2, 2] };
		const [[x, y]] = mapPlatePoints([[5, 0]], from as never, to as never);
		expect(x).toBeCloseTo(100, 6);
		expect(y).toBeCloseTo(110, 6);
	});

	it('aller-retour sans perte', () => {
		const a = { position: [3, 4, 0], rotation: [0, 0, 37], scale: [1.5, 1.5, 1] };
		const b = { position: [-20, 7, 0], rotation: [0, 0, -12], scale: [1, 1, 1] };
		const pts: [number, number][] = [
			[1, 2],
			[-7.5, 9]
		];
		const back = mapPlatePoints(
			mapPlatePoints(pts, a as never, b as never),
			b as never,
			a as never
		);
		back.forEach(([x, y], i) => {
			expect(x).toBeCloseTo(pts[i][0], 5);
			expect(y).toBeCloseTo(pts[i][1], 5);
		});
	});
});
