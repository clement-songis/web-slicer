import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '$lib/i18n';
import { outsideNotices } from './outside-notices';

describe('outsideNotices (audit 42 I1)', () => {
	beforeAll(() => setLocale('fr'));
	afterAll(() => setLocale('en'));
	it('à cheval : texte d’Orca, erreur bloquante avec « Disposer »', () => {
		const [n] = outsideNotices(
			[{ id: 'a', name: 'cube20.stl', state: 'partly', plateIndex: 0 }],
			1
		);
		expect(n.level).toBe('error');
		expect(n.kind).toBe('object_outside_plate');
		expect(n.action).toBe('arrange');
		expect(n.objectId).toBe('a');
		expect(n.text).toContain('Les objets suivants dépassent les limites de la plaque');
		expect(n.text).toContain('cube20.stl');
	});
	it('dehors : demande de replacer l’objet ; préfixe du plateau si plusieurs', () => {
		const [n] = outsideNotices([{ id: 'b', name: 'cyl', state: 'outside', plateIndex: 1 }], 2);
		expect(n.text).toMatch(/^Plateau 2 : cyl est hors du plateau/);
	});
});
