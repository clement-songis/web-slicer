import { beforeEach, describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import {
	CAMERA_DEFAULTS,
	DEFAULT_PREFERENCES,
	PREFERENCES_KEY,
	appPreferences,
	clampOrbitSpeed,
	clearRememberedChoices,
	loadPreferences,
	parsePreferences,
	preferencesKey,
	savePreferences,
	setPreferencesUser
} from './preferences';

/** Stockage en mémoire dont les clés sont énumérables (comme localStorage). */
function memoryStorage(): Storage {
	const s: Record<string, string> = {};
	Object.defineProperties(s, {
		getItem: { value: (k: string) => (k in s ? s[k] : null) },
		setItem: { value: (k: string, v: string) => (s[k] = String(v)) },
		removeItem: { value: (k: string) => delete s[k] },
		clear: { value: () => Object.keys(s).forEach((k) => delete s[k]) }
	});
	return s as unknown as Storage;
}

describe('préférences phase 22 (Orca PreferencesDialog)', () => {
	beforeEach(() => {
		(globalThis as { localStorage?: unknown }).localStorage = memoryStorage();
		setPreferencesUser(null);
	});

	it('défauts d’Orca (AppConfig.cpp)', () => {
		expect(DEFAULT_PREFERENCES.camera).toEqual({
			style: 'default',
			orbitSpeed: 1,
			zoomToMouse: false,
			freeCamera: false,
			reverseWheel: false,
			leftDrag: 'rotate',
			middleDrag: 'pan',
			rightDrag: 'pan'
		});
		expect(DEFAULT_PREFERENCES.autoArrange).toBe(true);
		expect(DEFAULT_PREFERENCES.autoFlush).toBe('all');
		expect(DEFAULT_PREFERENCES.showUnsupportedPresets).toBe(false);
	});

	it('lecture tolérante de la caméra', () => {
		const p = parsePreferences(
			JSON.stringify({
				camera: { style: 'touchpad', orbitSpeed: 9, leftDrag: 'zoom', reverseWheel: true }
			})
		);
		expect(p.camera.style).toBe('touchpad');
		expect(p.camera.orbitSpeed).toBe(2);
		expect(p.camera.leftDrag).toBe(CAMERA_DEFAULTS.leftDrag);
		expect(p.camera.reverseWheel).toBe(true);
		expect(parsePreferences('{"autoFlush":"color"}').autoFlush).toBe('color');
		expect(parsePreferences('{"autoFlush":"sometimes"}').autoFlush).toBe('all');
	});

	it('vitesse d’orbite bornée comme Orca (0,05–2, 2 décimales)', () => {
		expect(clampOrbitSpeed(0)).toBe(0.05);
		expect(clampOrbitSpeed(3)).toBe(2);
		expect(clampOrbitSpeed(0.333)).toBe(0.33);
		expect(clampOrbitSpeed(Number.NaN)).toBe(1);
	});

	it('persistance par utilisateur, reprise de la clé globale', () => {
		localStorage.setItem(PREFERENCES_KEY, JSON.stringify({ units: 'in' }));
		expect(setPreferencesUser('alice').units).toBe('in');
		savePreferences({ ...get(appPreferences), currency: 'CHF' });
		expect(localStorage.getItem(preferencesKey('alice'))).toContain('CHF');
		// Clé historique recopiée (lecteurs existants), mais bob a ses propres défauts.
		expect(localStorage.getItem(PREFERENCES_KEY)).toContain('CHF');
		expect(setPreferencesUser('bob').currency).toBe('€');
		savePreferences({ ...get(appPreferences), autoArrange: false });
		expect(setPreferencesUser('alice').currency).toBe('CHF');
		expect(get(appPreferences).autoArrange).toBe(true);
		expect(setPreferencesUser('bob').autoArrange).toBe(false);
		expect(loadPreferences().autoArrange).toBe(false);
	});

	it('« Effacer mes choix » efface les choix mémorisés', () => {
		localStorage.setItem('web-slicer.recenter', 'muted');
		expect(clearRememberedChoices()).toBe(1);
		expect(localStorage.getItem('web-slicer.recenter')).toBeNull();
		expect(clearRememberedChoices()).toBe(0);
	});
});
