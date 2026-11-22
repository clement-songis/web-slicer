// Phase 46 (audit 44 FIX1-a) : instances regroupées sous l'objet dans la liste.
import { describe, expect, it } from 'vitest';
import { instanceListLayout } from './instances';

describe('instanceListLayout', () => {
	it('regroupe les instances sous le premier membre, à sa place', () => {
		const roots = [
			{ id: 'c1', instanceGroup: 'g' },
			{ id: 'b' },
			{ id: 'c2', instanceGroup: 'g' },
			{ id: 'c3', instanceGroup: 'g' }
		];
		expect(instanceListLayout(roots).map((e) => [e.node.id, e.instances.map((i) => i.id)])).toEqual(
			[
				['c1', ['c2', 'c3']],
				['b', []]
			]
		);
	});
	it('groupe à un seul membre et objets sans groupe : inchangés', () => {
		const roots = [{ id: 'a', instanceGroup: 'solo' }, { id: 'b' }];
		expect(instanceListLayout(roots).map((e) => [e.node.id, e.instances.length])).toEqual([
			['a', 0],
			['b', 0]
		]);
	});
});
