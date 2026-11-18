// Recalcul automatique des volumes de purge (Orca `auto_calculate_flush`,
// Plater.cpp:3640, 9544 ; Tab.cpp:1850) : après un changement de couleur
// (« color change ») ou de couleur/filament (« all »), seules les lignes et
// colonnes du slot changé sont recalculées ; les autres valeurs, éventuellement
// saisies à la main, sont conservées. Jamais déclenché par une saisie manuelle
// ni par « Appliquer » du dialogue ; le multiplicateur n'est jamais modifié.
import type { AutoFlush } from '$lib/settings/preferences';

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
