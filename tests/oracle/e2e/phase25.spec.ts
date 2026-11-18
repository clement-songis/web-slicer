// Phase 25 — manques UX restants de la phase 24 : « Trancher tout » et aperçu
// mémorisé par plateau, page Filaments de l'assistant, menu objet en
// sous-menus, descriptions du menu Calibration, chargement de l'aperçu sans
// reconstructions redondantes.
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, fixture, importModel } from '../lib/app';

const CALIBRATION = /^Calibration$/;

async function objectMenu(page: Page) {
	await page.getByTestId('object-row').first().click({ button: 'right' });
	const menu = page.getByRole('menu');
	await expect(menu).toBeVisible();
	return menu;
}

test('menu objet : sous-menus titrés, toutes les actions visibles sans défilement', async ({
	page
}) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	const menu = await objectMenu(page);
	await expect(menu).toHaveAttribute('data-layout', 'sections');
	for (const id of ['object', 'instances', 'placement', 'mirror', 'parts', 'mesh', 'split', 'export'])
		await expect(page.getByTestId(`menu-section-section.${id}`)).toBeVisible();
	// 41 actions + 2 d'unités, chacune à un clic après le clic droit.
	await expect(menu.getByRole('menuitem')).toHaveCount(43);
	const fits = await menu.evaluate((el) => ({
		scroll: el.scrollHeight > el.clientHeight + 1,
		bottom: el.getBoundingClientRect().bottom <= window.innerHeight
	}));
	expect(fits).toEqual({ scroll: false, bottom: true });
	// Une action rangée dans un sous-menu reste exécutable directement.
	await page
		.getByTestId('menu-section-section.instances')
		.getByRole('menuitem', { name: /add instance|ajouter une instance/i })
		.click();
	await expect(menu).toHaveCount(0);
});

test('menu Calibration : description sous chaque entrée, libellés de parité inchangés', async ({
	page
}) => {
	await bootstrap(page);
	await page.getByRole('button', { name: CALIBRATION }).first().click();
	const menu = page.getByRole('menu');
	await expect(menu.getByTestId('menu-hint')).toHaveCount(14);
	// Nom accessible = libellé exact (la description est hors du nom).
	await expect(page.getByRole('menuitem', { name: 'Flow rate (pass 1)', exact: true })).toBeVisible();
	await expect(page.getByRole('menuitem', { name: /^Pressure advance$/ })).toBeVisible();
	await expect(page.getByRole('menuitem', { name: 'Flow ratio', exact: true })).toHaveAccessibleDescription(
		/YOLO/
	);
});

test.describe('libellés français', () => {
	test.use({ locale: 'fr-FR' });
	test('Calibration : passes du ratio de débit nommées comme telles', async ({ page }) => {
		await page.addInitScript(() => localStorage.setItem('PARAGLIDE_LOCALE', 'fr'));
		await bootstrap(page);
		await page.getByRole('button', { name: CALIBRATION }).first().click();
		await expect(page.getByRole('menuitem', { name: 'Ratio de débit : passe 1', exact: true })).toBeVisible();
		await expect(page.getByRole('menuitem', { name: 'Ratio de débit (assistant)', exact: true })).toBeVisible();
	});
});

test('trancher tout : un aperçu par plateau, mémorisé au changement d’onglet', async ({ page }) => {
	test.setTimeout(300_000);
	await bootstrap(page);
	await importModel(page, 'cube20');
	await page.getByLabel('Configuration').getByRole('button', { name: 'Objets', exact: true }).click();
	await page.getByTestId('plate-tab').first().locator('xpath=../..').getByRole('button', { name: 'Ajouter un plateau', exact: true }).click();
	await page.getByTestId('plate-tab').last().getByRole('button').first().click();
	await importModel(page, 'cylinder');
	await page.getByTestId('plate-tab').first().locator('xpath=../..').getByRole('button', { name: 'Ajouter un plateau', exact: true }).click(); // 3ᵉ plateau, vide
	const tabs = page.getByTestId('plate-tab');
	await expect(tabs).toHaveCount(3);
	await tabs.nth(1).getByRole('button').first().click();

	const metaOf: string[] = [];
	let slicePosts = 0;
	page.on('request', (r) => {
		const m = /\/api\/gcodes\/([^/]+)\/preview\/meta$/.exec(r.url());
		if (m) metaOf.push(m[1]);
		if (r.method() === 'POST' && /\/slice$/.test(r.url())) slicePosts++;
	});
	await page.getByTestId('slice-menu-button').click();
	await expect(page.getByTestId('slice-all')).toBeEnabled();
	await page.getByTestId('slice-all').click();
	await expect(page.getByTestId('preview-view')).toHaveAttribute('data-layer-count', /^[1-9]\d*$/, {
		timeout: 240_000
	});
	// Les deux plateaux non vides finissent tranchés (le vide est sauté).
	await page.getByLabel('Configuration').getByRole('button', { name: 'Objets', exact: true }).click();
	await expect(tabs.nth(0)).toHaveAttribute('data-sliced', 'true', { timeout: 240_000 });
	await expect(tabs.nth(1)).toHaveAttribute('data-sliced', 'true');
	await expect(tabs.nth(2)).toHaveAttribute('data-sliced', 'false');
	await expect(page.getByTestId('editor-message')).toContainText(/2 \/ 2 plateaux tranchés/);
	expect(slicePosts).toBe(1);
	const plate2 = metaOf.at(-1);

	// Plateau 1 : son propre G-code, sans nouveau tranchage.
	await tabs.nth(0).getByRole('button').first().click();
	await expect.poll(() => metaOf.at(-1)).not.toBe(plate2);
	await expect(page.getByTestId('preview-canvas')).toBeVisible({ timeout: 60_000 });
	const plate1 = metaOf.at(-1);
	// Retour au plateau 2 : le résultat mémorisé revient.
	await tabs.nth(1).getByRole('button').first().click();
	await expect.poll(() => metaOf.at(-1)).toBe(plate2);
	// Plateau vide : pas d'aperçu d'un autre plateau.
	await tabs.nth(2).getByRole('button').first().click();
	await expect(page.getByTestId('preview-canvas')).toHaveCount(0);
	await expect(page.getByTestId('export-gcode-button')).toBeDisabled();
	expect(slicePosts).toBe(1);
	expect(plate1).not.toBe(plate2);
});

test('assistant : filaments proposés pour l’imprimante cochée, défaut des nouveaux projets', async ({
	page
}) => {
	test.setTimeout(180_000);
	await page.goto('/register');
	await page.getByTestId('auth-email').fill(`p25-${Date.now()}@harness.local`);
	await page.getByTestId('auth-password').fill('harness-password-123');
	await page.getByTestId('auth-submit').click();
	await expect(page).toHaveURL(/\/setup$/);
	const panel = page.getByTestId('wizard-filaments');
	await expect(panel).toContainText(/Cochez une imprimante/);
	await page.getByPlaceholder('Rechercher une imprimante…').fill('P1S');
	await page.getByRole('button', { name: /P1S/ }).first().click();
	const items = panel.getByTestId('wizard-filament');
	await expect(items.first()).toBeVisible({ timeout: 30_000 });
	// Défaut de la P1S (default_filament_profile) pré-coché.
	const basic = panel.locator('[data-testid=wizard-filament][data-name="Bambu PLA Basic @BBL X1C"]');
	await expect(basic.getByRole('checkbox')).toBeChecked();
	// L'utilisateur choisit un autre filament à la place.
	await basic.getByRole('checkbox').uncheck();
	await panel.getByTestId('wizard-filament-search').fill('PETG HF');
	const petg = panel.locator('[data-testid=wizard-filament][data-name="Bambu PETG HF @BBL X1C"]');
	await petg.getByRole('checkbox').check();
	await expect(panel.getByTestId('wizard-filament-count')).toHaveText(/^1 \/ \d+$/);
	await page.getByRole('button', { name: 'Confirmer' }).click();
	await expect(page).toHaveURL(/\/library$/);
	const prefs = (await (await page.request.get('/api/me/preferences')).json()) as {
		wizardFilaments?: string[];
	};
	expect(prefs.wizardFilaments).toEqual(['Bambu PETG HF @BBL X1C']);
	await page.getByTestId('new-project').click();
	await page.waitForURL(/\/projects\//, { timeout: 60_000 });
	await expect(page.getByTestId('preset-select-filament')).toContainText('Bambu PETG HF', {
		timeout: 30_000
	});
});

test('aperçu : tranches téléchargées une seule fois, rendu logiciel en lignes, page réactive', async ({
	page
}) => {
	test.setTimeout(300_000);
	await bootstrap(page);
	await importModel(page, 'cube20');
	await importModel(page, 'cylinder');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.getByTestId('add-filament-slot').click();
	await page.getByLabel('Configuration').getByRole('button', { name: 'Objets', exact: true }).click();
	await page.getByLabel('Extrudeur de cylinder.stl').selectOption('2');
	const layers: string[] = [];
	page.on('request', (r) => {
		const m = /\/preview\/layers\?(.+)$/.exec(r.url());
		if (m) layers.push(m[1]);
	});
	await page.getByTestId('slice-button').click();
	await expect(page.getByLabel("Coloration de l'aperçu")).toBeVisible({ timeout: 240_000 });
	await page.waitForTimeout(3000); // piste moteur (processed/meta) appliquée entre-temps
	expect(layers.length).toBeGreaterThan(0);
	expect(layers.length).toBe(new Set(layers).size);
	// Chromium sans GPU (SwiftShader) : « Auto » rend en lignes (volumes au choix).
	await expect(page.getByTestId('preview-canvas')).toHaveAttribute('data-render-mode', 'lines');
	await page.getByLabel("Coloration de l'aperçu").selectOption('filament');
	await expect(page.getByTestId('legend-entry')).toHaveCount(2);
	await expect(page.getByTestId('preview-render-auto')).toHaveAttribute('data-tip', /logiciel/);
});

void fixture;
