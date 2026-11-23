<script lang="ts">
	// Page file d'attente (T071/US7) : file active (états, progression, annulation),
	// historique, et mises à jour temps réel via WebSocket (`job.updated` /
	// `job.finished`, FR-031). La logique de file est pure (`lib/queue/queue.ts`) ;
	// cette page orchestre chargement, abonnement et actions.
	import { onMount } from 'svelte';
	import { t, tr } from '$lib/i18n';
	import { ApiError } from '$lib/api/client';
	import { cancelJob, getJob, listJobs } from '$lib/api/jobs';
	import { resolve } from '$app/paths';
	import { connection } from '$lib/api/connectivity';
	import { uploadToPrinter } from '$lib/api/printers';
	import type { JobResponse, PrinterResponse } from '$lib/api/types';
	import { subscribeEvents } from '$lib/queue/events';
	import {
		applyEvent,
		jobErrorText,
		partitionJobs,
		phaseLabel,
		progressPercent,
		queuePositions,
		statusMeta,
		unknownJobId,
		upsertJob
	} from '$lib/queue/queue';
	import type { PageData } from './$types';
	import AppSidebar from '$lib/nav/AppSidebar.svelte';

	let { data }: { data: PageData } = $props();

	let jobs = $state<JobResponse[]>([]);
	const printers = $derived(data.printers as PrinterResponse[]);
	let error = $state<string | null>(null);
	let justFinished = $state<string | null>(null);
	let busy = $state<Record<string, boolean>>({});
	// Cible d'envoi par job (imprimante sélectionnée + démarrage immédiat).
	let sendTarget = $state<Record<string, string>>({});
	let sendStart = $state<Record<string, boolean>>({});
	let sent = $state<Record<string, string>>({});

	const parts = $derived(partitionJobs(jobs));
	const positions = $derived(queuePositions(jobs));
	const projectNames = $derived(
		Object.fromEntries((data.projects ?? []).map((p) => [p.id, p.name])) as Record<string, string>
	);
	const projectName = (id: string) => projectNames[id] ?? $t('Project');

	// Phase 32 : resynchronisation après une coupure (événements perdus) et
	// jobs inconnus (lancés depuis un autre onglet après le chargement).
	async function refresh() {
		try {
			jobs = await listJobs();
		} catch {
			// hors ligne : le bandeau global l'indique, nouvel essai au retour
		}
	}
	const fetching: Record<string, boolean> = {};
	async function adopt(id: string) {
		if (fetching[id]) return;
		fetching[id] = true;
		try {
			jobs = upsertJob(jobs, await getJob(id));
		} catch {
			// job d'un autre compte ou déjà purgé : ignoré
		} finally {
			delete fetching[id];
		}
	}

	onMount(() => {
		jobs = data.jobs;
		// Abonnement temps réel : chaque événement patche la liste ; un
		// `job.finished` déclenche une notification éphémère (US7-AS1).
		let lastRestored = 0;
		const unsubConn = connection.subscribe((c) => {
			if (c.restored !== lastRestored) {
				lastRestored = c.restored;
				if (c.restored > 0) void refresh();
			}
		});
		const sub = subscribeEvents({
			onOpen(reconnected) {
				if (reconnected) void refresh();
			},
			onEvent(event) {
				const unknown = unknownJobId(jobs, event);
				if (unknown) void adopt(unknown);
				jobs = applyEvent(jobs, event);
				if (event.event === 'job.finished') {
					justFinished = event.id;
					setTimeout(() => {
						if (justFinished === event.id) justFinished = null;
					}, 5000);
				}
			}
		});
		return () => {
			sub.close();
			unsubConn();
		};
	});

	async function cancel(id: string) {
		busy = { ...busy, [id]: true };
		error = null;
		try {
			const updated = await cancelJob(id);
			jobs = jobs.map((j) => (j.id === updated.id ? updated : j));
		} catch (e) {
			error = e instanceof ApiError ? e.message : tr('Could not cancel');
		} finally {
			busy = { ...busy, [id]: false };
		}
	}

	async function sendToPrinter(job: JobResponse) {
		const printerId = sendTarget[job.id] || printers[0]?.id;
		if (!job.gcode_id || !printerId) return;
		busy = { ...busy, [job.id]: true };
		error = null;
		try {
			await uploadToPrinter(printerId, {
				gcode_id: job.gcode_id,
				start_now: sendStart[job.id] ?? false
			});
			sent = { ...sent, [job.id]: tr('Sent to printer') };
		} catch (e) {
			error = e instanceof ApiError ? e.message : tr('Could not send');
		} finally {
			busy = { ...busy, [job.id]: false };
		}
	}

	function formatDate(rfc3339: string): string {
		const d = new Date(rfc3339);
		return Number.isNaN(d.getTime()) ? rfc3339 : d.toLocaleString();
	}
</script>

<svelte:head><title>{$t('Slicing queue')}</title></svelte:head>

<div class="flex h-screen bg-surface text-content">
	<AppSidebar email={data.user.email} active="queue" />
	<div class="flex-1 overflow-auto">
		<main class="mx-auto flex max-w-3xl flex-col gap-6 p-6 text-content">
			<h1 class="ws-page-title">{$t('Slicing queue')}</h1>

			{#if error}
				<p class="rounded bg-danger-soft px-3 py-2 text-danger-content" role="alert">{error}</p>
			{/if}

			{#if justFinished}
				<p class="rounded bg-success-soft px-3 py-2 text-success-content" role="status">
					{$t('Slicing complete — G-code available.')}
				</p>
			{/if}

			<!-- File active -->
			<section class="flex flex-col gap-2" aria-label={$t('Active queue')}>
				<h2 class="ws-section-title">{$t('In progress ({n})', { n: parts.active.length })}</h2>
				{#if parts.active.length === 0}
					<p class="ws-empty rounded-lg border border-dashed border-border py-6">
						{$t('No slicing in progress.')}
					</p>
				{:else}
					{#each parts.active as job (job.id)}
						<article class="flex items-center gap-3 rounded border border-border p-3">
							<span
								class="rounded px-2 py-0.5 text-xs {statusMeta(job.status).badge}"
								data-testid="queue-status"
							>
								{statusMeta(job.status).label}{#if positions[job.id]}&nbsp;· {$t(
										'#{n}||queue position',
										{
											n: positions[job.id]
										}
									)}{/if}
							</span>
							<div class="flex min-w-0 flex-1 flex-col gap-1">
								<div class="flex justify-between gap-2 text-sm">
									<a
										class="truncate hover:underline"
										href={resolve('/projects/[id]', { id: job.project_id })}
										data-testid="queue-job-project"
										title={projectName(job.project_id)}
									>
										{projectName(job.project_id)}
										<span class="text-content-muted">
											· {$t('Plate {n}', { n: Number(job.plate_index) + 1 })}
										</span>
									</a>
									<span class="text-content-muted tabular-nums">{progressPercent(job)}%</span>
								</div>
								<div class="h-1.5 overflow-hidden rounded bg-overlay">
									<div class="h-full bg-primary" style:width="{progressPercent(job)}%"></div>
								</div>
								{#if job.status === 'queued'}
									<span class="text-xs text-content-subtle">
										{positions[job.id] === 1
											? $t('Waiting: starts as soon as a slice finishes')
											: $t('Waiting: {n} slice(s) ahead of this one', {
													n: (positions[job.id] ?? 1) - 1
												})}
									</span>
								{:else if job.phase}<span class="text-xs text-content-subtle">
										{phaseLabel(job.phase)}
									</span>{/if}
							</div>
							<button
								type="button"
								class="ws-btn ws-btn-sm ws-btn-secondary"
								disabled={busy[job.id]}
								onclick={() => cancel(job.id)}
							>
								{$t('Cancel')}
							</button>
						</article>
					{/each}
				{/if}
			</section>

			<!-- Historique -->
			<section class="flex flex-col gap-2" aria-label={$t('History')}>
				<h2 class="ws-section-title">
					{$t('History ({n})', { n: parts.history.length })}
				</h2>
				{#if parts.history.length === 0}
					<p class="ws-empty rounded-lg border border-dashed border-border py-6">
						{$t('No completed slices.')}
					</p>
				{:else}
					<table class="w-full border-collapse text-sm">
						<thead>
							<tr class="text-left text-xs text-content-muted">
								<th class="font-normal">{$t('Status')}</th>
								<th class="font-normal">{$t('Project')}</th>
								<th class="font-normal">{$t('Plate')}</th>
								<th class="font-normal">{$t('Date')}</th>
								<th class="font-normal">{$t('Result')}</th>
							</tr>
						</thead>
						<tbody>
							{#each parts.history as job (job.id)}
								<tr class="border-t border-border">
									<td class="py-1">
										<span class="rounded px-2 py-0.5 text-xs {statusMeta(job.status).badge}">
											{statusMeta(job.status).label}
										</span>
									</td>
									<td class="max-w-48 truncate" title={projectName(job.project_id)}>
										<a
											class="hover:underline"
											href={resolve('/projects/[id]', { id: job.project_id })}
											data-testid="queue-history-project"
										>
											{projectName(job.project_id)}
										</a>
									</td>
									<td class="tabular-nums">{Number(job.plate_index) + 1}</td>
									<td class="text-content-muted">{formatDate(job.updated_at)}</td>
									<td>
										{#if job.gcode_id}
											<div class="flex flex-col gap-1">
												<!-- Ressource API (téléchargement backend), pas une route SvelteKit. -->
												<!-- eslint-disable svelte/no-navigation-without-resolve -->
												<a
													class="text-primary hover:underline"
													href="/api/gcodes/{job.gcode_id}/download"
													download
												>
													{$t('Download')}
												</a>
												<!-- eslint-enable svelte/no-navigation-without-resolve -->
												{#if printers.length > 0}
													<!-- Envoi vers une imprimante déclarée (start_now, FR-061). -->
													<div class="flex flex-wrap items-center gap-1 text-xs">
														<select
															class="ws-field ws-field-sm"
															bind:value={sendTarget[job.id]}
															aria-label={$t('Target printer')}
														>
															{#each printers as printer (printer.id)}
																<option value={printer.id}>{printer.name}</option>
															{/each}
														</select>
														<label class="flex items-center gap-1 text-content-muted">
															<input type="checkbox" bind:checked={sendStart[job.id]} />
															{$t('Start')}
														</label>
														<button
															type="button"
															class="ws-btn ws-btn-sm ws-btn-secondary"
															disabled={busy[job.id]}
															onclick={() => sendToPrinter(job)}
														>
															{$t('Send')}
														</button>
													</div>
													{#if sent[job.id]}
														<span class="text-xs text-success" role="status">{sent[job.id]}</span>
													{/if}
												{/if}
											</div>
										{:else if job.status === 'failed'}
											<span class="text-danger" data-testid="queue-job-error">
												{jobErrorText(job.error)
													? $t('Failed: {error}', { error: jobErrorText(job.error) })
													: $t('Failed||job')}
											</span>
										{:else}
											<span class="text-content-subtle">—</span>
										{/if}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				{/if}
			</section>
		</main>
	</div>
</div>
