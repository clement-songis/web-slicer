<script lang="ts">
	// Page file d'attente (T071/US7) : file active (états, progression, annulation),
	// historique, et mises à jour temps réel via WebSocket (`job.updated` /
	// `job.finished`, FR-031). La logique de file est pure (`lib/queue/queue.ts`) ;
	// cette page orchestre chargement, abonnement et actions.
	import { onMount } from 'svelte';
	import { ApiError } from '$lib/api/client';
	import { cancelJob } from '$lib/api/jobs';
	import type { JobResponse } from '$lib/api/types';
	import { subscribeEvents } from '$lib/queue/events';
	import { applyEvent, partitionJobs, progressPercent, statusMeta } from '$lib/queue/queue';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	let jobs = $state<JobResponse[]>([]);
	let error = $state<string | null>(null);
	let justFinished = $state<string | null>(null);
	let busy = $state<Record<string, boolean>>({});

	const parts = $derived(partitionJobs(jobs));

	onMount(() => {
		jobs = data.jobs;
		// Abonnement temps réel : chaque événement patche la liste ; un
		// `job.finished` déclenche une notification éphémère (US7-AS1).
		const sub = subscribeEvents({
			onEvent(event) {
				jobs = applyEvent(jobs, event);
				if (event.event === 'job.finished') {
					justFinished = event.id;
					setTimeout(() => {
						if (justFinished === event.id) justFinished = null;
					}, 5000);
				}
			}
		});
		return () => sub.close();
	});

	async function cancel(id: string) {
		busy = { ...busy, [id]: true };
		error = null;
		try {
			const updated = await cancelJob(id);
			jobs = jobs.map((j) => (j.id === updated.id ? updated : j));
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Annulation impossible';
		} finally {
			busy = { ...busy, [id]: false };
		}
	}

	function formatDate(rfc3339: string): string {
		const d = new Date(rfc3339);
		return Number.isNaN(d.getTime()) ? rfc3339 : d.toLocaleString();
	}
</script>

<svelte:head><title>File de tranchage</title></svelte:head>

<main class="mx-auto flex max-w-3xl flex-col gap-6 p-6 text-slate-200">
	<h1 class="text-xl font-semibold text-slate-100">File de tranchage</h1>

	{#if error}
		<p class="rounded bg-red-900/40 px-3 py-2 text-red-300" role="alert">{error}</p>
	{/if}

	{#if justFinished}
		<p class="rounded bg-green-900/40 px-3 py-2 text-green-300" role="status">
			Tranchage terminé — G-code disponible.
		</p>
	{/if}

	<!-- File active -->
	<section class="flex flex-col gap-2" aria-label="File active">
		<h2 class="text-sm font-semibold text-slate-300">En cours ({parts.active.length})</h2>
		{#if parts.active.length === 0}
			<p class="text-sm text-slate-500">Aucun tranchage en cours.</p>
		{:else}
			{#each parts.active as job (job.id)}
				<article class="flex items-center gap-3 rounded border border-slate-700 p-3">
					<span class="rounded px-2 py-0.5 text-xs {statusMeta(job.status).badge}">
						{statusMeta(job.status).label}
					</span>
					<div class="flex min-w-0 flex-1 flex-col gap-1">
						<div class="flex justify-between text-sm">
							<span class="truncate">Plateau {Number(job.plate_index) + 1}</span>
							<span class="tabular-nums text-slate-400">{progressPercent(job)}%</span>
						</div>
						<div class="h-1.5 overflow-hidden rounded bg-slate-700">
							<div class="h-full bg-sky-500" style:width="{progressPercent(job)}%"></div>
						</div>
						{#if job.phase}<span class="text-xs text-slate-500">{job.phase}</span>{/if}
					</div>
					<button
						type="button"
						class="rounded bg-slate-700 px-2 py-1 text-xs hover:bg-slate-600 disabled:opacity-50"
						disabled={busy[job.id]}
						onclick={() => cancel(job.id)}
					>
						Annuler
					</button>
				</article>
			{/each}
		{/if}
	</section>

	<!-- Historique -->
	<section class="flex flex-col gap-2" aria-label="Historique">
		<h2 class="text-sm font-semibold text-slate-300">Historique ({parts.history.length})</h2>
		{#if parts.history.length === 0}
			<p class="text-sm text-slate-500">Aucun tranchage terminé.</p>
		{:else}
			<table class="w-full border-collapse text-sm">
				<thead>
					<tr class="text-left text-xs text-slate-400">
						<th class="font-normal">État</th>
						<th class="font-normal">Plateau</th>
						<th class="font-normal">Date</th>
						<th class="font-normal">Résultat</th>
					</tr>
				</thead>
				<tbody>
					{#each parts.history as job (job.id)}
						<tr class="border-t border-slate-800">
							<td class="py-1">
								<span class="rounded px-2 py-0.5 text-xs {statusMeta(job.status).badge}">
									{statusMeta(job.status).label}
								</span>
							</td>
							<td class="tabular-nums">{Number(job.plate_index) + 1}</td>
							<td class="text-slate-400">{formatDate(job.updated_at)}</td>
							<td>
								{#if job.gcode_id}
									<!-- Ressource API (téléchargement backend), pas une route SvelteKit. -->
									<!-- eslint-disable svelte/no-navigation-without-resolve -->
									<a
										class="text-sky-400 hover:underline"
										href="/api/gcodes/{job.gcode_id}/download"
										download
									>
										Télécharger
									</a>
									<!-- eslint-enable svelte/no-navigation-without-resolve -->
								{:else if job.status === 'failed'}
									<span class="text-red-400">Échec</span>
								{:else}
									<span class="text-slate-500">—</span>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</section>
</main>
