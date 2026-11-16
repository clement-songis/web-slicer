import { readFileSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { bootstrap, fixture, FIXTURES, importModel, sliceInUi } from '../lib/app';

const FILE = /^(file|fichier)$/i;
const EDIT = /^(edit|édition)$/i;
const VIEW = /^(view|vue|affichage)$/i;
const HELP = /^(help|aide)$/i;
const IMPORT = /^(import|importer)\b/i;
const EXPORT = /^(export|exporter)\b/i;
// Entrées de test (gelées) ; les fichiers produits vont dans le répertoire temporaire.
const MENU_A = join(FIXTURES, 'menu-a');

async function menu(page: Page, top: RegExp, item: RegExp, sub?: RegExp) {
	await page.getByRole('button', { name: top }).first().click();
	if (sub) await page.getByRole('menu').getByRole('button', { name: sub }).first().click();
	await page.getByRole('menuitem', { name: item }).first().click();
}
async function choose(
	page: Page,
	top: RegExp,
	item: RegExp,
	path: string | string[],
	sub?: RegExp,
) {
	const [chooser] = await Promise.all([
		page.waitForEvent('filechooser'),
		menu(page, top, item, sub),
	]);
	await chooser.setFiles(path);
}
async function ready(page: Page) {
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
}
async function download(page: Page, action: () => Promise<void>, timeout = 60_000) {
	const [dl] = await Promise.all([page.waitForEvent('download', { timeout }), action()]);
	return { name: dl.suggestedFilename(), bytes: readFileSync((await dl.path()) as string) };
}
function zipNames(bytes: Buffer): string[] {
	const end = bytes.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
	expect(end).toBeGreaterThanOrEqual(0);
	const count = bytes.readUInt16LE(end + 10);
	let at = bytes.readUInt32LE(end + 16);
	const names: string[] = [];
	for (let i = 0; i < count; i++) {
		const len = bytes.readUInt16LE(at + 28);
		const extra = bytes.readUInt16LE(at + 30) + bytes.readUInt16LE(at + 32);
		names.push(bytes.subarray(at + 46, at + 46 + len).toString('utf8'));
		at += 46 + len + extra;
	}
	return names;
}
async function selectFirst(page: Page) {
	await page.getByTestId('object-row').first().click();
}

test('aide : raccourcis, conseil du jour et assistant @F:menu.keyboard-shortcuts @F:menu.show-tip-of-the-day @F:menu.setup-wizard', async ({
	page,
}) => {
	await bootstrap(page);
	await menu(page, HELP, /keyboard shortcuts|raccourcis clavier/i);
	const shortcuts = page.getByRole('dialog', { name: /raccourcis clavier/i });
	await expect(shortcuts).toBeVisible();
	await expect(shortcuts).toContainText('Ctrl+Z');
	await shortcuts.getByRole('button', { name: /close|fermer/i }).click();
	await menu(page, HELP, /tip of the day|conseil du jour/i);
	const tip = page.getByTestId('tip-of-the-day');
	await expect(tip).toBeVisible();
	const before = await page.getByTestId('tip-text').textContent();
	await tip.getByRole('button', { name: /suivant|next/i }).click();
	await expect(page.getByTestId('tip-text')).not.toHaveText(before ?? '');
	await tip.getByRole('button', { name: /fermer|close/i }).click();
	await menu(page, HELP, /setup wizard|assistant/i);
	await expect(page).toHaveURL(/\/setup$/);
	await expect(page.getByRole('heading', { name: /sélection de l.imprimante/i })).toBeVisible();
});

test('les sept vues changent la caméra de la scène @F:menu.default-view @F:menu.top @F:menu.bottom @F:menu.front @F:menu.rear @F:menu.leftcameracamera @F:menu.rightcameracamera', async ({
	page,
}) => {
	await bootstrap(page);
	const scene = page.getByTestId('scene-canvas');
	for (const [item, view] of [
		[/^top\b|^dessus\b/i, 'top'],
		[/^bottom\b|^dessous\b/i, 'bottom'],
		[/^front\b|^face\b/i, 'front'],
		[/^rear\b|^arrière\b/i, 'rear'],
		[/^left\b|^gauche\b/i, 'left'],
		[/^right\b|^droite\b/i, 'right'],
		[/default view|vue par défaut/i, 'default'],
	] as const) {
		await menu(page, VIEW, item);
		await expect(scene).toHaveAttribute('data-camera-view', view);
	}
});

test('imports du menu : géométrie, ZIP et configuration @F:menu.import-3mf-stl-step-svg-obj-amf @F:menu.import-3mf-stl-step-svg-obj-amf-2 @F:menu.import-zip-archive @F:menu.import-configs', async ({
	page,
}) => {
	await bootstrap(page);
	await choose(page, FILE, /3mf\/stl/i, fixture('cube20'), IMPORT);
	await expect(page.getByTestId('object-row')).toHaveCount(1);
	await ready(page);
	await choose(page, FILE, /zip archive|archive zip/i, `${MENU_A}/models.zip`, IMPORT);
	await expect(page.getByTestId('object-row')).toHaveCount(2);
	await ready(page);
	await choose(page, FILE, /configs|configurations/i, `${MENU_A}/process.json`, IMPORT);
	await expect(page.getByTestId('editor-message')).toContainText(/1 profil\(s\) importé\(s\)/);
	await expect(page.getByTestId('editor-message')).not.toContainText(/commande non disponible/);
});

test('nouveau projet ouvre une scène vide distincte @F:menu.new-project', async ({ page }) => {
	const before = await bootstrap(page);
	await importModel(page, 'cube20');
	await ready(page);
	await menu(page, FILE, /new project|nouveau projet/i);
	await expect(page).not.toHaveURL(new RegExp(`/projects/${before}$`));
	await expect(page.getByTestId('scene-canvas')).toBeVisible();
	await expect(page.getByTestId('object-row')).toHaveCount(0);
});

test('enregistrer, enregistrer sous et rouvrir un 3MF conservent la scène @F:menu.save-project @F:menu.save-project-2 @F:menu.save-project-as @F:menu.save-project-as-2 @F:menu.open-project @F:menu.open-project-2 @F:menu.export-generic-3mf', async ({
	page,
}) => {
	test.setTimeout(240_000);
	const source = await bootstrap(page);
	await importModel(page, 'cube20');
	await ready(page);
	await menu(page, FILE, /^save project\b|^enregistrer le projet\b(?!.*sous)/i);
	await expect(page.getByTestId('editor-message')).toContainText(/projet sauvegardé/i);
	await page.reload();
	await expect(page.getByTestId('object-row')).toHaveCount(1, { timeout: 60_000 });
	const archive = await download(page, () =>
		menu(page, FILE, /generic 3mf|3mf générique/i, EXPORT),
	);
	expect(archive.name).toMatch(/\.3mf$/);
	expect(zipNames(archive.bytes)).toContain('3D/3dmodel.model');
	const exported = join(tmpdir(), `menu-a-exported-${Date.now()}.3mf`);
	const { writeFileSync } = await import('node:fs');
	writeFileSync(exported, archive.bytes);
	await menu(page, FILE, /save project as|enregistrer le projet sous/i);
	await expect(page).not.toHaveURL(new RegExp(`/projects/${source}$`));
	await expect(page.getByTestId('scene-canvas')).toBeVisible({ timeout: 60_000 });
	await expect(page.getByTestId('object-row')).toHaveCount(1, { timeout: 60_000 });
	const copy = page.url();
	await choose(page, FILE, /open project|ouvrir un projet/i, exported);
	await expect(page).not.toHaveURL(copy, { timeout: 60_000 });
	await expect(page.getByTestId('object-row')).toHaveCount(1, { timeout: 60_000 });
});

test('exports STL fusionné/séparés et bundle de presets @F:menu.export-all-objects-as-one-stl @F:menu.export-all-objects-as-stls @F:menu.export-preset-bundle', async ({
	page,
}) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await importModel(page, 'cylinder');
	await ready(page);
	const one = await download(page, () => menu(page, FILE, /one stl|un seul stl/i, EXPORT));
	expect(one.bytes.readUInt32LE(80)).toBeGreaterThan(12);
	const many = await download(page, () => menu(page, FILE, /as stls|stl séparés/i, EXPORT));
	expect(zipNames(many.bytes).filter((n) => n.endsWith('.stl'))).toHaveLength(2);
	const bundle = await download(page, () =>
		menu(page, FILE, /preset bundle|lot de presets|lot de préréglages/i, EXPORT),
	);
	const names = zipNames(bundle.bytes);
	expect(names.length).toBeGreaterThanOrEqual(3);
	expect(names.every((n) => n.endsWith('.json'))).toBe(true);
	expect(bundle.bytes.toString('utf8')).toContain('layer_height');
});

test('édition : copier/couper/coller/cloner/supprimer et dupliquer le plateau @F:menu.copy @F:menu.paste @F:menu.cut @F:menu.clone-selected @F:menu.delete-selected @F:menu.delete-all @F:menu.duplicate-current-plate', async ({
	page,
}) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await ready(page);
	await selectFirst(page);
	await menu(page, EDIT, /^copy\b|^copier\b/i);
	await menu(page, EDIT, /^paste\b|^coller\b/i);
	await expect(page.getByTestId('object-row')).toHaveCount(2);
	await menu(page, EDIT, /^cut\b|^couper\b/i);
	await expect(page.getByTestId('object-row')).toHaveCount(1);
	await menu(page, EDIT, /^paste\b|^coller\b/i);
	await expect(page.getByTestId('object-row')).toHaveCount(2);
	await menu(page, EDIT, /clone selected|cloner la sélection/i);
	await expect(page.getByTestId('object-row')).toHaveCount(3);
	await menu(page, EDIT, /duplicate current plate|dupliquer le plateau/i);
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	await expect(page.getByTestId('plate-tab')).toHaveCount(2);
	await expect(page.getByTestId('object-row')).toHaveCount(3);
	// Onglet Objets : la ligne contient des listes déroulantes, on clique sur le nom.
	await page.getByTestId('object-name').first().click();
	await menu(page, EDIT, /delete selected|supprimer la sélection/i);
	await expect(page.getByTestId('object-row')).toHaveCount(2);
	await menu(page, EDIT, /delete all|tout supprimer/i);
	await expect(page.getByTestId('object-row')).toHaveCount(0);
});

test('annuler et rétablir depuis le menu restaurent la suppression @F:menu.undo @F:menu.redo', async ({
	page,
}) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await ready(page);
	await page.waitForTimeout(700);
	await selectFirst(page);
	await menu(page, EDIT, /delete selected|supprimer la sélection/i);
	await expect(page.getByTestId('object-row')).toHaveCount(0);
	await page.waitForTimeout(700);
	await menu(page, EDIT, /^undo\b|^annuler\b/i);
	await expect(page.getByTestId('object-row')).toHaveCount(1);
	await menu(page, EDIT, /^redo\b|^rétablir\b/i);
	await expect(page.getByTestId('object-row')).toHaveCount(0);
});

test('exports G-code, plateau(s) et trajectoires OBJ @F:menu.export-g-code @F:menu.export-plate-sliced-file @F:menu.export-all-plate-sliced-file @F:menu.export-toolpaths-as-obj', async ({
	page,
}) => {
	test.setTimeout(500_000);
	await bootstrap(page);
	await importModel(page, 'cube20');
	await ready(page);
	await sliceInUi(page);
	for (const pattern of [
		/export g-code|exporter le g-code/i,
		/export plate sliced|fichier tranché du plateau/i,
	]) {
		const file = await download(page, () => menu(page, FILE, pattern, EXPORT), 60_000);
		expect(file.name).toMatch(/\.gcode$/);
		expect(file.bytes.toString('utf8')).toContain('G1');
	}
	const all = await download(
		page,
		() => menu(page, FILE, /all plate sliced|tous les plateaux/i, EXPORT),
		400_000,
	);
	// .gcode.3mf de libslic3r (Orca Plater::export_gcode_3mf), un G-code par plateau.
	expect(all.name).toMatch(/\.gcode\.3mf$/);
	expect(zipNames(all.bytes)).toContain('Metadata/plate_1.gcode');
	const obj = await download(page, () => menu(page, FILE, /toolpaths|trajectoires/i, EXPORT));
	expect(obj.name).toMatch(/\.obj$/);
	expect(obj.bytes.toString('utf8')).toMatch(/^v /m);
});
