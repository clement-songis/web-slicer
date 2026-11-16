import { describe, expect, it } from 'vitest';
import { translate } from '$lib/i18n';
import { loadSettingsTranslations } from './i18n';

describe('traductions paresseuses des réglages', () => {
	it("ne sont pas connues avant chargement, puis traduisent (l'anglais reste l'identité)", async () => {
		expect(translate('fr', 'Seam position')).toBe('Seam position');
		await loadSettingsTranslations('fr');
		expect(translate('fr', 'Seam position')).toBe('Position de la couture');
		expect(translate('en', 'Seam position')).toBe('Seam position');
	});

	it('chargement idempotent et sans effet pour une locale sans dictionnaire', async () => {
		await expect(loadSettingsTranslations('fr')).resolves.toBeUndefined();
		await expect(loadSettingsTranslations('en')).resolves.toBeUndefined();
	});

	it('les messages Paraglide restent prioritaires', async () => {
		await loadSettingsTranslations('fr');
		expect(translate('fr', 'Close')).toBe('Fermer');
	});
});
