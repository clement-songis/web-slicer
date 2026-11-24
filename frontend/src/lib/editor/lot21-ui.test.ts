import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '#lib/i18n';
import { buildSliceDraft } from './slice-draft';
import {
	extruderCount,
	nozzleLabel,
	readFilamentMap,
	readFilamentMapMode,
	writeFilamentMap
} from './filament-map';
import { paintedChannels } from './keep-paint';
import { compatibilityText } from './filament-compat';
import { DEFAULT_PREFERENCES, parsePreferences } from '#lib/settings/preferences';

// Messages attendus en français (langue des tests).
beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));

describe('brouillon au tranchage (SliceDraft)', () => {
	const same = { resolved: { layer_height: '0.2' }, current: { layer_height: '0.2' } };
	it('rien de modifié → pas de brouillon', () => {
		expect(
			buildSliceDraft({ process: same, printer: same, filament: { slot: 0, count: 2, ...same } })
		).toBeUndefined();
	});
	it('seules les valeurs modifiées, filament au bon index', () => {
		const d = buildSliceDraft({
			process: {
				resolved: { layer_height: '0.2', wall_loops: '2' },
				current: { layer_height: '0.12', wall_loops: '2' }
			},
			printer: same,
			filament: {
				slot: 1,
				count: 3,
				resolved: { nozzle_temperature: ['220'] },
				current: { nozzle_temperature: ['230'] }
			}
		})!;
		expect(d.process).toEqual({ layer_height: '0.12' });
		expect(d.printer).toBeUndefined();
		expect(d.filaments).toEqual([null, { nozzle_temperature: ['230'] }, null]);
	});
});

describe('filaments par buse (FilamentMapDialog)', () => {
	it('lecture tolérante, 1-based, bornée au nombre de buses', () => {
		expect(extruderCount(['0.4', '0.4'])).toBe(2);
		expect(extruderCount('0.4')).toBe(1);
		expect(readFilamentMap(['2', '1'], 3, 2)).toEqual([2, 1, 1]);
		expect(readFilamentMap('2 3', 2, 2)).toEqual([2, 1]);
		expect(writeFilamentMap([2, 1])).toEqual(['2', '1']);
		expect(readFilamentMapMode('Manual')).toBe('Manual');
		expect(readFilamentMapMode('nimp')).toBe('');
		expect(nozzleLabel(1, 2)).toBe('Gauche');
		expect(nozzleLabel(3, 4)).toBe('Buse 3');
	});
});

describe('peinture et températures', () => {
	it('canaux peints seulement', () => {
		expect(paintedChannels({ mmu: { '3': '8' }, seam: {} })).toEqual({ mmu: { '3': '8' } });
		expect(paintedChannels(undefined)).toEqual({});
	});
	it('verdict en clair', () => {
		expect(compatibilityText({ compatibility: 'compatible', blocking: false })).toBeNull();
		expect(compatibilityText({ compatibility: 'high_low_mixed', blocking: true })).toMatch(
			/refusé/
		);
		expect(compatibilityText({ compatibility: 'high_low_mixed', blocking: false })).toMatch(
			/autorisé/
		);
	});
	it('préférences Orca (défauts false)', () => {
		expect(DEFAULT_PREFERENCES.keepPainting).toBe(false);
		expect(DEFAULT_PREFERENCES.mixedTemperature).toBe(false);
		expect(parsePreferences('{"keepPainting":true}').keepPainting).toBe(true);
	});
});
