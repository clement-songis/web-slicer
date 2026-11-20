// Couverture des groupes de réglages par type de preset (T100/T101/T102) : le
// layout généré expose bien les 21 pages OrcaSlicer réparties process (8) /
// filament (7) / machine (6), montées par la colonne de configuration.
import { describe, expect, test } from 'vitest';
import { UI_LAYOUT, type PresetKind } from '../../generated/ui-layout';

function pagesOf(kind: PresetKind) {
	return UI_LAYOUT.filter((p) => p.kind === kind).map((p) => p.title);
}

describe('groupes de réglages par type', () => {
	test('process : 8 pages (T100)', () => {
		expect(pagesOf('process')).toEqual([
			'Quality',
			'Strength',
			'Speed',
			'Support',
			'Multimaterial',
			'Others',
			'Frequent',
			'Plate Settings'
		]);
	});

	test('filament : 7 pages (T101)', () => {
		// Ordre d'exécution d'Orca (TabFilament::build puis
		// add_filament_overrides_page appelé après Cooling, Tab.cpp) :
		// Setting Overrides n'est pas première malgré sa définition plus haute.
		expect(pagesOf('filament')).toEqual([
			'Filament',
			'Cooling',
			'Setting Overrides',
			'Advanced',
			'Multimaterial',
			'Dependencies',
			'Notes'
		]);
	});

	test('machine : 6 pages, page_name renommée Extruder (T102)', () => {
		expect(pagesOf('machine')).toEqual([
			'Basic information',
			'Machine G-code',
			'Notes',
			'Motion ability',
			'Multimaterial',
			'Extruder'
		]);
	});

	test('les trois groupes couvrent les 21 pages sans reste', () => {
		expect(pagesOf('process').length + pagesOf('filament').length + pagesOf('machine').length).toBe(
			UI_LAYOUT.length
		);
	});
});
