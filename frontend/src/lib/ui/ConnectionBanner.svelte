<script lang="ts">
	// Bandeau global de liaison serveur (phase 32) : apparaît dès qu'une requête
	// n'a pas pu joindre le serveur (ou que le navigateur passe hors ligne),
	// rassure sur le travail en cours (la page n'est pas perdue), sonde le retour
	// automatiquement et confirme brièvement le rétablissement. Orca est une
	// application locale : équivalent web de ses notifications de connexion.
	import { onMount } from 'svelte';
	import { connection, probeNow, watchBrowserConnectivity } from '$lib/api/connectivity';

	let restoredVisible = $state(false);
	let checking = $state(false);
	let lastRestored = 0;
	let timer: ReturnType<typeof setTimeout> | null = null;

	onMount(() => {
		const stop = watchBrowserConnectivity();
		const unsub = connection.subscribe((c) => {
			if (c.restored !== lastRestored) {
				lastRestored = c.restored;
				if (c.restored > 0) {
					restoredVisible = true;
					if (timer) clearTimeout(timer);
					timer = setTimeout(() => (restoredVisible = false), 4000);
				}
			}
			if (!c.reachable) restoredVisible = false;
		});
		return () => {
			stop();
			unsub();
			if (timer) clearTimeout(timer);
		};
	});

	async function retry() {
		checking = true;
		try {
			await probeNow();
		} finally {
			checking = false;
		}
	}
</script>

{#if !$connection.reachable}
	<div
		class="fixed bottom-4 left-1/2 z-[60] flex max-w-[min(36rem,calc(100vw-2rem))] -translate-x-1/2 items-center gap-3 rounded-lg border border-warning bg-surface-raised px-4 py-2.5 text-sm text-content shadow-lg"
		role="status"
		aria-live="polite"
		data-testid="connection-banner"
		data-state={$connection.browserOffline ? 'offline' : 'unreachable'}
	>
		<span class="h-2.5 w-2.5 shrink-0 animate-pulse rounded-full bg-warning" aria-hidden="true"
		></span>
		<span class="min-w-0 flex-1">
			<strong class="font-semibold">
				{$connection.browserOffline ? 'Hors ligne.' : 'Serveur injoignable.'}
			</strong>
			Votre travail reste affiché ; vous pourrez enregistrer et trancher dès le retour de la connexion.
			<span class="text-content-muted">Nouvelle tentative automatique…</span>
		</span>
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-secondary shrink-0"
			onclick={retry}
			disabled={checking}
			data-testid="connection-retry"
		>
			{checking ? 'Vérification…' : 'Réessayer'}
		</button>
	</div>
{:else if restoredVisible}
	<div
		class="pointer-events-none fixed bottom-4 left-1/2 z-[60] -translate-x-1/2 rounded-lg border border-success bg-surface-raised px-4 py-2 text-sm text-content shadow-lg"
		role="status"
		aria-live="polite"
		data-testid="connection-restored"
	>
		Connexion au serveur rétablie.
	</div>
{/if}
