import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '#lib/i18n';
import { defaultEmbossParams, embossPartOffset } from './emboss';

beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));

describe('relief posé sur l’objet sélectionné (phase 26)', () => {
	it('par défaut : en relief, sur l’objet sélectionné (gizmo Emboss d’Orca)', () => {
		const p = defaultEmbossParams();
		expect(p.embossed).toBe(true);
		expect(p.onObject).toBe(true);
	});

	it('pièce centrée en XY, posée sur la face supérieure du maillage recentré', () => {
		// Cube de 20 mm recentré (Z max = 10) + texte de 2 mm d'épaisseur.
		expect(embossPartOffset(10, [30, 8, 2])).toEqual([0, 0, 11]);
		expect(embossPartOffset(2.5, [10, 10, 0.4])).toEqual([0, 0, 2.7]);
	});
});
