// Modèles pratiques (Orca « Add Handy models », GUI_Factories.cpp:576) :
// catalogue servi par le backend (`GET /api/handy-models`), ajout au projet
// (`POST /api/projects/{id}/handy-models/{key}`), libellés traduits et
// suggestion de réglage (« String Hell »).
import { api } from '#lib/api/client';
import type { HandyModelAdded, HandyModelInfo, HandyModelSuggestion } from '#lib/api/types';
import type { ObjectMenuEntry, ObjectMenuItem } from '#lib/scene/menus';
import { registerTranslations, tr } from '#lib/i18n';

export type { HandyModelAdded, HandyModelInfo, HandyModelSuggestion };

export const listHandyModels = () => api.get<HandyModelInfo[]>('/handy-models');
export const addHandyModel = (projectId: string, key: string) =>
	api.post<HandyModelAdded>(`/projects/${projectId}/handy-models/${encodeURIComponent(key)}`);

/**
 * Libellé affiché (fr, dictionnaire paresseux) et rôle de chaque modèle (Orca ne
 * dit pas à quoi ils servent) ; le rôle est une clé anglaise traduite au rendu.
 */
const LABELS: Record<string, { label: string; hint: string }> = {
	'Orca Cube': { label: 'Cube Orca', hint: 'interlocking cube and cap' },
	'OrcaSliced Combo': { label: 'Combo OrcaSliced', hint: 'demo plate' },
	'Orca Tolerance Test': { label: 'Test de tolérance Orca', hint: 'clearance between parts' },
	'3DBenchy': { label: '3DBenchy', hint: 'general test' },
	'Cali Cat': { label: 'Cali Cat', hint: 'quick calibration' },
	'Autodesk FDM Test': { label: 'Test FDM Autodesk', hint: 'overhangs, bridges, details' },
	'Voron Cube': { label: 'Cube Voron', hint: 'dimensions and finish' },
	'Stanford Bunny': { label: 'Lapin de Stanford', hint: 'organic surfaces' },
	'Orca String Hell': { label: 'String Hell Orca', hint: 'stringing test' }
};

export function handyLabel(info: Pick<HandyModelInfo, 'label'>): string {
	return LABELS[info.label]?.label ?? info.label;
}
export function handyHint(info: Pick<HandyModelInfo, 'label'>): string | null {
	return LABELS[info.label]?.hint ?? null;
}

export const HANDY_ACTION_PREFIX = 'handy.';
/** Libellé d'Orca (clé de parité), traduit « Modèles pratiques ». */
export const HANDY_MENU_LABEL = 'Add Handy models';

registerTranslations('fr', {
	[HANDY_MENU_LABEL]: 'Modèles pratiques',
	...Object.fromEntries(Object.entries(LABELS).map(([en, v]) => [en, v.label]))
});

/** Clé du modèle pratique d'une action de menu (`handy.<clé>`), ou null. */
export function handyKeyOf(action: string): string | null {
	return action.startsWith(HANDY_ACTION_PREFIX) ? action.slice(HANDY_ACTION_PREFIX.length) : null;
}

/**
 * Menu d'ajout + sous-menu « Modèles pratiques » inséré après les primitives
 * (Orca : sous-menu après « Add Primitive »). Liste vide → menu inchangé.
 */
export function withHandyMenu(
	items: readonly ObjectMenuEntry[],
	models: readonly HandyModelInfo[]
): ObjectMenuEntry[] {
	if (!models.length) return [...items];
	const sub: ObjectMenuItem = {
		label: HANDY_MENU_LABEL,
		action: 'handy',
		items: models.map((m) => ({
			label: m.label,
			hint: handyHint(m) ?? undefined,
			action: HANDY_ACTION_PREFIX + m.key
		}))
	};
	// Après la dernière primitive (avant le premier séparateur qui suit « Torus »).
	const out = [...items];
	const torus = out.findIndex((e) => e !== 'separator' && e.action === 'add.torus');
	out.splice(torus >= 0 ? torus + 1 : out.length, 0, 'separator', sub);
	return out;
}

/**
 * Suggestion applicable ? Toutes les clés `when_all` du process édité doivent
 * valoir « vrai » ou être strictement positives (Orca : only_one_wall_top &&
 * min_width_top_surface > 0).
 */
export function suggestionApplies(
	suggestion: HandyModelSuggestion | undefined | null,
	values: Record<string, unknown>
): boolean {
	if (!suggestion || !suggestion.when_all.length) return false;
	return suggestion.when_all.every((key) => {
		const raw = values[key];
		const v = Array.isArray(raw) ? raw[0] : raw;
		if (v === true) return true;
		if (typeof v === 'number') return v > 0;
		if (typeof v === 'string') {
			const s = v.trim().replace('%', '');
			if (s === 'true') return true;
			const n = Number(s);
			return Number.isFinite(n) && n > 0;
		}
		return false;
	});
}

/** Message de la suggestion String Hell, en clair. */
export function suggestionText(s: HandyModelSuggestion): string {
	if (s.apply.min_width_top_surface !== undefined)
		return tr(
			'For the stringing test to be reliable, the top surface must be printed as a single wall over its whole width: “Top surface minimum width” goes to 0 (process setting, to be saved afterwards).'
		);
	return s.message;
}
