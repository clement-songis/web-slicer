// Trait de pinceau glissé (parité GLGizmoPainterBase) : échantillons
// successifs (point, facette, caméra) en repère maillage pendant l'appui, puis
// un seul envoi au moteur au relâchement. Le moteur relie les échantillons par
// des capsules (`DoublePointCursor`) comme Orca entre deux positions souris.
// Pur → testable.
import type { MeshPick } from '../../mesh';

type Vec3 = [number, number, number];

/** Taille maximale d'un envoi (`MAX_STROKE_POINTS` du moteur). */
export const MAX_BATCH_POINTS = 4096;

export interface StrokeBatch {
	points: Vec3[];
	facets: number[];
}

export class StrokeRecorder {
	readonly points: Vec3[] = [];
	readonly facets: number[] = [];
	camera: Vec3 | null = null;

	/** `minSpacing` (mm) : échantillons plus rapprochés ignorés (Orca échantillonne
	 *  au pixel ; la capsule couvre l'intervalle, la densité n'ajoute rien). */
	constructor(private readonly minSpacing: number) {}

	/** Ajoute un échantillon touché ; renvoie vrai s'il est retenu. */
	add(pick: MeshPick): boolean {
		if (pick.triangle === null) return false;
		const last = this.points[this.points.length - 1];
		if (last) {
			const d = Math.hypot(
				pick.localPoint[0] - last[0],
				pick.localPoint[1] - last[1],
				pick.localPoint[2] - last[2]
			);
			if (d < this.minSpacing) return false;
		}
		this.points.push([...pick.localPoint]);
		this.facets.push(pick.triangle);
		if (pick.localCamera) this.camera = [...pick.localCamera];
		return true;
	}

	get length(): number {
		return this.points.length;
	}

	/** Lots ≤ `max` points, chevauchant d'un point (capsules continues). */
	batches(max = MAX_BATCH_POINTS): StrokeBatch[] {
		const out: StrokeBatch[] = [];
		if (this.points.length === 0) return out;
		const step = Math.max(1, max - 1);
		for (let start = 0; ; start += step) {
			const end = Math.min(this.points.length, start + max);
			out.push({ points: this.points.slice(start, end), facets: this.facets.slice(start, end) });
			if (end >= this.points.length) break;
		}
		return out;
	}
}
