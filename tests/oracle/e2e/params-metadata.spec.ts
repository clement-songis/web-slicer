// Paramètres « métadonnée / préréglages » (proposition params-review, à geler par le parent).
// Chemin cible : tests/oracle/e2e/params-metadata.spec.ts
// Ces clés n'ont pas d'effet exécutable propre : Orca les DÉRIVE des presets actifs
// (PresetBundle::full_config, PresetBundle.cpp:4144/4199 ; CLI OrcaSlicer.cpp:2816-3163)
// ou les utilise pour filtrer/choisir des presets (Preset::is_compatible_with_printer,
// PresetBundle::load_selections → default_print_profile/default_filament_profile).
// L'oracle est donc le G-code Orca de base (refs/base.gcode, presets de harness/oracle.json)
// et les profils système, pas une valeur injectée.
// CONTRAT: aucun sélecteur nouveau (bootstrap, import, slice, export, preset-select-*, new-project).
import { expect, test } from '@playwright/test';
import {
	bootstrap, downloadGcode, importModel, oracleCfg, readJsonRef, readRef, sliceInUi, type FlatConfig
} from '../lib/app';
import { configBlock } from '../lib/gcode';

const DERIVED = [
	'print_settings_id', 'printer_settings_id', 'filament_settings_id', 'printer_model',
	'inherits_group', 'filament_ids', 'default_print_profile', 'default_filament_profile', 'printer_variant'
] as const;

test('clés dérivées des presets actifs : identiques au G-code Orca de base @F:param.print_settings_id @F:param.printer_settings_id @F:param.filament_settings_id @F:param.printer_model @F:param.inherits_group @F:param.filament_ids', async ({ page }) => {
	const ref = readRef('base.gcode');
	test.skip(!ref, 'référence Orca absente');
	const expected = configBlock(ref!);
	await bootstrap(page);
	await importModel(page, oracleCfg.param_model);
	await sliceInUi(page);
	const got = configBlock(await downloadGcode(page));
	for (const k of DERIVED) {
		if (expected[k] === undefined) continue;
		expect(got[k], `${k} dérivé des presets (${oracleCfg.printer} / ${oracleCfg.process} / ${oracleCfg.filament})`).toBe(expected[k]);
	}
});

test('nouveau projet : process et filament par défaut de l’imprimante @F:param.default_print_profile @F:param.default_filament_profile', async ({ page }) => {
	const base = readJsonRef<FlatConfig>('base_config.json');
	test.skip(!base, 'base_config.json absent');
	await bootstrap(page);
	await page.goto('/');
	await page.getByTestId('new-project').click();
	await page.waitForURL(/\/projects\//);
	// Les listes de presets se chargent de façon asynchrone : attendre la sélection effective.
	const opt = (scope: string) => async () =>
		(await page.getByTestId(`preset-select-${scope}`).locator('option:checked').textContent())?.trim() ?? '';
	await expect.poll(opt('print'), { timeout: 20_000 }).toContain(String(base!.default_print_profile));
	const fil = base!.default_filament_profile;
	await expect.poll(opt('filament'), { timeout: 20_000 }).toContain(Array.isArray(fil) ? fil[0] : String(fil));
});

test('liste des process filtrée par compatible_printers(_condition) comme Orca @F:param.compatible_printers @F:param.compatible_printers_condition @F:param.compatible_prints @F:param.compatible_prints_condition', async ({ page }) => {
	await bootstrap(page);
	// Imprimante Voron : aucun process système réservé à une autre machine (ex. « @BBL X1C ») ne doit être proposé,
	// et les filaments « @System » génériques restent proposés (compatible_printers vide).
	const options = (scope: string) => page.getByTestId(`preset-select-${scope}`).locator('option:not([disabled])');
	await expect.poll(async () => options('print').count(), { timeout: 20_000 }).toBeGreaterThan(0);
	await expect.poll(async () => options('filament').count(), { timeout: 20_000 }).toBeGreaterThan(0);
	const prints = await options('print').allTextContents();
	expect(prints.some((t) => t.includes(oracleCfg.process)), `${oracleCfg.process} proposé`).toBe(true);
	expect(prints.filter((t) => /@BBL|@Bambu|@Prusa|@Creality/.test(t)), 'process incompatibles proposés').toEqual([]);
	const fils = await options('filament').allTextContents();
	expect(fils.some((t) => t.includes(oracleCfg.filament))).toBe(true);
	expect(fils.filter((t) => /@BBL X1C|@Prusa MK4/.test(t)), 'filaments réservés à une autre machine').toEqual([]);
});
