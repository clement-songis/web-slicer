// Tests du modèle d'arbre d'objets (T053).
import { describe, expect, test } from 'vitest';
import { ObjectTree } from './objects.svelte';

describe('ObjectTree', () => {
	test('ajoute des objets à la racine', () => {
		const t = new ObjectTree();
		const a = t.add('cube');
		const b = t.add('sphère');
		expect(t.roots().map((n) => n.id)).toEqual([a.id, b.id]);
		expect(a.kind).toBe('object');
		expect(a.extruder).toBe(0);
	});

	test('duplique un objet avec un nouvel id et le suffixe (copie)', () => {
		const t = new ObjectTree();
		const a = t.add('cube');
		a.settings['wall_loops'] = 3;
		const dup = t.duplicate(a.id)!;
		expect(dup.id).not.toBe(a.id);
		expect(dup.name).toBe('cube (copie)');
		expect(dup.settings['wall_loops']).toBe(3);
		// Copie indépendante.
		dup.settings['wall_loops'] = 5;
		expect(a.settings['wall_loops']).toBe(3);
	});

	test('duplique un groupe et son sous-arbre en remappant les parents', () => {
		const t = new ObjectTree();
		const a = t.add('a');
		const b = t.add('b');
		const grp = t.group([a.id, b.id])!;
		const dup = t.duplicate(grp.id)!;
		const clones = t.children(dup.id);
		expect(clones.length).toBe(2);
		expect(clones.every((c) => c.id !== a.id && c.id !== b.id)).toBe(true);
	});

	test('group / ungroup reparente les membres', () => {
		const t = new ObjectTree();
		const a = t.add('a');
		const b = t.add('b');
		const grp = t.group([a.id, b.id])!;
		expect(t.get(a.id)!.parentId).toBe(grp.id);
		expect(t.roots().map((n) => n.id)).toEqual([grp.id]);

		expect(t.ungroup(grp.id)).toBe(true);
		expect(t.get(a.id)!.parentId).toBeNull();
		expect(t.get(grp.id)).toBeUndefined();
	});

	test('group refuse moins de deux membres', () => {
		const t = new ObjectTree();
		const a = t.add('a');
		expect(t.group([a.id])).toBeUndefined();
	});

	test('remove supprime le sous-arbre', () => {
		const t = new ObjectTree();
		const a = t.add('a');
		const b = t.add('b');
		const grp = t.group([a.id, b.id])!;
		expect(t.remove(grp.id)).toBe(true);
		expect(t.list().length).toBe(0);
	});

	test('masquage et verrou effectifs héritent des ancêtres', () => {
		const t = new ObjectTree();
		const a = t.add('a');
		const b = t.add('b');
		const grp = t.group([a.id, b.id])!;
		t.setHidden(grp.id, true);
		t.setLocked(grp.id, true);
		expect(t.isHidden(a.id)).toBe(true);
		expect(t.isLocked(a.id)).toBe(true);
		// Le nœud lui-même n'est pas marqué, seul l'effectif hérite.
		expect(t.get(a.id)!.hidden).toBe(false);
	});

	test('extrudeur effectif hérité du groupe si non défini', () => {
		const t = new ObjectTree();
		const a = t.add('a');
		const b = t.add('b');
		const grp = t.group([a.id, b.id])!;
		t.setExtruder(grp.id, 2);
		expect(t.effectiveExtruder(a.id)).toBe(2);
		t.setExtruder(a.id, 1);
		expect(t.effectiveExtruder(a.id)).toBe(1); // propre prioritaire
	});

	test('setSetting définit puis efface une surcharge', () => {
		const t = new ObjectTree();
		const a = t.add('a');
		t.setSetting(a.id, 'layer_height', 0.2);
		expect(t.get(a.id)!.settings['layer_height']).toBe(0.2);
		t.setSetting(a.id, 'layer_height', undefined);
		expect('layer_height' in t.get(a.id)!.settings).toBe(false);
	});
});
