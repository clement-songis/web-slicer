// Préférence « Grouper les presets filament utilisateur » (Orca
// Preferences.cpp group_filament_presets ; PresetComboBoxes.cpp : All "0" /
// None "1" / By type "2" / By vendor "3", défaut None). Orca ne l'applique
// qu'au sélecteur de filament (type et fabricant sont des clés filament).
// Pur → testable ; le chargement des valeurs passe par un `fetcher` injecté.

import { tr } from '#lib/i18n';

export type UserGrouping = 'all' | 'none' | 'type' | 'vendor';

export const USER_GROUPINGS: { value: UserGrouping; label: string }[] = [
	{ value: 'all', label: 'All (a single “My Presets” group)' },
	{ value: 'none', label: 'None (no subgroup)' },
	{ value: 'type', label: 'By filament type' },
	{ value: 'vendor', label: 'By vendor' }
];

/** Libellé (clé i18n) du groupe unique des presets utilisateur ; traduit à l'appel. */
export const USER_GROUP_LABEL = 'My Presets';
/** Orca « Unspecified » : type / fabricant vide (clé i18n, traduite au rendu). */
export const UNSPECIFIED = 'Unspecified';

export interface UserPresetMeta {
	type?: string;
	vendor?: string;
}

interface Named {
	id: string;
	name: string;
}

export interface UserPresetGroup<T> {
	/** Libellé d'`<optgroup>` ; vide = options sans groupe. */
	label: string;
	presets: T[];
}

/** Regroupe les presets utilisateur selon la préférence (ordre d'entrée conservé dans un groupe). */
export function groupUserPresets<T extends Named>(
	user: readonly T[],
	grouping: UserGrouping,
	meta: Readonly<Record<string, UserPresetMeta>> = {}
): UserPresetGroup<T>[] {
	if (user.length === 0) return [];
	if (grouping === 'none') return [{ label: '', presets: [...user] }];
	if (grouping === 'all') return [{ label: tr(USER_GROUP_LABEL), presets: [...user] }];
	const buckets = new Map<string, T[]>();
	for (const p of user) {
		const raw = (grouping === 'type' ? meta[p.id]?.type : meta[p.id]?.vendor)?.trim();
		const key = raw || UNSPECIFIED;
		buckets.set(key, [...(buckets.get(key) ?? []), p]);
	}
	return [...buckets.entries()]
		.sort(([a], [b]) => (a === UNSPECIFIED ? 1 : b === UNSPECIFIED ? -1 : a.localeCompare(b)))
		.map(([key, presets]) => ({ label: `${tr(USER_GROUP_LABEL)} · ${key}`, presets }));
}

function first(value: unknown): string | undefined {
	const v = Array.isArray(value) ? value[0] : value;
	return typeof v === 'string' && v.trim() ? v.trim() : undefined;
}

/** Type et fabricant d'un preset filament résolu (`filament_type[0]`, `filament_vendor[0]`). */
export function metaFromValues(values: Record<string, unknown> | null | undefined): UserPresetMeta {
	return { type: first(values?.filament_type), vendor: first(values?.filament_vendor) };
}

const cache = new Map<string, Promise<UserPresetMeta>>();

/**
 * Charge (une fois par id, en parallèle) le type et le fabricant des presets
 * utilisateur. Un échec isolé donne « Non précisé » sans bloquer les autres.
 */
export async function loadUserPresetMeta(
	ids: readonly string[],
	fetchValues: (id: string) => Promise<Record<string, unknown>>
): Promise<Record<string, UserPresetMeta>> {
	const entries = await Promise.all(
		ids.map(async (id) => {
			let pending = cache.get(id);
			if (!pending) {
				pending = fetchValues(id)
					.then(metaFromValues)
					.catch(() => {
						cache.delete(id);
						return {};
					});
				cache.set(id, pending);
			}
			return [id, await pending] as const;
		})
	);
	return Object.fromEntries(entries);
}

/** Vide le cache (preset utilisateur modifié / supprimé, tests). */
export function clearUserPresetMeta(id?: string): void {
	if (id) cache.delete(id);
	else cache.clear();
}
