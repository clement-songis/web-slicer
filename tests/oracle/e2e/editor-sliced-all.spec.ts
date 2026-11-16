// Tous les plateaux (parité Orca) : « Exporter tous les plateaux tranchés » télécharge
// le .gcode.3mf de libslic3r (POST /export/sliced, Metadata/plate_<n>.gcode) et
// « Envoyer › Tous les plateaux » tranche puis envoie chaque plateau à l'imprimante
// (POST /printers/{id}/upload-sliced).
import { readFileSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

const MOONRAKER = process.env.MOONRAKER_URL ?? 'http://127.0.0.1:17125';

function zipNames(bytes: Buffer): string[] {
	const end = bytes.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
	expect(end).toBeGreaterThanOrEqual(0);
	const count = bytes.readUInt16LE(end + 10);
	let at = bytes.readUInt32LE(end + 16);
	const names: string[] = [];
	for (let i = 0; i < count; i++) {
		const len = bytes.readUInt16LE(at + 28);
		const extra = bytes.readUInt16LE(at + 30) + bytes.readUInt16LE(at + 32);
		names.push(bytes.subarray(at + 46, at + 46 + len).toString('utf8'));
		at += 46 + len + extra;
	}
	return names;
}

async function twoPlates(page: Page) {
	await importModel(page, 'cube20');
	await importModel(page, 'cylinder');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.getByRole('button', { name: 'Objets', exact: true }).click();
	await page.getByRole('button', { name: 'Ajouter un plateau' }).click();
	await page.getByLabel(/^Plateau de /).nth(1).selectOption({ index: 1 });
	await expect(page.getByTestId('plate-tab')).toHaveCount(2);
}

test('exporter tous les plateaux : .gcode.3mf avec un G-code par plateau @F:menu.export-all-plate-sliced-file', async ({ page }) => {
	test.setTimeout(500_000);
	await bootstrap(page);
	await twoPlates(page);
	await page.getByRole('button', { name: /^(file|fichier)$/i }).first().click();
	await page.getByRole('menu').getByRole('button', { name: /^(export|exporter)\b/i }).first().click();
	const [dl] = await Promise.all([
		page.waitForEvent('download', { timeout: 400_000 }),
		page.getByRole('menuitem', { name: /all plate sliced|tous les plateaux/i }).first().click()
	]);
	expect(dl.suggestedFilename()).toMatch(/\.gcode\.3mf$/);
	const names = zipNames(readFileSync((await dl.path()) as string));
	expect(names).toContain('Metadata/plate_1.gcode');
	expect(names).toContain('Metadata/plate_2.gcode');
});

test('envoyer tous les plateaux à l imprimante @F:printer.send-all', async ({ page }) => {
	test.setTimeout(500_000);
	await bootstrap(page);
	const list = async () =>
		((await (await page.request.get(`${MOONRAKER}/server/files/list?root=gcodes`)).json()).result as { path: string }[]).map(
			(f) => f.path
		);
	const existing = await (await page.request.get('/api/printers')).json();
	const created = await page.request.post('/api/printers', {
		data: { name: 'Simulateur', moonraker_url: MOONRAKER, machine_preset_id: existing[0].machine_preset_id }
	});
	expect(created.status()).toBe(201);
	await page.reload();
	await twoPlates(page);
	const before = new Set(await list());
	// Aucun plateau tranché : l'envoi de tous les plateaux reste possible.
	await page.getByTestId('send-to-printer').click();
	const dialog = page.getByTestId('send-dialog');
	await expect(dialog.getByTestId('send-scope-all')).toHaveAttribute('aria-pressed', 'true');
	await dialog.getByTestId('send-printer-select').selectOption({ index: 0 });
	await dialog.getByTestId('send-confirm').click();
	await expect(dialog.getByTestId('send-success')).toBeVisible({ timeout: 400_000 });
	await expect
		.poll(async () => (await list()).filter((p) => !before.has(p) && /\.gcode(\.3mf)?$/.test(p)).length)
		.toBeGreaterThanOrEqual(1);
});
