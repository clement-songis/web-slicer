<script lang="ts">
	// Onglet « Appareil » de l'éditeur (T117, US8) : liste les imprimantes Moonraker
	// déclarées, leur état d'impression (badge, progression, températures) et les
	// contrôles pause/reprise/annulation. La logique d'état reste pure
	// (`lib/printers/printers.ts`) ; ce composant orchestre transport + affichage,
	// comme la route `/printers`.
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { ApiError } from '$lib/api/client';
	import {
		listPrinters,
		getPrinterStatus,
		pausePrinter,
		resumePrinter,
		cancelPrinter
	} from '$lib/api/printers';
	import {
		fromStatusResponse,
		stateMeta,
		progressPercent,
		formatTemp,
		canPause,
		canResume,
		canCancel,
		type StatusMap
	} from './printers';
	import type { PrinterResponse } from '$lib/api/types';

	let printers = $state<PrinterResponse[]>([]);
	let statuses = $state<StatusMap>({});
	let loading = $state(true);
	let error = $state<string | null>(null);

	async function refresh(id: string) {
		try {
			const s = await getPrinterStatus(id);
			statuses = { ...statuses, [id]: fromStatusResponse(s) };
		} catch (e) {
			// Une imprimante hors ligne ne bloque pas les autres.
			statuses = { ...statuses };
			if (e instanceof ApiError) error = e.message;
		}
	}

	async function control(id: string, action: 'pause' | 'resume' | 'cancel') {
		try {
			if (action === 'pause') await pausePrinter(id);
			else if (action === 'resume') await resumePrinter(id);
			else await cancelPrinter(id);
			await refresh(id);
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'commande refusée';
		}
	}

	onMount(async () => {
		try {
			printers = await listPrinters();
			await Promise.all(printers.map((p) => refresh(p.id)));
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'chargement des imprimantes impossible';
		} finally {
			loading = false;
		}
	});
</script>

<div class="mx-auto flex w-full max-w-3xl flex-col gap-4 p-6">
	<div class="flex items-center justify-between">
		<h2 class="text-lg font-semibold text-content">Appareils</h2>
		<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
		<a href={resolve('/printers')} class="text-sm text-primary hover:underline"
			>Gérer les imprimantes</a
		>
	</div>

	{#if error}
		<div class="rounded bg-danger-soft px-3 py-2 text-sm text-danger-content">{error}</div>
	{/if}

	{#if loading}
		<p class="text-sm text-content-subtle">Chargement…</p>
	{:else if printers.length === 0}
		<div class="rounded border border-border bg-surface-raised p-6 text-center text-content-muted">
			<p>Aucune imprimante déclarée.</p>
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
			<a href={resolve('/printers')} class="text-primary hover:underline">Déclarer une imprimante</a
			>
		</div>
	{:else}
		{#each printers as printer (printer.id)}
			{@const st = statuses[printer.id]}
			{@const meta = stateMeta(st?.state ?? '')}
			<div class="flex flex-col gap-2 rounded border border-border bg-surface-raised p-4">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-2">
						<span class="font-medium text-content">{printer.name}</span>
						<span class="rounded px-2 py-0.5 text-xs {meta.badge}">{meta.label}</span>
					</div>
					<button class="text-xs text-primary hover:underline" onclick={() => refresh(printer.id)}
						>Rafraîchir</button
					>
				</div>
				<p class="truncate text-xs text-content-subtle">{printer.moonraker_url}</p>

				{#if st}
					<div class="h-2 w-full overflow-hidden rounded bg-overlay">
						<div class="h-full bg-primary" style="width: {progressPercent(st.progress)}%"></div>
					</div>
					<dl class="grid grid-cols-2 gap-x-4 text-xs text-content-muted">
						<div class="flex justify-between">
							<dt>Buse</dt>
							<dd class="tabular-nums">{formatTemp(st.extruderTemp, st.extruderTarget)}</dd>
						</div>
						<div class="flex justify-between">
							<dt>Plateau</dt>
							<dd class="tabular-nums">{formatTemp(st.bedTemp, st.bedTarget)}</dd>
						</div>
					</dl>
					{#if st.filename}
						<p class="truncate text-xs text-content-muted">Fichier : {st.filename}</p>
					{/if}
					<div class="flex gap-2">
						<button
							class="rounded border border-border-strong px-2 py-1 text-xs hover:bg-overlay disabled:opacity-40"
							disabled={!canPause(st.state)}
							onclick={() => control(printer.id, 'pause')}>Pause</button
						>
						<button
							class="rounded border border-border-strong px-2 py-1 text-xs hover:bg-overlay disabled:opacity-40"
							disabled={!canResume(st.state)}
							onclick={() => control(printer.id, 'resume')}>Reprendre</button
						>
						<button
							class="rounded border border-border-strong px-2 py-1 text-xs hover:bg-overlay disabled:opacity-40"
							disabled={!canCancel(st.state)}
							onclick={() => control(printer.id, 'cancel')}>Annuler</button
						>
					</div>
				{:else}
					<p class="text-xs text-content-subtle">État indisponible (imprimante injoignable).</p>
				{/if}
			</div>
		{/each}
	{/if}
</div>
