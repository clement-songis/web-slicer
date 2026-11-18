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

/** Préréglages de qualité (raccourcis vers les curseurs d'Orca ; « Fin » = défauts d'Orca). */
export const STEP_PRESETS: readonly {
	id: 'fine' | 'balanced' | 'light';
	label: string;
	hint: string;
	linear_deflection: number;
	angle_deflection: number;
}[] = [
	{
		id: 'fine',
		label: 'Fin',
		hint: 'défaut d’Orca',
		linear_deflection: 0.003,
		angle_deflection: 0.5
	},
	{
		id: 'balanced',
		label: 'Équilibré',
		hint: 'courbes lisses',
		linear_deflection: 0.01,
		angle_deflection: 0.5
	},
	{
		id: 'light',
		label: 'Léger',
		hint: 'peu de triangles',
		linear_deflection: 0.05,
		angle_deflection: 1
	}
];

/** Préréglage correspondant aux déflexions (null = personnalisé). */
export function presetOf(o: Pick<StepMeshOptions, 'linear_deflection' | 'angle_deflection'>) {
	const n = normalizeStepOptions(o);
	return (
		STEP_PRESETS.find(
			(p) =>
				p.linear_deflection === n.linear_deflection && p.angle_deflection === n.angle_deflection
		)?.id ?? null
	);
}

/** Écart de triangles lisible : « +120 % », « −35 % », « = » (null si inconnu). */
export function triangleDelta(before: number | null | undefined, after: number | null | undefined) {
	if (!before || after == null) return null;
	const pct = Math.round(((after - before) / before) * 100);
	if (pct === 0) return '=';
	return `${pct > 0 ? '+' : '−'}${Math.abs(pct)} %`;
}

/** Saisie numérique d'une déflexion (virgule acceptée) ; message d'Orca hors bornes. */
export function parseDeflection(
	raw: string,
	kind: 'linear' | 'angle'
): { value: number } | { error: string } {
	const v = Number(raw.trim().replace(',', '.'));
	const r = kind === 'linear' ? LINEAR_RANGE : ANGLE_RANGE;
	if (!raw.trim() || !Number.isFinite(v) || v < r.min || v > r.max)
		return {
			error:
				kind === 'linear'
					? 'Valeur attendue entre 0,001 et 0,1 mm.'
					: 'Valeur attendue entre 0,01 et 1 rad.'
		};
	return { value: v };
}

/**
 * Préférences mémorisées (Orca app_config : linear_defletion, angle_defletion,
 * is_split_compound, enable_step_mesh_setting) : réglages du dernier import et
 * « ne plus afficher ». Stockage local du navigateur.
 */
export interface StepMeshPrefs {
	options: StepMeshOptions;
	/** Proposer le dialogue après chaque import STEP. */
	ask: boolean;
}
export const STEP_PREFS_KEY = 'web-slicer.step-mesh';

type KV = Pick<Storage, 'getItem' | 'setItem'>;
const store = (): KV | null => {
	try {
		return typeof localStorage === 'undefined' ? null : localStorage;
	} catch {
		return null;
	}
};

export function loadStepPrefs(kv: KV | null = store()): StepMeshPrefs {
	try {
		const raw = kv?.getItem(STEP_PREFS_KEY);
		if (raw) {
			const v = JSON.parse(raw) as Partial<StepMeshPrefs>;
			return { options: normalizeStepOptions(v.options ?? {}), ask: v.ask !== false };
		}
	} catch {
		/* préférences illisibles : défauts */
	}
	return { options: { ...STEP_DEFAULTS }, ask: true };
}

export function saveStepPrefs(p: StepMeshPrefs, kv: KV | null = store()): void {
	try {
		kv?.setItem(
			STEP_PREFS_KEY,
			JSON.stringify({ options: normalizeStepOptions(p.options), ask: p.ask })
		);
	} catch {
		/* stockage indisponible : sans effet */
	}
}
