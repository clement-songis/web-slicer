import { describe, expect, it } from 'vitest';
import { copyName, uniqueCopyName } from './presets';

describe('noms de copie de preset (parité Orca)', () => {
	it('numérote à partir de la 2e copie', () => {
		expect(copyName('Voron')).toBe('Voron (copie)');
		expect(copyName('Voron', 2)).toBe('Voron (copie 2)');
	});
	it('choisit le premier nom libre', () => {
		expect(uniqueCopyName('P', [])).toBe('P (copie)');
		expect(uniqueCopyName('P', ['P (copie)'])).toBe('P (copie 2)');
		expect(uniqueCopyName('P', ['P (copie)', 'P (copie 2)', 'P (copie 4)'])).toBe('P (copie 3)');
	});
});
