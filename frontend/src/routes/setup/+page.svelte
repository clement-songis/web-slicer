<script lang="ts">
	// Wizard d'onboarding : la grille de sélection façon OrcaSlicer en plein
	// écran. Confirmer crée une imprimante par modèle choisi (buse par défaut) puis
	// entre dans la bibliothèque. Aucune sortie sans imprimante (Confirmer inactif
	// à 0) : la garde `requireOnboardedUser` ramène ici tant qu'aucune n'existe.
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { ApiError } from '$lib/api/client';
	import { createPrinter } from '$lib/api/printers';
	import PrinterCatalog from '$lib/printers/PrinterCatalog.svelte';
	import type { PrinterCatalogModel } from '$lib/api/types';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	let busy = $state(false);
	let error = $state<string | null>(null);

	async function confirm(models: PrinterCatalogModel[]) {
		if (models.length === 0) return;
		busy = true;
		error = null;
		try {
			// Une imprimante par modèle : nom = modèle, buse par défaut (0.4 mm).
			for (const m of models) {
				await createPrinter({ name: m.model, machine_preset_id: m.default_machine_preset_id });
			}
			await goto(resolve('/library'));
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Erreur inattendue';
		} finally {
			busy = false;
		}
	}
</script>

<div class="mx-auto flex h-screen max-w-4xl flex-col p-4">
	<header class="mb-3 text-center">
		<h1 class="text-2xl font-semibold text-accent">Sélection de l'imprimante</h1>
		<p class="text-sm text-content-muted">
			Choisissez la ou les imprimantes que vous possédez pour commencer.
		</p>
	</header>

	{#if error}
		<p class="mb-2 text-center text-sm text-danger" role="alert">{error}</p>
	{/if}

	<div class="min-h-0 flex-1 overflow-hidden rounded border border-border bg-surface-raised">
		<PrinterCatalog vendors={data.catalog} onconfirm={confirm} confirmLabel="Confirmer" {busy} />
	</div>
</div>
