// Export Draco (Fichier › Export › … DRC, menu objet › Export as … DRC) :
// téléchargement réel, en-tête « DRACO », une entrée par objet dans le ZIP.
import { readFileSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

const FILE = /^(file|fichier)$/i;
const EXPORT = /^(export|exporter)\b/i;

async function fileMenu(page: Page, item: RegExp) {
	await page.getByRole('button', { name: FILE }).first().click();
	await page.getByRole('menu').getByRole('button', { name: EXPORT }).first().click();
	await page.getByRole('menuitem', { name: item }).first().click();
}
async function objectMenu(page: Page, row: number, item: RegExp) {
	await page.getByTestId('object-row').nth(row).click({ button: 'right' });
	const entry = page.getByRole('menuitem', { name: item }).first();
	await entry.scrollIntoViewIfNeeded();
	await entry.click();
}
async function download(page: Page, action: () => Promise<void>) {
	const [dl] = await Promise.all([page.waitForEvent('download', { timeout: 60_000 }), action()]);
	return { name: dl.suggestedFilename(), bytes: readFileSync((await dl.path()) as string) };
}
/** Entrées d'une archive ZIP (répertoire central) : nom + offset local. */
function zipEntries(bytes: Buffer): { name: string; data: Buffer }[] {
	const end = bytes.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
	expect(end).toBeGreaterThanOrEqual(0);
	const count = bytes.readUInt16LE(end + 10);
	let at = bytes.readUInt32LE(end + 16);
	const out: { name: string; data: Buffer }[] = [];
	for (let i = 0; i < count; i++) {
		const size = bytes.readUInt32LE(at + 20);
		const len = bytes.readUInt16LE(at + 28);
		const extra = bytes.readUInt16LE(at + 30) + bytes.readUInt16LE(at + 32);
		const local = bytes.readUInt32LE(at + 42);
		const name = bytes.subarray(at + 46, at + 46 + len).toString('utf8');
		const start = local + 30 + bytes.readUInt16LE(local + 26) + bytes.readUInt16LE(local + 28);
		out.push({ name, data: bytes.subarray(start, start + size) });
		at += 46 + len + extra;
	}
	return out;
}
const isDraco = (b: Buffer) => b.subarray(0, 5).toString('latin1') === 'DRACO';

async function twoObjects(page: Page) {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await importModel(page, 'cylinder');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
}

test('Fichier › Export all objects as one DRC / as DRCs @F:menu.export-all-objects-as-one-drc @F:menu.export-all-objects-as-drcs', async ({
	page,
}) => {
	await twoObjects(page);
	const one = await download(page, () => fileMenu(page, /one drc|un seul drc/i));
	expect(one.name).toMatch(/\.drc$/);
	expect(isDraco(one.bytes)).toBe(true);
	const many = await download(page, () => fileMenu(page, /as drcs|drc séparés/i));
	expect(many.name).toMatch(/\.zip$/);
	const entries = zipEntries(many.bytes);
	expect(entries).toHaveLength(2);
	for (const entry of entries) {
		expect(entry.name).toMatch(/\.drc$/);
		expect(isDraco(entry.data)).toBe(true);
	}
	// Le fichier fusionné contient plus de géométrie qu'un objet seul.
	expect(one.bytes.length).toBeGreaterThan(Math.min(...entries.map((e) => e.data.length)));
});

test('menu objet › Export as one DRC / as DRCs sur la sélection @F:ctx.export-as-one-drc @F:ctx.export-as-drcs', async ({
	page,
}) => {
	await twoObjects(page);
	const one = await download(page, () => objectMenu(page, 1, /export as one drc|exporter en un seul drc/i));
	expect(one.name).toMatch(/\.drc$/);
	expect(isDraco(one.bytes)).toBe(true);
	const many = await download(page, () => objectMenu(page, 1, /export as drcs|exporter en plusieurs drc/i));
	const entries = zipEntries(many.bytes);
	// Seul l'objet ciblé (cylindre) est exporté.
	expect(entries).toHaveLength(1);
	expect(entries[0].name).toMatch(/cylinder.*\.drc$/);
	expect(isDraco(entries[0].data)).toBe(true);
});
