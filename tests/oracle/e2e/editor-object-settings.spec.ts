// Réglages par objet et peinture, de bout en bout : scène de l'éditeur →
// sauvegarde → POST /projects/{id}/slice → backend (slicing.rs) → moteur
// (ModelObject.config, FacetsAnnotation) → G-code. Attentes géométriques
// (pas de 3MF de référence Orca) :
//   - wall_loops=5 sur un objet sur deux : cet objet a ~2× plus d'extrusions
//     de paroi interne que l'autre (preset : wall_loops=3 → 2 parois internes, 5 → 4) ;
//   - peinture « support » (enforcer) sur un modèle en surplomb, supports en
//     mode manuel (comme Orca : `enable_support=1`, `support_type=normal(manual)` ;
//     sans `enable_support`, Orca ignore les facettes peintes — PrintObject::has_support) :
//     des supports apparaissent, absents sans peinture.
// Il n'existe pas encore d'UI d'édition des réglages par objet ni de pinceau
// pilotable en test : la surcharge et la peinture sont injectées dans le
// document scène (format exact de l'éditeur : `settings`, `painting` 3MF Orca).
import { expect, test, type Page } from '@playwright/test';
import { inflateRawSync } from 'node:zlib';
import { bootstrap, importModel } from '../lib/app';

type SceneObject = {
	id: string;
	modelId: string;
	settings?: Record<string, unknown>;
	painting?: Record<string, Record<string, string>>;
	volumes?: { id: string; type: string; size: number[]; offset: number[]; settings: Record<string, unknown> }[];
};
type Project = { id: string; version: number; scene: { objects: SceneObject[] }; active_presets: unknown };

/** Facette entière peinte « enforcer » au format Orca (cf. painting/facets.ts). */
const ENFORCER_HEX = '4';

async function importAndSave(page: Page, models: string[]): Promise<string> {
	const projectId = await bootstrap(page);
	for (const m of models) {
		await importModel(page, m);
		await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	}
	const [res] = await Promise.all([
		page.waitForResponse((r) => r.request().method() === 'PUT' && /\/api\/projects\/[^/]+$/.test(r.url())),
		page.getByTestId('save-project').click()
	]);
	expect(res.status()).toBeLessThan(300);
	return projectId;
}

async function getProject(page: Page, id: string): Promise<Project> {
	const r = await page.request.get(`/api/projects/${id}`);
	expect(r.status()).toBe(200);
	return r.json();
}

async function putScene(page: Page, p: Project) {
	const r = await page.request.put(`/api/projects/${p.id}`, {
		data: { expected_version: p.version, scene: p.scene, active_presets: p.active_presets }
	});
	expect(r.status(), await r.text()).toBe(200);
}

async function sliceGcode(page: Page, id: string): Promise<string> {
	const r = await page.request.post(`/api/projects/${id}/slice`, { data: { plate_index: 0 } });
	expect(r.status(), await r.text()).toBeLessThan(300);
	const job = ((await r.json()) as { jobs: { id: string }[] }).jobs[0];
	let gcodeId: string | null = null;
	await expect
		.poll(
			async () => {
				const j = (await (await page.request.get(`/api/jobs/${job.id}`)).json()) as { status: string; gcode_id?: string; error?: string };
				if (j.status === 'failed') throw new Error(`tranchage en échec : ${JSON.stringify(j)}`);
				gcodeId = j.gcode_id ?? null;
				return j.status;
			},
			{ timeout: 170_000, intervals: [1000] }
		)
		.toMatch(/^(done|completed|succeeded)$/);
	expect(gcodeId).toBeTruthy();
	const g = await page.request.get(`/api/gcodes/${gcodeId}/download`);
	expect(g.status()).toBe(200);
	return g.text();
}

/** Extrusions de paroi interne par objet (marqueurs « ; printing object … id:N »). */
function innerWallMovesByObject(gcode: string): Map<string, number> {
	const counts = new Map<string, number>();
	let object: string | null = null;
	let type = '';
	for (const line of gcode.split('\n')) {
		const start = /^; printing object .* id:(\d+) copy/.exec(line);
		if (start) { object = start[1]; continue; }
		if (line.startsWith('; stop printing object')) { object = null; continue; }
		if (line.startsWith(';TYPE:')) { type = line.slice(6); continue; }
		if (object !== null && type === 'Inner wall' && /^G1 .*E\d|^G1 .*E\.\d/.test(line))
			counts.set(object, (counts.get(object) ?? 0) + 1);
	}
	return counts;
}

test('réglage par objet : wall_loops=5 sur un seul objet atteint le G-code @F:editor.object-settings', async ({ page }) => {
	const id = await importAndSave(page, ['cube20', 'cube20']);
	const project = await getProject(page, id);
	expect(project.scene.objects).toHaveLength(2);
	project.scene.objects[1].settings = { ...(project.scene.objects[1].settings ?? {}), wall_loops: '5' };
	await putScene(page, project);

	const counts = [...innerWallMovesByObject(await sliceGcode(page, id)).values()].sort((a, b) => a - b);
	expect(counts, 'deux objets attendus dans le G-code').toHaveLength(2);
	// 2 parois internes (preset) contre 4 (surcharge) : rapport ≈ 2.
	expect(counts[1] / counts[0]).toBeGreaterThan(1.6);
	expect(counts[1] / counts[0]).toBeLessThan(2.6);
});

test('peinture de supports : les facettes peintes génèrent des supports @F:editor.paint-supports', async ({ page }) => {
	const id = await importAndSave(page, ['overhang']);
	const manual = { enable_support: '1', support_type: 'normal(manual)' };
	const before = await getProject(page, id);
	before.scene.objects[0].settings = { ...(before.scene.objects[0].settings ?? {}), ...manual };
	await putScene(page, before);
	const without = await sliceGcode(page, id);
	expect(without, 'supports manuels : aucun support sans peinture').not.toMatch(/^;TYPE:Support/m);

	const project = await getProject(page, id);
	const supports: Record<string, string> = {};
	// Toutes les facettes en « enforcer » (indices hors maillage ignorés par le moteur) :
	// seules les zones en surplomb reçoivent effectivement des supports.
	for (let t = 0; t < 2000; t++) supports[String(t)] = ENFORCER_HEX;
	project.scene.objects[0].painting = { supports };
	await putScene(page, project);

	const painted = await sliceGcode(page, id);
	expect(painted, 'supports peints absents du G-code').toMatch(/^;TYPE:Support/m);
});

// --- Analyse géométrique du G-code -----------------------------------------

type Move = { x: number; y: number; z: number; type: string; extrude: boolean; block: number };

/** Déplacements G1 avec rôle (`;TYPE:`) et hauteur courante. */
function moves(gcode: string): Move[] {
	const out: Move[] = [];
	let x = 0, y = 0, z = 0, type = '', block = 0;
	for (const line of gcode.split('\n')) {
		if (line.startsWith(';TYPE:')) { type = line.slice(6); block++; continue; }
		const z0 = /^;Z:([\d.]+)/.exec(line);
		if (z0) { z = Number(z0[1]); continue; }
		if (!/^G[01] /.test(line)) continue;
		const gx = /X(-?[\d.]+)/.exec(line), gy = /Y(-?[\d.]+)/.exec(line);
		if (gx) x = Number(gx[1]);
		if (gy) y = Number(gy[1]);
		out.push({ x, y, z, type, block, extrude: /E\.?\d/.test(line) && !/E-/.test(line) });
	}
	return out;
}

/**
 * Point de départ de chaque bloc de paroi externe (= couture : position juste
 * avant la première extrusion du bloc `;TYPE:Outer wall`).
 */
function outerWallStarts(ms: Move[], filter: (m: Move) => boolean = () => true): Move[] {
	const starts: Move[] = [];
	const seen = new Set<number>();
	for (let i = 1; i < ms.length; i++) {
		const m = ms[i];
		if (m.type !== 'Outer wall' || !m.extrude || seen.has(m.block) || !filter(m)) continue;
		seen.add(m.block);
		starts.push(ms[i - 1]);
	}
	return starts;
}

/** Centre XY de l'objet = milieu de la boîte des parois externes. */
function wallCenter(ms: Move[]): [number, number] {
	const w = ms.filter((m) => m.extrude && m.type === 'Outer wall');
	const xs = w.map((m) => m.x), ys = w.map((m) => m.y);
	return [(Math.min(...xs) + Math.max(...xs)) / 2, (Math.min(...ys) + Math.max(...ys)) / 2];
}

/** Paint par indices de triangle (ordre des facettes du STL de la fixture). */
function paintTriangles(indices: number[], hex = ENFORCER_HEX): Record<string, string> {
	return Object.fromEntries(indices.map((t) => [String(t), hex]));
}

test('supports peints : seulement sous la zone peinte @F:editor.paint-supports-local', async ({ page }) => {
	// overhang = tronc pyramidal inversé (gen_models.py : frustum carré, 4 facettes par
	// côté i dans l'ordre [fond, dessus, flanc, flanc]) ; côté 0 = flanc tourné vers +Y.
	const id = await importAndSave(page, ['overhang']);
	const results: Record<string, number[]> = {};
	for (const [side, tris] of [['+Y', [2, 3]], ['-Y', [10, 11]]] as const) {
		const p = await getProject(page, id);
		p.scene.objects[0].settings = { enable_support: '1', support_type: 'normal(manual)' };
		p.scene.objects[0].painting = { supports: paintTriangles([...tris]) };
		await putScene(page, p);
		const ms = moves(await sliceGcode(page, id));
		const [, cy] = wallCenter(ms);
		const sup = ms.filter((m) => m.extrude && m.type.startsWith('Support'));
		expect(sup.length, `supports attendus sous le flanc ${side}`).toBeGreaterThan(0);
		results[side] = sup.map((m) => m.y - cy);
	}
	// Supports majoritairement sous le flanc peint (les couches de base/interface
	// d'Orca débordent un peu de la projection de la zone peinte).
	const range = (v: number[]) => `[${Math.min(...v).toFixed(1)}, ${Math.max(...v).toFixed(1)}] (${v.length})`;
	const share = (v: number[], sign: number) => v.filter((d) => d * sign > 0).length / v.length;
	expect(share(results['+Y'], 1), `flanc +Y peint : supports en y-cy ${range(results['+Y'])}`).toBeGreaterThan(0.8);
	expect(share(results['-Y'], -1), `flanc -Y peint : supports en y-cy ${range(results['-Y'])}`).toBeGreaterThan(0.8);
});

test('couture peinte : position des coutures sur la face peinte @F:editor.paint-seam', async ({ page }) => {
	// cube20 (gen_models.py box) : facettes 8-9 = face x-, 10-11 = face x+.
	const id = await importAndSave(page, ['cube20']);
	const seamSide: Record<string, number> = {};
	for (const [side, tris] of [['x+', [10, 11]], ['x-', [8, 9]]] as const) {
		const p = await getProject(page, id);
		p.scene.objects[0].painting = { seam: paintTriangles([...tris]) };
		await putScene(page, p);
		const ms = moves(await sliceGcode(page, id));
		const [cx] = wallCenter(ms);
		// Couture = point de départ de chaque boucle de paroi externe (dernier
		// déplacement avant la première extrusion du bloc).
		const starts = outerWallStarts(ms, (m) => m.z > 0.3).map((m) => m.x - cx);
		expect(starts.length).toBeGreaterThan(10);
		seamSide[side] = starts.filter((dx) => (side === 'x+' ? dx > 9 : dx < -9)).length / starts.length;
	}
	expect(seamSide['x+'], 'coutures sur la face x+ peinte').toBeGreaterThan(0.8);
	expect(seamSide['x-'], 'coutures sur la face x- peinte').toBeGreaterThan(0.8);
});

async function addVolumeFromMenu(page: Page, label: RegExp, type: string) {
	await page.getByTestId('object-row').first().click({ button: 'right' });
	await page.getByRole('menuitem', { name: label }).click();
	await expect(page.locator(`[data-testid="object-volume"][data-type="${type}"]`)).toHaveCount(1);
}

async function saveEditor(page: Page) {
	const [res] = await Promise.all([
		page.waitForResponse((r) => r.request().method() === 'PUT' && /\/api\/projects\/[^/]+$/.test(r.url())),
		page.getByTestId('save-project').click()
	]);
	expect(res.status()).toBeLessThan(300);
}

test('volume négatif : ajouté depuis le menu, persistant, creuse l’objet @F:editor.volume-negative', async ({ page }) => {
	const id = await importAndSave(page, ['cube20']);
	const midOuterLoops = (g: string) => {
		// Blocs de paroi externe distincts dans la couche à z = 10 mm.
		return outerWallStarts(moves(g), (m) => Math.abs(m.z - 10) < 0.11).length;
	};
	const before = midOuterLoops(await sliceGcode(page, id));
	await addVolumeFromMenu(page, /add negative volume/i, 'negative');
	await saveEditor(page);
	const p = await getProject(page, id);
	expect(p.scene.objects[0].volumes?.map((v) => [v.type, v.size])).toEqual([['negative', [10, 10, 10]]]);
	// Rechargement : le volume est restauré dans l'éditeur.
	await page.reload();
	await expect(page.getByTestId('object-row')).toHaveCount(1, { timeout: 60_000 });
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.getByTestId('object-row').first().click();
	await expect(page.locator('[data-testid="object-volume"][data-type="negative"]')).toHaveCount(1);
	// Cavité de 10 mm au centre : une boucle de paroi externe de plus à mi-hauteur.
	expect(midOuterLoops(await sliceGcode(page, id))).toBe(before + 1);
});

test('modificateur : ses réglages s’appliquent dans son volume @F:editor.volume-modifier', async ({ page }) => {
	const id = await importAndSave(page, ['cube20']);
	const midSparse = (g: string) => moves(g).filter((m) => Math.abs(m.z - 10) < 0.11 && m.extrude && m.type === 'Sparse infill').length;
	const before = midSparse(await sliceGcode(page, id));
	expect(before).toBeGreaterThan(0);
	await addVolumeFromMenu(page, /add modifier/i, 'modifier');
	await saveEditor(page);
	const p = await getProject(page, id);
	const volume = p.scene.objects[0].volumes![0];
	expect(volume.type).toBe('modifier');
	volume.size = [30, 30, 30]; // couvre tout l'objet
	volume.settings = { sparse_infill_density: '0%' };
	await putScene(page, p);
	expect(midSparse(await sliceGcode(page, id)), 'remplissage supprimé par le modificateur').toBe(0);
});

test('bloqueur de supports : supprime les supports auto dans son volume @F:editor.volume-blocker', async ({ page }) => {
	const id = await importAndSave(page, ['overhang']);
	const auto = { enable_support: '1', support_type: 'normal(auto)', support_threshold_angle: '70' };
	let p = await getProject(page, id);
	p.scene.objects[0].settings = auto;
	await putScene(page, p);
	const supports = (g: string) => moves(g).filter((m) => m.extrude && m.type.startsWith('Support')).length;
	expect(supports(await sliceGcode(page, id)), 'supports auto attendus sans bloqueur').toBeGreaterThan(0);
	await page.reload();
	await expect(page.getByTestId('object-row')).toHaveCount(1, { timeout: 60_000 });
	await addVolumeFromMenu(page, /add support blocker/i, 'blocker');
	await saveEditor(page);
	p = await getProject(page, id);
	expect(p.scene.objects[0].settings).toMatchObject(auto);
	p.scene.objects[0].volumes![0].size = [60, 60, 60];
	await putScene(page, p);
	expect(supports(await sliceGcode(page, id)), 'bloqueur couvrant : aucun support').toBe(0);
});

/** Entrées texte d'une archive ZIP (répertoire central ; stockées ou deflate). */
function unzipText(buf: Buffer): Map<string, string> {
	const out = new Map<string, string>();
	let eocd = buf.length - 22;
	while (eocd >= 0 && buf.readUInt32LE(eocd) !== 0x06054b50) eocd--;
	let p = buf.readUInt32LE(eocd + 16);
	const count = buf.readUInt16LE(eocd + 10);
	for (let i = 0; i < count; i++) {
		const method = buf.readUInt16LE(p + 10);
		const size = buf.readUInt32LE(p + 20);
		const nameLen = buf.readUInt16LE(p + 28), extra = buf.readUInt16LE(p + 30), comment = buf.readUInt16LE(p + 32);
		const local = buf.readUInt32LE(p + 42);
		const name = buf.toString('utf8', p + 46, p + 46 + nameLen);
		const data = local + 30 + buf.readUInt16LE(local + 26) + buf.readUInt16LE(local + 28);
		const raw = buf.subarray(data, data + size);
		if (/\.(model|config|xml|rels)$/.test(name)) out.set(name, (method === 8 ? inflateRawSync(raw) : raw).toString('utf8'));
		p += 46 + nameLen + extra + comment;
	}
	return out;
}

test('export 3MF : emporte peinture et volumes (types, réglages) @F:editor.export-3mf-paint-volumes', async ({ page }) => {
	const id = await importAndSave(page, ['cube20']);
	const p = await getProject(page, id);
	p.scene.objects[0].painting = { supports: paintTriangles([0, 1]), seam: paintTriangles([10, 11]) };
	p.scene.objects[0].volumes = [
		{ id: 'n1', type: 'negative', size: [10, 10, 10], offset: [0, 0, 0], settings: {} },
		{ id: 'm1', type: 'modifier', size: [30, 30, 5], offset: [0, 0, 5], settings: { sparse_infill_density: '0%' } },
		{ id: 'b1', type: 'blocker', size: [5, 5, 5], offset: [0, 0, -5], settings: {} }
	];
	await putScene(page, p);
	const res = await page.request.get(`/api/projects/${id}/export/3mf`);
	expect(res.status(), await res.text().catch(() => '')).toBe(200);
	const files = unzipText(Buffer.from(await res.body()));
	const models = [...files].filter(([n]) => n.endsWith('.model')).map(([, t]) => t).join('\n');
	const settings = files.get('Metadata/model_settings.config') ?? '';
	// Peinture : attributs par triangle du writer Orca 2.4.1.
	expect(models).toMatch(/<triangle [^>]*paint_supports="4"/);
	expect(models).toMatch(/<triangle [^>]*paint_seam="4"/);
	// Volumes : un sous-objet par volume, <part subtype=…> et réglages du modificateur.
	expect(models).toMatch(/<components>[\s\S]*<component objectid=/);
	for (const subtype of ['normal_part', 'negative_part', 'modifier_part', 'support_blocker'])
		expect(settings).toContain(`subtype="${subtype}"`);
	expect(settings).toMatch(/key="sparse_infill_density" value="0%"/);
});

test('3MF : export → import → export restitue volumes et peinture @F:editor.import-3mf-paint-volumes', async ({ page }) => {
	const id = await importAndSave(page, ['cube20']);
	const p = await getProject(page, id);
	const volumes = [
		{ id: 'n1', type: 'negative', name: 'Trou', size: [10, 10, 10], offset: [0, 0, 0], settings: {} },
		{ id: 'm1', type: 'modifier', name: 'Modif', size: [30, 30, 5], offset: [0, 0, 5], settings: { sparse_infill_density: '0%' } },
		{ id: 'b1', type: 'blocker', name: 'Bloqueur', size: [5, 5, 5], offset: [2, 0, -5], settings: {} }
	];
	p.scene.objects[0].painting = { supports: paintTriangles([0, 1]), seam: paintTriangles([10, 11]) };
	p.scene.objects[0].volumes = volumes;
	await putScene(page, p);
	const first = Buffer.from(await (await page.request.get(`/api/projects/${id}/export/3mf`)).body());

	// Import comme nouveau projet.
	const imp = await page.request.post('/api/projects/import', {
		// Multipart = « formulaire » : même origine exigée par le CSRF de SvelteKit (prod).
		headers: { origin: new URL(page.url()).origin },
		multipart: { file: { name: 'roundtrip.3mf', mimeType: 'model/3mf', buffer: first } }
	});
	expect(imp.status(), await imp.text()).toBeLessThan(300);
	const imported = (await imp.json()) as Project;
	const object = (await getProject(page, imported.id)).scene.objects[0];
	expect(object.painting).toEqual({ supports: { '0': '4', '1': '4' }, seam: { '10': '4', '11': '4' } });
	expect(
		object.volumes?.map((v) => ({ type: v.type, size: v.size, offset: v.offset, settings: v.settings }))
	).toEqual(volumes.map((v) => ({ type: v.type, size: v.size, offset: v.offset, settings: v.settings })));

	// Réexport : géométrie, sous-types, réglages et peinture identiques.
	const second = Buffer.from(await (await page.request.get(`/api/projects/${imported.id}/export/3mf`)).body());
	const strip = (files: Map<string, string>) => {
		const keep = (t?: string) => (t ?? '').replace(/key="name" value="[^"]*"/g, '');
		return {
			model: keep([...files].filter(([n]) => n.endsWith('.model')).map(([, t]) => t).join('')).replace(/transform="[^"]*"/g, ''),
			parts: keep(files.get('Metadata/model_settings.config')).match(/<part[\s\S]*?<\/part>/g)
		};
	};
	expect(strip(unzipText(second))).toEqual(strip(unzipText(first)));
});
