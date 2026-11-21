// Configuration des tests oracle (gelée). La pile est démarrée par harness/stack.sh.
import { defineConfig } from '@playwright/test';

export default defineConfig({
	testDir: '.',
	testMatch: ['e2e/**/*.spec.ts', 'params/**/*.spec.ts', 'golden/**/*.spec.ts', 'scenarios/**/*.spec.ts'],
	timeout: 180_000,
	expect: { timeout: 15_000 },
	fullyParallel: true,
	workers: Number(process.env.ORACLE_WORKERS ?? 4),
	retries: 0,
	reporter: 'line',
	use: {
		locale: 'fr-FR',
		baseURL: process.env.BASE_URL ?? 'http://127.0.0.1:5173',
		acceptDownloads: true,
		// Échec rapide sur sélecteur absent : feedback court et peu coûteux pour l'agent.
		actionTimeout: 15_000,
		navigationTimeout: 30_000,
		viewport: { width: 1600, height: 1000 },
		trace: 'off',
		screenshot: 'off',
		video: 'off'
	}
});
