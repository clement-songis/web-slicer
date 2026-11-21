// Phase 32 — sixième audit en navigateur : comptes (connexion, mot de passe
// oublié, retour à la page demandée, administration), bibliothèque (noms
// uniques), file à plusieurs jobs, reprise après rechargement pendant un
// tranchage, serveur injoignable / hors ligne.
// CONTRAT: auth-forgot, auth-forgot-help, auth-invite, admin-page, admin-invite,
// admin-invite-link, nav-admin, error-page (data-status), error-retry,
// connection-banner (data-state), connection-retry, connection-restored,
// queue-job-project, queue-history-project, queue-status.
import { expect, test, type Page } from '@playwright/test';
import { bootstrap, importModel, uniqueEmail } from '../lib/app';

test.use({ locale: 'fr-FR' });

// Requêtes du serveur seulement : un motif « **/api/** » couvrirait aussi les
// modules Vite (/src/lib/api/…) et la page ne se chargerait plus du tout.
const API = (url: URL) => url.pathname.startsWith('/api/');

async function projectName(page: Page, id: string): Promise<string> {
	const res = await page.request.get(`/api/projects/${id}`);
	return ((await res.json()) as { name: string }).name;
}

test('bibliothèque : deux « Nouveau projet » de suite → noms uniques, pas d’erreur SQL @F:ui.library-crud', async ({
	page
}) => {
	await bootstrap(page);
	const ids: string[] = [];
	for (let i = 0; i < 2; i++) {
		await page.goto('/library');
		await page.getByTestId('new-project').click();
		await page.waitForURL(/\/projects\/[^/]+$/, { timeout: 30_000 });
		ids.push(page.url().split('/').pop()!);
	}
	await page.goto('/library');
	await expect(page.getByText(/UNIQUE constraint/)).toHaveCount(0);
	const names = await Promise.all(ids.map((id) => projectName(page, id)));
	expect(new Set(names).size).toBe(2);
	expect(names.every((n) => /^Sans titre( \(\d+\))?$/.test(n))).toBe(true);
});

test('connexion : identifiants faux expliqués, mot de passe oublié, retour à la page demandée @F:core.auth-ui', async ({
	page
}) => {
	const email = uniqueEmail();
	const reg = await page.request.post('/api/auth/register', {
		data: { email, password: 'harness-password-123' }
	});
	expect(reg.status()).toBe(201);
	await page.context().clearCookies();
	await page.goto('/printers');
	await expect(page).toHaveURL(/\/login\?next=%2Fprinters$/);
	await page.getByTestId('auth-email').fill(email);
	await page.getByTestId('auth-password').fill('mauvais-mot-de-passe');
	await page.getByTestId('auth-submit').click();
	await expect(page.getByRole('alert')).toHaveText('E-mail ou mot de passe incorrect.');
	await page.getByTestId('auth-forgot').click();
	await expect(page.getByTestId('auth-forgot-help')).toContainText('administrateur');
	await page.getByTestId('auth-password').fill('harness-password-123');
	await page.getByTestId('auth-submit').click();
	await expect(page).toHaveURL(/\/printers$/, { timeout: 30_000 });
});

test('administration : refusée à un utilisateur, lien d’invitation pour un admin', async ({
	page
}) => {
	await bootstrap(page);
	await page.goto('/admin');
	await expect(page.getByTestId('error-page')).toHaveAttribute('data-status', '403');
	await page.goto('/library');
	await expect(page.getByTestId('nav-admin')).toHaveCount(0);
	// Compte administrateur simulé (le premier compte de l'instance n'est pas
	// déterministe dans la pile partagée) : seule l'interface est vérifiée.
	const me = (await (await page.request.get('/api/auth/me')).json()) as Record<string, string>;
	await page.route('**/api/auth/me', (r) =>
		r.fulfill({ json: { ...me, role: 'admin' } })
	);
	await page.route('**/api/admin/instance', (r) =>
		r.fulfill({ json: { registration_policy: 'invite', upload_limit_bytes: 524288000 } })
	);
	await page.route('**/api/admin/invitations', (r) =>
		r.fulfill({ status: 201, json: { token: 'jeton-p32', expires_at: '2030-01-01T00:00:00Z' } })
	);
	await page.goto('/library');
	await page.getByTestId('nav-admin').click();
	await expect(page.getByTestId('admin-page')).toBeVisible();
	await expect(page.getByTestId('admin-policy-invite')).toBeChecked();
	await expect(page.getByTestId('admin-upload-limit')).toHaveValue('500');
	await page.getByTestId('admin-invite').click();
	const link = page.getByTestId('admin-invite-link');
	await expect(link).toHaveValue(/\/register\?invite=jeton-p32$/);
	await page.context().clearCookies();
	await page.unrouteAll();
	await page.goto('/register?invite=jeton-p32');
	await expect(page.getByTestId('auth-invite')).toHaveValue('jeton-p32');
});

test('serveur injoignable pendant l’édition : message clair, bandeau, retour détecté', async ({
	page
}) => {
	await bootstrap(page);
	await importModel(page, 'cube20');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.route(API, (r) => r.abort('connectionrefused'));
	await page.getByTestId('save-project').click();
	await expect(page.getByTestId('editor-message')).toContainText('Serveur injoignable');
	const banner = page.getByTestId('connection-banner');
	await expect(banner).toBeVisible();
	await expect(banner).toHaveAttribute('data-state', 'unreachable');
	await page.unroute(API);
	// Une réponse d'une requête de fond peut déjà avoir rétabli la liaison.
	if (await banner.isVisible()) await page.getByTestId('connection-retry').click({ timeout: 2_000 }).catch(() => {});
	await expect(page.getByTestId('connection-restored')).toBeVisible();
	await expect(banner).toHaveCount(0);
	await page.context().setOffline(true);
	await expect(banner).toHaveAttribute('data-state', 'offline');
	await page.context().setOffline(false);
});

test('chargement sans serveur : page d’erreur explicite et « Réessayer »', async ({ page }) => {
	await bootstrap(page);
	await page.route(API, (r) => r.abort('connectionrefused'));
	await page.goto('/library');
	await expect(page.getByTestId('error-page')).toContainText('Serveur injoignable');
	await page.unroute(API);
	await page.getByTestId('error-retry').click();
	await expect(page.getByTestId('new-project')).toBeVisible({ timeout: 30_000 });
});

test('projet importé jamais enregistré, rouvert : « Trancher » enregistre puis tranche @F:core.slice-ui', async ({
	page
}) => {
	test.setTimeout(240_000);
	const id = await bootstrap(page);
	await importModel(page, 'cube20');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	await page.goto(`/projects/${id}`);
	await expect(page.getByTestId('object-row')).toHaveCount(1, { timeout: 60_000 });
	const slice = page.waitForResponse(
		(r) => r.request().method() === 'POST' && /\/slice$/.test(r.url()),
		{ timeout: 120_000 }
	);
	await page.getByTestId('slice-button').click();
	expect((await slice).status()).toBeLessThan(300);
	await expect(page.getByTestId('preview-view')).toHaveAttribute('data-layer-count', /^[1-9]\d*$/, {
		timeout: 170_000
	});
});

test('rechargement pendant un tranchage : suivi repris, aperçu affiché sans relancer @F:ui.queue', async ({
	page
}) => {
	test.setTimeout(300_000);
	await bootstrap(page);
	await importModel(page, 'cube20');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	const slice = page.waitForResponse(
		(r) => r.request().method() === 'POST' && /\/slice$/.test(r.url()),
		{ timeout: 120_000 }
	);
	await page.getByTestId('slice-button').click();
	expect((await slice).status()).toBeLessThan(300);
	let slicePosts = 0;
	page.on('request', (r) => {
		if (r.method() === 'POST' && /\/slice$/.test(r.url())) slicePosts++;
	});
	await page.reload();
	await expect(page.getByTestId('preview-view')).toHaveAttribute('data-layer-count', /^[1-9]\d*$/, {
		timeout: 200_000
	});
	expect(slicePosts).toBe(0);
});

test('file : job lancé dans un autre onglet affiché en direct avec le nom du projet @F:ui.queue', async ({
	page,
	context
}) => {
	test.setTimeout(240_000);
	const id = await bootstrap(page);
	await importModel(page, 'cube20');
	await expect(page.getByTestId('import-status')).toHaveCount(0, { timeout: 60_000 });
	const name = await projectName(page, id);
	const queue = await context.newPage();
	await queue.goto('/queue');
	await expect(queue.getByRole('heading', { name: 'File de tranchage' })).toBeVisible();
	await page.getByTestId('slice-button').click();
	// En cours puis historique : le job apparaît sans recharger la file.
	await expect(
		queue
			.locator('[data-testid=queue-job-project], [data-testid=queue-history-project]')
			.filter({ hasText: name })
			.first()
	).toBeVisible({ timeout: 170_000 });
	await expect(page.getByTestId('preview-view')).toHaveAttribute('data-layer-count', /^[1-9]\d*$/, {
		timeout: 170_000
	});
	await queue.reload();
	await expect(
		queue.getByTestId('queue-history-project').filter({ hasText: name }).first()
	).toBeVisible();
});
