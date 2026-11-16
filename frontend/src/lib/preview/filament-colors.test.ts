import { describe, expect, it, vi } from 'vitest';
import { DEFAULT_FILAMENT_COLORS } from './colorations';
import {
	colourFromResolvedValues,
	filamentColorsFromResolved,
	loadFilamentColors,
	parseFilamentColours,
	parseOrcaColor,
	rgbToHex
} from './filament-colors';

const RED: [number, number, number] = [1, 0, 0];
const GREEN: [number, number, number] = [0, 1, 0];

describe('parseOrcaColor', () => {
	it('accepte #RRGGBB, sans #, #RGB et ignore alpha', () => {
		expect(parseOrcaColor('#FF0000')).toEqual(RED);
		expect(parseOrcaColor('00ff00')).toEqual(GREEN);
		expect(parseOrcaColor('#0f0')).toEqual(GREEN);
		expect(parseOrcaColor('#FF0000CC')).toEqual(RED);
		expect(parseOrcaColor(' #ff0000 ')).toEqual(RED);
	});
	it('rejette les valeurs invalides', () => {
		expect(parseOrcaColor('red')).toBeNull();
		expect(parseOrcaColor('#12345')).toBeNull();
		expect(parseOrcaColor(42)).toBeNull();
		expect(parseOrcaColor(null)).toBeNull();
	});
});

describe('parseFilamentColours', () => {
	it('liste JSON (forme resolved)', () => {
		expect(parseFilamentColours(['#FF0000', '#00FF00'])).toEqual([RED, GREEN]);
	});
	it('chaîne simple, liste ; ou , et JSON encodé', () => {
		expect(parseFilamentColours('#FF0000')).toEqual([RED]);
		expect(parseFilamentColours('#FF0000;#00FF00')).toEqual([RED, GREEN]);
		expect(parseFilamentColours('#FF0000, #00FF00')).toEqual([RED, GREEN]);
		expect(parseFilamentColours('["#FF0000","#00FF00"]')).toEqual([RED, GREEN]);
		expect(parseFilamentColours(['["#FF0000"]'])).toEqual([RED]);
	});
	it('remplace une entrée invalide par la couleur par défaut de son index', () => {
		expect(parseFilamentColours(['#FF0000', 'nope'])).toEqual([RED, DEFAULT_FILAMENT_COLORS[1]]);
	});
	it('vide → []', () => {
		expect(parseFilamentColours(undefined)).toEqual([]);
		expect(parseFilamentColours('')).toEqual([]);
		expect(parseFilamentColours([])).toEqual([]);
	});
});

describe('colourFromResolvedValues / filamentColorsFromResolved', () => {
	it('priorise filament_colour puis default_filament_colour', () => {
		expect(colourFromResolvedValues({ filament_colour: ['#FF0000'] })).toEqual(RED);
		expect(
			colourFromResolvedValues({ filament_colour: [''], default_filament_colour: '#00FF00' })
		).toEqual(GREEN);
		expect(colourFromResolvedValues({})).toBeNull();
	});
	it('un preset par extrudeur, repli par index', () => {
		const colors = filamentColorsFromResolved([
			{ id: 'a', values: { filament_colour: ['#FF0000'] } },
			null,
			{ filament_colour: '#00FF00' }
		]);
		expect(colors).toEqual([RED, DEFAULT_FILAMENT_COLORS[1], GREEN]);
	});
});

describe('loadFilamentColors', () => {
	it('undefined sans filament', async () => {
		expect(await loadFilamentColors([], vi.fn())).toBeUndefined();
	});
	it('charge en parallèle et tolère un échec', async () => {
		const fetcher = vi.fn(async (id: string) => {
			if (id === 'bad') throw new Error('404');
			return { values: { filament_colour: [id === 'r' ? '#FF0000' : '#00FF00'] } };
		});
		expect(await loadFilamentColors(['r', 'bad', 'g'], fetcher)).toEqual([
			RED,
			DEFAULT_FILAMENT_COLORS[1],
			GREEN
		]);
		expect(fetcher).toHaveBeenCalledTimes(3);
	});
});

describe('rgbToHex', () => {
	it('aller-retour', () => {
		expect(rgbToHex(RED)).toBe('#ff0000');
		expect(rgbToHex(parseOrcaColor('#1A2b3C')!)).toBe('#1a2b3c');
	});
});
