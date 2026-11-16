<script lang="ts">
	// Wizard d'onboarding : la grille de sélection façon OrcaSlicer en plein
	// écran. Confirmer crée une imprimante par modèle et buse cochée (défaut 0.4) puis
	// entre dans la bibliothèque. Aucune sortie sans imprimante (Confirmer inactif
	// à 0) : la garde `requireOnboardedUser` ramène ici tant qu'aucune n'existe.
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { ApiError } from '$lib/api/client';
	import { createPrinter } from '$lib/api/printers';
	import AppSidebar from '$lib/nav/AppSidebar.svelte';
	import PrinterCatalog from '$lib/printers/PrinterCatalog.svelte';
	import type { PrinterCatalogModel } from '$lib/api/types';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	let busy = $state(false);
	let error = $state<string | null>(null);

	async function confirm(models: PrinterCatalogModel[], nozzles?: Record<string, string[]>) {
		if (models.length === 0) return;
		busy = true;
		error = null;
		try {
			// Une imprimante par buse cochée (assistant d'Orca) : nom = modèle, suivi
			// du diamètre quand plusieurs buses du même modèle sont retenues.
			for (const m of models) {
				const ids = nozzles?.[`${m.vendor}::${m.model}`] ?? [m.default_machine_preset_id];
				for (const id of ids) {
					const nozzle = m.variants.find((v) => v.machine_preset_id === id)?.nozzle;
					const name = ids.length > 1 && nozzle ? `${m.model} (buse ${nozzle} mm)` : m.model;
					await createPrinter({ name, machine_preset_id: id });
				}
			}
			await goto(resolve('/library'));
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Erreur inattendue';
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head><title>Sélection de l'imprimante — Web-Slicer</title></svelte:head>

<div class="flex h-screen bg-surface text-content">
	<AppSidebar email={data.user.email} />

	<main class="flex min-w-0 flex-1 flex-col p-4">
		<div class="mx-auto flex min-h-0 w-full max-w-4xl flex-1 flex-col">
			<header class="mb-3 text-center">
				<h1 class="ws-page-title">Sélection de l'imprimante</h1>
				<p class="text-sm text-content-muted">
					Choisissez la ou les imprimantes que vous possédez pour commencer.
				</p>
			</header>

			{#if error}
				<p class="mb-2 text-center text-sm text-danger" role="alert">{error}</p>
			{/if}

			<div class="min-h-0 flex-1 overflow-hidden rounded border border-border bg-surface-raised">
				<PrinterCatalog
					vendors={data.catalog}
					onconfirm={confirm}
					confirmLabel="Confirmer"
					chooseNozzle
					{busy}
				/>
			</div>
		</div>
	</main>
</div>
