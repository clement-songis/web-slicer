<script lang="ts">
	// Page d'erreur (phase 32) : serveur injoignable au chargement, page
	// introuvable, accès refusé. Toujours une issue : réessayer ou revenir à la
	// bibliothèque — jamais le « 500 Internal Error » nu de SvelteKit.
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { probeNow } from '$lib/api/connectivity';

	const code = $derived(page.error?.code ?? '');
	const offline = $derived(code === 'network' || code === 'unavailable');
	const title = $derived(
		offline
			? 'Serveur injoignable'
			: page.status === 404
				? 'Page introuvable'
				: page.status === 403
					? 'Accès refusé'
					: 'Une erreur est survenue'
	);
	let retrying = $state(false);

	async function retry() {
		retrying = true;
		await probeNow();
		location.reload();
	}
</script>

<svelte:head><title>{title} — Web-Slicer</title></svelte:head>

<main
	class="flex min-h-screen items-center justify-center bg-surface p-4 text-content"
	data-testid="error-page"
	data-status={page.status}
>
	<div class="w-full max-w-md rounded-lg bg-surface-raised p-8 shadow">
		<h1 class="ws-page-title mb-2">{title}</h1>
		<p class="mb-1 text-sm text-content-muted">
			{#if offline}
				La page n'a pas pu être chargée : le serveur ne répond pas ou la connexion est coupée. Rien
				n'est perdu côté serveur ; réessayez dans quelques instants.
			{:else if page.status === 404}
				Cette adresse ne correspond à aucune page (projet supprimé ou lien erroné).
			{:else}
				{page.error?.message ?? 'Erreur inattendue.'}
			{/if}
		</p>
		{#if !offline && page.status !== 404}
			<p class="mb-4 text-xs text-content-subtle">Code {page.status}</p>
		{/if}
		<div class="mt-6 flex flex-wrap gap-2">
			<button
				type="button"
				class="ws-btn ws-btn-primary"
				onclick={retry}
				disabled={retrying}
				data-testid="error-retry"
			>
				{retrying ? 'Nouvel essai…' : 'Réessayer'}
			</button>
			<a href={resolve('/library')} class="ws-btn ws-btn-secondary">Bibliothèque</a>
		</div>
	</div>
</main>
