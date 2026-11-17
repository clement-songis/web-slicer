import { describe, expect, it } from 'vitest';
import {
	autoBrimByHeight,
	clampTowerPosition,
	estimateWipeTowerSize,
	limitDepthByHeight,
	showWipeTower
} from './wipe-tower';

const base = {
	width: 60,
	primeVolume: 45,
	layerHeight: 0.2,
	infillGap: 150,
	wallType: 'rectangle',
	ribWidth: 8,
	extraRibLength: 0,
	maxHeight: 20,
	plateExtruders: 2,
	nozzles: 1
};

describe('tour d essuyage (estimation Orca)', () => {
	it('profondeur minimale interpolée selon la hauteur', () => {
		expect(limitDepthByHeight(2)).toBe(5);
		expect(limitDepthByHeight(52.5)).toBeCloseTo(12.5);
		expect(limitDepthByHeight(400)).toBe(60);
		expect(autoBrimByHeight(50)).toBe(4);
		expect(autoBrimByHeight(150)).toBe(8);
	});
	it('mur rectangulaire : largeur fixe, profondeur = volume / (couche × largeur) × écart', () => {
		const [w, d, h] = estimateWipeTowerSize(base);
		expect(w).toBe(60);
		// 45 × 1 / (0,2 × 60) × 1,5 = 5,625 ; minimum à 20 mm de haut = 5 + 15/95 × 15
		expect(d).toBeCloseTo(Math.max(5.625, 5 + (15 / 95) * 15));
		expect(h).toBe(20);
	});
	it('mur à nervures : carré', () => {
		const [w, d] = estimateWipeTowerSize({ ...base, wallType: 'rib' });
		expect(w).toBe(d);
		expect(w).toBeGreaterThan(0);
	});
	it('deux buses : chaque filament purge, plus le volume de changement', () => {
		const one = estimateWipeTowerSize({ ...base, maxHeight: 1 })[1];
		const two = estimateWipeTowerSize({ ...base, maxHeight: 1, nozzles: 2, changeLength: 10 })[1];
		expect(two).toBeGreaterThan(one);
	});
	it('rien sur le plateau : taille nulle', () => {
		expect(estimateWipeTowerSize({ ...base, plateExtruders: 0 })).toEqual([0, 0, 0]);
	});
	it('position bornée au plateau (marge 1 mm + bordure)', () => {
		expect(clampTowerPosition(-10, 300, [60, 20, 10], { width: 250, depth: 250 }, 3)).toEqual([
			4, 226
		]);
		expect(clampTowerPosition(15, 220, [60, 20, 10], { width: 250, depth: 250 }, 3)).toEqual([
			15, 220
		]);
	});
	it('affichage : 2 filaments, tour activée, objet par objet seulement avec un objet', () => {
		expect(showWipeTower({ enabled: true, plateExtruders: 2, objects: 2, byObject: false })).toBe(
			true
		);
		expect(showWipeTower({ enabled: true, plateExtruders: 1, objects: 2, byObject: false })).toBe(
			false
		);
		expect(showWipeTower({ enabled: false, plateExtruders: 2, objects: 2, byObject: false })).toBe(
			false
		);
		expect(showWipeTower({ enabled: true, plateExtruders: 2, objects: 2, byObject: true })).toBe(
			false
		);
		expect(showWipeTower({ enabled: true, plateExtruders: 2, objects: 1, byObject: true })).toBe(
			true
		);
	});
});
