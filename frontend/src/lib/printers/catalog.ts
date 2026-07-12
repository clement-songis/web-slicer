// Logique pure de la grille de sélection d'imprimante (Phase 14, T133) : portage
// du dialogue OrcaSlicer « Sélection de l'imprimante ». Multi-sélection par
// **modèle**, compteurs `X / N` et « tout sélectionner » par marque, recherche
// et modes de vue. Aucune dépendance au DOM : testable sous vitest ; le composant
// `PrinterCatalog.svelte` (T134) ne fait que la disposition.
import type { PrinterCatalogModel, PrinterCatalogVendor } from '$lib/api/types';

/** Densité d'affichage de la grille (bascules en haut à droite du dialogue). */
export type CatalogView = 'list' | 'grid' | 'large';

/** Base statique des vignettes copiées depuis les profils vendeurs (T134). */
const COVER_BASE = '/printers';

/** Clé de sélection stable d'un modèle (marque + modèle). */
export function modelKey(model: PrinterCatalogModel): string {
	return `${model.vendor}::${model.model}`;
}

/** URL statique de la vignette ; le composant retombe sur un placeholder si 404. */
export function coverUrl(model: PrinterCatalogModel): string {
	return `${COVER_BASE}/${model.cover}`;
}

/** Filtre le catalogue par texte (marque ou modèle) ; marques vides retirées. */
export function filterCatalog(
	vendors: PrinterCatalogVendor[],
	query: string
): PrinterCatalogVendor[] {
	const q = query.trim().toLowerCase();
	if (!q) return vendors;
	return vendors
		.map((v) => ({
			vendor: v.vendor,
			models: v.models.filter(
				(m) => m.model.toLowerCase().includes(q) || m.vendor.toLowerCase().includes(q)
			)
		}))
		.filter((v) => v.models.length > 0);
}

export function isSelected(selected: ReadonlySet<string>, model: PrinterCatalogModel): boolean {
	return selected.has(modelKey(model));
}

/** Bascule la sélection d'un modèle (retourne un nouveau `Set`, immuable). */
export function toggleModel(
	selected: ReadonlySet<string>,
	model: PrinterCatalogModel
): Set<string> {
	const next = new Set(selected);
	const k = modelKey(model);
	if (next.has(k)) next.delete(k);
	else next.add(k);
	return next;
}

/** Nombre de modèles sélectionnés dans une marque (compteur `X / N`). */
export function vendorSelectedCount(
	selected: ReadonlySet<string>,
	vendor: PrinterCatalogVendor
): number {
	return vendor.models.reduce((n, m) => (selected.has(modelKey(m)) ? n + 1 : n), 0);
}

/** Vrai si tous les modèles d'une marque sont sélectionnés. */
export function vendorAllSelected(
	selected: ReadonlySet<string>,
	vendor: PrinterCatalogVendor
): boolean {
	return vendor.models.length > 0 && vendor.models.every((m) => selected.has(modelKey(m)));
}

/** Case « tout sélectionner » d'une marque : tout cocher, sinon tout décocher. */
export function toggleVendorAll(
	selected: ReadonlySet<string>,
	vendor: PrinterCatalogVendor
): Set<string> {
	const next = new Set(selected);
	if (vendorAllSelected(selected, vendor)) {
		for (const m of vendor.models) next.delete(modelKey(m));
	} else {
		for (const m of vendor.models) next.add(modelKey(m));
	}
	return next;
}

/** Modèles sélectionnés (aplatis) — à la confirmation, chacun → une imprimante. */
export function selectedModels(
	vendors: PrinterCatalogVendor[],
	selected: ReadonlySet<string>
): PrinterCatalogModel[] {
	return vendors.flatMap((v) => v.models.filter((m) => selected.has(modelKey(m))));
}
