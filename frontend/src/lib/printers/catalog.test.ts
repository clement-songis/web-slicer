// Tests de la logique de la grille de sélection d'imprimante (T133).
import { describe, expect, it } from 'vitest';
import type { PrinterCatalogModel, PrinterCatalogVendor } from '$lib/api/types';
import {
	coverUrl,
	filterCatalog,
	isSelected,
	modelKey,
	selectedModels,
	toggleModel,
	toggleVendorAll,
	vendorAllSelected,
	vendorSelectedCount
} from './catalog';

function model(vendor: string, name: string, id: string): PrinterCatalogModel {
	return {
		vendor,
		model: name,
		cover: `${vendor}/${name}_cover.png`,
		variants: [{ machine_preset_id: id, nozzle: '0.4' }],
		default_machine_preset_id: id
	};
}

const catalog: PrinterCatalogVendor[] = [
	{ vendor: 'Anet', models: [model('Anet', 'A8', 'p1')] },
	{ vendor: 'Prusa', models: [model('Prusa', 'MK4', 'p2'), model('Prusa', 'MINI', 'p3')] }
];

describe('catalogue de sélection d’imprimante', () => {
	it('produit une clé de modèle stable et l’URL de vignette', () => {
		const m = model('Prusa', 'MK4', 'p2');
		expect(modelKey(m)).toBe('Prusa::MK4');
		expect(coverUrl(m)).toBe('/printers/Prusa/MK4_cover.png');
	});

	it('filtre par marque ou modèle et retire les marques vides', () => {
		expect(filterCatalog(catalog, '').length).toBe(2);
		const mini = filterCatalog(catalog, 'mini');
		expect(mini.length).toBe(1);
		expect(mini[0].vendor).toBe('Prusa');
		expect(mini[0].models.map((m) => m.model)).toEqual(['MINI']);
		// Recherche par marque.
		expect(filterCatalog(catalog, 'anet').length).toBe(1);
		// Aucun résultat.
		expect(filterCatalog(catalog, 'zzz').length).toBe(0);
	});

	it('bascule la sélection d’un modèle de façon immuable', () => {
		const empty = new Set<string>();
		const one = toggleModel(empty, catalog[1].models[0]);
		expect(empty.size).toBe(0); // inchangé
		expect(isSelected(one, catalog[1].models[0])).toBe(true);
		const off = toggleModel(one, catalog[1].models[0]);
		expect(isSelected(off, catalog[1].models[0])).toBe(false);
	});

	it('compte et bascule une marque entière', () => {
		const prusa = catalog[1];
		let sel = new Set<string>();
		expect(vendorSelectedCount(sel, prusa)).toBe(0);
		expect(vendorAllSelected(sel, prusa)).toBe(false);

		sel = toggleVendorAll(sel, prusa); // tout cocher
		expect(vendorSelectedCount(sel, prusa)).toBe(2);
		expect(vendorAllSelected(sel, prusa)).toBe(true);

		// Un seul modèle sélectionné → « tout sélectionner » complète le reste.
		const partial = toggleModel(new Set<string>(), prusa.models[0]);
		const filled = toggleVendorAll(partial, prusa);
		expect(vendorAllSelected(filled, prusa)).toBe(true);

		sel = toggleVendorAll(sel, prusa); // tout décocher
		expect(vendorSelectedCount(sel, prusa)).toBe(0);
	});

	it('aplatit les modèles sélectionnés pour la confirmation', () => {
		let sel = new Set<string>();
		sel = toggleModel(sel, catalog[0].models[0]); // Anet A8
		sel = toggleModel(sel, catalog[1].models[0]); // Prusa MK4
		const picks = selectedModels(catalog, sel);
		expect(picks.map((m) => m.default_machine_preset_id).sort()).toEqual(['p1', 'p2']);
	});
});
