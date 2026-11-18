// Maillage STEP (Orca : StepMeshDialog) : déflexions linéaire / angulaire et
// découpe des compounds, envoyées à `POST /api/models/{id}/step-mesh` (lot 17).
// Bornes et défauts du dialogue d'Orca. Pur → testable.
import { api } from '$lib/api/client';

export interface StepMeshOptions {
	/** Déflexion linéaire (mm) : écart maximal corde / surface. */
	linear_deflection: number;
	/** Déflexion angulaire (rad) entre facettes voisines. */
	angle_deflection: number;
	/** Une pièce par solide d'un compound (« <nom>-SOLID-n »). */
	split_compound: boolean;
}

export interface StepMeshResult {
	triangles: number;
	/** Nombre annoncé par le dialogue d'Orca (Step::get_triangle_num). */
	dialog_triangles: number | null;
	objects: { name: string; triangles: number }[];
}

/** Défauts d'Orca (StepMeshDialog : 0,003 mm, 0,5 rad, sans découpe). */
export const STEP_DEFAULTS: StepMeshOptions = {
	linear_deflection: 0.003,
	angle_deflection: 0.5,
	split_compound: false
};
/** Bornes des curseurs d'Orca. */
export const LINEAR_RANGE = { min: 0.001, max: 0.1, step: 0.001 } as const;
export const ANGLE_RANGE = { min: 0.01, max: 1, step: 0.01 } as const;

const clamp = (v: number, r: { min: number; max: number; step: number }) => {
	if (!Number.isFinite(v)) return r.min;
	const snapped = Math.round(v / r.step) * r.step;
	return Number(Math.min(r.max, Math.max(r.min, snapped)).toFixed(3));
};

/** Options bornées (curseurs d'Orca), prêtes à envoyer. */
export function normalizeStepOptions(o: Partial<StepMeshOptions>): StepMeshOptions {
	return {
		linear_deflection: clamp(o.linear_deflection ?? STEP_DEFAULTS.linear_deflection, LINEAR_RANGE),
		angle_deflection: clamp(o.angle_deflection ?? STEP_DEFAULTS.angle_deflection, ANGLE_RANGE),
		split_compound: !!o.split_compound
	};
}

export function sameStepOptions(a: StepMeshOptions, b: StepMeshOptions): boolean {
	return (
		a.linear_deflection === b.linear_deflection &&
		a.angle_deflection === b.angle_deflection &&
		a.split_compound === b.split_compound
	);
}

/** Fichier STEP (dialogue proposé après l'import). */
export function isStep(name: string): boolean {
	return /\.(step|stp)$/i.test(name);
}

/** Nombre lisible (séparateur de milliers). */
export function formatTriangles(n: number): string {
	return n.toLocaleString('fr-FR');
}

export const remeshStep = (modelId: string, options: StepMeshOptions) =>
	api.post<StepMeshResult>(`/models/${modelId}/step-mesh`, normalizeStepOptions(options));
