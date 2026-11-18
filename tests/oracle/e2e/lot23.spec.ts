// Lot 23 — derniers manques de l'audit Orca : « Parcourir » pour Creality Print
// (CrealityDiscoveryDialog) et Flashforge (Flashforge::discover_printers),
// préférences « Page au démarrage » (default_page) et « Projets récents
// affichés » (max_recent_count).
import { expect, test, type Page } from '@playwright/test';
import { bootstrap } from '../lib/app';

const FILE = /^(file|fichier)$/i;

async function openPreferences(page: Page) {
	await page.getByRole('button', { name: FILE }).first().click();
	await page.getByRole('menuitem', { name: /pr[ée]f[ée]rences/i }).first().click();
	const dialog = page.getByTestId('preferences-dialog');
	await expect(dialog).toBeVisible();
	return dialog;
}

const savedPrefs = (page: Page) =>
	page.waitForResponse(
		(r) => r.url().includes('/api/me/preferences') && r.request().method() === 'PUT',
		{ timeout: 10_000 }
	);

for (const [type, service, none] of [
	['crealityprint', 'creality', /Aucune imprimante Creality/],
	['flashforge', 'flashforge', /Aucune imprimante Flashforge/]
] as const) {
	test(`parcourir le réseau : ${type}`, async ({ page }) => {
		test.setTimeout(120_000);
		await bootstrap(page);
		await page.goto('/printers');
		await page.getByTestId('printer-add').click();
		await page.getByTestId('host-type').selectOption(type);
		const discover = page.waitForResponse(
			(r) => r.url().includes(`/api/printers/discover?service=${service}`),
			{ timeout: 60_000 }
		);
		await page.getByTestId('host-browse').click();
		const results = page.getByTestId('host-browse-results');
		await expect(results).toContainText(type === 'flashforge' ? /Flashforge/ : /Creality/);
		expect((await discover).status()).toBe(200);
		// Recherche réelle depuis le serveur : liste ou message propre au constructeur.
		await expect(results).toContainText(new RegExp(`${none.source}|Utiliser|Choisi`), {
			timeout: 60_000
		});
	});
}

test('préférences : page au démarrage et projets récents affichés', async ({ page }) => {
	const first = await bootstrap(page);
	// Deuxième projet, plus récent.
	const res = await page.request.post('/api/projects', { data: { name: 'Lot 23 récent' } });
	expect(res.ok()).toBe(true);
	const second = ((await res.json()) as { id: string }).id;
	expect(second).not.toBe(first);

	let dialog = await openPreferences(page);
	let saved = savedPrefs(page);
	await dialog.getByTestId('pref-default-page').selectOption('prepare');
	await saved;
	await page.goto('/');
	await expect(page).toHaveURL(new RegExp(`/projects/${second}$`));
	await expect(page.getByTestId('scene-canvas')).toBeVisible();

	dialog = await openPreferences(page);
	saved = savedPrefs(page);
	await dialog.getByTestId('pref-default-page').selectOption('home');
	await dialog.getByTestId('pref-max-recent').fill('1');
	await dialog.getByTestId('pref-max-recent').press('Tab');
	await saved;
	await page.goto('/');
	await expect(page).toHaveURL(/\/library$/);
	const cards = page.getByRole('link', { name: /^Ouvrir / });
	await expect(cards).toHaveCount(1);
	await expect(page.getByTestId('library-hidden')).toContainText('1 projet plus ancien masqué');
	await page.getByTestId('library-show-all').click();
	await expect(cards).toHaveCount(2);
	await expect(page.getByTestId('library-hidden')).toHaveCount(0);
});
