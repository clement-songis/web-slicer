import { describe, expect, it } from 'vitest';
import {
	afterSlotAdded,
	afterSlotRemoved,
	compactProjectConfig,
	effectiveColours,
	maxFlush,
	NEW_FILAMENT_COLOURS,
	nextFilamentColour,
	parseProjectConfig,
	resizeFlushMatrix,
	sameProjectConfig,
	setSlotColour,
	setTowerPosition,
	towerPosition
} from './multi-filament';

describe('configuration multi-filament du projet', () => {
	it('lecture tolérante', () => {
		expect(
			parseProjectConfig({
				filament_colour: ['#FF0000', 'rouge', 3],
				flush_volumes_matrix: [0, 1, 2, 0],
				flush_multiplier: '1.5'
			})
		).toEqual({
			filament_colour: ['#ff0000', '', ''],
			flush_volumes_matrix: [0, 1, 2, 0],
			flush_multiplier: 1.5
		});
		expect(parseProjectConfig({ flush_volumes_matrix: [0, 1, 2] })).toEqual({});
		expect(parseProjectConfig(null)).toEqual({});
	});
	it('couleurs : surcharge par slot, retour au preset', () => {
		let cfg = setSlotColour({}, 1, '#00FF00');
		expect(effectiveColours(['#111111', '#222222'], cfg)).toEqual(['#111111', '#00ff00']);
		cfg = setSlotColour(cfg, 1, null);
		expect(effectiveColours(['#111111', '#222222'], cfg)).toEqual(['#111111', '#222222']);
		expect(compactProjectConfig(cfg)).toBeNull();
	});
	it('matrice : redimensionnée en conservant les valeurs', () => {
		expect(resizeFlushMatrix(undefined, 2)).toEqual([0, 280, 280, 0]);
		expect(resizeFlushMatrix([0, 100, 200, 0], 3)).toEqual([0, 100, 280, 200, 0, 280, 280, 280, 0]);
		expect(
			afterSlotAdded({ flush_volumes_matrix: [0, 5, 6, 0] }, 3).flush_volumes_matrix
		).toHaveLength(9);
	});
	it('retrait d un slot : couleur et ligne/colonne retirées', () => {
		const cfg = afterSlotRemoved(
			{
				filament_colour: ['#000000', '#ffffff', '#ff0000'],
				flush_volumes_matrix: [0, 1, 2, 3, 0, 5, 6, 7, 0]
			},
			1
		);
		expect(cfg.filament_colour).toEqual(['#000000', '#ff0000']);
		expect(cfg.flush_volumes_matrix).toEqual([0, 2, 6, 0]);
		expect(
			afterSlotRemoved({ flush_volumes_matrix: [0, 1, 2, 0] }, 1).flush_volumes_matrix
		).toBeUndefined();
	});
	it('purge maximale avec multiplicateur', () => {
		expect(maxFlush({ flush_volumes_matrix: [0, 100, 300, 0], flush_multiplier: 2 }, 2)).toBe(600);
	});
	it('position de la tour par plateau', () => {
		let cfg = setTowerPosition({}, 1, 100, 50, [15, 220]);
		expect(cfg.wipe_tower_x).toEqual([15, 100]);
		expect(towerPosition(cfg, 1, [0, 0])).toEqual([100, 50]);
		expect(towerPosition(cfg, 3, [0, 0])).toEqual([100, 50]);
		expect(towerPosition({}, 0, [15, 220])).toEqual([15, 220]);
		cfg = parseProjectConfig({ wipe_tower_x: [1, 'x'], wipe_tower_y: [2] });
		expect(cfg.wipe_tower_x).toBeUndefined();
		expect(compactProjectConfig(setTowerPosition({}, 0, 3, 4, [0, 0]))).toEqual({
			wipe_tower_x: [3],
			wipe_tower_y: [4]
		});
	});
	it('multiplicateur explicite conservé, même à 1 (défaut Orca 0,3)', () => {
		expect(compactProjectConfig({ flush_multiplier: 1 })).toEqual({ flush_multiplier: 1 });
		expect(compactProjectConfig({})).toBeNull();
	});
});

describe('couleur des filaments ajoutés (palette Orca)', () => {
	it('slot 2, 3… : couleurs successives de la palette, distinctes du premier filament', () => {
		expect(nextFilamentColour(1)).toBe('#00c1ae');
		expect(nextFilamentColour(2)).toBe('#f4e2c1');
		expect(nextFilamentColour(1 + NEW_FILAMENT_COLOURS.length)).toBe('#00c1ae');
	});
	it('posée sur le nouveau slot sans toucher aux autres', () => {
		const cfg = setSlotColour(afterSlotAdded({}, 2), 1, nextFilamentColour(1));
		expect(cfg.filament_colour).toEqual(['', '#00c1ae']);
		expect(compactProjectConfig(afterSlotRemoved(cfg, 1))).toBeNull();
	});
	it('comparaison avec la config enregistrée (sauvegarde avant tranchage)', () => {
		expect(sameProjectConfig(undefined, {})).toBe(true);
		expect(sameProjectConfig(null, { filament_colour: ['', ''] })).toBe(true);
		expect(
			sameProjectConfig(
				{ flush_multiplier: 1, filament_colour: ['#FFFFFF', '#000000'] },
				{ filament_colour: ['#ffffff', '#000000'], flush_multiplier: 1 }
			)
		).toBe(true);
		const saved = { filament_colour: ['#ffffff', '#000000'] };
		expect(sameProjectConfig(saved, { ...saved, flush_volumes_matrix: [0, 120, 300, 0] })).toBe(
			false
		);
		expect(sameProjectConfig(saved, { filament_colour: ['#ffffff', '#ff0000'] })).toBe(false);
	});
});
