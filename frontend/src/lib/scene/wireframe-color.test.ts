import { describe, expect, test } from 'vitest';
import { wireframeColor } from './scene-colors';

describe('fil de fer : getWireframeColor (mm_gouraud.fs d’Orca)', () => {
	test('surface claire → arêtes sombres ; sinon presque blanches', () => {
		expect(wireframeColor('#ffffff')).toBe('#1c2a35');
		expect(wireframeColor('#f5d742')).toBe('#1c2a35');
		expect(wireframeColor('#9aa7b4')).toBe('#fcfcfc');
		expect(wireframeColor('#e07b39')).toBe('#fcfcfc');
		expect(wireframeColor('rouge')).toBe('#fcfcfc');
	});
});
