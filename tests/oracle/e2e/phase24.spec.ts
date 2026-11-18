// Phase 24 — audit d'utilisation réelle : corrections d'interface vérifiées en
// navigateur réel (menus de la barre visibles, 3MF multi-filament ouvert avec
// ses slots, couleur des filaments ajoutés et pastilles de peinture, carte de
// résultat de calibration, plateau dans l'aperçu, lot de préréglages, envoi
// vers une imprimante Bambu Lab).
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, fixture, importModel, uniqueEmail } from '../lib/app';

const FILE = /^(file|fichier)$/i;

async function bootstrapWith(page: Page, printer: string, filament: string, process: string) {
	const res = await page.request.post('/api/_harness/bootstrap', {
		data: {
			email: uniqueEmail(),
			password: 'harness-password-123',
			printer,
			filament,
			process
		}
	});
	expect(res.status(), await res.text()).toBe(200);
	const { project_id } = (await res.json()) as { project_id: string };
	return project_id;
}

test('menus de la barre : le menu déroulant est réellement visible (non rogné)', async ({
	page
}) => {
	await bootstrap(page);
	await page.getByRole('button', { name: FILE }).first().click();
	const item = page.getByRole('menuitem').first();
	await expect(item).toBeVisible();
	// Le point central de la première entrée appartient bien au menu (et non à
	// l'arrière-plan) : un conteneur overflow-hidden le rendait invisible.
	const onTop = await item.evaluate((el) => {
		const r = el.getBoundingClientRect();
		const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
		return !!hit && (el === hit || el.contains(hit));
	});
	expect(onTop).toBe(true);
});

test('ouvrir un 3MF Bambu multi-filament : un slot par filament, extrudeurs conservés', async ({
	page
}) => {
	await bootstrapWith(
		page,
		'Bambu Lab H2D 0.4 nozzle',
		'Bambu PLA Basic @BBL H2D',
		'0.20mm Standard @BBL H2D'
	);
	await page.goto('/library');
	await page.locator('input[type=file]').first().setInputFiles(fixture('scenes/bambu_h2d.3mf'));
	await page.waitForURL(/\/projects\//, { timeout: 60_000 });
	await expect(page.getByTestId('object-row')).toHaveCount(2, {
		timeout: 60_000
	});
	await expect(page.getByTestId('filament-slot')).toHaveCount(2, {
		timeout: 30_000
	});
	await expect(page.getByTestId('filament-swatch').nth(1)).toHaveAttribute(
		'data-colour',
		'#ff0000'
	);
	await expect(page.getByLabel('Extrudeur de cylinder.stl')).toHaveValue('2');
});

test('filament ajouté : couleur distincte (palette Orca), pastilles de peinture multi-matériaux', async ({
	page
}) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await page.getByTestId('add-filament-slot').click();
	await expect(page.getByTestId('filament-swatch').nth(1)).toHaveAttribute(
		'data-colour',
		'#00c1ae'
	);
	await page.getByTestId('object-row').first().click();
	await page.getByTestId('tool-mm-paint').click();
	const second = page.getByTestId('paint-filament-2');
	await expect(second).toBeVisible();
	await second.click();
	await expect(second).toHaveAttribute('aria-checked', 'true');
	await expect(page.getByTestId('paint-filament-1')).toHaveAttribute('aria-checked', 'false');
});

test('projet de calibration PA : hauteur mesurée → K, historique prérempli, appliqué au filament', async ({
	page
}) => {
	const first = await bootstrap(page);
	await page
		.getByRole('button', { name: /^Calibration$/ })
		.first()
		.click();
	await page
		.getByRole('menuitem', { name: /^Pressure advance$/ })
		.first()
		.click();
	await page.getByTestId('calib-confirm').click();
	await page.waitForURL((u) => !String(u).includes(first), { timeout: 30_000 });
	const card = page.getByTestId('calib-result');
	await expect(card).toBeVisible({ timeout: 30_000 });
	await card.getByTestId('calib-result-input').fill('17');
	await expect(card.getByTestId('calib-result-k')).toHaveText('K = 0.034');
	await card.getByTestId('calib-result-record').click();
	const history = page.getByTestId('pa-history-dialog');
	await expect(history.getByTestId('pa-k')).toHaveValue('0.034');
	await expect(history.getByTestId('pa-method')).toHaveValue('pa_tower');
	await history.getByTestId('pa-add').click();
	await expect(history.getByTestId('pa-row')).toHaveCount(1);
	await page.keyboard.press('Escape');
	await card.getByTestId('calib-result-apply').click();
	await expect(page.getByTestId('editor-message')).toContainText('K = 0.034');
	await expect(page.getByTestId('settings-dialog')).toBeVisible();
});

test('aperçu : le plateau est affiché sous les parcours', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await page.getByTestId('slice-button').click();
	await expect(page.getByTestId('preview-view')).toHaveAttribute('data-layer-count', /^[1-9]\d*$/, {
		timeout: 170_000
	});
	// Jamais « tranchez la scène » une fois le tranchage terminé (au plus « Chargement… »).
	await expect(page.getByText(/tranchez la scène pour l/)).toHaveCount(0);
	await expect(page.getByTestId('preview-scene')).toHaveAttribute(
		'data-preview-bed',
		/^\d+(\.\d+)?x\d+(\.\d+)?$/
	);
});

test('lot de préréglages : libellé français et import depuis le dialogue', async ({ page }) => {
	await bootstrap(page);
	await page.getByRole('button', { name: FILE }).first().click();
	await page.getByRole('menuitem', { name: /^(Lot de préréglages|Preset Bundle)$/ }).click();
	await expect(page.getByTestId('export-preset-bundle')).toHaveText('Exporter les préréglages');
	const [chooser] = await Promise.all([
		page.waitForEvent('filechooser'),
		page.getByTestId('import-preset-bundle').click()
	]);
	expect(chooser).toBeTruthy();
});

test('envoyer vers une Bambu Lab sans adresse : réseau Bambu non pris en charge expliqué', async ({
	page
}) => {
	const id = await bootstrapWith(
		page,
		'Bambu Lab P1S 0.4 nozzle',
		'Bambu PLA Basic @BBL X1C',
		'0.20mm Standard @BBL X1C'
	);
	await page.goto('/projects/' + id);
	await expect(page.getByTestId('scene-canvas')).toBeVisible();
	await importModel(page, 'cube20');
	await page.getByTestId('slice-button').click();
	await expect(page.getByTestId('preview-view')).toHaveAttribute('data-layer-count', /^[1-9]\d*$/, {
		timeout: 170_000
	});
	await page.getByTestId('send-to-printer').click();
	await expect(page.getByTestId('send-connection-hint')).toContainText('réseau Bambu');
	await expect(page.getByTestId('send-host-settings')).toHaveAttribute('href', /\/printers$/);
});
