// Éditeur : slots filament et duplication de plateau, persistés par la sauvegarde projet.
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel, oracleCfg } from '../lib/app';

const MODEL = oracleCfg.param_model;

/** Sauvegarde manuelle : attend la fin du PUT /api/projects/{id} (sauvegarde asynchrone). */
async function saveProject(page: Page, projectId: string) {
	const [res] = await Promise.all([
		page.waitForResponse(
			(r) => r.request().method() === 'PUT' && new URL(r.url()).pathname === `/api/projects/${projectId}`
		),
		page.getByTestId('save-project').click()
	]);
	expect(res.status(), `PUT /api/projects/${projectId} → ${res.status()}`).toBe(200);
	await expect(page.getByTestId('save-project')).toBeEnabled();
}

test('slots filament : ajout et retrait persistés après rechargement @F:editor.filament-slots', async ({ page }) => {
	const projectId = await bootstrap(page);
	const slots = page.getByTestId('filament-slot');
	await expect(slots).toHaveCount(1);

	await page.getByTestId('add-filament-slot').click();
	await expect(slots).toHaveCount(2);
	await expect(slots.nth(0)).toHaveAttribute('data-slot', '0');
	await expect(slots.nth(1)).toHaveAttribute('data-slot', '1');
	// Le slot ajouté devient le slot actif.
	await expect(page.locator('[data-testid="filament-slot"][data-active="true"]')).toHaveAttribute('data-slot', '1');

	await saveProject(page, projectId);
	await page.reload();
	await expect(page.getByTestId('scene-canvas')).toBeVisible();
	await expect(slots).toHaveCount(2);

	await page.locator('[data-testid="filament-slot"][data-slot="1"]').getByTestId('remove-filament-slot').click();
	await expect(slots).toHaveCount(1);
	await expect(slots.first()).toHaveAttribute('data-active', 'true');

	await saveProject(page, projectId);
	await page.reload();
	await expect(page.getByTestId('scene-canvas')).toBeVisible();
	await expect(slots).toHaveCount(1);
});

test('Édition › Dupliquer le plateau courant : copie active avec ses objets @F:editor.plate-duplicate', async ({ page }) => {
	const projectId = await bootstrap(page);
	await importModel(page, MODEL);
	const rows = page.getByTestId('object-row');
	await expect(rows).toHaveCount(1);
	// L'objet est listé avant la fin de l'upload : la duplication refuse les objets sans modèle serveur.
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });

	await page.getByRole('button', { name: 'Edit', exact: true }).click();
	await page.getByRole('menuitem', { name: 'Duplicate Current Plate' }).click();

	// La barre de plateaux vit dans l'onglet « Objets » de la colonne de configuration.
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	const sourceTab = page.getByRole('button', { name: 'Plateau 1 (1)', exact: true });
	const copyTab = page.getByRole('button', { name: 'Plateau 1 (copie) (1)', exact: true });
	await expect(page.getByRole('button', { name: /^Supprimer Plateau / })).toHaveCount(2);
	await expect(sourceTab).toBeVisible();
	await expect(copyTab).toBeVisible();
	// Plateau actif = la copie (bordure d'accent sur son conteneur).
	await expect(copyTab.locator('..')).toHaveClass(/border-accent/);

	// La liste d'objets n'affiche que le plateau actif : 1 ligne par plateau, 2 au total.
	await expect(rows).toHaveCount(1);
	await sourceTab.click();
	await expect(sourceTab.locator('..')).toHaveClass(/border-accent/);
	await expect(rows).toHaveCount(1);

	await saveProject(page, projectId);
	await page.reload();
	await expect(page.getByTestId('scene-canvas')).toBeVisible();
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	await expect(page.getByRole('button', { name: /^Supprimer Plateau / })).toHaveCount(2);
	await expect(page.getByRole('button', { name: 'Plateau 1 (1)', exact: true })).toBeVisible();
	await expect(page.getByRole('button', { name: 'Plateau 1 (copie) (1)', exact: true })).toBeVisible();
});
