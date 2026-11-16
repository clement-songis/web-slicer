import { describe, expect, it } from 'vitest';
import { createLatestLoader } from './latest-load';

const tick = (ms: number) => new Promise((r) => setTimeout(r, ms));

describe('createLatestLoader', () => {
	it("un appel remplacé n'applique rien et attend le plus récent", async () => {
		const loader = createLatestLoader<'machine'>();
		const applied: string[] = [];
		const first = loader.run('machine', async (isCurrent) => {
			await tick(5);
			if (isCurrent()) applied.push('first');
		});
		let secondDone = false;
		const second = loader.run('machine', async (isCurrent) => {
			await tick(30);
			if (isCurrent()) applied.push('second');
			secondDone = true;
		});
		await first;
		// `editorReady` attendant `first` voit l'état final (plateau résolu).
		expect(secondDone).toBe(true);
		expect(applied).toEqual(['second']);
		await second;
	});

	it('chaîne plusieurs remplacements et isole les portées', async () => {
		const loader = createLatestLoader<'machine' | 'process'>();
		const order: string[] = [];
		const a = loader.run('machine', async () => {
			await tick(1);
		});
		loader.run('machine', async () => {
			await tick(10);
		});
		loader.run('machine', async (isCurrent) => {
			await tick(20);
			if (isCurrent()) order.push('machine-3');
		});
		const p = loader.run('process', async (isCurrent) => {
			if (isCurrent()) order.push('process');
		});
		await p;
		expect(order).toEqual(['process']);
		await a;
		expect(order).toEqual(['process', 'machine-3']);
	});

	it('pending() résout immédiatement sans chargement', async () => {
		await expect(createLatestLoader<'x'>().pending('x')).resolves.toBeUndefined();
	});
});
