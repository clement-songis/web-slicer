// Audit 47 I18N-b : nom de plaque traduit dans les refus de plaque, comme
// Print.cpp:1771 (`L(bed_type_name)`) et Plater.cpp:18313 (err.params[1]) —
// msgstr d'OrcaSlicer_fr.po:10922 (« Cool Plate/Plaque PLA »).
// FUZZ-a : textes de l'avertissement de GLGizmoFuzzySkin.cpp:337/343.
import { afterAll, describe, expect, test } from 'vitest';
import { setLocale, translate } from '$lib/i18n';
import { translateOrcaMessage } from './validation';
import catalog from './validation-messages.fr.json';

afterAll(() => setLocale('en'));

describe('I18N-b : nom de plaque traduit', () => {
	test('refus du tranchage', () => {
		expect(translateOrcaMessage('Plate 1: Cool Plate does not support filament 1', catalog)).toBe(
			'Plaque 1 : Cool Plate/Plaque PLA ne prend pas en charge le filament 1'
		);
		expect(
			translateOrcaMessage('Plate 2: Textured PEI Plate does not support filament 3', catalog)
		).toBe('Plaque 2 : Plaque PEI texturée ne prend pas en charge le filament 3');
	});
	test('plaques sans msgstr actif : nom anglais conservé (comme Orca)', () => {
		expect(
			translateOrcaMessage('Plate 1: High Temp Plate does not support filament 1', catalog)
		).toBe('Plaque 1 : High Temp Plate ne prend pas en charge le filament 1');
	});
	test('notice de validation « pas recommandé »', () => {
		const fr = translateOrcaMessage(
			"Plate 1: Cool Plate is not suggested to be used to print filament 1 (Bambu PETG Basic). If you still want to do this print job, please set this filament's bed temperature to non-zero.",
			catalog
		);
		expect(fr).toContain(
			'il n’est pas recommandé d’utiliser Cool Plate/Plaque PLA pour imprimer le filament 1 (Bambu PETG Basic)'
		);
	});
});

describe('FUZZ-a : avertissement traduit', () => {
	test('msgstr d’OrcaSlicer_fr.po', () => {
		expect(
			translate('fr', 'Warning: Fuzzy skin is disabled, painted fuzzy skin will not take effect!')
		).toBe(
			'Attention : La surface irrégulière est désactivée, la surface irrégulière peinte ne prendra pas effet !'
		);
		expect(translate('fr', 'Enable painted fuzzy skin for this object')).toBe(
			'Activer la surface irrégulière peinte pour cet objet'
		);
	});
});
