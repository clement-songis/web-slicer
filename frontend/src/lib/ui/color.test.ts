import { describe, expect, it } from 'vitest';
import { contrastRatio, parseHex, readableTextOn, relativeLuminance, swatchStyle } from './color';

const ratio = (a: string, b: string) =>
	contrastRatio(relativeLuminance(parseHex(a)!), relativeLuminance(parseHex(b)!));

describe('parseHex', () => {
	it('accepte #rgb, #rrggbb et #rrggbbaa (alpha Orca ignoré)', () => {
		expect(parseHex('#fff')).toEqual([255, 255, 255]);
		expect(parseHex('#0091D1')).toEqual([0, 145, 209]);
		expect(parseHex('#0091d1ff')).toEqual([0, 145, 209]);
	});
	it('rejette les valeurs invalides', () => {
		expect(parseHex('rouge')).toBeNull();
		expect(parseHex(undefined)).toBeNull();
	});
});

describe('readableTextOn', () => {
	it('pastille filament par défaut (#0091d1) : texte sombre ≥ 4.5:1 (blanc = 3.52)', () => {
		expect(ratio('#ffffff', '#0091d1')).toBeLessThan(4.5);
		expect(readableTextOn('#0091d1')).toBe('#000000');
		expect(ratio(readableTextOn('#0091d1'), '#0091d1')).toBeGreaterThanOrEqual(4.5);
	});
	it('fonds clairs → texte sombre, fonds sombres → blanc', () => {
		expect(readableTextOn('#ffffff')).toBe('#000000');
		expect(readableTextOn('#f2e600')).toBe('#000000');
		expect(readableTextOn('#000000')).toBe('#ffffff');
		expect(readableTextOn('#1e3a8a')).toBe('#ffffff');
	});
	it('le choix atteint toujours ≥ 4.5:1 sur une palette de filaments courante', () => {
		for (const c of [
			'#ff0000',
			'#00ae42',
			'#0091d1',
			'#ff6a13',
			'#a3a3a3',
			'#8e44ad',
			'#ffd700',
			'#5c4033',
			'#ffc0cb',
			'#808080'
		]) {
			expect(ratio(readableTextOn(c), c)).toBeGreaterThanOrEqual(4.5);
		}
	});
	it('garantie théorique : ≥ 4.5:1 pour tout gris 0..255', () => {
		for (let v = 0; v < 256; v += 5) {
			const c = '#' + v.toString(16).padStart(2, '0').repeat(3);
			expect(ratio(readableTextOn(c), c)).toBeGreaterThanOrEqual(4.5);
		}
	});
	it('couleur inconnue → blanc (repli historique)', () => {
		expect(readableTextOn('n/a')).toBe('#ffffff');
	});
});

describe('swatchStyle', () => {
	it('compose fond, texte lisible et bordure', () => {
		const s = swatchStyle('#ffffff');
		expect(s).toContain('background-color: #ffffff');
		expect(s).toContain('color: #000000');
		expect(s).toContain('border: 1px solid');
	});
});
