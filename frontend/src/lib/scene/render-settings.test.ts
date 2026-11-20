import { describe, expect, test } from 'vitest';
import { FrameMeter, shouldRunFrame } from './render-settings';

describe('graphismes : plafond et compteur d’images/s', () => {
	test('sans plafond, toutes les images passent', () => {
		expect(shouldRunFrame(10, 9, 0)).toBe(true);
	});
	test('plafond 30 sur un écran 60 Hz : une image sur deux', () => {
		const frame = 1000 / 60;
		let last = -Infinity;
		let ran = 0;
		for (let i = 0; i < 60; i++) {
			const now = i * frame + (i % 2 ? 0.4 : -0.4); // gigue d'écran
			if (shouldRunFrame(now, last, 30)) {
				last = now;
				ran++;
			}
		}
		expect(ran).toBe(30);
	});
	test('plafond 60 sur un écran 60 Hz : aucune image sautée malgré la gigue', () => {
		const frame = 1000 / 60;
		let last = -Infinity;
		let ran = 0;
		for (let i = 0; i < 60; i++) {
			const now = i * frame + (i % 2 ? 0.8 : -0.8);
			if (shouldRunFrame(now, last, 60)) {
				last = now;
				ran++;
			}
		}
		expect(ran).toBe(60);
	});
	test('compteur : images comptées entre deux lectures', () => {
		const m = new FrameMeter();
		expect(m.sample(0)).toBeNull();
		m.frames = 12;
		expect(m.sample(500)).toBe(24);
		expect(m.sample(1500)).toBe(0);
	});
});
