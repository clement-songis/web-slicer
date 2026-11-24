<script lang="ts">
	// Page d'erreur (phase 32) : serveur injoignable au chargement, page
	// introuvable, accès refusé. Toujours une issue : réessayer ou revenir à la
	// bibliothèque — jamais le « 500 Internal Error » nu de SvelteKit.
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { probeNow } from '#lib/api/connectivity';
	import { t } from '#lib/i18n';

	const code = $derived(page.error?.code ?? '');
	const offline = $derived(code === 'network' || code === 'unavailable');
	const title = $derived(
		$t(
			offline
				? 'Server unreachable'
				: page.status === 404
					? 'Page not found'
					: page.status === 403
						? 'Access denied'
						: 'An error occurred'
		)
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
				{$t(
					'The page could not be loaded: the server is not responding or the connection is down. Nothing is lost on the server; try again in a few moments.'
				)}
			{:else if page.status === 404}
				{$t('This address does not match any page (deleted project or wrong link).')}
			{:else}
				{page.error?.message ?? $t('Unexpected error.')}
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
				{retrying ? $t('Retrying…') : $t('Retry')}
			</button>

			<a href={resolve('library')} class="ws-btn ws-btn-secondary">{$t('Library')}</a>
		</div>
	</div>
</main>
