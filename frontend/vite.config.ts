import tailwindcss from '@tailwindcss/vite';
import adapter from '@sveltejs/adapter-node';
import { sveltekit } from '@sveltejs/kit/vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import Icons from 'unplugin-icons/vite';
import { FileSystemIconLoader } from 'unplugin-icons/loaders';
import { defineConfig } from 'vite';

export default defineConfig({
	server: {
		// Les scénarios E2E doivent rester sur le même document : une optimisation
		// tardive de dépendances Vite ne doit pas recharger un formulaire en cours.
		hmr: process.env.HARNESS_E2E !== '1',
		// Transforme les parcours d'entrée dès le démarrage. Sans ce préchauffage,
		// la découverte tardive de dépendances peut recharger la page auth pendant
		// le premier formulaire et perdre sa soumission (notamment en harness).
		warmup: {
			clientFiles: [
				'./src/routes/(auth)/register/+page.svelte',
				'./src/routes/(auth)/login/+page.svelte',
				'./src/routes/setup/+page.svelte',
				'./src/routes/projects/[id]/+page.svelte'
			]
		},
		// Autorise l'accès via un tunnel Cloudflare (URL éphémère *.trycloudflare.com) :
		// Vite bloque sinon les hôtes inconnus dans l'en-tête Host.
		allowedHosts: ['.trycloudflare.com'],
		// En dev, les appels `/api/*` sont relayés au backend axum (cookie de
		// session same-origin). Surcharger la cible via BACKEND_URL au besoin.
		// `ws: true` relaie aussi l'upgrade WebSocket `/api/ws` (flux d'événements
		// de la file : progression et fin de tranchage) — sans quoi l'UI reste
		// bloquée sur « tranchage en cours ».
		proxy: {
			'/api': {
				target: process.env.BACKEND_URL ?? 'http://127.0.0.1:8080',
				changeOrigin: true,
				ws: true
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

			// Production : serveur Node autonome (`node build`, cf. README « Production »).
			// Les appels `/api/*` y sont relayés au backend par `src/hooks.server.ts`.
			adapter: adapter(),
			// Le contrôle CSRF intégré de SvelteKit s'exécute avant `handle` et
			// refusait (403) les POST multipart relayés vers `/api/*` dès que
			// l'Origin différait de l'origine vue par Node (reverse proxy, ORIGIN
			// absent). Il est désactivé ici (« * ») et réimplémenté à l'identique
			// dans `src/hooks.server.ts` pour les routes SvelteKit, hors `/api/*`
			// (relayé au backend, qui porte sa propre authentification).
			csrf: { trustedOrigins: ['*'] }
		})
	]
});
