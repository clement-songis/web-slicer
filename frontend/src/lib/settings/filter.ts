// Filtrage de l'arbre de réglages (T042) : mode d'affichage + recherche.
// Fonctions pures (aucun import Svelte) → testables isolément.
//
// Parité OrcaSlicer : chaque paramètre porte un `mode` minimal (simple <
// advanced < expert < develop). La recherche est un ajout UX (FR-005).

import { PARAMS, type ParamDef, type ParamMode } from '../../generated/params';
import type { UiOption, UiPage } from '../../generated/ui-layout';
import { tr } from '$lib/i18n';

/** Modes exposés à l'utilisateur, dans l'ordre Orca. */
export type DisplayMode = 'simple' | 'advanced' | 'expert' | 'develop';

/** Rang de visibilité : un paramètre est visible si son rang ≤ rang actif. */
export const MODE_RANK: Record<ParamMode, number> = {
	simple: 0,
	advanced: 1,
	expert: 2,
	develop: 3
};

/** Le mode actif révèle-t-il un paramètre de mode `paramMode` ? */
export function modeAllows(paramMode: ParamMode, active: DisplayMode): boolean {
	return MODE_RANK[paramMode] <= MODE_RANK[active];
}

/** Minuscules sans diacritiques : « epaisseur » trouve « Épaisseur ». */
function fold(s: string): string {
	return s
		.normalize('NFD')
		.replace(/[\u0300-\u036f]/g, '')
		.toLowerCase();
}

/**
 * La requête matche-t-elle la clé, le libellé ou l'infobulle — en anglais
 * (clés Orca) comme dans la locale courante (libellé/infobulle traduits) ?
 */
export function matchesQuery(def: ParamDef, query: string): boolean {
	const q = fold(query.trim());
	if (!q) return true;
	return [def.key, def.label, def.tooltip, tr(def.label), def.tooltip ? tr(def.tooltip) : ''].some(
		(text) => fold(text).includes(q)
	);
}

/** Visibilité d'une option paramètre selon le mode et la recherche. */
export function optionVisible(def: ParamDef, mode: DisplayMode, query: string): boolean {
	if (query.trim()) {
		// Recherche : atteint tous les modes visibles ; develop reste explicite.
		return (def.mode !== 'develop' || mode === 'develop') && matchesQuery(def, query);
	}
	return modeAllows(def.mode, mode);
}

/** Une ligne dynamique (générée par Orca) n'a pas de définition : visible hors
 *  recherche uniquement (rien à matcher). */
function dynamicVisible(query: string): boolean {
	return query.trim().length === 0;
}

/** Garde les options visibles d'une section (clés inconnues ignorées). */
function filterOptions(options: UiOption[], mode: DisplayMode, query: string): UiOption[] {
	return options.filter((option) => {
		if (typeof option !== 'string') return dynamicVisible(query);
		const def = PARAMS[option];
		return def ? optionVisible(def, mode, query) : false;
	});
}

/**
 * Filtre l'arbre pages → sections → options selon le mode et la recherche.
 * Les sections devenues vides sont retirées ; les pages vides aussi, sauf
 * `keepEmptyPages` : hors recherche, les onglets restent stables d'un mode à
 * l'autre (un réglage avancé est masqué en mode simple, pas son onglet).
 */
export function filterLayout(
	layout: UiPage[],
	mode: DisplayMode,
	query: string,
	keepEmptyPages = false
): UiPage[] {
	return layout
		.map((page) => ({
			...page,
			sections: page.sections
				.map((section) => ({
					...section,
					options: filterOptions(section.options, mode, query)
				}))
				.filter((section) => section.options.length > 0)
		}))
		.filter((page) => keepEmptyPages || page.sections.length > 0);
}
