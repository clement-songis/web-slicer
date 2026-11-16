// Bibliothèque : recherche (insensible à la casse et aux accents), tri mémorisé,
// date relative, états vides ; page Imprimantes : état vide avec action, commandes
// d'impression affichées seulement quand l'état est connu.
import { expect, test, type Page } from '@playwright/test';
import { bootstrap } from '../lib/app';

const cards = (page: Page) => page.getByRole('link', { name: /^Ouvrir / });

test('bibliothèque : recherche, tri, aucun résultat @F:ui.library-search', async ({ page }) => {
	await bootstrap(page);
	for (const name of ['Écrou M3', 'bride']) {
		const r = await page.request.post('/api/projects', { data: { name } });
		expect(r.status()).toBeLessThan(300);
	}
	await page.goto('/library');
	await expect(page.getByRole('heading', { name: 'Récemment ouvert' })).toBeVisible();
	const total = await cards(page).count();
	expect(total).toBeGreaterThanOrEqual(3);
	await expect(page.getByTestId('library-count')).toHaveText(String(total));

	await page.getByTestId('library-search').fill('ecrou');
	await expect(cards(page)).toHaveCount(1);
	await expect(cards(page).first()).toHaveAttribute('aria-label', 'Ouvrir Écrou M3');
	await expect(page.getByTestId('library-count')).toHaveText(`1 / ${total}`);

	await page.getByTestId('library-search').fill('zzz');
	await expect(page.getByTestId('library-no-match')).toBeVisible();
	await page.getByRole('button', { name: 'Effacer la recherche' }).click();
	await expect(cards(page)).toHaveCount(total);

	// Tri par nom, mémorisé après rechargement.
	await page.getByTestId('library-sort').selectOption('name');
	const byName = await cards(page).evaluateAll((els) => els.map((e) => e.getAttribute('aria-label')!));
	expect(byName).toEqual([...byName].sort((a, b) => a.localeCompare(b, undefined, { sensitivity: 'base' })));
	await page.reload();
	await expect(page.getByTestId('library-sort')).toHaveValue('name');
	await expect(page.locator('main time').first()).toHaveText("à l'instant");
});

test('imprimantes : commandes d impression masquées sans état, aucune ligne grisée @F:ui.printers-page', async ({ page }) => {
	await bootstrap(page);
	await page.goto('/printers');
	const row = page.getByTestId('printer-row').first();
	await expect(row).toBeVisible();
	await expect(row.getByRole('button', { name: 'Pause' })).toHaveCount(0);
	await expect(row.getByRole('button', { name: 'Tester' })).toHaveCount(0);
	await expect(row).toContainText('Sans URL Moonraker');
	// Suppression de la seule imprimante → état vide avec accès au catalogue.
	await row.getByRole('button', { name: 'Supprimer' }).click();
	await expect(page.getByTestId('printers-empty')).toBeVisible();
	await page.getByTestId('printers-empty').getByRole('button', { name: 'Ouvrir le catalogue' }).click();
	await expect(page.getByRole('dialog', { name: 'Ajouter des imprimantes' })).toBeVisible();
	await page.keyboard.press('Escape');
	await expect(page.getByRole('dialog', { name: 'Ajouter des imprimantes' })).toHaveCount(0);
});
