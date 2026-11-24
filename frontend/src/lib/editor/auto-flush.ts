// Recalcul automatique des volumes de purge (Orca `auto_calculate_flush`,
// Plater.cpp:3640, 9544 ; Tab.cpp:1850) : après un changement de couleur
// (« color change ») ou de couleur/filament (« all »), seules les lignes et
// colonnes du slot changé sont recalculées ; les autres valeurs, éventuellement
// saisies à la main, sont conservées. Jamais déclenché par une saisie manuelle
// ni par « Appliquer » du dialogue ; le multiplicateur n'est jamais modifié.
import type { AutoFlush } from '#lib/settings/preferences';

export interface FlushSnapshot {
	colours: readonly string[];
	filaments: readonly (string | null)[];
}

/** Slots dont la purge doit être recalculée entre deux états. */
export function changedSlots(
	prev: FlushSnapshot | null,
	next: FlushSnapshot,
	mode: AutoFlush
): number[] {
	if (!prev || mode === 'disabled') return [];
	const n = next.colours.length;
	// Ajout / retrait de slot : la matrice est redimensionnée ailleurs.
	if (n < 2 || prev.colours.length !== n) return [];
	const out: number[] = [];
	for (let i = 0; i < n; i++) {
		const colour = (prev.colours[i] ?? '').toLowerCase() !== (next.colours[i] ?? '').toLowerCase();
		const filament = mode === 'all' && (prev.filaments[i] ?? null) !== (next.filaments[i] ?? null);
		if (colour || filament) out.push(i);
	}
	return out;
}

/**
 * Matrice n×n : lignes et colonnes de `slots` prises dans `computed`, le reste
 * dans `current` (absente ou mal dimensionnée → tout `computed`).
 */
export function mergeFlushSlots(
	current: readonly number[] | undefined,
	computed: readonly number[],
	n: number,
	slots: readonly number[]
): number[] {
	if (computed.length !== n * n) return current ? [...current] : [];
	if (!current || current.length !== n * n) return [...computed];
	const set = new Set(slots);
	const out: number[] = [];
	for (let r = 0; r < n; r++)
		for (let c = 0; c < n; c++)
			out.push(set.has(r) || set.has(c) ? computed[r * n + c] : current[r * n + c]);
	return out;
}

/** Volumes de purge du projet (surcharges ; absents = preset). */
export interface FlushState {
	matrix?: readonly number[];
	multiplier?: number;
}

function sameFlush(a: FlushState, b: FlushState): boolean {
	if ((a.multiplier ?? null) !== (b.multiplier ?? null)) return false;
	const x = a.matrix ?? null;
	const y = b.matrix ?? null;
	if (x === y) return true;
	if (!x || !y || x.length !== y.length) return false;
	return x.every((v, i) => v === y[i]);
}

/**
 * Matrice à poser à l'issue d'un recalcul automatique lancé dans l'état
 * `started` ; `now` = état à la réception. null = ne rien changer : volumes ou
 * multiplicateur saisis entre-temps (la saisie manuelle prime, comme dans Orca
 * où le calcul est synchrone) ou réponse inexploitable. Le multiplicateur
 * n'est jamais touché par le recalcul.
 */
export function autoFlushResult(
	started: FlushState,
	now: FlushState,
	computed: readonly number[],
	n: number,
	slots: readonly number[]
): number[] | null {
	if (!slots.length || !sameFlush(started, now)) return null;
	const merged = mergeFlushSlots(now.matrix, computed, n, slots);
	return merged.length === n * n ? merged.map((v) => Math.round(v)) : null;
}

/**
 * Matrice du projet utilisable telle quelle (N×N, valeurs finies ≥ 0), sinon
 * null : sans matrice valide, le tranchage la calcule depuis les couleurs
 * (FlushVolCalculator, comme le CLI d'Orca), et c'est elle que le dialogue
 * doit montrer (Orca WipingDialog affiche la matrice du projet, recalculée à
 * chaque changement de couleur).
 */
export function usableFlushMatrix(
	matrix: readonly number[] | undefined,
	n: number
): number[] | null {
	if (!matrix || n < 1 || matrix.length !== n * n) return null;
	if (!matrix.every((v) => Number.isFinite(v) && v >= 0)) return null;
	return [...matrix];
}

/**
 * Phase 47b : réponse de `POST /api/flush-volumes {project_id, effective: true}`
 * lue pour le dialogue (N filaments) : sous-matrice de la 1ʳᵉ buse (`base`,
 * éditée), sous-matrices des autres buses (`tail`, réécrites telles quelles
 * avec la matrice) et origine. null si la taille ne correspond pas (couleurs
 * locales ≠ projet enregistré, réponse inattendue).
 */
export function effectiveFlushView(
	res: { matrix: readonly number[]; source?: string; nozzles?: number } | null | undefined,
	n: number
): { base: number[]; tail: number[]; source: 'project' | 'auto' } | null {
	if (!res || n < 2 || !Array.isArray(res.matrix)) return null;
	const nozzles = Math.max(1, Math.floor(res.nozzles ?? 1));
	const size = n * n;
	const m = res.matrix;
	if (m.length !== size && m.length !== size * nozzles) return null;
	if (!m.every((v) => Number.isFinite(v) && v >= 0)) return null;
	const rounded = m.map((v) => Math.round(v));
	return {
		base: rounded.slice(0, size),
		tail: rounded.slice(size),
		source: res.source === 'project' ? 'project' : 'auto'
	};
}

/**
 * Résultat d'« Appliquer » du dialogue des volumes de purge : seules les
 * cellules réellement modifiées (`draft` ≠ `base`, la matrice affichée à
 * l'ouverture = celle du tranchage) sont réécrites ; undefined = rien à
 * changer (la matrice du projet, ou son absence — calcul automatique au
 * tranchage — est conservée). `base` null = matrice utilisée inconnue
 * (calcul indisponible) : le brouillon est pris tel quel s'il diffère.
 */
export function flushApplyMatrix(
	base: readonly number[] | null,
	draft: readonly number[],
	n: number
): number[] | undefined {
	if (draft.length !== n * n) return undefined;
	if (!base || base.length !== n * n) return [...draft];
	let changed = false;
	const out = base.map((v, i) => {
		if (draft[i] === v) return v;
		changed = true;
		return draft[i];
	});
	return changed ? out : undefined;
}
