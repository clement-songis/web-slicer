// Menus Fichier / Vue / Application (parité Orca côté web) : export STL depuis
// la scène (un fichier / un par objet), export du G-code de tous les plateaux
// (ZIP), ouverture d'un G-code local dans l'aperçu, projection orthogonale,
// dialogue de préférences.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

const FILE = /^(file|fichier)$/i;
const VIEW = /^(view|vue|affichage)$/i;

/** Ouvre un menu principal puis (optionnel) un sous-menu, et clique l'item. */
async function menu(page: Page, top: RegExp, item: RegExp, sub?: RegExp) {
	await page.getByRole('button', { name: top }).first().click();
	// Sous-menu dépliable en place (nom accessible suivi d'une flèche ▸).
	if (sub) await page.getByRole('menu').getByRole('button', { name: sub }).first().click();
	await page.getByRole('menuitem', { name: item }).first().click();
}

async function download(page: Page, action: () => Promise<void>, timeout = 30_000) {
	const [dl] = await Promise.all([page.waitForEvent('download', { timeout }), action()]);
	return { name: dl.suggestedFilename(), bytes: readFileSync((await dl.path()) as string) };
}

/** Noms des entrées d'une archive ZIP (répertoire central). */
function zipNames(bytes: Buffer): string[] {
	const end = bytes.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
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

async function ready(page: Page) {
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
}

test('export STL : un fichier binaire puis une archive par objet @F:editor.menu-export-stl', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await ready(page);

	const one = await download(page, () =>
		menu(page, FILE, /one stl|un seul stl|un stl/i, /^(export|exporter)\b/i)
	);
	expect(one.name).toMatch(/\.stl$/);
	expect(one.bytes.subarray(0, 5).toString()).not.toBe('solid');
	const triangles = one.bytes.readUInt32LE(80);
	expect(triangles).toBe(12);
	expect(one.bytes.length).toBe(84 + 50 * triangles);
	// Normale de la première facette : unitaire (vraies normales).
	const n = [0, 1, 2].map((k) => one.bytes.readFloatLE(84 + k * 4));
	expect(Math.hypot(n[0], n[1], n[2])).toBeCloseTo(1, 4);

	await importModel(page, 'cylinder');
	await ready(page);
	const many = await download(page, () =>
		menu(page, FILE, /as stls|en stls|plusieurs stl/i, /^(export|exporter)\b/i)
	);
	expect(many.name).toMatch(/\.zip$/);
	expect(many.bytes.subarray(0, 2).toString()).toBe('PK');
	const names = zipNames(many.bytes);
	expect(names).toHaveLength(2);
	expect(names.every((n) => n.endsWith('.stl'))).toBe(true);
});

test('export de tous les plateaux : .gcode.3mf avec un G-code par plateau @F:editor.menu-export-plates', async ({ page }) => {
	test.setTimeout(400_000);
	await bootstrap(page);
	await importModel(page, 'cube20');
	await ready(page);
	await page.getByTestId('toolbar-addplate').click();
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	await expect(page.getByTestId('plate-tab')).toHaveCount(2);
	await page.getByTestId('plate-tab').nth(1).getByRole('button').first().click();
	await importModel(page, 'twoparts');
	await ready(page);

	const archive = await download(
		page,
		() => menu(page, FILE, /all plate sliced|tous les plateaux/i, /^(export|exporter)\b/i),
		360_000
	);
	// .gcode.3mf écrit par libslic3r (Orca Plater::export_gcode_3mf) : un
	// Metadata/plate_<n>.gcode par plateau tranché.
	expect(archive.name).toMatch(/\.gcode\.3mf$/);
	const names = zipNames(archive.bytes);
	expect(names.filter((n) => /^Metadata\/plate_\d+\.gcode$/.test(n)).sort()).toEqual([
		'Metadata/plate_1.gcode',
		'Metadata/plate_2.gcode'
	]);
});

test('ouvrir un G-code local : aperçu sans tranchage @F:editor.menu-open-gcode', async ({ page }) => {
	await bootstrap(page);
	const gcode = fileURLToPath(
		new URL('../../../engine/tests/fixtures/cube_bbl.gcode', import.meta.url)
	);
	const [chooser] = await Promise.all([
		page.waitForEvent('filechooser'),
		menu(page, FILE, /open g-code|ouvrir.*g-code/i)
	]);
	await chooser.setFiles(gcode);
	const preview = page.getByTestId('preview-view');
	await expect(preview).toBeVisible();
	await expect(preview).toHaveAttribute('data-layer-count', /^[1-9]\d*$/, { timeout: 30_000 });
	await expect(page.getByTestId('preview-canvas')).toHaveAttribute(
		'data-visible-segments',
		/^[1-9]\d*$/
	);
});

test('vue orthogonale et préférences @F:editor.menu-view-prefs', async ({ page }) => {
	await bootstrap(page);
	await expect(page.getByTestId('scene-canvas')).toHaveAttribute('data-projection', 'perspective');
	await menu(page, VIEW, /orthogonal|orthogonale/i);
	await expect(page.getByTestId('scene-canvas')).toHaveAttribute('data-projection', 'orthographic');
	await menu(page, VIEW, /perspective/i);
	await expect(page.getByTestId('scene-canvas')).toHaveAttribute('data-projection', 'perspective');

	await menu(page, FILE, /preferences|préférences/i);
	const dialog = page.getByTestId('preferences-dialog');
	await expect(dialog).toBeVisible();
	await dialog.getByTestId('pref-units').selectOption('in');
	await expect
		.poll(() => page.evaluate(() => localStorage.getItem('web-slicer.preferences')))
		.toContain('"units":"in"');
});
