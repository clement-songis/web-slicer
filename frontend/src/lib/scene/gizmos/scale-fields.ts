// Champs d'échelle du panneau de transformation (audit 40 B3) : Orca affiche
// l'échelle en POURCENTAGE (« Scale ratios », 100 % par défaut) et une ligne
// « Size » en mm, avec un verrou d'échelle uniforme (GizmoObjectManipulation.cpp,
// m_uniform_scale, actif par défaut). Module pur : le panneau convertit, le
// parent applique la transformation.
import { clampScale } from '../transform';

export type Vec3 = [number, number, number];

const round2 = (v: number) => Math.round(v * 100) / 100;

/** Facteur → pourcentage affiché (2 décimales, « 100 », « 150 », « -100 » en miroir). */
export function scalePercent(scale: number): number {
	return round2(scale * 100);
}

/** Taille affichée (mm) : taille du maillage à l'échelle 1 × |échelle|. */
export function scaledSize(base: Vec3, scale: Vec3): Vec3 {
	return [0, 1, 2].map((i) => round2(Math.abs(base[i] * scale[i]))) as Vec3;
}

/**
 * Nouvelle échelle quand l'axe `axis` doit valoir `magnitude` (facteur positif) :
 * signe d'origine conservé (miroir), autres axes multipliés du même rapport si
 * l'échelle est uniforme (Orca : verrou fermé).
 */
export function withAxisScale(
	scale: Vec3,
	axis: number,
	magnitude: number,
	uniform: boolean
): Vec3 | null {
	if (!Number.isFinite(magnitude) || magnitude <= 0) return null;
	const target = clampScale(magnitude);
	const sign = (v: number) => (v < 0 ? -1 : 1);
	if (!uniform) {
		const next = [...scale] as Vec3;
		next[axis] = sign(scale[axis]) * target;
		return next;
	}
	const old = Math.abs(scale[axis]);
	if (!(old > 0)) return scale.map((v) => sign(v) * target) as Vec3;
	const k = target / old;
	return scale.map((v, i) =>
		i === axis ? sign(v) * target : sign(v) * clampScale(Math.abs(v) * k)
	) as Vec3;
}

/** Saisie en % sur un axe. */
export function scaleFromPercent(
	scale: Vec3,
	axis: number,
	percent: number,
	uniform: boolean
): Vec3 | null {
	return withAxisScale(scale, axis, Math.abs(percent) / 100, uniform);
}

/** Saisie d'une taille (mm) sur un axe : échelle = taille / taille à l'échelle 1. */
export function scaleFromSize(
	scale: Vec3,
	base: Vec3,
	axis: number,
	mm: number,
	uniform: boolean
): Vec3 | null {
	if (!(base[axis] > 0)) return null;
	return withAxisScale(scale, axis, mm / base[axis], uniform);
}
