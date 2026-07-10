// Tests de sérialisation des peintures (T056, analyse G1) : codec de facette
// fidèle à Orca, round-trip du document, et sélection au pinceau.
import { describe, expect, test } from 'bun:test';
import { meshFromTriangleSoup } from '../../loaders';
import { encodeFacet, decodeFacet, ENFORCER, BLOCKER } from './facets';
import { TrianglePainting, trianglesInRadius } from './painting';

describe('encodeFacet / decodeFacet', () => {
	test('valeurs canoniques Orca : enforcer=4, blocker=8', () => {
		expect(encodeFacet(ENFORCER)).toBe('4');
		expect(encodeFacet(BLOCKER)).toBe('8');
	});

	test('round-trip des états simples et étendus', () => {
		for (const state of [1, 2, 3, 4, 7, 10, 18]) {
			expect(decodeFacet(encodeFacet(state))).toBe(state);
		}
	});

	test('état nul → chaîne vide', () => {
		expect(encodeFacet(0)).toBe('');
		expect(decodeFacet('')).toBe(0);
	});
});

describe('TrianglePainting', () => {
	test('peint, lit et efface par canal', () => {
		const p = new TrianglePainting();
		p.paint('supports', [0, 1, 2], ENFORCER);
		expect(p.get('supports', 1)).toBe(ENFORCER);
		expect(p.get('seam', 1)).toBe(0);
		p.paint('supports', [1], 0); // efface
		expect(p.get('supports', 1)).toBe(0);
		expect(p.isEmpty()).toBe(false);
	});

	test('sérialisation → document 3MF Orca et retour', () => {
		const p = new TrianglePainting();
		p.paint('supports', [5], ENFORCER);
		p.paint('mmu', [7], 4);
		const doc = p.serialize();
		expect(doc.supports).toEqual({ '5': '4' });
		expect(doc.seam).toBeUndefined();

		const back = TrianglePainting.deserialize(doc);
		expect(back.get('supports', 5)).toBe(ENFORCER);
		expect(back.get('mmu', 7)).toBe(4);
	});

	test('un document vide donne une peinture vide', () => {
		expect(TrianglePainting.deserialize({}).isEmpty()).toBe(true);
	});
});

describe('trianglesInRadius', () => {
	test('sélectionne les triangles dont le barycentre est dans le rayon', () => {
		// Triangle A près de l'origine, triangle B loin.
		const mesh = meshFromTriangleSoup([
			0, 0, 0, 1, 0, 0, 0, 1, 0, 100, 100, 0, 101, 100, 0, 100, 101, 0
		]);
		const hits = trianglesInRadius(mesh, [0, 0, 0], 5);
		expect(hits).toEqual([0]);
	});
});
