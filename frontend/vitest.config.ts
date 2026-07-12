import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Runner des modules réactifs (`*.svelte.ts` à runes) : bun test ne sait pas
// exécuter les runes (`$state`), on délègue donc à vitest via le plugin svelte
// (compilation runes). Le reste de la suite reste sous `bun test` — vitest
// n'inclut que les fichiers `*.vitest.ts`, que bun ignore de son côté.
export default defineConfig({
	plugins: [svelte({ compilerOptions: { runes: true } })],
	test: {
		include: ['src/**/*.vitest.ts'],
		environment: 'node'
	}
});
