// Phase 27 — points de mesure à taille écran constante (repères d'Orca).
import { describe, expect, it } from 'vitest';
import { MARKER_PX, markerWorldRadius } from './guides';

describe('markerWorldRadius', () => {
	it('perspective : rayon proportionnel à la distance (taille écran constante)', () => {
		const near = markerWorldRadius(MARKER_PX, 100, 800, { fov: 45 });
		const far = markerWorldRadius(MARKER_PX, 400, 800, { fov: 45 });
		expect(far / near).toBeCloseTo(4, 6);
		// 5 px sur 800 px de haut à 100 mm, champ 45° : 2·100·tan(22,5°)/800·5 ≈ 0,518 mm.
		expect(near).toBeCloseTo((2 * 100 * Math.tan(Math.PI / 8) * 5) / 800, 6);
	});

	it('orthographique : suit le zoom, indépendant de la distance', () => {
		const a = markerWorldRadius(5, 10, 500, { orthoHeight: 200, zoom: 1 });
		const b = markerWorldRadius(5, 900, 500, { orthoHeight: 200, zoom: 1 });
		const zoomed = markerWorldRadius(5, 10, 500, { orthoHeight: 200, zoom: 4 });
		expect(a).toBeCloseTo(2, 6);
		expect(b).toBeCloseTo(a, 6);
		expect(zoomed).toBeCloseTo(0.5, 6);
	});

	it('bornes : jamais nul ni démesuré (viewport vide, caméra collée)', () => {
		expect(markerWorldRadius(5, 0, 0, { fov: 45 })).toBeGreaterThan(0);
		expect(markerWorldRadius(5, 1e9, 10, { fov: 45 })).toBeLessThanOrEqual(500);
	});
});
