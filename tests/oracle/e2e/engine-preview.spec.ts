// Aperçu alimenté par GCodeProcessor (moteur), chargement par fenêtre :
// - au tranchage, seul le méta (/preview/processed/meta) est demandé : la
//   coloration par type, le temps de couche et le Résumé n'ont pas besoin des
//   mouvements ;
// - une coloration par segment (Vitesse réelle) charge les mouvements
//   (/preview/processed/moves?from=&to=) par blocs de 16 couches moteur, pour
//   les seules couches affichées ;
// - le Résumé affiche le temps simulé (= en-tête du G-code), les légendes
//   Vitesse/Débit réels et temps de couche portent des données moteur.
import { expect, test, type Request } from '@playwright/test';
import { bootstrap, importModel } from '../lib/app';

const CHUNK_LAYERS = 16;

test('aperçu : méta GCodeProcessor au tranchage, mouvements par fenêtre, Résumé, vitesse/débit réels @F:preview.engine', async ({
	page
}) => {
	test.setTimeout(300_000);
	await bootstrap(page);
	await importModel(page, 'cube20');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });

	const moves: { from: number; to: number }[] = [];
	page.on('request', (r: Request) => {
		const u = new URL(r.url());
		if (!/\/preview\/processed\/moves$/.test(u.pathname)) return;
		moves.push({ from: Number(u.searchParams.get('from')), to: Number(u.searchParams.get('to')) });
		// Jamais de chargement complet sans fenêtre.
		expect(u.searchParams.has('from') && u.searchParams.has('to'), 'moves sans from/to').toBe(true);
	});

	// 1. Tranchage : méta seul.
	const metaP = page.waitForResponse((r) => /preview\/processed\/meta/.test(r.url()), { timeout: 200_000 });
	await page.getByTestId('slice-button').click();
	const meta = await metaP;
	expect(meta.status()).toBe(200);
	const m = await meta.json();
	expect(m.version).toBeGreaterThanOrEqual(2);
	expect(m.layers.every((l: { first_line?: number }) => typeof l.first_line === 'number')).toBe(true);
	const summary = page.getByTestId('engine-summary');
	await expect(summary).toBeVisible({ timeout: 60_000 });
	await page.waitForTimeout(1500);
	expect(moves, 'aucun mouvement chargé en coloration par type').toEqual([]);

	// Temps de l'en-tête du G-code = temps simulé affiché.
	const gcodeUrl = meta.url().replace(/preview\/processed\/meta.*/, 'download');
	const gcode = await (await page.request.get(gcodeUrl)).text();
	const hdr = gcode.match(/total estimated time: ([^\n]+)|estimated printing time \(normal mode\) = ([^\n]+)/);
	const shown = (await page.getByTestId('engine-total-time').textContent())?.trim();
	expect(shown, 'temps du Résumé').toBe((hdr?.[1] ?? hdr?.[2])?.trim());
	await expect(page.getByTestId('engine-role-times')).toContainText(/Paroi/);
	await expect(page.getByTestId('engine-extruders')).toBeVisible();

	// 2. Vitesse réelle sur les 10 premières couches : un seul bloc (couches 0–15).
	const slider = page.getByTestId('layer-slider');
	await slider.fill('9');
	await expect(slider).toHaveAttribute('data-current-layer', '9');
	const select = page.getByLabel("Coloration de l'aperçu");
	const firstMoves = page.waitForResponse((r) => /preview\/processed\/moves/.test(r.url()), { timeout: 60_000 });
	await select.selectOption({ label: 'Vitesse réelle' });
	expect((await firstMoves).status()).toBe(200);
	const legend = page.getByTestId('preview-legend-panel');
	await expect(legend).toContainText('mm/s');
	await expect(legend).not.toContainText(/aucune donnée/i);
	await page.waitForTimeout(1000);
	expect(moves.length, `fenêtre basse : ${JSON.stringify(moves)}`).toBe(1);
	expect(moves[0]).toEqual({ from: 0, to: CHUNK_LAYERS - 1 });

	// 3. Couche haute au sommet : les blocs manquants seulement, chacun ≤ 16 couches.
	const top = String(m.layer_count - 1);
	await slider.fill(top);
	await expect(slider).toHaveAttribute('data-current-layer', top);
	const expected = Math.ceil(m.layer_count / CHUNK_LAYERS);
	await expect.poll(() => moves.length, { timeout: 30_000 }).toBe(expected);
	for (const w of moves) expect(w.to - w.from).toBeLessThan(CHUNK_LAYERS);
	expect(new Set(moves.map((w) => w.from)).size, 'aucun bloc demandé deux fois').toBe(moves.length);

	// 4. Débit réel et temps de couche : données moteur, pas de nouveau chargement.
	await select.selectOption({ label: 'Débit réel' });
	await expect(legend).toContainText('mm³/s');
	await expect(legend).not.toContainText(/aucune donnée/i);
	await select.selectOption({ label: 'Temps de couche (linéaire)' });
	await expect(legend).not.toContainText(/aucune donnée/i);
	expect(moves.length).toBe(expected);
});
