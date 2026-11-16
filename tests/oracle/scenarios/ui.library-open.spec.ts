// Bibliothèque de projets (/library, parité accueil Orca « Récemment ouvert ») :
// renommer, dupliquer, supprimer (persistés après rechargement), ouvrir un
// modèle 3D comme nouveau projet. Non couvert jusqu'ici par l'oracle.
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, fixture } from '../lib/app';

const cards = (page: Page) => page.getByRole('link', { name: /^Ouvrir / });

async function openLibrary(page: Page) {
	await page.goto('/library');
	await expect(page.getByRole('heading', { name: 'Récemment ouvert' })).toBeVisible();
}

test('bibliothèque : « Ouvrir un projet » avec un modèle 3D crée un projet et ouvre l’éditeur @F:ui.library-open', async ({ page }) => {
	await bootstrap(page);
	await openLibrary(page);
	const input = page.locator('input[type="file"]');
	await input.setInputFiles(fixture('cube20'));
	await expect(page).toHaveURL(/\/projects\/[0-9a-f-]+$/, { timeout: 60_000 });
	await expect(page.getByTestId('object-row')).toHaveCount(1, { timeout: 60_000 });
	await openLibrary(page);
	await expect(cards(page)).toHaveCount(2);
});
