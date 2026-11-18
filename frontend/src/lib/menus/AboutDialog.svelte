<script module lang="ts">
	import { registerTranslations } from '$lib/i18n';
	registerTranslations('fr', { About: 'À propos' });
	/** Version d'OrcaSlicer dont le moteur et les profils sont repris. */
	export const ORCA_VERSION = '2.4.1';
</script>

<script lang="ts">
	// « À propos » (Orca AboutDialog.cpp) : version du serveur, moteur de
	// tranchage, licences (AGPL-3.0 d'OrcaSlicer / libslic3r) et crédits.
	import { api } from '$lib/api/client';
	import { APP_NAME } from '$lib/version';
	import { dialogFocus } from '$lib/ui/dialog-focus';

	interface Props {
		open: boolean;
		onclose: () => void;
	}
	let { open, onclose }: Props = $props();

	let version = $state<string | null>(null);
	$effect(() => {
		if (!open || version) return;
		api
			.get<{ version: string }>('/health')
			.then((h) => (version = h.version))
			.catch(() => (version = '—'));
	});

	const CREDITS: { name: string; role: string; license: string; url: string }[] = [
		{
			name: 'OrcaSlicer',
			role: 'moteur de tranchage (libslic3r), profils d’imprimantes et de filaments, modèles pratiques',
			license: 'AGPL-3.0',
			url: 'https://github.com/SoftFever/OrcaSlicer'
		},
		{
			name: 'Bambu Studio, PrusaSlicer, Slic3r',
			role: 'projets dont OrcaSlicer est dérivé',
			license: 'AGPL-3.0',
			url: 'https://github.com/prusa3d/PrusaSlicer'
		},
		{
			name: 'three.js / Threlte',
			role: 'affichage 3D',
			license: 'MIT',
			url: 'https://threejs.org'
		},
		{ name: 'Svelte', role: 'interface', license: 'MIT', url: 'https://svelte.dev' }
	];
</script>

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-scrim"
		role="presentation"
		onclick={onclose}
	>
		<div
			class="ws-dialog w-[32rem] max-w-[calc(100vw-2rem)] p-4"
			role="dialog"
			aria-modal="true"
			aria-labelledby="about-title"
			data-testid="about-dialog"
			tabindex="-1"
			use:dialogFocus
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && onclose()}
		>
			<h2 id="about-title" class="ws-dialog-title">À propos de {APP_NAME}</h2>
			<dl class="mt-2 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
				<dt class="text-content-muted">Version</dt>
				<dd data-testid="about-version">{version ?? '…'}</dd>
				<dt class="text-content-muted">Parité</dt>
				<dd>OrcaSlicer {ORCA_VERSION} (moteur et profils)</dd>
			</dl>
			<p class="mt-3 text-sm text-content-muted">
				Trancheur 3D dans le navigateur. Le tranchage est fait par le moteur d’OrcaSlicer, sans
				réécriture : les G-codes produits sont ceux qu’OrcaSlicer produirait.
			</p>
			<h3 class="ws-section-title mt-4 mb-1">Logiciels libres utilisés</h3>
			<ul class="flex flex-col gap-1.5 text-sm" data-testid="about-credits">
				{#each CREDITS as c (c.name)}
					<li class="flex items-baseline justify-between gap-3">
						<span>
							<!-- Lien externe (site du projet) : pas une route de l'application. -->
							<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
							<a class="ws-link" href={c.url} target="_blank" rel="noopener noreferrer">{c.name}</a>
							<span class="text-xs text-content-subtle"> — {c.role}</span>
						</span>
						<span class="ws-badge shrink-0">{c.license}</span>
					</li>
				{/each}
			</ul>
			<p class="mt-3 text-xs text-content-subtle">
				Le moteur étant sous licence AGPL-3.0, le code source de ce service est mis à disposition de
				ses utilisateurs selon les mêmes termes.
			</p>
			<div class="mt-4 flex justify-end">
				<button type="button" class="ws-btn ws-btn-primary" onclick={onclose}>Fermer</button>
			</div>
		</div>
	</div>
{/if}
