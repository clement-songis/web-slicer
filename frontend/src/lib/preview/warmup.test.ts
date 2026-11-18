import { describe, expect, it, vi } from 'vitest';
import { warmupScene } from './warmup';

describe('warmupScene (précompilation des shaders de l aperçu)', () => {
	it('attend la compilation et mesure sa durée', async () => {
		let t = 100;
		const compile = vi.fn(async () => {
			t += 42;
		});
		const r = await warmupScene(compile, 1000, () => t);
		expect(compile).toHaveBeenCalledOnce();
		expect(r).toEqual({ ms: 42, timedOut: false });
	});

	it('reprend le rendu après le délai de garde si la compilation traîne', async () => {
		vi.useFakeTimers();
		const p = warmupScene(
			() => new Promise(() => {}),
			500,
			() => 0
		);
		await vi.advanceTimersByTimeAsync(600);
		await expect(p).resolves.toEqual({ ms: 0, timedOut: true });
		vi.useRealTimers();
	});

	it('une compilation en échec ne bloque pas l aperçu', async () => {
		await expect(warmupScene(() => Promise.reject(new Error('gl')), 500)).resolves.toMatchObject({
			timedOut: false
		});
	});
});
