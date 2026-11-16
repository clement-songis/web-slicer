// Éditeur : outils de maillage serveur (réparation admesh, simplification
// quadrique libslic3r). Parcours UI réels contre la pile : menu contextuel de
// l'objet → endpoint `/api/models/{id}/{repair,simplify}` → rechargement du
// maillage, transformations conservées, persistance après rechargement.
import { expect, test, type Page, type Request } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

/** STL binaire d'un cube de 20 mm auquel il manque `drop` facettes (trou). */
function brokenCube(drop = 1): Buffer {
	const v = (x: number, y: number, z: number) => [x * 20, y * 20, z * 20];
	const quads = [
		[v(0, 0, 0), v(1, 0, 0), v(1, 1, 0), v(0, 1, 0)],
		[v(0, 0, 1), v(0, 1, 1), v(1, 1, 1), v(1, 0, 1)],
		[v(0, 0, 0), v(0, 0, 1), v(1, 0, 1), v(1, 0, 0)],
		[v(0, 1, 0), v(1, 1, 0), v(1, 1, 1), v(0, 1, 1)],
		[v(0, 0, 0), v(0, 1, 0), v(0, 1, 1), v(0, 0, 1)],
		[v(1, 0, 0), v(1, 0, 1), v(1, 1, 1), v(1, 1, 0)]
	];
	const tris = quads.flatMap(([a, b, c, d]) => [[a, c, b], [a, d, c]]).slice(drop);
	const buf = Buffer.alloc(84 + tris.length * 50);
	buf.writeUInt32LE(tris.length, 80);
	tris.forEach((t, i) => {
		const o = 84 + i * 50 + 12;
		t.forEach((p, k) => p.forEach((c, j) => buf.writeFloatLE(c, o + k * 12 + j * 4)));
	});
	return buf;
}

async function objectMenu(page: Page, row = 0) {
	await page.getByTestId('object-row').nth(row).click({ button: 'right' });
	await expect(page.getByRole('menu')).toBeVisible();
}

/** Transformations des objets de la scène sauvegardée (PUT /api/projects/{id}). */
async function savedTransforms(page: Page): Promise<string> {
	const [req] = await Promise.all([
		page.waitForRequest((r: Request) => r.method() === 'PUT' && /\/api\/projects\/[^/]+$/.test(r.url())),
		page.getByTestId('save-project').click()
	]);
	const out: unknown[] = [];
	const walk = (n: unknown) => {
		if (Array.isArray(n)) return n.forEach(walk);
		if (n && typeof n === 'object') {
			const o = n as Record<string, unknown>;
			if ('position' in o || 'rotation' in o || 'scale' in o) {
				out.push({ id: o.id, position: o.position, rotation: o.rotation, scale: o.scale });
			}
			Object.values(o).forEach(walk);
		}
	};
	walk(req.postDataJSON()?.scene);
	expect(out.length, 'aucune transformation dans la scène sauvegardée').toBeGreaterThan(0);
	// `req.response()` évite la course (réponse arrivée avant l'écoute).
	const res = await req.response();
	expect(res?.status(), 'sauvegarde du projet').toBeLessThan(300);
	return JSON.stringify(out);
}

test('réparation : maillage troué réparé par le serveur et rechargé @F:editor.mesh-repair', async ({ page }) => {
	await bootstrap(page);
	const rows = page.getByTestId('object-row');
	await page.getByTestId('import-input').setInputFiles({ name: 'holed.stl', mimeType: 'model/stl', buffer: brokenCube(1) });
	await expect(rows).toHaveCount(1, { timeout: 60_000 });
	// L'objet n'est réparable qu'une fois converti côté serveur.
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	const before = await savedTransforms(page);

	await objectMenu(page);
	const [repairRes] = await Promise.all([
		page.waitForResponse((r) => /\/api\/models\/[^/]+\/repair$/.test(r.url()), { timeout: 60_000 }),
		page.getByRole('menuitem', { name: /fix model|réparer/i }).click()
	]);
	expect(repairRes.status(), await repairRes.text()).toBe(200);
	const report = await repairRes.json();
	const msg = page.getByTestId('editor-message');
	await expect(msg).toContainText('Maillage réparé', { timeout: 30_000 });
	await expect(msg).toContainText(`${report.triangles} triangles`);
	// Le trou (1 facette retirée) a été comblé et le maillage rechargé est étanche.
	expect(report.watertight).toBe(true);
	await expect(msg).not.toContainText('aucune correction');
	expect(await savedTransforms(page)).toBe(before);
});

test('simplification : outil câblé à l’endpoint, maillage remplacé et persistant @F:editor.mesh-simplify', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cylinder');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	// Deux instances du même modèle : le maillage simplifié doit couvrir les deux.
	await objectMenu(page);
	await page.getByRole('menuitem', { name: /add instance|ajouter une instance/i }).click();
	await expect(page.getByTestId('object-row')).toHaveCount(2);
	const before = await savedTransforms(page);

	await objectMenu(page);
	await page.getByRole('menuitem', { name: /simplif/i }).click();
	const tool = page.getByTestId('simplify-tool');
	await expect(tool).toBeVisible();
	await expect(tool).toContainText('Maillage actuel : 256 triangles');
	await tool.getByTestId('simplify-mode-ratio').check();
	await tool.getByTestId('simplify-ratio').fill('50');
	const [res] = await Promise.all([
		page.waitForResponse((r) => /\/api\/models\/[^/]+\/simplify$/.test(r.url()), { timeout: 60_000 }),
		tool.getByTestId('simplify-apply').click()
	]);
	expect(res.status(), await res.text()).toBe(200);
	const report = await res.json();
	expect(res.request().postDataJSON()).toEqual({ mode: 'ratio', decimate_ratio: 50 });
	expect(report.triangles_before).toBe(256);
	expect(report.triangles_after).toBeLessThan(256);
	const msg = page.getByTestId('editor-message');
	await expect(msg).toContainText(`Maillage simplifié : 256 → ${report.triangles_after} triangles`, { timeout: 30_000 });
	await expect(tool).toContainText(`Maillage actuel : ${report.triangles_after} triangles`);
	// L'autre instance porte aussi le maillage simplifié (même modèle serveur).
	await page.getByTestId('object-row').nth(1).click();
	await expect(tool).toContainText(`Maillage actuel : ${report.triangles_after} triangles`);
	expect(await savedTransforms(page)).toBe(before);

	// Le maillage serveur est remplacé : il reste simplifié après rechargement.
	await page.reload();
	await expect(page.getByTestId('object-row')).toHaveCount(2, { timeout: 60_000 });
	await objectMenu(page);
	await page.getByRole('menuitem', { name: /simplif/i }).click();
	await expect(page.getByTestId('simplify-tool')).toContainText(`Maillage actuel : ${report.triangles_after} triangles`, { timeout: 30_000 });
});

test('simplification : erreur serveur affichée clairement @F:editor.mesh-simplify-error', async ({ page }) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.route(/\/api\/models\/[^/]+\/simplify$/, (route) =>
		route.fulfill({ status: 503, contentType: 'application/json', body: JSON.stringify({ error: 'moteur indisponible', message: 'moteur indisponible' }) })
	);
	await objectMenu(page);
	await page.getByRole('menuitem', { name: /simplif/i }).click();
	await page.getByTestId('simplify-apply').click();
	await expect(page.getByTestId('editor-message')).toContainText('Simplification du maillage impossible');
	await expect(page.getByTestId('simplify-apply')).toBeEnabled();
});
