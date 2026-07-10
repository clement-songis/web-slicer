// Filtrage de l'arbre de réglages (T042) : mode d'affichage + recherche.
// Fonctions pures (aucun import Svelte) → testables isolément.
//
// Parité OrcaSlicer : chaque paramètre porte un `mode` minimal (simple <
// advanced < expert) ; le mode « develop » est interne et jamais montré dans
// l'UI. La recherche est un ajout UX (FR-005) : quand une requête est saisie,
// on ne montre que les options qui correspondent (clé/label/infobulle), quel
// que soit leur mode (hors develop).

import { PARAMS, type ParamDef, type ParamMode } from '../../generated/params';
import type { UiOption, UiPage } from '../../generated/ui-layout';

/** Modes exposés à l'utilisateur (develop reste interne). */
export type DisplayMode = 'simple' | 'advanced' | 'expert';

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

/** La requête matche-t-elle la clé, le libellé ou l'infobulle ? */
export function matchesQuery(def: ParamDef, query: string): boolean {
	const q = query.trim().toLowerCase();
	if (!q) return true;
	return (
		def.key.toLowerCase().includes(q) ||
		def.label.toLowerCase().includes(q) ||
		def.tooltip.toLowerCase().includes(q)
	);
}

/** Visibilité d'une option paramètre selon le mode et la recherche. */
export function optionVisible(def: ParamDef, mode: DisplayMode, query: string): boolean {
	if (query.trim()) {
		// Recherche : atteint tous les modes visibles, jamais develop.
		return def.mode !== 'develop' && matchesQuery(def, query);
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
 * Les sections et pages devenues vides sont retirées (rien à afficher).
 */
export function filterLayout(layout: UiPage[], mode: DisplayMode, query: string): UiPage[] {
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
		.filter((page) => page.sections.length > 0);
}
