// Oreilles de bord (T057, gizmo BrimEars) : points d'ancrage où ajouter des
// « oreilles » de brim, plus les paramètres associés du registre
// (`brim_ears`, `brim_ears_detection_length`, `brim_ears_max_angle`).
// Modèle pur → testable ; persisté dans le document scène.

/** Point d'oreille de bord (coordonnées plateau, mm). */
export interface BrimEarPoint {
	x: number;
	y: number;
}

/** Paramètres du registre pilotant les oreilles de bord. */
export const BRIM_EARS_KEYS = [
	'brim_ears',
	'brim_ears_detection_length',
	'brim_ears_max_angle'
] as const;

/** Document sérialisé des oreilles de bord d'un objet. */
export interface BrimEarsDocument {
	points: [number, number][];
}

/** Ensemble de points d'oreilles de bord d'un objet. */
export class BrimEars {
	private points: BrimEarPoint[] = [];

	/** Ajoute un point d'oreille et le renvoie. */
	add(x: number, y: number): BrimEarPoint {
		const p = { x, y };
		this.points.push(p);
		return p;
	}

	/** Supprime le point d'indice `i` (ignore un indice hors limites). */
	removeAt(i: number): void {
		if (i >= 0 && i < this.points.length) this.points.splice(i, 1);
	}

	clear(): void {
		this.points = [];
	}

	list(): readonly BrimEarPoint[] {
		return this.points;
	}

	get count(): number {
		return this.points.length;
	}

	/** Sérialise pour le document scène. */
	serialize(): BrimEarsDocument {
		return { points: this.points.map((p) => [p.x, p.y]) };
	}

	/** Reconstruit depuis un document scène. */
	static deserialize(doc: BrimEarsDocument): BrimEars {
		const ears = new BrimEars();
		for (const [x, y] of doc.points ?? []) ears.add(x, y);
		return ears;
	}
}
