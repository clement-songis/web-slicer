import { describe, expect, it } from 'vitest';
import {
	effectiveMapMode,
	filamentMapChoice,
	isBambuMachine,
	listedFilaments,
	nextNozzle,
	shouldPromptFilamentMap
} from './filament-map';

// Orca FilamentMapDialog.cpp try_pop_up_before_slice.
describe('répartition des filaments avant tranchage', () => {
	it('deux buses Bambu Lab et préférence active seulement', () => {
		expect(shouldPromptFilamentMap({ enabled: true, extruders: 2, bambu: true })).toBe(true);
		expect(shouldPromptFilamentMap({ enabled: false, extruders: 2, bambu: true })).toBe(false);
		expect(shouldPromptFilamentMap({ enabled: true, extruders: 1, bambu: true })).toBe(false);
		// Changeurs d'outils (≥ 3) et autres marques exclus (Orca #12390).
		expect(shouldPromptFilamentMap({ enabled: true, extruders: 4, bambu: true })).toBe(false);
		expect(shouldPromptFilamentMap({ enabled: true, extruders: 2, bambu: false })).toBe(false);
	});

	it('détection Bambu Lab (printer_model ou nom du preset)', () => {
		expect(isBambuMachine({ printer_model: 'Bambu Lab H2D' })).toBe(true);
		expect(isBambuMachine({}, 'Bambu Lab H2D 0.4 nozzle')).toBe(true);
		expect(isBambuMachine({}, 'Mon H2D @BBL H2D')).toBe(true);
		expect(isBambuMachine({ printer_model: 'Prusa XL' }, 'Prusa XL 5T 0.4 nozzle')).toBe(false);
	});

	it('mode appliqué : plateau, sinon global, sinon « Auto For Flush »', () => {
		expect(effectiveMapMode('Manual', 'Auto For Match')).toBe('Manual');
		expect(effectiveMapMode('Default', ['Auto For Match'])).toBe('Auto For Match');
		expect(effectiveMapMode(undefined, undefined)).toBe('Auto For Flush');
		expect(effectiveMapMode('n’importe', 'Default')).toBe('Auto For Flush');
	});

	it('filaments listés : ceux du plateau, sinon tous', () => {
		expect(listedFilaments([3, 1, 3, 9], 4)).toEqual([1, 3]);
		expect(listedFilaments([], 3)).toEqual([1, 2, 3]);
	});

	it('validation : la carte n’est écrite qu’en manuel', () => {
		expect(filamentMapChoice('Auto For Flush', [1, 2])).toEqual([
			['filament_map_mode', 'Auto For Flush']
		]);
		expect(filamentMapChoice('Manual', [1, 2, 2])).toEqual([
			['filament_map_mode', 'Manual'],
			['filament_map', ['1', '2', '2']]
		]);
	});

	it('clic sur une pastille : gauche ⇄ droite', () => {
		expect(nextNozzle(1, 2)).toBe(2);
		expect(nextNozzle(2, 2)).toBe(1);
	});
});
