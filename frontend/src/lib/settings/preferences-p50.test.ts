import { describe, expect, it } from 'vitest';
import {
	DEFAULT_PREFERENCES,
	GCODE_THUMBNAILS_DEFAULT,
	normalizeGcodeThumbnails,
	parsePreferences
} from './preferences';

// Audit 48 DLG48-b — vignettes G-code personnalisables (liste de formats,
// vide = défauts, jamais écrits dans la préférence).
describe('préférences phase 50 (vignettes G-code, DLG48-b)', () => {
	it('défaut : préférence vide, jamais remplie par les défauts', () => {
		expect(DEFAULT_PREFERENCES.gcodeThumbnails).toBe('');
		// Le défaut moteur reste exposé pour l'affichage (placeholder)…
		expect(GCODE_THUMBNAILS_DEFAULT).toBe('48x48/PNG,300x300/PNG');
		// …mais parsePreferences ne l'écrit jamais dans la préférence.
		expect(parsePreferences(null).gcodeThumbnails).toBe('');
		expect(parsePreferences(JSON.stringify({ gcodeThumbnails: '   ' })).gcodeThumbnails).toBe('');
		expect(
			parsePreferences(JSON.stringify({ gcodeThumbnails: ' , 96x96/PNG , ,' })).gcodeThumbnails
		).toBe('96x96/PNG');
	});

	it('normalisation : espaces et entrées vides autour des virgules', () => {
		expect(normalizeGcodeThumbnails('')).toBe('');
		expect(normalizeGcodeThumbnails('  ')).toBe('');
		expect(normalizeGcodeThumbnails('48x48/PNG, 300x300/PNG')).toBe('48x48/PNG,300x300/PNG');
		expect(normalizeGcodeThumbnails(' 96x96/QOI ,,96x96 ')).toBe('96x96/QOI,96x96');
		// Les défauts ne sont jamais substitués à une valeur vide.
		expect(normalizeGcodeThumbnails(',,,')).toBe('');
	});

	it('lecture : valeur absente → défaut (vide), valeur invalide → défaut', () => {
		expect(parsePreferences(JSON.stringify({ maxRecent: 5 })).gcodeThumbnails).toBe('');
		expect(parsePreferences('{invalide').gcodeThumbnails).toBe('');
		expect(parsePreferences(JSON.stringify({ gcodeThumbnails: 42 })).gcodeThumbnails).toBe('');
	});
});
