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

test('bibliothèque : renommer, dupliquer, supprimer un projet (persistés) @F:ui.library-crud', async ({ page }) => {
	await bootstrap(page);
	await openLibrary(page);
	await expect(cards(page)).toHaveCount(1);
	const item = page.locator('li').filter({ has: cards(page).first() });

	// Renommer (bouton au survol, validation par Entrée).
	await item.hover();
	await item.getByRole('button', { name: 'Renommer' }).click();
	const field = item.getByRole('textbox');
	await field.fill('Pièce de test');
	await field.press('Enter');
	await expect(page.getByRole('link', { name: 'Ouvrir Pièce de test' })).toBeVisible();

	// Dupliquer : deux projets.
	await page.locator('li').first().hover();
	await page.locator('li').first().getByRole('button', { name: 'Dupliquer' }).click();
	await expect(cards(page)).toHaveCount(2);

	// Persistance après rechargement.
	await openLibrary(page);
	await expect(cards(page)).toHaveCount(2);
	await expect(page.getByRole('link', { name: 'Ouvrir Pièce de test' }).first()).toBeVisible();

	// Supprimer (mode suppression puis corbeille d'une vignette).
	await page.getByRole('button', { name: 'Supprimer', exact: true }).click();
	await page.getByTitle(/^Supprimer /).first().click();
	await expect(cards(page)).toHaveCount(1);
	await openLibrary(page);
	await expect(cards(page)).toHaveCount(1);
});
