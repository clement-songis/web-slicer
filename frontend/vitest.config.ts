import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { fileURLToPath } from 'node:url';

// Runner unique de la suite frontend : vitest. Le plugin svelte compile les
// modules réactifs à runes (`*.svelte.ts`, ex. `objects`/`plates`) que les
// tests importent ; l'alias `#lib` reproduit celui de SvelteKit (les tests et
// modules l'utilisent). Environnement `node` : la suite est de la logique pure
// (aucun test ne touche le DOM ; les parties DOM sont couvertes en navigateur).
export default defineConfig({
	plugins: [
		// Paraglide 3 : le code généré (`src/lib/paraglide/messages.js`) importe
		// le module virtuel `virtual:paraglide/*`, fourni par ce plugin seul —
		// sans lui, toute la suite touchant i18n échoue à l'import.
		paraglideVitePlugin({
			project: './project.inlang',
			outdir: './src/lib/paraglide',
			strategy: ['localStorage', 'preferredLanguage', 'baseLocale']
		}),
		svelte({ compilerOptions: { runes: true } })
	],
	resolve: {
		alias: {
			'#lib': fileURLToPath(new URL('./src/lib', import.meta.url))
		}
	},
	test: {
		include: ['src/**/*.test.ts'],
		environment: 'node'
	}
});
