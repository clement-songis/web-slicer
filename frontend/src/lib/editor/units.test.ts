import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '$lib/i18n';
import {
	convertScale,
	convertedFrom,
	detectUnits,
	formatSize,
	meshVolume,
	scaledVolume,
	unitActionOf,
	unitMenuItems,
	withUnitItems
} from './units';

function cube(side: number) {
	const v = [0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 0, 0, 1, 1, 0, 1, 1, 1, 1, 0, 1, 1].map(
		(c) => c * side
	);
	const f = [
		0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0,
		4, 3, 4, 7
	];
	return { positions: new Float32Array(v), indices: new Uint32Array(f) };
}

// Messages attendus en français (langue des tests).
beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));

describe('conversion d’unités (Orca Model.cpp:830-872)', () => {
	it('volume d’un maillage fermé', () => {
		expect(meshVolume(cube(2))).toBeCloseTo(8, 5);
		expect(scaledVolume(1, [2, 2, 2])).toBe(8);
	});
	it('seuils d’Orca : mètres < 0,008 mm³ puis pouces < 8 mm³', () => {
		expect(detectUnits(0.001)).toBe('meter');
		expect(detectUnits(1)).toBe('inch');
		expect(detectUnits(7.99)).toBe('inch');
		expect(detectUnits(8)).toBeNull();
		expect(detectUnits(0)).toBeNull();
	});
	it('conversion et retour par l’échelle (miroir conservé)', () => {
		expect(convertScale([1, 1, 1], 'inch')).toEqual([25.4, 25.4, 25.4]);
		expect(convertScale([-1, 1, 1], 'meter')).toEqual([-1000, 1000, 1000]);
		const back = convertScale([25.4, 25.4, 25.4], 'inch', true);
		back.forEach((v) => expect(v).toBeCloseTo(1, 9));
		expect(convertedFrom([25.4, 25.4, 25.4])).toBe('inch');
		expect(convertedFrom([1000, -1000, 1000])).toBe('meter');
		expect(convertedFrom([25.4, 1, 1])).toBeNull();
	});
	it('entrées du menu objet selon l’état (Convert / Restore)', () => {
		expect(unitMenuItems([1, 1, 1]).map((i) => i !== 'separator' && i.label)).toEqual([
			'Convert from inches',
			'Convert from meters'
		]);
		expect(unitMenuItems([25.4, 25.4, 25.4]).map((i) => i !== 'separator' && i.label)).toEqual([
			'Restore to inches'
		]);
		const menu = withUnitItems([
			{ label: 'Scale to build volume', action: 'object.scaleToBed' },
			{ label: 'X', action: 'x' }
		]);
		expect(menu.map((i) => i !== 'separator' && i.action)).toEqual([
			'object.scaleToBed',
			'object.convertFromInches',
			'object.convertFromMeters',
			'x'
		]);
		// Menu en sections (phase 25) : insertion dans la section de « Scale to build volume ».
		const sectioned = withUnitItems([
			{ label: 'A', action: 'section.a', section: true, items: [{ label: 'Y', action: 'y' }] },
			{
				label: 'P',
				action: 'section.p',
				section: true,
				items: [
					{ label: 'Scale to build volume', action: 'object.scaleToBed' },
					{ label: 'Z', action: 'z' }
				]
			}
		]);
		expect(sectioned).toHaveLength(2);
		const p = sectioned[1];
		expect(p !== 'separator' && p.items?.map((i) => i.action)).toEqual([
			'object.scaleToBed',
			'object.convertFromInches',
			'object.convertFromMeters',
			'z'
		]);
		expect(unitActionOf('object.restoreToMeters')).toEqual({ unit: 'meter', restore: true });
		expect(unitActionOf('object.center')).toBeNull();
	});
	it('taille lisible', () => {
		expect(formatSize([1, 1, 1])).toBe('1 × 1 × 1 mm');
		expect(formatSize([25.4, 254, 0.05])).toBe('25,4 × 254 × 0,050 mm');
	});
});
