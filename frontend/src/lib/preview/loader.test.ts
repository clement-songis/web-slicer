// Tests du chargement paresseux par fenêtre + budget mémoire (T084, SC-007).
import { describe, expect, test } from 'vitest';
import type { PreviewSegments } from './decode';
import { LazyPreviewLoader, segmentsBytes } from './loader';

/** Fabrique un lot de `count` segments synthétiques (contenu indifférent). */
function fakeSegments(from: number, to: number, count: number): PreviewSegments {
	return {
		from,
		to,
		count,
		start: new Float32Array(count * 3),
		end: new Float32Array(count * 3),
		feedrate: new Float32Array(count),
		width: new Float32Array(count),
		height: new Float32Array(count),
		kind: new Uint8Array(count),
		extruder: new Uint8Array(count),
		layer: new Uint16Array(count)
	};
}

/** Loader instrumenté : compte les récupérations et les plages demandées. */
function instrumented(opts: {
	layerCount: number;
	perLayer?: number;
	chunkSize?: number;
	windowRadius?: number;
	maxBytes?: number;
}) {
	const ranges: Array<[number, number]> = [];
	const perLayer = opts.perLayer ?? 100;
	const loader = new LazyPreviewLoader({
		layerCount: opts.layerCount,
		chunkSize: opts.chunkSize,
		windowRadius: opts.windowRadius,
		maxBytes: opts.maxBytes,
		fetchRange: async (from, to) => {
			ranges.push([from, to]);
			const layers = to - from + 1;
			return fakeSegments(from, to, layers * perLayer);
		}
	});
	return { loader, ranges };
}

describe('LazyPreviewLoader windowing', () => {
	test('loads only the chunks covering the window around the cursor', async () => {
		const { loader } = instrumented({ layerCount: 500, chunkSize: 20, windowRadius: 40 });
		await loader.ensure(250);
		// Fenêtre [210, 290] → tranches 10..14.
		expect(loader.loadedChunks()).toEqual([10, 11, 12, 13, 14]);
	});

	test('does not refetch chunks already resident', async () => {
		const { loader, ranges } = instrumented({ layerCount: 500, chunkSize: 20, windowRadius: 20 });
		await loader.ensure(100);
		const firstCount = ranges.length;
		await loader.ensure(100); // même curseur → aucun nouvel appel
		expect(ranges.length).toBe(firstCount);
	});

	test('clamps the window at the domain edges', async () => {
		const { loader } = instrumented({ layerCount: 50, chunkSize: 20, windowRadius: 40 });
		await loader.ensure(0);
		// [0, 40] → tranches 0..2, jamais d'indice négatif.
		expect(loader.loadedChunks()).toEqual([0, 1, 2]);
	});
});

describe('LazyPreviewLoader memory budget', () => {
	test('evicts far chunks to stay under the byte budget', async () => {
		// 100 segments/couche × 40 o = 4000 o/couche ; 20 couches/tranche = 80 000 o.
		// Budget 200 000 o → au plus 2 tranches résidentes.
		const { loader } = instrumented({
			layerCount: 1000,
			chunkSize: 20,
			windowRadius: 0,
			perLayer: 100,
			maxBytes: 200_000
		});
		await loader.ensure(10); // tranche 0
		await loader.ensure(210); // tranche 10
		await loader.ensure(410); // tranche 20
		await loader.ensure(610); // tranche 30
		expect(loader.residentBytes()).toBeLessThanOrEqual(200_000);
		// La tranche courante reste ; les anciennes sont évincées.
		expect(loader.loadedChunks()).toContain(30);
		expect(loader.loadedChunks().length).toBeLessThanOrEqual(2);
	});

	test('never evicts a chunk inside the current window', async () => {
		// Fenêtre large (5 tranches) mais budget trop petit : la fenêtre est
		// protégée, le résident peut dépasser le budget sans planter.
		const { loader } = instrumented({
			layerCount: 200,
			chunkSize: 20,
			windowRadius: 40,
			perLayer: 100,
			maxBytes: 1000
		});
		await loader.ensure(100); // tranches 3..7 (fenêtre) toutes protégées
		expect(loader.loadedChunks()).toEqual([3, 4, 5, 6, 7]);
	});

	test('revisiting an evicted chunk refetches it', async () => {
		const { loader, ranges } = instrumented({
			layerCount: 1000,
			chunkSize: 20,
			windowRadius: 0,
			perLayer: 100,
			maxBytes: 80_000 // une seule tranche à la fois
		});
		await loader.ensure(10); // tranche 0
		await loader.ensure(210); // tranche 10 → évince 0
		expect(loader.loadedChunks()).toEqual([10]);
		await loader.ensure(10); // tranche 0 de nouveau → refetch
		const fetchesForChunk0 = ranges.filter(([from]) => from === 0).length;
		expect(fetchesForChunk0).toBe(2);
	});
});

describe('segmentsBytes', () => {
	test('accounts 40 bytes per segment', () => {
		expect(segmentsBytes(fakeSegments(0, 0, 1000))).toBe(40_000);
	});
});
