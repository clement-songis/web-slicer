// Chargement paresseux des couches de prévisualisation (T084, SC-007). Les gros
// G-codes comptent des millions de segments (40 o chacun) : tout garder en
// mémoire dépasse le budget GPU/JS. Ce loader découpe les couches en **tranches**
// (chunks), ne charge que la **fenêtre** autour du curseur, et **évince** les
// tranches lointaines (LRU) pour tenir un budget mémoire. Logique pure : la
// fonction de récupération est injectée (testable sans réseau).

import { RECORD_BYTES, type PreviewSegments } from './decode';

/** Récupère une plage de couches `[from, to]` incluses. */
export type FetchRange = (from: number, to: number) => Promise<PreviewSegments>;

export interface LoaderOptions {
	/** Nombre total de couches (borne le domaine). */
	layerCount: number;
	/** Récupération d'une plage (réseau réel ou stub de test). */
	fetchRange: FetchRange;
	/** Couches par tranche (défaut 20). */
	chunkSize?: number;
	/** Rayon de la fenêtre autour du curseur, en couches (défaut 40). */
	windowRadius?: number;
	/** Budget mémoire des segments résidents en octets (défaut 64 Mio). */
	maxBytes?: number;
}

const DEFAULT_CHUNK_SIZE = 20;
const DEFAULT_WINDOW_RADIUS = 40;
const DEFAULT_MAX_BYTES = 64 * 1024 * 1024;

/** Octets résidents d'un lot de segments (tableaux typés ≈ `count * 40`). */
export function segmentsBytes(segments: PreviewSegments): number {
	return segments.count * RECORD_BYTES;
}

interface CachedChunk {
	segments: PreviewSegments;
	bytes: number;
}

/**
 * Loader paresseux à fenêtre glissante et budget mémoire. Usage :
 * `await loader.ensure(cursorLayer)` puis `loader.window(cursorLayer)` pour les
 * segments à rendre.
 */
export class LazyPreviewLoader {
	private readonly layerCount: number;
	private readonly fetchRange: FetchRange;
	private readonly chunkSize: number;
	private readonly windowRadius: number;
	private readonly maxBytes: number;

	private readonly cache = new Map<number, CachedChunk>();
	private readonly lastUse = new Map<number, number>();
	private clock = 0;
	private resident = 0;

	constructor(opts: LoaderOptions) {
		this.layerCount = Math.max(0, opts.layerCount);
		this.fetchRange = opts.fetchRange;
		this.chunkSize = Math.max(1, opts.chunkSize ?? DEFAULT_CHUNK_SIZE);
		this.windowRadius = Math.max(0, opts.windowRadius ?? DEFAULT_WINDOW_RADIUS);
		this.maxBytes = Math.max(1, opts.maxBytes ?? DEFAULT_MAX_BYTES);
	}

	/** Nombre de tranches couvrant le domaine. */
	get chunkCount(): number {
		return Math.ceil(this.layerCount / this.chunkSize);
	}

	/** Indice de tranche d'une couche. */
	chunkOf(layer: number): number {
		return Math.floor(this.clampLayer(layer) / this.chunkSize);
	}

	/** Plage `[from, to]` (incluse) d'une tranche. */
	chunkRange(index: number): { from: number; to: number } {
		const from = index * this.chunkSize;
		const to = Math.min(this.layerCount - 1, from + this.chunkSize - 1);
		return { from, to };
	}

	/** Tranches nécessaires pour la fenêtre autour de `cursor`. */
	chunksForWindow(cursor: number): number[] {
		if (this.layerCount === 0) return [];
		const lo = this.clampLayer(cursor - this.windowRadius);
		const hi = this.clampLayer(cursor + this.windowRadius);
		const first = this.chunkOf(lo);
		const last = this.chunkOf(hi);
		const out: number[] = [];
		for (let i = first; i <= last; i++) out.push(i);
		return out;
	}

	/** Octets de segments actuellement résidents. */
	residentBytes(): number {
		return this.resident;
	}

	/** Indices de tranches en cache (triés) — diagnostic/tests. */
	loadedChunks(): number[] {
		return [...this.cache.keys()].sort((a, b) => a - b);
	}

	/**
	 * Garantit que la fenêtre autour de `cursor` est chargée, puis évince les
	 * tranches hors fenêtre jusqu'à retomber sous le budget mémoire.
	 */
	async ensure(cursor: number): Promise<void> {
		const needed = this.chunksForWindow(cursor);
		for (const index of needed) {
			if (!this.cache.has(index)) {
				const { from, to } = this.chunkRange(index);
				const segments = await this.fetchRange(from, to);
				const bytes = segmentsBytes(segments);
				this.cache.set(index, { segments, bytes });
				this.resident += bytes;
			}
			this.lastUse.set(index, this.clock++);
		}
		this.evict(new Set(needed));
	}

	/** Segments des tranches de la fenêtre autour de `cursor` (déjà chargées). */
	window(cursor: number): PreviewSegments[] {
		return this.chunksForWindow(cursor)
			.map((i) => this.cache.get(i)?.segments)
			.filter((s): s is PreviewSegments => s !== undefined);
	}

	/** Évince les tranches les moins récemment utilisées hors de `keep`. */
	private evict(keep: Set<number>): void {
		while (this.resident > this.maxBytes) {
			let victim = -1;
			let oldest = Infinity;
			for (const [index] of this.cache) {
				if (keep.has(index)) continue;
				const used = this.lastUse.get(index) ?? 0;
				if (used < oldest) {
					oldest = used;
					victim = index;
				}
			}
			if (victim < 0) break; // tout le reste est protégé (fenêtre)
			const chunk = this.cache.get(victim);
			if (chunk) this.resident -= chunk.bytes;
			this.cache.delete(victim);
			this.lastUse.delete(victim);
		}
	}

	private clampLayer(layer: number): number {
		if (this.layerCount === 0) return 0;
		return Math.max(0, Math.min(this.layerCount - 1, Math.floor(layer)));
	}
}
