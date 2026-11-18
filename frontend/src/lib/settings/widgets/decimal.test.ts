import { describe, expect, it } from 'vitest';
import { decimalSeparator, fromDisplayDecimal, toDisplayDecimal } from './types';

describe('séparateur décimal homogène (phase 28)', () => {
	it('séparateur de la langue', () => {
		expect(decimalSeparator('fr-FR')).toBe(',');
		expect(decimalSeparator('en-US')).toBe('.');
	});
	it('affichage localisé, stockage au point', () => {
		expect(toDisplayDecimal('0.42', ',')).toBe('0,42');
		expect(toDisplayDecimal('12.5%', ',')).toBe('12,5%');
		expect(toDisplayDecimal('0.42', '.')).toBe('0.42');
		expect(toDisplayDecimal(undefined, ',')).toBe('');
		expect(fromDisplayDecimal('0,45')).toBe('0.45');
		expect(fromDisplayDecimal('12,5%')).toBe('12.5%');
		expect(fromDisplayDecimal(',5')).toBe('0.5');
		expect(fromDisplayDecimal('0.4')).toBe('0.4');
	});
});
