// Audit 46 GCHK-a : trajet hors du plateau (bit 2) / trop haut (bit 3) —
// même bandeau que H2D, texte d'Orca (GLCanvas3D.cpp:10101-10102).
import { describe, expect, test } from 'vitest';
import { setLocale } from '$lib/i18n';
import { gcodeCheckError, gcodeCheckKind } from './nozzle-range-error';
import { translateOrcaMessage } from './validation';
import catalog from './validation-messages.fr.json';

describe('contrôle des trajets hors plateau', () => {
	test('message d’Orca, traduit, bit 2 → plate-boundary', () => {
		setLocale('en');
		const raw = {
			error_code: 4,
			message: 'A G-code path goes beyond the plate boundaries.',
			issues: []
		};
		expect(gcodeCheckError(raw)).toBe('A G-code path goes beyond the plate boundaries.');
		expect(gcodeCheckKind(raw)).toBe('plate-boundary');
		setLocale('fr');
		expect(gcodeCheckError(raw)).toBe('Un chemin du G-code va au-delà de la limite du plateau.');
		setLocale('en');
	});
	test('bit 2 sans texte reconnu : message d’Orca quand même', () => {
		expect(gcodeCheckError({ error_code: 4, message: '' })).toBe(
			'A G-code path goes beyond the plate boundaries.'
		);
	});
	test('hauteur (bit 3) et buse (bit 0) combinées : une ligne chacune', () => {
		const raw = {
			error_code: 9,
			message:
				'Filament 2 is placed in the right nozzle, but the generated G-code path exceeds the printable range of the right nozzle.\nA G-code path goes beyond the max print height.'
		};
		const text = gcodeCheckError(raw)!.split('\n');
		expect(text).toEqual([
			'A G-code path goes beyond the max print height.',
			'Filament 2 is placed in the right nozzle, but the generated G-code path exceeds the printable range of the right nozzle.'
		]);
		expect(gcodeCheckKind(raw)).toBe('plate-height');
	});
	test('bits multi-buses seuls : nozzle-range (inchangé)', () => {
		const raw = {
			error_code: 1,
			message:
				'Filament 2 is placed in the right nozzle, but the generated G-code path exceeds the printable range of the right nozzle.'
		};
		expect(gcodeCheckKind(raw)).toBe('nozzle-range');
		expect(gcodeCheckKind(null)).toBeNull();
	});
	test('notice de validation traduite (catalogue)', () => {
		expect(translateOrcaMessage('A G-code path goes beyond the plate boundaries.', catalog)).toBe(
			'Un chemin du G-code va au-delà de la limite du plateau.'
		);
	});
});

describe('I18N-a : refus de plaque du tranchage traduit', () => {
	test('« Plate 1: Cool Plate does not support filament 1 »', () => {
		expect(translateOrcaMessage('Plate 1: Cool Plate does not support filament 1', catalog)).toBe(
			// Audit 47 I18N-b : nom de plaque traduit (OrcaSlicer_fr.po:10922).
			'Plaque 1 : Cool Plate/Plaque PLA ne prend pas en charge le filament 1'
		);
	});
});
