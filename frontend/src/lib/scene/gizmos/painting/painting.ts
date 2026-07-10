// Modèle de peinture d'un objet (T056) : quatre canaux de peinture au triangle
// (supports, couture, fuzzy skin, segmentation MMU), application par pinceau
// (rayon) et sérialisation au format 3MF Orca (chaînes hexadécimales par
// facette). Pur → testable ; la couche gizmo raycaste et appelle `paint`.
import type { SceneMesh } from '../../mesh';
import { decodeFacet, encodeFacet } from './facets';

/** Canaux de peinture (attributs 3MF Orca correspondants). */
export type PaintChannel = 'supports' | 'seam' | 'fuzzy' | 'mmu';

export const CHANNELS: PaintChannel[] = ['supports', 'seam', 'fuzzy', 'mmu'];

/** Attribut 3MF Orca de chaque canal. */
export const CHANNEL_ATTR: Record<PaintChannel, string> = {
	supports: 'slic3rpe:custom_supports',
	seam: 'slic3rpe:custom_seam',
	fuzzy: 'slic3rpe:fuzzy_skin',
	mmu: 'slic3rpe:mmu_segmentation'
};

/** Document de peinture sérialisé : par canal, index de triangle → hex Orca. */
export type PaintDocument = Partial<Record<PaintChannel, Record<string, string>>>;

/** Peinture au triangle d'un objet, tous canaux confondus. */
export class TrianglePainting {
	private readonly channels: Record<PaintChannel, Map<number, number>> = {
		supports: new Map(),
		seam: new Map(),
		fuzzy: new Map(),
		mmu: new Map()
	};

	/** État peint d'un triangle dans un canal (0 = non peint). */
	get(channel: PaintChannel, triangle: number): number {
		return this.channels[channel].get(triangle) ?? 0;
	}

	/** Peint des triangles dans un canal (état ≤ 0 efface). */
	paint(channel: PaintChannel, triangles: Iterable<number>, state: number): void {
		const map = this.channels[channel];
		for (const t of triangles) {
			if (state <= 0) map.delete(t);
			else map.set(t, state);
		}
	}

	/** Efface tout un canal. */
	clear(channel: PaintChannel): void {
		this.channels[channel].clear();
	}

	isEmpty(): boolean {
		return CHANNELS.every((c) => this.channels[c].size === 0);
	}

	/** Sérialise en document 3MF Orca (chaînes hex par facette). */
	serialize(): PaintDocument {
		const doc: PaintDocument = {};
		for (const c of CHANNELS) {
			const map = this.channels[c];
			if (map.size === 0) continue;
			const out: Record<string, string> = {};
			for (const [triangle, state] of map) out[String(triangle)] = encodeFacet(state);
			doc[c] = out;
		}
		return doc;
	}

	/** Reconstruit depuis un document 3MF Orca. */
	static deserialize(doc: PaintDocument): TrianglePainting {
		const painting = new TrianglePainting();
		for (const c of CHANNELS) {
			const entries = doc[c];
			if (!entries) continue;
			for (const [triangle, hex] of Object.entries(entries)) {
				const state = decodeFacet(hex);
				if (state > 0) painting.channels[c].set(Number(triangle), state);
			}
		}
		return painting;
	}
}

/**
 * Indices des triangles dont le barycentre est à moins de `radius` du point
 * `center` (pinceau / sphère). Base data-driven de l'application de peinture.
 */
export function trianglesInRadius(
	mesh: SceneMesh,
	center: [number, number, number],
	radius: number
): number[] {
	const hits: number[] = [];
	const r2 = radius * radius;
	const count = mesh.indices.length / 3;
	for (let t = 0; t < count; t++) {
		let cx = 0;
		let cy = 0;
		let cz = 0;
		for (let k = 0; k < 3; k++) {
			const i = mesh.indices[t * 3 + k];
			cx += mesh.positions[i * 3];
			cy += mesh.positions[i * 3 + 1];
			cz += mesh.positions[i * 3 + 2];
		}
		cx /= 3;
		cy /= 3;
		cz /= 3;
		const dx = cx - center[0];
		const dy = cy - center[1];
		const dz = cz - center[2];
		if (dx * dx + dy * dy + dz * dz <= r2) hits.push(t);
	}
	return hits;
}
