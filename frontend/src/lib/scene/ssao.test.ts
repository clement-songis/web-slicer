import { describe, expect, test } from 'vitest';
import { PLATE_TOLERANCE_MM, SSAO_FRAGMENT, ssaoPlate } from './ssao';

describe('SSAO de la vue réaliste (ssao.fs d’Orca)', () => {
	test('plateau exclu : rectangle, ou disque pour un plateau non rectangulaire', () => {
		expect(
			ssaoPlate({ center: { x: 128, y: 128 }, width: 256, depth: 256, rectangular: true })
		).toEqual({ rect: [0, 0, 256, 256], circle: [128, 128, 0] });
		expect(
			ssaoPlate({ center: { x: 0, y: 0 }, width: 200, depth: 200, rectangular: false }).circle
		).toEqual([0, 0, 100]);
		expect(PLATE_TOLERANCE_MM).toBeGreaterThan(0.05);
	});
	test('constantes du shader d’Orca reprises', () => {
		for (const k of [
			'0.55',
			'mix(0.45, 0.70, upFactor)',
			'pow(ao, 2.2)',
			'0.015 * (0.5',
			'mix(2.0, 4.0'
		])
			expect(SSAO_FRAGMENT).toContain(k);
	});
});
