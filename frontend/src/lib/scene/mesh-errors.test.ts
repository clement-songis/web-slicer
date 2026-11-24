import { beforeAll, afterAll, describe, expect, test } from 'vitest';
import { setLocale } from '#lib/i18n';
import { meshErrorsTooltip, meshWarnings } from './mesh-errors';
import type { ObjectMeshErrors } from '../../generated/api/ObjectMeshErrors';

const base: ObjectMeshErrors = {
	object_id: 'o',
	open_edges: 0,
	edges_fixed: 0,
	degenerate_facets: 0,
	facets_removed: 0,
	facets_reversed: 0,
	backwards_edges: 0,
	repaired_errors: 0,
	manifold: true,
	repaired: false,
	warning: false,
	tooltip: ''
};

describe('erreurs de maillage (get_mesh_errors_info d’Orca)', () => {
	test('infobulle : réparées, restantes (pluriel), invitation à réparer', () => {
		expect(meshErrorsTooltip(base)).toBe('');
		expect(
			meshErrorsTooltip({
				...base,
				repaired: true,
				repaired_errors: 2,
				manifold: false,
				open_edges: 3
			})
		).toBe(
			'2 erreurs réparées\nErreurs restantes :\n\t3 arêtes non-manifold\n\nCliquer sur l’icône pour réparer l’objet modèle'
		);
		expect(meshErrorsTooltip({ ...base, manifold: false, open_edges: 1 })).toBe(
			'Erreurs restantes :\n\t1 arête non-manifold\n\nCliquer sur l’icône pour réparer l’objet modèle'
		);
	});

	test('seuls les objets avec warning reçoivent une alerte', () => {
		const w = meshWarnings([
			base,
			{ ...base, object_id: 'bad', warning: true, manifold: false, open_edges: 4 }
		]);
		expect(Object.keys(w)).toEqual(['bad']);
		expect(w.bad.openEdges).toBe(4);
		expect(w.bad.tooltip).toContain('4 arêtes non-manifold');
	});
});

// Les libellés de ces modules sont traduits (tr) : valeurs attendues en français.
beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));
