// Menu Calibration › Température (Plater::calib_temp d'OrcaSlicer) : crée un
// projet « tour de température » dérivé des presets actifs, le tranche, et
// vérifie dans le G-code la descente de température par paliers de 5 °C
// (M104 S230 … S190), comme le CalibMode::Calib_Temp_Tower d'Orca.
import { expect, test } from '@playwright/test';
import { bootstrap, downloadGcode, sliceInUi } from '../lib/app';

test('Calibration › Température : tour tranchée, M104 S230…S190 par paliers @F:editor.calibration-temp', async ({ page }) => {
	test.setTimeout(420_000);
	const sourceId = await bootstrap(page);

	await page.getByRole('button', { name: 'Calibration', exact: true }).click();
	await page.getByRole('menuitem', { name: 'Temperature' }).click();
	const dialog = page.getByTestId('calib-dialog');
	await expect(dialog).toBeVisible();
	await dialog.getByTestId('calib-start').fill('230');
	await dialog.getByTestId('calib-end').fill('190');
	await dialog.getByTestId('calib-confirm').click();

	// Nouveau projet (autre id) avec la tour restaurée comme objet unique.
	await expect(page).not.toHaveURL(new RegExp(sourceId), { timeout: 180_000 });
	await expect(page.getByTestId('object-row')).toHaveCount(1, { timeout: 180_000 });
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 120_000 });

	await sliceInUi(page);
	const gcode = await downloadGcode(page);

	// Températures posées au changement de couche (bloc de calibration).
	const temps = [...gcode.matchAll(/^M104 S(\d+)/gm)].map((m) => Number(m[1]));
	const distinct = [...new Set(temps)];
	expect(distinct[0], 'première consigne').toBe(230);
	expect(distinct.at(-1), 'dernière consigne').toBe(190);
	expect(distinct).toEqual([230, 225, 220, 215, 210, 205, 200, 195, 190]);
	for (let i = 1; i < temps.length; i++) expect(temps[i]).toBeLessThanOrEqual(temps[i - 1]);
	// Aucun placeholder laissé brut dans les commandes (le bloc de config en fin
	// de fichier recopie, lui, la macro source : on ne regarde que les lignes M104).
	expect(gcode).not.toMatch(/^M104 S\{/m);
});
