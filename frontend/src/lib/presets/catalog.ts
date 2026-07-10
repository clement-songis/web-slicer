// Logique pure des sélecteurs de presets (T046) : arbre imprimante
// vendeur → modèle → buse, filtre de compatibilité (FR-021), badge d'héritage.
// Sans dépendance Svelte → testable isolément.

import type { PresetSummary } from '../api/types';

/** Une buse disponible pour un modèle d'imprimante (preset machine). */
export interface NozzleOption {
	nozzle: string;
	preset: PresetSummary;
}

/** Un modèle d'imprimante et ses buses. */
export interface PrinterModel {
	model: string;
	nozzles: NozzleOption[];
}

/** Un vendeur et ses modèles d'imprimante. */
export interface PrinterVendor {
	vendor: string;
	models: PrinterModel[];
}

const NOZZLE_RE = /\s([\d.]+)\s*nozzle$/i;

/** Diamètre de buse extrait d'un nom de preset machine (« … 0.4 nozzle »). */
export function parseNozzle(name: string): string | null {
	const m = name.match(NOZZLE_RE);
	return m ? m[1] : null;
}

/** Nom de modèle : le nom du preset sans le suffixe de buse. */
export function printerModel(name: string): string {
	return name.replace(NOZZLE_RE, '').trim();
}

/**
 * Regroupe des presets machine en arbre vendeur → modèle → buse, trié. Les
 * presets sans buse identifiable sont classés sous la buse « — ».
 */
export function groupPrinters(presets: PresetSummary[]): PrinterVendor[] {
	const vendors = new Map<string, Map<string, NozzleOption[]>>();
	for (const preset of presets) {
		if (preset.kind !== 'machine') continue;
		const vendor = preset.vendor ?? 'Autre';
		const model = printerModel(preset.name);
		const nozzle = parseNozzle(preset.name) ?? '—';
		const models = vendors.get(vendor) ?? new Map<string, NozzleOption[]>();
		const nozzles = models.get(model) ?? [];
		nozzles.push({ nozzle, preset });
		models.set(model, nozzles);
		vendors.set(vendor, models);
	}
	return [...vendors.entries()]
		.map(([vendor, models]) => ({
			vendor,
			models: [...models.entries()]
				.map(([model, nozzles]) => ({
					model,
					nozzles: nozzles.sort((a, b) => a.nozzle.localeCompare(b.nozzle))
				}))
				.sort((a, b) => a.model.localeCompare(b.model))
		}))
		.sort((a, b) => a.vendor.localeCompare(b.vendor));
}

/** Forme minimale portant une liste de compatibilité imprimante. */
export interface Compatible {
	compatiblePrinters?: string[] | null;
}

/**
 * Un preset est compatible si sa liste est vide/absente (universel) ou contient
 * l'imprimante active. Sans imprimante active, tout est montré (FR-021).
 */
export function isCompatible(
	compatiblePrinters: string[] | null | undefined,
	printerName: string | null
): boolean {
	if (!compatiblePrinters || compatiblePrinters.length === 0) return true;
	if (!printerName) return true;
	return compatiblePrinters.includes(printerName);
}

/** Garde les presets compatibles avec l'imprimante active. */
export function filterByPrinter<T extends Compatible>(items: T[], printerName: string | null): T[] {
	return items.filter((item) => isCompatible(item.compatiblePrinters, printerName));
}

/** Forme minimale portant l'origine et le parent d'héritage. */
export interface Inheritable {
	origin: string;
	inherits?: string | null;
}

/** Un preset dérivé = utilisateur avec un parent (`inherits`). */
export function isDerived(preset: Inheritable): boolean {
	return preset.origin === 'user' && !!preset.inherits;
}

/** Libellé de badge d'héritage, ou `null` s'il n'hérite de rien. */
export function inheritanceLabel(preset: Inheritable): string | null {
	return preset.inherits ? `hérite de ${preset.inherits}` : null;
}

/** Sépare une liste en presets système et utilisateur (ordre préservé). */
export function partitionByOrigin<T extends { origin: string }>(
	presets: T[]
): { system: T[]; user: T[] } {
	return {
		system: presets.filter((p) => p.origin === 'system'),
		user: presets.filter((p) => p.origin === 'user')
	};
}
