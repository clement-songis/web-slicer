import tailwindcss from '@tailwindcss/vite';
import adapter from '@sveltejs/adapter-auto';
import { sveltekit } from '@sveltejs/kit/vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import Icons from 'unplugin-icons/vite';
import { FileSystemIconLoader } from 'unplugin-icons/loaders';
import { defineConfig } from 'vite';

export default defineConfig({
	server: {
		// En dev, les appels `/api/*` sont relayés au backend axum (cookie de
		// session same-origin). Surcharger la cible via BACKEND_URL au besoin.
		proxy: {
			'/api': {
				target: process.env.BACKEND_URL ?? 'http://127.0.0.1:8080',
				changeOrigin: true
			}
		}
	},
	plugins: [
		tailwindcss(),
		// i18n : compile messages/{en,fr}.json → src/lib/paraglide/ (généré).
		paraglideVitePlugin({
			project: './project.inlang',
			outdir: './src/lib/paraglide',
			strategy: ['localStorage', 'preferredLanguage', 'baseLocale']
		}),
		// Icônes : sets Iconify (Lucide…) + collection locale `~icons/custom/*`
		// alimentée par les SVG de `src/lib/icons/custom/` (tree-shakées à froid).
		// Convention : les SVG custom utilisent `currentColor` (cf. le README du dossier).
		Icons({
			compiler: 'svelte',
			customCollections: {
				custom: FileSystemIconLoader('./src/lib/icons/custom')
			}
		}),
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},

			// adapter-auto only supports some environments, see https://svelte.dev/docs/kit/adapter-auto for a list.
			// If your environment is not supported, or you settled on a specific environment, switch out the adapter.
			// See https://svelte.dev/docs/kit/adapters for more information about adapters.
			adapter: adapter()
		})
	]
});
