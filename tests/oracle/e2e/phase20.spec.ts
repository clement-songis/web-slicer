// Phase 20 : branchement du lot 16 — OBJ coloré (ObjColorDialog), fichiers joints
// au projet (Auxiliaries), éditeur de ramming, précompilation des shaders de l'aperçu.
import { expect, test, type Page } from '@playwright/test';
import { join } from 'node:path';
import { bootstrap, importModel, openSettings, setMode, sliceInUi, REPO } from '../lib/app';

const OBJ = join(REPO, 'engine', 'tests', 'fixtures', 'obj_colors');
const DEFAULT_RAMMING =
	'120 100 6.6 6.8 7.2 7.6 7.9 8.2 8.7 9.4 9.9 10.0| 0.05 6.6 0.45 6.8 0.95 7.8 1.45 8.3 1.95 9.7 2.45 10 2.95 7.6 3.45 7.6 3.95 7.6 4.45 7.6 4.95 7.6';

const slots = (page: Page) => page.locator('[data-testid="filament-slot"]');

async function savedScene(page: Page, projectId: string) {
	await page.keyboard.press('Control+s');
	await expect
		.poll(async () => JSON.stringify((await (await page.request.get(`/api/projects/${projectId}`)).json()).scene ?? {}), { timeout: 20_000 })
		.toContain('mmu');
	return (await (await page.request.get(`/api/projects/${projectId}`)).json()).scene;
}

test('OBJ + MTL colorés : dialogue des couleurs, nouveaux filaments, peinture MMU @F:ui.obj-colors', async ({ page }) => {
	const projectId = await bootstrap(page);
	const before = await slots(page).count();
	await page.getByTestId('import-input').setInputFiles([join(OBJ, 'cube_mtl.obj'), join(OBJ, 'cube_mtl.mtl')]);
	const dlg = page.getByTestId('obj-color-dialog');
	await expect(dlg).toBeVisible({ timeout: 90_000 });
	await expect(page.getByTestId('object-row')).toHaveCount(1);
	const rows = dlg.getByTestId('obj-color-row');
	await expect(rows).toHaveCount(3);
	// Nombre de couleurs : fusion des plus proches, puis retour à 3.
	await dlg.getByTestId('obj-color-count').fill('2');
	await dlg.getByTestId('obj-color-count').press('Enter');
	await expect(rows).toHaveCount(2);
	await dlg.getByTestId('obj-color-count').fill('3');
	await dlg.getByTestId('obj-color-count').press('Enter');
	await expect(rows).toHaveCount(3);
	// Par défaut (comme Orca) : un nouveau filament par couleur.
	await expect(dlg.getByTestId('obj-color-summary')).toContainText('3 filaments ajoutés');
	// Une couleur sur un filament existant.
	await rows.nth(0).getByTestId('obj-color-target').selectOption('0');
	await expect(dlg.getByTestId('obj-color-summary')).toContainText('2 filaments ajoutés');
	await dlg.getByTestId('obj-color-apply').click();
	await expect(dlg).toHaveCount(0);
	await expect(slots(page)).toHaveCount(before + 2);
	const scene = await savedScene(page, projectId);
	const text = JSON.stringify(scene);
	expect(text).toContain('mmu');
	// Couleurs projet des nouveaux filaments.
	expect(scene.projectConfig?.filament_colour?.filter((c: string) => c).length).toBe(2);
});

test('OBJ à couleurs de sommets : « Importer sans les couleurs » laisse un seul filament @F:ui.obj-colors', async ({ page }) => {
	await bootstrap(page);
	const before = await slots(page).count();
	await page.getByTestId('import-input').setInputFiles(join(OBJ, 'cube_vcolor.obj'));
	const dlg = page.getByTestId('obj-color-dialog');
	await expect(dlg).toBeVisible({ timeout: 90_000 });
	await expect(dlg.getByTestId('obj-color-row')).toHaveCount(2);
	// « Plus proches » : aucun filament ajouté.
	await dlg.getByTestId('obj-color-match').click();
	await expect(dlg.getByTestId('obj-color-summary')).toContainText('0 filament ajouté');
	await dlg.getByTestId('obj-color-skip').click();
	await expect(dlg).toHaveCount(0);
	await expect(slots(page)).toHaveCount(before);
});

test('OBJ sans couleur : pas de dialogue @F:ui.obj-colors', async ({ page }) => {
	await bootstrap(page);
	const obj = 'v 0 0 0\nv 20 0 0\nv 0 20 0\nv 0 0 20\nf 1 3 2\nf 1 2 4\nf 2 3 4\nf 1 4 3\n';
	await page.getByTestId('import-input').setInputFiles({ name: 'tetra.obj', mimeType: 'text/plain', buffer: Buffer.from(obj) });
	await expect(page.getByTestId('object-row')).toHaveCount(1, { timeout: 60_000 });
	await page.waitForTimeout(4000);
	await expect(page.getByTestId('obj-color-dialog')).toHaveCount(0);
});

// PNG 1×1 rouge.
const PNG = Buffer.from(
	'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==',
	'base64'
);

test('fichiers joints au projet : 4 dossiers, vignette, téléversement, suppression @F:ui.auxiliaries', async ({ page }) => {
	const projectId = await bootstrap(page);
	await page.getByRole('button', { name: /^\s*(Projet|Project)\s*$/ }).first().click();
	const panel = page.getByTestId('aux-panel');
	await expect(panel).toBeVisible();
	await expect(panel.getByTestId('aux-folder')).toHaveCount(4);
	await expect(panel.getByTestId('aux-empty')).toBeVisible();
	// Photo du modèle : vignette chargée.
	await panel.getByTestId('aux-input').setInputFiles({ name: 'vue.png', mimeType: 'image/png', buffer: PNG });
	const thumb = panel.getByTestId('aux-thumb');
	await expect(thumb).toHaveCount(1);
	await expect.poll(() => thumb.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(1);
	// Même nom : dédoublonné, pas écrasé.
	await panel.getByTestId('aux-input').setInputFiles({ name: 'vue.png', mimeType: 'image/png', buffer: PNG });
	await expect(panel.getByTestId('aux-file')).toHaveCount(2);
	await expect(panel.locator('[data-testid="aux-file"][data-name="vue (2).png"]')).toBeVisible();
	// Autre dossier.
	await panel.locator('[data-testid="aux-folder"][data-folder="Bill of Materials"]').click();
	await panel.getByTestId('aux-input').setInputFiles({ name: 'pieces.txt', mimeType: 'text/plain', buffer: Buffer.from('vis M3 ×4') });
	await expect(panel.getByTestId('aux-file')).toHaveCount(1);
	await expect(panel.getByTestId('aux-total')).toContainText('3 fichiers');
	const list = await (await page.request.get(`/api/projects/${projectId}/auxiliaries`)).json();
	expect(list.map((e: { path: string }) => e.path).sort()).toEqual(['Bill of Materials/pieces.txt', 'Model Pictures/vue (2).png', 'Model Pictures/vue.png']);
	// Suppression confirmée.
	await panel.locator('[data-testid="aux-folder"][data-folder="Model Pictures"]').click();
	await panel.locator('[data-testid="aux-file"][data-name="vue.png"]').getByTestId('aux-delete').click();
	await panel.getByTestId('aux-delete-confirm').click();
	await expect(panel.getByTestId('aux-file')).toHaveCount(1);
	const after = await (await page.request.get(`/api/projects/${projectId}/auxiliaries`)).json();
	expect(after).toHaveLength(2);
	// Rechargement : la liste vient du serveur.
	await page.reload();
	await page.getByRole('button', { name: /^\s*(Projet|Project)\s*$/ }).first().click();
	await expect(page.getByTestId('aux-total')).toContainText('2 fichiers');
});

async function openRamming(page: Page) {
	const dialog = await openSettings(page, 'filament');
	await setMode(page, 'advanced');
	await dialog.getByTestId('settings-tab-multimaterial').click();
	const row = dialog.locator('[data-param="filament_ramming_parameters"]');
	await expect(row).toBeVisible();
	await row.getByTestId('ramming-open').click();
	const dlg = page.getByTestId('ramming-dialog');
	await expect(dlg).toBeVisible();
	return { dialog, row, dlg };
}

test('ramming : courbe, chaîne exacte conservée, largeur, points et durée @F:ui.ramming', async ({ page }) => {
	await bootstrap(page);
	let { row, dlg } = await openRamming(page);
	// Chaîne d'origine (défaut Orca) : 10 pas de 0,25 s, 11 points dont 10 visibles (≤ 2,5 s).
	await expect(dlg.getByTestId('ramming-step')).toHaveCount(10);
	await expect(dlg.getByTestId('ramming-point')).toHaveCount(6);
	await expect(dlg.getByTestId('ramming-time')).toHaveValue('2500');
	const original = await dlg.getByTestId('ramming-string').textContent();
	expect(original).toBe(DEFAULT_RAMMING);
	// Appliquer sans rien changer : chaîne exacte.
	await dlg.getByTestId('ramming-apply').click();
	await expect(dlg).toHaveCount(0);
	({ row, dlg } = await openRamming(page));
	await expect(dlg.getByTestId('ramming-string')).toHaveText(DEFAULT_RAMMING);
	// Largeur seule : seuls les deux premiers nombres changent.
	await dlg.getByTestId('ramming-width').fill('140');
	await expect(dlg.getByTestId('ramming-string')).toHaveText(DEFAULT_RAMMING.replace(/^120 100/, '140 100'));
	// Point déplacé au clavier (Maj+↑ = +1 mm³/s) : débits recalculés.
	const p = dlg.getByTestId('ramming-point').nth(2);
	await p.focus();
	for (let i = 0; i < 5; i++) await p.press('Shift+ArrowUp');
	await expect(p).toHaveAttribute('aria-valuenow', '12.8');
	const changed = await dlg.getByTestId('ramming-string').textContent();
	expect(changed).toMatch(/^140 100 /);
	expect(changed).toContain('| 0.05 6.6 0.45 6.8 0.95 12.8 ');
	expect(changed).not.toBe(DEFAULT_RAMMING.replace(/^120 100/, '140 100'));
	// Durée 3 s : 12 pas.
	await dlg.getByTestId('ramming-time').fill('3000');
	await dlg.getByTestId('ramming-time').press('Enter');
	await expect(dlg.getByTestId('ramming-step')).toHaveCount(12);
	const final = (await dlg.getByTestId('ramming-string').textContent())!;
	expect(final.split('|')[0].trim().split(/\s+/)).toHaveLength(14);
	await expect(dlg.getByTestId('ramming-volume')).toContainText('mm³');
	await dlg.getByTestId('ramming-apply').click();
	await expect(row.getByTestId('ramming-summary')).toContainText('3 s');
	({ dlg } = await openRamming(page));
	await expect(dlg.getByTestId('ramming-string')).toHaveText(final);
	// Valeurs par défaut d'Orca : chaîne d'origine exacte.
	await dlg.getByTestId('ramming-reset').click();
	await expect(dlg.getByTestId('ramming-string')).toHaveText(DEFAULT_RAMMING);
});

test('aperçu : shaders précompilés avant la première image @F:ui.preview-warmup', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await sliceInUi(page);
	const scene = page.getByTestId('preview-scene');
	await expect(scene).toHaveAttribute('data-preview-ready', 'true', { timeout: 30_000 });
	expect(Number(await scene.getAttribute('data-preview-compile-ms'))).toBeGreaterThanOrEqual(0);
	await expect(page.getByTestId('preview-canvas')).toHaveAttribute('data-visible-segments', /^[1-9]\d*$/);
	// Bascule en lignes : nouvelle précompilation, rendu repris.
	await page.getByTestId('preview-render-lines').click();
	await expect(page.getByTestId('preview-canvas')).toHaveAttribute('data-render-mode', 'lines');
	await expect(scene).toHaveAttribute('data-preview-ready', 'true');
});
