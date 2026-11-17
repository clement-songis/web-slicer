// Pilotage de l'application via le contrat harness/CONTRACT.md (gelé).
import { expect, type APIRequestContext, type Page } from '@playwright/test';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export const ORACLE = join(dirname(fileURLToPath(import.meta.url)), '..');
export const REPO = join(ORACLE, '..', '..');
export const REFS = join(ORACLE, 'refs');
export const FIXTURES = join(ORACLE, 'fixtures');

export const oracleCfg = JSON.parse(readFileSync(join(REPO, 'harness', 'oracle.json'), 'utf8')) as {
	printer: string;
	filament: string;
	process: string;
	param_model: string;
};

export type FlatConfig = Record<string, string | string[]>;

export function readRef(rel: string): string | null {
	const p = join(REFS, rel);
	return existsSync(p) ? readFileSync(p, 'utf8') : null;
}

export function readJsonRef<T>(rel: string): T | null {
	const s = readRef(rel);
	return s === null ? null : (JSON.parse(s) as T);
}

export function fixture(model: string): string {
	// Scène multi-objets : chemin relatif avec extension (ex. scenes/two_objects.3mf).
	return join(FIXTURES, model.endsWith('.3mf') ? model : `${model}.stl`);
}

export function slug(s: string): string {
	return s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

/** §1 — tranche via l'API de harness (même moteur que l'UI). */
export async function harnessSlice(request: APIRequestContext, model: string, config: FlatConfig): Promise<string> {
	const res = await request.post('/api/_harness/slice', {
		multipart: {
			model: model.endsWith('.3mf')
				? { name: model.split('/').pop()!, mimeType: 'model/3mf', buffer: readFileSync(fixture(model)) }
				: { name: `${model}.stl`, mimeType: 'model/stl', buffer: readFileSync(fixture(model)) },
			config: JSON.stringify(config)
		},
		timeout: 170_000
	});
	expect(res.status(), `POST /api/_harness/slice → ${res.status()} : ${(await res.text()).slice(0, 600)}`).toBe(200);
	return res.text();
}

let counter = 0;
export function uniqueEmail(): string {
	return `t${Date.now().toString(36)}${process.pid}${counter++}@harness.local`;
}

/** §1 — crée un utilisateur + un projet prêt (presets de oracle.json), session ouverte dans `page`. */
export async function bootstrap(page: Page): Promise<string> {
	const res = await page.request.post('/api/_harness/bootstrap', {
		data: {
			email: uniqueEmail(),
			password: 'harness-password-123',
			printer: oracleCfg.printer,
			filament: oracleCfg.filament,
			process: oracleCfg.process
		}
	});
	expect(res.status(), `bootstrap → ${res.status()} : ${(await res.text()).slice(0, 400)}`).toBe(200);
	const { project_id } = (await res.json()) as { project_id: string };
	await page.goto(`/projects/${project_id}`);
	await expect(page.getByTestId('scene-canvas')).toBeVisible();
	return project_id;
}

export async function importModel(page: Page, model: string): Promise<void> {
	const before = await page.getByTestId('object-row').count();
	await page.getByTestId('import-input').setInputFiles(fixture(model));
	await expect(page.getByTestId('object-row')).toHaveCount(before + 1, { timeout: 60_000 });
}

export async function sliceInUi(page: Page): Promise<number> {
	await page.getByTestId('slice-button').click();
	const preview = page.getByTestId('preview-view');
	await expect(preview).toBeVisible({ timeout: 170_000 });
	await expect(preview).toHaveAttribute('data-layer-count', /^[1-9]\d*$/, { timeout: 170_000 });
	return Number(await preview.getAttribute('data-layer-count'));
}

export async function downloadGcode(page: Page): Promise<string> {
	const [dl] = await Promise.all([page.waitForEvent('download', { timeout: 60_000 }), page.getByTestId('export-gcode-button').click()]);
	expect(dl.suggestedFilename()).toMatch(/\.gcode$/);
	const p = await dl.path();
	return readFileSync(p, 'utf8');
}

export type Scope = 'print' | 'filament' | 'printer' | 'plate' | 'object';
export type ParamCase = {
	id: string;
	key: string;
	type: string;
	value: string | string[] | null;
	ui: boolean;
	mode: string;
	nullable?: boolean;
	tab?: string;
	section?: string;
	scope?: Scope;
	requires?: Record<string, string | string[]>;
	/** Scène multi-objets/multi-filament (fixtures/scenes/<scene>.3mf) : config plate
	 * complète dans refs/param/<clé>.config.json, tranchée telle quelle. */
	scene?: string;
	/** Modèle de fixtures propre au cas (défaut : oracle.json param_model). */
	model?: string;
};

/** Niveaux de visibilité Orca (ConfigOptionMode) : une option de niveau N est visible à partir du mode N. */
export const MODES = ['simple', 'advanced', 'expert', 'develop'] as const;
export function lowerMode(mode: string): string | null {
	const i = MODES.indexOf(mode as (typeof MODES)[number]);
	return i > 0 ? MODES[i - 1] : null;
}

/** Ouvre l'endroit où Orca édite ce réglage : dialogue de preset, réglages du plateau ou de l'objet. */
export async function openSettings(page: Page, scope: Scope) {
	const dialog = page.getByTestId('settings-dialog');
	if (await dialog.isVisible()) return dialog;
	if (scope === 'plate') await page.getByTestId('plate-settings-button').first().click();
	else if (scope === 'object') {
		// Réglages par objet : panneau latéral (pas un dialogue), comme Orca.
		await page.getByTestId('object-row').first().click();
		const panel = page.getByTestId('object-settings-panel');
		await expect(panel).toBeVisible();
		return panel;
	} else await page.getByTestId(`open-settings-${scope}`).click();
	await expect(dialog).toBeVisible();
	return dialog;
}

export async function setMode(page: Page, mode: string) {
	await page.getByTestId('settings-dialog').getByTestId(`settings-mode-${mode}`).click();
}

/** Ligne `[data-param=key]` dans l'onglet Orca, au mode demandé (expert par défaut). */
export async function paramRow(page: Page, scope: Scope, tab: string | undefined, key: string, mode = 'develop') {
	const dialog = await openSettings(page, scope);
	if (scope === 'object') {
		// Une surcharge d'objet n'existe qu'une fois ajoutée via « + Réglage ».
		const row = dialog.locator(`[data-param="${key}"]`);
		if (!(await row.count())) {
			await dialog.getByTestId('object-settings-add').click();
			await dialog.getByTestId('object-settings-search').fill(key);
			await dialog.locator(`[data-testid="object-settings-result"][data-key="${key}"]`).click();
		}
		return row;
	}
	await setMode(page, mode);
	if (tab) {
		const tabBtn = dialog.getByTestId(`settings-tab-${slug(tab)}`);
		if (scope !== 'plate' && scope !== 'object') await tabBtn.click();
		else if (await tabBtn.count()) await tabBtn.click();
	}
	return dialog.locator(`[data-param="${key}"]`);
}

export async function controlOf(row: ReturnType<Page['locator']>) {
	const indexed = row.locator('[data-index="0"]');
	return (await indexed.count()) ? indexed.first() : row.locator('input:not([data-override-toggle]), select, textarea').first();
}

/** Ouvre la ligne et renvoie son contrôle principal (active la surcharge si l'option est « nullable »). */
export async function openParam(page: Page, scope: string, tab: string | undefined, key: string, mode = 'develop') {
	const row = await paramRow(page, scope as Scope, tab, key, mode);
	await expect(row, `ligne de réglage [data-param="${key}"] introuvable (${scope} › ${tab ?? '—'}, mode ${mode})`).toBeVisible();
	const toggle = row.locator('[data-override-toggle]');
	if ((await toggle.count()) && !(await toggle.isChecked())) await toggle.check();
	return controlOf(row);
}

/** Valeur affichée par un contrôle, au format Orca (« 1 »/« 0 » pour une case). */
export async function readControl(control: ReturnType<Page['locator']>): Promise<string> {
	if ((await control.getAttribute('type')) === 'checkbox') return (await control.isChecked()) ? '1' : '0';
	return control.inputValue();
}

/** Égalité tolérante au formatage numérique (« 0.20 » == « 0.2 », « 50 % » == « 50% »). */
export function looseEqual(a: string, b: string): boolean {
	const norm = (s: string) => s.replace(/\s+/g, '').toLowerCase();
	if (norm(a) === norm(b)) return true;
	const na = Number(norm(a).replace(/%$/, ''));
	const nb = Number(norm(b).replace(/%$/, ''));
	return Number.isFinite(na) && Number.isFinite(nb) && Math.abs(na - nb) < 1e-9 && a.trim().endsWith('%') === b.trim().endsWith('%');
}

/** Saisit une valeur au format Orca dans le contrôle ; renvoie false si le champ est désactivé. */
export async function setControl(control: ReturnType<Page['locator']>, value: string): Promise<boolean> {
	if (await control.isDisabled()) return false;
	const tag = await control.evaluate((el) => el.tagName.toLowerCase());
	const type = await control.getAttribute('type');
	if (type === 'checkbox') await control.setChecked(value === '1' || value === 'true');
	else if (tag === 'select') await control.selectOption(value);
	else {
		await control.fill(value);
		await control.press('Tab');
	}
	return true;
}

export function scalar(v: string | string[]): string {
	return Array.isArray(v) ? v[0] : v;
}
