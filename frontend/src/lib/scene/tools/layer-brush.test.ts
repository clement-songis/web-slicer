import { describe, expect, test } from 'vitest';
import { LayerBrushDriver, actionFor, ORCA_LAYER_STRENGTH, type Timer } from './layer-brush';

class FakeTimer implements Timer {
	fn: (() => void) | null = null;
	set(fn: () => void) {
		this.fn = fn;
		return 1;
	}
	clear() {
		this.fn = null;
	}
	tick() {
		this.fn?.();
	}
}

describe('LayerBrushDriver', () => {
	test('mapping des boutons comme GLCanvas3D', () => {
		expect(actionFor(false, false)).toBe('decrease');
		expect(actionFor(true, false)).toBe('increase');
		expect(actionFor(false, true)).toBe('reduce');
		expect(actionFor(true, true)).toBe('smooth');
	});

	test('0,005 par évènement : appui, glisser, minuteur ; arrêt au relâchement', async () => {
		const calls: [number, string, number][] = [];
		const timer = new FakeTimer();
		const d = new LayerBrushDriver(async (z, a, delta) => {
			calls.push([z, a, delta]);
		}, timer);
		d.start(5, 'increase');
		d.move(6);
		timer.tick();
		d.stop();
		timer.tick();
		await d.idle();
		expect(calls).toEqual([
			[5, 'increase', ORCA_LAYER_STRENGTH],
			[6, 'increase', ORCA_LAYER_STRENGTH],
			[6, 'increase', ORCA_LAYER_STRENGTH]
		]);
		expect(d.active).toBe(false);
	});

	test('requêtes séquentielles et file bornée', async () => {
		let inflight = 0;
		let maxInflight = 0;
		let resolveAll: () => void = () => {};
		const gate = new Promise<void>((r) => (resolveAll = r));
		const d = new LayerBrushDriver(async () => {
			inflight++;
			maxInflight = Math.max(maxInflight, inflight);
			await gate;
			inflight--;
		}, new FakeTimer());
		d.start(1, 'decrease');
		for (let i = 0; i < 20; i++) d.move(i);
		resolveAll();
		await d.idle();
		expect(maxInflight).toBe(1);
	});
});
