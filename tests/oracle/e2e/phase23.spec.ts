// Phase 23 — préférences serveur (GET/PUT /api/me/preferences : lecture au
// changement de session, écriture différée, cache localStorage, migration
// unique) et répartition des filaments par buse avant le tranchage (Orca
// FilamentMapDialog / try_pop_up_before_slice, préférence pop_up_filament_map_dialog).
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel, uniqueEmail } from '../lib/app';

const FILE = /^(file|fichier)$/i;

async function openPreferences(page: Page) {
	await page.getByRole('button', { name: FILE }).first().click();
	await page.getByRole('menuitem', { name: /pr[ée]f[ée]rences/i }).first().click();
	const dialog = page.getByTestId('preferences-dialog');
	await expect(dialog).toBeVisible();
	return dialog;
}

async function serverPrefs(page: Page): Promise<Record<string, unknown>> {
	const res = await page.request.get('/api/me/preferences');
	expect(res.status()).toBe(200);
	return (await res.json()) as Record<string, unknown>;
}

test('préférences enregistrées sur le serveur, relues sans cache local', async ({ page }) => {
	await bootstrap(page);
	// Compte neuf, aucune préférence locale à migrer : le serveur reste vide.
	expect(await serverPrefs(page)).toEqual({});
	const dialog = await openPreferences(page);
	await dialog.getByTestId('pref-tab-camera').click();
	const put = page.waitForRequest((r) => r.url().endsWith('/api/me/preferences') && r.method() === 'PUT');
	await dialog.getByTestId('pref-leftDrag').selectOption('pan');
	await dialog.getByTestId('pref-tab-behaviour').click();
	await dialog.getByTestId('pref-filament-map-popup').check();
	await dialog.getByTestId('preferences-close').click();
	// Écriture différée : un seul PUT pour la rafale.
	const body = (await put).postDataJSON() as Record<string, unknown>;
	expect(body).toMatchObject({ filamentMapPopup: true, camera: { leftDrag: 'pan' } });
	await expect.poll(async () => (await serverPrefs(page)).filamentMapPopup).toBe(true);
	// Clé historique toujours écrite (lecteurs existants).
	expect(await page.evaluate(() => localStorage.getItem('web-slicer.preferences'))).toContain('"leftDrag":"pan"');

	// Cache local effacé : tout revient du serveur.
	await page.evaluate(() => localStorage.clear());
	await page.reload();
	await expect(page.getByTestId('scene-canvas')).toBeVisible();
	const d2 = await openPreferences(page);
	await d2.getByTestId('pref-tab-camera').click();
	await expect(d2.getByTestId('pref-leftDrag')).toHaveValue('pan');
	await d2.getByTestId('pref-tab-behaviour').click();
	await expect(d2.getByTestId('pref-filament-map-popup')).toBeChecked();
});

test('migration unique des préférences locales vers un compte sans préférences', async ({ browser }) => {
	const ctx = await browser.newContext();
	// Préférences d'avant la persistance serveur (clé historique seulement).
	await ctx.addInitScript(() => {
		if (sessionStorage.getItem('seeded')) return;
		sessionStorage.setItem('seeded', '1');
		localStorage.setItem('web-slicer.preferences', JSON.stringify({ currency: 'CHF', units: 'in' }));
	});
	const page = await ctx.newPage();
	await bootstrap(page);
	await expect.poll(async () => (await serverPrefs(page)).currency, { timeout: 15_000 }).toBe('CHF');
	expect((await serverPrefs(page)).units).toBe('in');
	await ctx.close();
});

test('tranchage H2D : fenêtre de répartition des filaments par buse', async ({ page }) => {
	const res = await page.request.post('/api/_harness/bootstrap', {
		data: {
			email: uniqueEmail(),
			password: 'harness-password-123',
			printer: 'Bambu Lab H2D 0.4 nozzle',
			filament: 'Bambu PLA Basic @BBL H2D',
			process: '0.20mm Standard @BBL H2D'
		}
	});
	expect(res.status(), await res.text()).toBe(200);
	const { project_id } = await res.json();
	await page.goto('/projects/' + project_id);
	await expect(page.getByTestId('scene-canvas')).toBeVisible();
	await page.getByTestId('add-filament-slot').click();
	await importModel(page, 'cube20');

	// Préférence inactive (défaut Orca) : pas de fenêtre.
	let slices = 0;
	page.on('request', (r) => {
		if (/\/api\/projects\/[^/]+\/slice$/.test(r.url()) && r.method() === 'POST') slices++;
	});

	const prefs = await openPreferences(page);
	await prefs.getByTestId('pref-tab-behaviour').click();
	await prefs.getByTestId('pref-filament-map-popup').check();
	await prefs.getByTestId('preferences-close').click();

	await page.getByTestId('slice-button').click();
	let dialog = page.getByTestId('filament-map-dialog');
	await expect(dialog).toBeVisible({ timeout: 20_000 });
	await expect(dialog.getByTestId('filament-map-kind-auto')).toHaveAttribute('aria-checked', 'true');
	// Annuler : aucun tranchage.
	await dialog.getByTestId('filament-map-cancel').click();
	await expect(dialog).toHaveCount(0);
	await page.waitForTimeout(500);
	expect(slices).toBe(0);

	await page.getByTestId('slice-button').click();
	dialog = page.getByTestId('filament-map-dialog');
	await dialog.getByTestId('filament-map-kind-manual').click();
	// Seul le filament du plateau est listé, sur la buse gauche ; clic → droite.
	await expect(dialog.getByTestId('filament-map-nozzle-1').getByTestId('filament-map-chip-1')).toBeVisible();
	await expect(dialog.getByTestId('filament-map-chip-2')).toHaveCount(0);
	await dialog.getByTestId('filament-map-chip-1').click();
	await expect(dialog.getByTestId('filament-map-nozzle-2').getByTestId('filament-map-chip-1')).toBeVisible();
	const slice = page.waitForRequest((r) => /\/api\/projects\/[^/]+\/slice$/.test(r.url()) && r.method() === 'POST');
	await dialog.getByTestId('filament-map-confirm').click();
	await slice;
	await expect(dialog).toHaveCount(0);
	// Choix écrit sur le plateau, enregistré avant le tranchage.
	await expect
		.poll(async () => JSON.stringify((await (await page.request.get('/api/projects/' + project_id)).json()).scene), {
			timeout: 20_000
		})
		.toMatch(/"filament_map_mode":"Manual"[\s\S]*"filament_map":\["2","1"\]|"filament_map":\["2","1"\][\s\S]*"filament_map_mode":"Manual"/);

	// « Ne plus demander » : la préférence se désactive.
	await expect(page.getByTestId('slice-button')).toBeEnabled({ timeout: 170_000 });
	await page.getByTestId('slice-button').click();
	dialog = page.getByTestId('filament-map-dialog');
	await expect(dialog.getByTestId('filament-map-kind-manual')).toHaveAttribute('aria-checked', 'true');
	await dialog.getByTestId('filament-map-stop-asking').check();
	await dialog.getByTestId('filament-map-confirm').click();
	await expect(dialog).toHaveCount(0);
	await expect.poll(async () => (await serverPrefs(page)).filamentMapPopup, { timeout: 15_000 }).toBe(false);
});
