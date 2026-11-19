// Phase 29 — validation en direct (POST /validate, Print::validate d'Orca) :
// notifications traduites, objet fautif surligné, lien vers l'option ; cause des
// échecs de conversion ; frictions de la 4e passe d'audit (n° de filament).
import { expect, test } from '@playwright/test';
import { writeFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { bootstrap, importModel } from '../lib/app';

test.use({ locale: 'fr-FR' });

test('objet par objet : collision signalée en direct, objet surligné, « Disposer » corrige @F:editor.plate-settings', async ({ page }) => {
	test.setTimeout(240_000);
	await bootstrap(page);
	await importModel(page, 'cube20');
	await importModel(page, 'cylinder');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	// Scène valide : aucune notification.
	await page.waitForTimeout(2500);
	await expect(page.getByTestId('validation-notice')).toHaveCount(0);
	await page.getByTestId('plate-setting-print_sequence').selectOption('by object');
	const notice = page.locator('[data-testid=validation-notice][data-level=error]');
	// Message d'Orca « %1% is too close to others… » traduit par son catalogue.
	await expect(notice.first()).toContainText('cylinder.stl est trop proche des autres', {
		timeout: 60_000
	});
	await expect(page.getByTestId('validation-panel')).toHaveAttribute('data-errors', '1');
	await expect(page.getByTestId('validation-panel')).toContainText('tranchage impossible');
	// Objet fautif : lueur dans la vue, pastille dans la liste, bouton Trancher signalé.
	await expect(page.getByTestId('scene-canvas')).toHaveAttribute('data-issue-objects', /:error/);
	await expect(page.locator('[data-testid=object-issue][data-level=error]')).toHaveCount(1);
	await expect(page.getByTestId('slice-button')).toHaveAttribute('data-blocked', 'true');
	// « Sélectionner » : l'objet désigné par Orca devient la sélection.
	await notice.first().getByTestId('validation-goto-object').click();
	await expect(page.getByTestId('object-row').filter({ hasText: 'cylinder.stl' })).toHaveClass(/bg-primary/);
	await notice.first().getByTestId('validation-arrange').click();
	await expect(page.getByTestId('validation-notice')).toHaveCount(0, { timeout: 60_000 });
	await expect(page.getByTestId('object-issue')).toHaveCount(0);
	await expect(page.getByTestId('slice-button')).not.toHaveAttribute('data-blocked', 'true');
});

test('réglage invalide : erreur en direct, lien « Régler » vers l’option surlignée @F:core.settings-ui', async ({ page }) => {
	test.setTimeout(240_000);
	await bootstrap(page);
	await importModel(page, 'cube20');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.getByTestId('open-settings-print').click();
	const input = page.locator('[data-testid=settings-dialog] [data-param=layer_height] input').first();
	await input.fill('0.6');
	await input.press('Tab');
	await page.keyboard.press('Escape');
	await expect(page.getByTestId('settings-dialog')).toHaveCount(0);
	// Brouillon non enregistré : validé tel quel (Orca valide le preset édité).
	const notice = page.locator('[data-testid=validation-notice][data-opt=layer_height]');
	await expect(notice).toContainText('La hauteur de la couche ne peut pas dépasser le diamètre de la buse', {
		timeout: 60_000
	});
	await expect(notice.getByTestId('validation-goto-option')).toContainText('Hauteur de couche');
	await notice.getByTestId('validation-goto-option').click();
	const dialog = page.getByTestId('settings-dialog');
	await expect(dialog).toBeVisible();
	await expect(dialog.locator('[data-param=layer_height][data-focused=true]')).toBeVisible();
	// Correction : l'erreur disparaît sans trancher.
	const again = dialog.locator('[data-param=layer_height] input').first();
	await again.fill('0.2');
	await again.press('Tab');
	await page.keyboard.press('Escape');
	await expect(page.getByTestId('validation-notice')).toHaveCount(0, { timeout: 60_000 });
});

test('conversion : la cause du moteur est affichée (notification, puis liste à la réouverture) @F:core.import', async ({ page }) => {
	test.setTimeout(180_000);
	await bootstrap(page);
	const dir = mkdtempSync(join(tmpdir(), 'p29-'));
	const bad = join(dir, 'casse.stl');
	writeFileSync(bad, 'solid casse\nfacet normal 0 0 1\n  outer loop\n');
	await page.getByTestId('import-input').setInputFiles(bad);
	await expect(page.getByRole('alert').filter({ hasText: 'casse.stl' })).toContainText(
		'Le chargement du fichier modèle a échoué',
		{ timeout: 60_000 }
	);
	// Le modèle reste rattaché au projet : à la réouverture, ligne en erreur avec la cause.
	await page.reload();
	const row = page.getByTestId('object-row').filter({ hasText: 'casse.stl' });
	await expect(row.getByTestId('object-issue-text')).toContainText(
		'Conversion impossible : Le chargement du fichier modèle a échoué',
		{ timeout: 60_000 }
	);
	await expect(row.locator('[data-testid=object-issue][data-level=error]')).toBeVisible();
	await expect(page.getByRole('alert').filter({ hasText: 'Conversion impossible de « casse.stl »' })).toBeVisible();
});

test('multi-filament : un objet affiche son n° de filament (pas « auto ») @F:editor.filament-slots', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await page.getByTestId('add-filament-slot').click();
	const select = page.getByLabel('Extrudeur de cube20.stl');
	await expect(select).toHaveValue('1');
	await expect(select.locator('option')).toHaveText(['1', '2']);
	await expect(page.getByTestId('object-extruder-swatch')).toHaveCount(1);
});
