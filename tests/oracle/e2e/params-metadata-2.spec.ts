// Paramètres « métadonnée / préréglages », lot params-metadata-2 (suite de params-metadata.spec.ts).
// Chemin cible : tests/oracle/e2e/params-metadata-2.spec.ts
// Oracle = code Orca (ORCA_SRC 2.4.1), cité à chaque test :
//  - Preset::is_compatible_with_printer / PresetCollection::update_compatible : un preset dont
//    compatible_printers est vide est filtré par compatible_printers_condition évaluée sur la
//    config de l'imprimante ÉDITÉE (printer_notes compris) ; les presets incompatibles ne sont
//    pas proposés (Preset.cpp, PresetComboBoxes).
//  - PresetBundle::full_fff_config → 3MF Metadata/project_settings.config (bbs_3mf.cpp
//    _add_project_config_file_to_archive, ConfigBase::save_to_json) : printer_notes, filament_notes,
//    inherits_group = [process, filaments…, imprimante] (preset utilisateur → parent système).
//  - GCode.cpp:5636 (append_full_config) : la ligne « ; extruder_colour = » du G-code reprend
//    filament_colour, quelle que soit la valeur saisie pour extruder_colour.
//  - Export G-code de la GUI (Plater → GCodeThumbnails::export_thumbnails_to_file) : un bloc
//    « ; <tag> begin WxH LEN » par entrée de `thumbnails` (WxH/FORMAT), tag selon le format
//    (PNG → thumbnail, JPG → thumbnail_JPG, QOI → thumbnail_QOI).
// CONTRAT: aucun sélecteur nouveau (bootstrap, import, slice, export, preset-select-*, open-settings-*,
//          settings-tab-*, data-param, settings-apply). API utilisées en plus de §1 : POST /api/presets
//          (création d'un preset utilisateur, comme « Enregistrer sous » d'Orca) et
//          GET /api/projects/<id>/export/3mf (cible du lien « Export Generic 3MF… » de l'UI).
import { expect, test, type Page } from '@playwright/test';
import { inflateRawSync } from 'node:zlib';
import {
	bootstrap, downloadGcode, importModel, openParam, oracleCfg, setControl, sliceInUi
} from '../lib/app';
import { configBlock } from '../lib/gcode';

/** Entrées d'une archive ZIP (3MF), sans dépendance. */
function unzip(buf: Buffer): Record<string, Buffer> {
	let eocd = buf.length - 22;
	while (eocd >= 0 && buf.readUInt32LE(eocd) !== 0x06054b50) eocd--;
	const out: Record<string, Buffer> = {};
	let p = buf.readUInt32LE(eocd + 16);
	for (let i = 0, n = buf.readUInt16LE(eocd + 10); i < n; i++) {
		const method = buf.readUInt16LE(p + 10), size = buf.readUInt32LE(p + 20);
		const nl = buf.readUInt16LE(p + 28), el = buf.readUInt16LE(p + 30), cl = buf.readUInt16LE(p + 32);
		const off = buf.readUInt32LE(p + 42);
		const name = buf.toString('utf8', p + 46, p + 46 + nl);
		const start = off + 30 + buf.readUInt16LE(off + 26) + buf.readUInt16LE(off + 28);
		const data = buf.subarray(start, start + size);
		out[name] = method === 8 ? inflateRawSync(data) : Buffer.from(data);
		p += 46 + nl + el + cl;
	}
	return out;
}

/** project_settings.config du 3MF exporté pour le projet ouvert dans `page`. */
async function projectSettings(page: Page): Promise<Record<string, unknown>> {
	const id = /\/projects\/([^/?#]+)/.exec(page.url())![1];
	const res = await page.request.get(`/api/projects/${id}/export/3mf`);
	expect(res.status()).toBe(200);
	const entries = unzip(Buffer.from(await res.body()));
	return JSON.parse(entries['Metadata/project_settings.config'].toString('utf8'));
}

async function presetId(page: Page, kind: string, name: string): Promise<string> {
	const res = await page.request.get(`/api/presets?kind=${kind}`);
	expect(res.status()).toBe(200);
	const list = (await res.json()) as { id: string; name: string }[];
	const hit = list.find((p) => p.name === name);
	expect(hit, `preset ${kind} « ${name} »`).toBeTruthy();
	return hit!.id;
}

/** Preset utilisateur dérivé d'un preset système (Orca : « Enregistrer sous », inherits = parent). */
async function userPreset(page: Page, kind: string, parent: string, name: string, values: Record<string, unknown>) {
	const res = await page.request.post('/api/presets', {
		data: { kind, name, inherits: await presetId(page, kind, parent), values }
	});
	expect(res.status(), await res.text()).toBe(201);
}

const offered = (page: Page, scope: string) => async () =>
	page.getByTestId(`preset-select-${scope}`).locator('option:not([disabled])').allTextContents();

async function saveSetting(page: Page, scope: 'printer' | 'filament' | 'print', tab: string, key: string, value: string) {
	const control = await openParam(page, scope, tab, key);
	// Valeurs /resolved en chargement : champs grisés (fieldset disabled) jusqu'à la fin.
	await expect(control, `${key} éditable`).toBeEnabled({ timeout: 30_000 });
	expect(await setControl(control, value), `${key} éditable`).toBe(true);
	await page.getByTestId('settings-apply').click();
	await expect(page.getByTestId('settings-dialog')).toBeHidden({ timeout: 20_000 });
}

test('preset utilisateur dont la condition exclut l’imprimante : absent des listes, comme Orca @F:param.compatible_printers_condition @F:param.compatible_printers', async ({ page }) => {
	await bootstrap(page);
	// compatible_printers vide (sinon la liste prime, Preset::is_compatible_with_printer) + condition fausse.
	await userPreset(page, 'process', oracleCfg.process, 'PM2 process incompatible', {
		compatible_printers: [], compatible_printers_condition: 'printer_model=="PM2 imprimante inexistante"'
	});
	await userPreset(page, 'process', oracleCfg.process, 'PM2 process compatible', {
		compatible_printers: [], compatible_printers_condition: 'nozzle_diameter[0]==0.4'
	});
	await page.reload();
	await expect.poll(offered(page, 'print'), { timeout: 20_000 }).toContainEqual(expect.stringContaining('PM2 process compatible'));
	expect((await offered(page, 'print')()).filter((t) => t.includes('PM2 process incompatible')), 'preset incompatible proposé').toEqual([]);
});

test('notes imprimante : une condition filament sur printer_notes suit la note éditée @F:param.printer_notes', async ({ page }) => {
	await bootstrap(page);
	await userPreset(page, 'filament', oracleCfg.filament, 'PM2 PLA marqué', {
		compatible_printers: [], compatible_printers_condition: 'printer_notes=~/.*PM2_MARK.*/'
	});
	await page.reload();
	await expect.poll(offered(page, 'filament'), { timeout: 20_000 }).toContainEqual(expect.stringContaining(oracleCfg.filament));
	expect((await offered(page, 'filament')()).filter((t) => t.includes('PM2 PLA marqué')), 'filament proposé sans la note').toEqual([]);
	await saveSetting(page, 'printer', 'Notes', 'printer_notes', 'atelier PM2_MARK');
	await expect.poll(offered(page, 'filament'), { timeout: 20_000 }).toContainEqual(expect.stringContaining('PM2 PLA marqué'));
	// …et la note fait partie de la config projet (full_fff_config → project_settings.config).
	expect((await projectSettings(page)).printer_notes).toBe('atelier PM2_MARK');
});

test('notes filament : enregistrées dans la config projet du 3MF @F:param.filament_notes', async ({ page }) => {
	await bootstrap(page);
	await saveSetting(page, 'filament', 'Notes', 'filament_notes', 'PM2 sécher 4 h');
	expect((await projectSettings(page)).filament_notes).toEqual(['PM2 sécher 4 h']);
});

test('preset utilisateur dérivé : inherits_group du 3MF = parents système, inherits effacé @F:param.inherits', async ({ page }) => {
	await bootstrap(page);
	await saveSetting(page, 'printer', 'Notes', 'printer_notes', 'PM2 dérivé');
	const s = await projectSettings(page);
	// [process, filament, imprimante] : seuls les presets utilisateur ont un parent (presets système : « »).
	expect(s.inherits_group).toEqual(['', '', oracleCfg.printer]);
	expect(s.inherits, 'inherits effacé par full_fff_config').toBeUndefined();
	expect(s.compatible_printers, 'compatible_printers effacé par full_fff_config').toBeUndefined();
});

test('extruder_colour : le G-code reprend filament_colour comme Orca @F:param.extruder_colour', async ({ page }) => {
	await bootstrap(page);
	// extruder_colour n'est pas éditable dans Orca (ligne commentée, Tab.cpp:5191) : la couleur
	// vient du filament. L'imprimante garde son extruder_colour (Voron : #FCE94F).
	await userPreset(page, 'filament', oracleCfg.filament, 'PM2 PLA rouge', { filament_colour: ['#FF0000'] });
	await page.reload();
	await importModel(page, oracleCfg.param_model);
	const select = page.getByTestId('preset-select-filament');
	await expect.poll(offered(page, 'filament'), { timeout: 20_000 }).toContainEqual(expect.stringContaining('PM2 PLA rouge'));
	const label = (await offered(page, 'filament')()).find((t) => t.includes('PM2 PLA rouge'))!;
	await select.selectOption({ label });
	await expect(select.locator('option:checked')).toContainText('PM2 PLA rouge');
	await sliceInUi(page);
	const cfg = configBlock(await downloadGcode(page));
	expect(cfg.filament_colour).toBe('#FF0000');
	expect(cfg.extruder_colour, 'extruder_colour réécrit depuis filament_colour (GCode.cpp:5636)').toBe('#FF0000');
});

for (const [format, tag, tags] of [
	['PNG', 'thumbnail', '@F:param.thumbnails'],
	['QOI', 'thumbnail_QOI', '@F:param.thumbnails @F:param.thumbnails_format']
] as const) {
	test(`vignette ${format} dans le G-code exporté depuis l'UI ${tags}`, async ({ page }) => {
		await bootstrap(page);
		await importModel(page, oracleCfg.param_model);
		await saveSetting(page, 'printer', 'Basic information', 'thumbnails', `96x96/${format}`);
		await sliceInUi(page);
		const gcode = await downloadGcode(page);
		const cfg = configBlock(gcode);
		expect(cfg.thumbnails).toBe(`96x96/${format}`);
		// Tab.cpp:4512 : thumbnails_format suit le format de la 1re entrée de thumbnails ; clé absente
		// du G-code (hors PrintConfig statique) mais écrite dans la config projet du 3MF.
		expect((await projectSettings(page)).thumbnails_format).toBe(format);
		const begins = gcode.split('\n').filter((l) => /^; thumbnail(_\w+)? begin /.test(l));
		expect(begins, 'un bloc par entrée de thumbnails').toHaveLength(1);
		const m = /^; (\S+) begin (\d+)x(\d+) (\d+)$/.exec(begins[0])!;
		expect([m[1], m[2], m[3]]).toEqual([tag, '96', '96']);
		// Placé comme GCode::_do_export : après l'en-tête, avant les largeurs d'extrusion.
		expect(gcode.indexOf('; HEADER_BLOCK_END')).toBeLessThan(gcode.indexOf(begins[0]));
		expect(gcode.indexOf(begins[0])).toBeLessThan(gcode.indexOf('; external perimeters extrusion width'));
		const start = gcode.indexOf(begins[0]) + begins[0].length + 1;
		const rows = gcode.slice(start, gcode.indexOf(`; ${tag} end`, start)).trimEnd().split('\n');
		expect(rows.every((l) => l.startsWith('; ') && l.length <= 80), 'lignes base64 de 78 caractères').toBe(true);
		const body = rows.map((l) => l.slice(2)).join('');
		expect(body.length, 'longueur base64 annoncée').toBe(Number(m[4]));
		const bytes = Buffer.from(body, 'base64');
		if (format === 'PNG') {
			expect(bytes.subarray(0, 4).toString('latin1')).toBe('\x89PNG');
			expect([bytes.readUInt32BE(16), bytes.readUInt32BE(20)]).toEqual([96, 96]);
		} else {
			expect(bytes.subarray(0, 4).toString('latin1')).toBe('qoif');
			expect([bytes.readUInt32BE(4), bytes.readUInt32BE(8)]).toEqual([96, 96]);
		}
	});
}
