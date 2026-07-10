// Vue d'assemblage éclatée (T058, gizmo Assembly) : écarte les pièces depuis le
// centre de la scène d'un facteur donné, pour visualiser un assemblage. Pur →
// testable ; purement visuel (n'altère pas les transformations enregistrées).

export type Vec3 = [number, number, number];

/** Pièce à écarter : identifiant et centre (mm, repère plateau). */
export interface AssemblyPart {
	id: string;
	center: Vec3;
}

/** Centre moyen d'un ensemble de pièces. */
export function sceneCenter(parts: AssemblyPart[]): Vec3 {
	if (parts.length === 0) return [0, 0, 0];
	const sum: Vec3 = [0, 0, 0];
	for (const p of parts) {
		sum[0] += p.center[0];
		sum[1] += p.center[1];
		sum[2] += p.center[2];
	}
	return [sum[0] / parts.length, sum[1] / parts.length, sum[2] / parts.length];
}

/**
 * Positions éclatées : chaque pièce est déplacée de `factor × (centre − centre
 * scène)`. `factor = 0` laisse les pièces en place ; une valeur positive les
 * écarte radialement.
 */
export function explode(parts: AssemblyPart[], factor: number): { id: string; position: Vec3 }[] {
	const c = sceneCenter(parts);
	return parts.map((p) => ({
		id: p.id,
		position: [
			p.center[0] + (p.center[0] - c[0]) * factor,
			p.center[1] + (p.center[1] - c[1]) * factor,
			p.center[2] + (p.center[2] - c[2]) * factor
		]
	}));
}
