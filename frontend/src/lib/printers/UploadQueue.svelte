<script lang="ts">
	// Historique des envois (lot 42 M3) — équivalent de la « Print host upload
	// queue » d'Orca (PrintHostQueueDialog : état, hôte, taille, fichier,
	// message d'erreur), conservé par imprimante côté serveur. `printerId` absent
	// = toutes les imprimantes du compte (vue unique, comme la file d'Orca).
	import Dialog from '$lib/ui/Dialog.svelte';
	import { clearPrinterUploads, listAllUploads, listPrinterUploads } from '$lib/api/printers';
	import type { PrinterUploadEntry } from '$lib/api/types';
	import { hostMeta } from './hosts';
	import {
		formatUploadDate,
		formatUploadSize,
		hasPendingUploads,
		uploadActionLabel,
		uploadPrinterIds,
		uploadStatusLabel
	} from './uploads';
	import { sendErrorMessage } from './send';
	import { t } from '$lib/i18n';

	interface Props {
		open: boolean;
		/** Imprimante dont on affiche l'historique ; absent = toutes. */
		printerId?: string | null;
		/** Nom affiché dans le titre (imprimante seule). */
		printerName?: string | null;
		/** Lignes à mettre en évidence (envoi qui vient d'être fait). */
		highlight?: readonly string[];
		onclose: () => void;
	}

	let { open, printerId = null, printerName = null, highlight = [], onclose }: Props = $props();

	let entries = $state<PrinterUploadEntry[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let confirmClear = $state(false);
	let clearing = $state(false);
	let selected = $state<string | null>(null);
	const allPrinters = $derived(!printerId);
	const selectedEntry = $derived(entries.find((e) => e.id === selected) ?? null);

	async function load() {
		loading = true;
		error = null;
		try {
			entries = printerId ? await listPrinterUploads(printerId) : await listAllUploads();
		} catch (e) {
			error = sendErrorMessage(e);
		} finally {
			loading = false;
		}
	}

	// Chargement à l'ouverture ; rafraîchi tant qu'un envoi est en cours.
	let loadedFor = '';
	$effect(() => {
		const key = open ? (printerId ?? '*') : '';
		if (key === loadedFor) return;
		loadedFor = key;
		confirmClear = false;
		selected = null;
		if (key) void load();
		else entries = [];
	});
	$effect(() => {
		if (!open || !hasPendingUploads(entries)) return;
		const timer = setTimeout(() => void load(), 1500);
		return () => clearTimeout(timer);
	});

	async function clear() {
		if (!confirmClear) {
			confirmClear = true;
			return;
		}
		clearing = true;
		error = null;
		try {
			const ids = printerId ? [printerId] : uploadPrinterIds(entries);
			for (const id of ids) await clearPrinterUploads(id);
			confirmClear = false;
			await load();
		} catch (e) {
			error = sendErrorMessage(e);
		} finally {
			clearing = false;
		}
	}

	function statusClass(status: string): string {
		if (status === 'completed') return 'text-success-text';
		if (status === 'error') return 'text-danger-text';
		return 'text-content-muted';
	}
	// Libellés calculés hors gabarit : relus au changement de langue.
	function label(fn: () => string): string {
		void $t;
		return fn();
	}
</script>

<Dialog
	{open}
	label={$t('Print host upload queue')}
	title={printerName
		? $t('{title} — {name}', { title: $t('Print host upload queue'), name: printerName })
		: $t('Print host upload queue')}
	size="xl"
	{onclose}
	data-testid="upload-queue"
>
	<div class="flex flex-col gap-3">
		{#if error}
			<p
				class="rounded-md bg-danger-soft px-3 py-2 text-sm text-danger-content"
				role="alert"
				data-testid="upload-queue-error"
			>
				{error}
			</p>
		{/if}
		{#if loading && entries.length === 0}
			<p class="text-sm text-content-muted" role="status">{$t('Loading...')}</p>
		{:else if entries.length === 0}
			<p class="ws-empty text-sm text-content-muted" data-testid="upload-queue-empty">
				{$t('No upload yet: files sent to a printer are listed here.')}
			</p>
		{:else}
			<div class="max-h-[55vh] overflow-auto rounded-md border border-border">
				<table class="w-full text-left text-xs" data-testid="upload-queue-table">
					<thead class="sticky top-0 bg-surface-sunken text-content-muted">
						<tr>
							<th class="px-2 py-1.5 font-medium">{$t('ID')}</th>
							<th class="px-2 py-1.5 font-medium">{$t('Status')}</th>
							{#if allPrinters}<th class="px-2 py-1.5 font-medium">{$t('Printer')}</th>{/if}
							<th class="px-2 py-1.5 font-medium">{$t('Host')}</th>
							<th class="px-2 py-1.5 text-right font-medium">{$t('Size')}</th>
							<th class="px-2 py-1.5 font-medium">{$t('Filename')}</th>
							<th class="px-2 py-1.5 font-medium">{$t('Date')}</th>
							<th class="px-2 py-1.5 font-medium">{$t('Message')}</th>
						</tr>
					</thead>
					<tbody>
						{#each entries as entry, i (entry.id)}
							<tr
								class="border-t border-border {highlight.includes(entry.id) ? 'bg-primary/10' : ''}"
								data-testid="upload-row"
								data-upload-id={entry.id}
								data-status={entry.status}
								data-highlight={highlight.includes(entry.id)}
							>
								<td class="px-2 py-1.5 tabular-nums text-content-subtle">{entries.length - i}</td>
								<td class="px-2 py-1.5 {statusClass(entry.status)}" data-testid="upload-status"
									>{label(() =>
										uploadStatusLabel(entry.status)
									)}{#if entry.status === 'completed' && entry.action !== 'upload'}<span
											class="text-content-subtle"
										>
											· {label(() => uploadActionLabel(entry.action))}</span
										>{/if}</td
								>
								{#if allPrinters}<td class="px-2 py-1.5">{entry.printer_name ?? '—'}</td>{/if}
								<td class="max-w-[12rem] truncate px-2 py-1.5" title={entry.host}
									>{hostMeta(entry.host_type).label} · {entry.host}</td
								>
								<td class="px-2 py-1.5 text-right tabular-nums"
									>{label(() => formatUploadSize(entry.size_bytes))}</td
								>
								<td
									class="max-w-[16rem] truncate px-2 py-1.5 font-medium"
									title={entry.path ?? entry.filename}
									data-testid="upload-filename">{entry.filename}</td
								>
								<td class="px-2 py-1.5 whitespace-nowrap tabular-nums"
									>{label(() => formatUploadDate(entry.created_at))}</td
								>
								<td class="max-w-[14rem] px-2 py-1.5 text-danger-text" data-testid="upload-message">
									{#if entry.error}
										<button
											type="button"
											class="block max-w-full truncate text-left underline decoration-dotted"
											title={entry.error}
											aria-expanded={selected === entry.id}
											onclick={() => (selected = selected === entry.id ? null : entry.id)}
											>{entry.error}</button
										>
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			{#if selectedEntry?.error}
				<!-- Orca : « Show error message » — message complet de la ligne choisie. -->
				<p
					class="rounded-md bg-danger-soft px-3 py-2 text-xs break-words text-danger-content"
					data-testid="upload-error-detail"
				>
					{selectedEntry.error}
				</p>
			{/if}
		{/if}
	</div>
	{#snippet footer()}
		<button
			type="button"
			class="ws-btn ws-btn-ghost mr-auto"
			data-testid="upload-queue-clear"
			disabled={clearing || entries.length === 0}
			onclick={clear}
			>{confirmClear
				? allPrinters
					? $t('Clear the history of all printers?')
					: $t('Clear the history?')
				: $t('Clear history')}</button
		>
		<button
			type="button"
			class="ws-btn ws-btn-secondary"
			data-testid="upload-queue-refresh"
			disabled={loading}
			onclick={load}>{$t('Refresh')}</button
		>
		<button type="button" class="ws-btn ws-btn-primary" onclick={onclose}>{$t('Close')}</button>
	{/snippet}
</Dialog>
