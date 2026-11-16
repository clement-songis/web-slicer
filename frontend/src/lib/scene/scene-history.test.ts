import { describe, expect, it } from 'vitest';
import type { SceneNode } from './objects.svelte';
import { objectDiff, SceneHistory, takeSnapshot, type SnapshotSource } from './scene-history';

const node = (id: string, name = id): SceneNode => ({
	id,
	name,
	kind: 'object',
	parentId: null,
	locked: false,
	hidden: false,
	printable: true,
	extruder: 0,
	settings: {}
});

function src(ids: string[], x = 0): SnapshotSource {
	return {
		nodes: ids.map((id) => node(id)),
		plates: {
			plates: [{ id: 'plate-1', name: 'Plateau 1', plateType: '', objectIds: [...ids] }],
			activeId: 'plate-1'
		},
		objects: ids.map((id) => ({ id, position: [x, 0, 0] as [number, number, number] })),
		volumes: {},
		models: Object.fromEntries(ids.map((id) => [id, 'm-' + id]))
	};
}

describe('takeSnapshot', () => {
	it('copie profonde : modifier la source ne change pas l’instantané', () => {
		const s = src(['n1']);
		const snap = takeSnapshot(s);
		s.nodes[0].name = 'autre';
		s.plates.plates[0].objectIds.push('n9');
		(s.objects[0].position as number[])[0] = 42;
		expect(snap.nodes[0].name).toBe('n1');
		expect(snap.plates.plates[0].objectIds).toEqual(['n1']);
		expect(snap.transforms.n1.position).toEqual([0, 0, 0]);
		expect(snap.transforms.n1.scale).toEqual([1, 1, 1]);
	});
});

describe('SceneHistory', () => {
	it('undo / redo, états équivalents ignorés, branche redo tronquée', () => {
		const h = new SceneHistory();
		h.reset(takeSnapshot(src(['n1'])));
		expect(h.record(takeSnapshot(src(['n1'])))).toBe(false);
		expect(h.record(takeSnapshot(src(['n1'], 10)))).toBe(true);
		expect(h.record(takeSnapshot(src(['n1', 'n2'], 10)))).toBe(true);
		expect(Object.keys(h.undo()!.transforms)).toEqual(['n1']);
		expect(h.undo()!.transforms.n1.position[0]).toBe(0);
		expect(h.canUndo).toBe(false);
		expect(h.redo()!.transforms.n1.position[0]).toBe(10);
		h.record(takeSnapshot(src(['n1'], 99)));
		expect(h.canRedo).toBe(false);
	});

	it('un glissement (états intermédiaires non enregistrés) donne une seule entrée', () => {
		const h = new SceneHistory();
		h.reset(takeSnapshot(src(['n1'])));
		// L'éditeur n'enregistre qu'à la fin du geste : un seul record.
		h.record(takeSnapshot(src(['n1'], 30)));
		expect(h.undo()!.transforms.n1.position[0]).toBe(0);
		expect(h.canUndo).toBe(false);
	});

	it('restoring : les changements de la restauration ne sont pas ré-enregistrés', () => {
		const h = new SceneHistory();
		h.reset(takeSnapshot(src(['n1'])));
		h.record(takeSnapshot(src(['n1'], 5)));
		const prev = h.undo()!;
		h.restoring(() => {
			expect(h.record(takeSnapshot(src(['n1'], 7)))).toBe(false);
		}, prev);
		expect(h.record(takeSnapshot(src(['n1'])))).toBe(false); // = état restauré
		expect(h.canRedo).toBe(true);
	});
});

describe('objectDiff', () => {
	it('objets à recréer / à retirer', () => {
		const a = takeSnapshot(src(['n1']));
		const b = takeSnapshot(src(['n1', 'n2']));
		expect(objectDiff(a, b)).toEqual({ added: ['n2'], removed: [] });
		expect(objectDiff(b, a)).toEqual({ added: [], removed: ['n2'] });
	});
});
