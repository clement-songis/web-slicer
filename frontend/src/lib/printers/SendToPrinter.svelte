<script lang="ts">
	import { dialogFocus } from '$lib/ui/dialog-focus';
	// Envoi du G-code tranché vers une imprimante Moonraker (FR-061/FR-062) :
	// bouton + dialogue (cible, démarrage immédiat, succès/erreur lisibles).
	// Autonome : la page ne fournit que l'identifiant du G-code courant.
	import IconSend from '~icons/lucide/send';
	import { resolve } from '$app/paths';
	import {
		listPrinters,
		updatePrinter,
		uploadToPrinter,
		type UploadSlicedResponse
	} from '$lib/api/printers';
	import type { PrinterResponse } from '$lib/api/types';
	import {
		normalizeMoonrakerUrl,
		printerOptionLabel,
		sendChoices,
		sendErrorMessage,
		sendSuccessMessage
	} from './send';

	interface Props {
		/** G-code tranché courant ; absent → bouton désactivé. */
		gcodeId: string | null | undefined;
		/** Incrémenté par Ctrl+Maj+G pour ouvrir le même dialogue que le bouton. */
		openRequest?: number;
		/** Nombre de plateaux du projet : au-delà de 1, le dialogue propose
		 *  « Tous les plateaux » (Orca « Send all » / « Print all »). */
		plateCount?: number;
		/** Tranche tous les plateaux puis les envoie (POST …/upload-sliced). */
		sendAll?: (
			printerId: string,
			startNow: boolean,
			progress: (message: string) => void
		) => Promise<UploadSlicedResponse>;
	}

	let { gcodeId, openRequest = 0, plateCount = 1, sendAll }: Props = $props();

	const canSendAll = $derived(!!sendAll && plateCount > 1);
	/** Portée de l'envoi : plateau tranché courant, ou tous les plateaux. */
	let scope = $state<'plate' | 'all'>('plate');
	let progressMessage = $state<string | null>(null);

	let open = $state(false);
	let loading = $state(false);
	let sending = $state(false);
	let targets = $state<PrinterResponse[]>([]);
	let printerId = $state('');
	let startNow = $state(false);
	let success = $state<string | null>(null);
	let error = $state<string | null>(null);

	const selected = $derived(targets.find((p) => p.id === printerId));
	// Imprimante choisie sans URL Moonraker (ex. celle créée à l'inscription) :
	// l'URL se saisit ici, sans quitter l'éditeur (Orca : bouton de connexion
	// à côté du choix d'imprimante, Plater.cpp `m_printer_connect`).
	const needsUrl = $derived(!!selected && !selected.moonraker_url?.trim());
	let urlDraft = $state('');
	let savingUrl = $state(false);
	let urlError = $state<string | null>(null);

	async function saveUrl() {
		if (!selected || savingUrl) return;
		const url = normalizeMoonrakerUrl(urlDraft);
		if (!url) {
			urlError = 'URL invalide : par exemple http://192.168.1.20:7125';
			return;
		}
		savingUrl = true;
		urlError = null;
		try {
			const saved = await updatePrinter(selected.id, {
				name: selected.name,
				moonraker_url: url,
				machine_preset_id: selected.machine_preset_id
			});
			targets = targets.map((p) => (p.id === saved.id ? saved : p));
			printerId = saved.id;
			urlDraft = '';
		} catch (e) {
			urlError = sendErrorMessage(e, selected.name);
		} finally {
			savingUrl = false;
		}
	}

	async function openDialog() {
		open = true;
		// Sans G-code courant, seul l'envoi de tous les plateaux est possible.
		scope = !gcodeId && canSendAll ? 'all' : scope === 'all' && canSendAll ? 'all' : 'plate';
		progressMessage = null;
		success = null;
		error = null;
		loading = true;
		try {
			targets = sendChoices(await listPrinters());
			if (!targets.some((p) => p.id === printerId)) printerId = targets[0]?.id ?? '';
		} catch (e) {
			error = sendErrorMessage(e);
		} finally {
			loading = false;
		}
	}

	let handledOpenRequest = $state(0);
	$effect(() => {
		if (openRequest !== handledOpenRequest) {
			handledOpenRequest = openRequest;
			if (openRequest > 0 && (gcodeId || canSendAll) && !open) void openDialog();
		}
	});

	async function send() {
		if (!selected || sending) return;
		if (scope === 'plate' && !gcodeId) return;
		// URL saisie mais pas encore enregistrée : l'enregistrer d'abord (pas
		// d'étape cachée « Enregistrer l'URL » avant de pouvoir envoyer).
		if (needsUrl) {
			await saveUrl();
			if (needsUrl) return;
		}
		sending = true;
		success = null;
		error = null;
		try {
			if (scope === 'all' && sendAll) {
				const res = await sendAll(selected.id, startNow, (m) => (progressMessage = m));
				success =
					res.files.length === 1
						? sendSuccessMessage(selected.name, res.files[0])
						: `${res.files.length} fichiers envoyés à ${selected.name} : ${res.files.map((f) => f.path).join(', ')}.`;
			} else if (gcodeId) {
				const result = await uploadToPrinter(selected.id, {
					gcode_id: gcodeId,
					start_now: startNow
				});
				success = sendSuccessMessage(selected.name, result);
			}
		} catch (e) {
			error = sendErrorMessage(e, selected.name);
		} finally {
			sending = false;
			progressMessage = null;
		}
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && !sending) open = false;
	}
</script>

<svelte:window onkeydown={open ? onKeydown : undefined} />

<button
	type="button"
	class="ws-btn ws-btn-primary inline-flex"
	data-testid="send-to-printer"
	disabled={!gcodeId && !canSendAll}
	title={gcodeId || canSendAll ? "Envoyer le G-code à l'imprimante" : "Tranchez d'abord le plateau"}
	onclick={openDialog}
>
	<IconSend class="h-4 w-4" aria-hidden="true" />
	Envoyer
</button>

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4"
		role="presentation"
		onclick={() => !sending && (open = false)}
	>
		<div
			class="ws-dialog flex w-full max-w-md flex-col gap-4 p-5"
			role="dialog"
			aria-modal="true"
			aria-labelledby="send-title"
			data-testid="send-dialog"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key !== 'Escape' && e.stopPropagation()}
			use:dialogFocus
		>
			<h2 id="send-title" class="ws-dialog-title">Envoyer à l'imprimante</h2>

			{#if canSendAll}
				<div class="ws-seg flex w-full" role="group" aria-label="Plateaux à envoyer">
					<button
						type="button"
						class="ws-seg-item flex-1 {scope === 'plate' ? 'ws-seg-on' : ''}"
						aria-pressed={scope === 'plate'}
						data-testid="send-scope-plate"
						disabled={sending || !gcodeId}
						title={gcodeId ? undefined : "Tranchez d'abord le plateau actif"}
						onclick={() => (scope = 'plate')}>Plateau tranché</button
					>
					<button
						type="button"
						class="ws-seg-item flex-1 {scope === 'all' ? 'ws-seg-on' : ''}"
						aria-pressed={scope === 'all'}
						data-testid="send-scope-all"
						disabled={sending}
						onclick={() => (scope = 'all')}>Tous les plateaux ({plateCount})</button
					>
				</div>
				{#if scope === 'all'}
					<p class="text-xs text-content-subtle">
						Tous les plateaux sont tranchés puis envoyés : un seul .gcode.3mf si l'imprimante
						l'accepte, sinon un G-code par plateau (démarrage immédiat impossible dans ce cas).
					</p>
				{/if}
			{/if}

			{#if loading}
				<p class="text-sm text-content-muted">Chargement des imprimantes…</p>
			{:else if targets.length === 0 && !error}
				<p class="text-sm text-content-muted" data-testid="send-no-printer">
					Aucune imprimante déclarée.
					<a class="text-primary underline" href={resolve('/printers')}>Déclarer une imprimante</a>
				</p>
			{:else if targets.length > 0}
				<label class="flex flex-col gap-1 text-sm">
					<span class="text-content-muted">Imprimante</span>
					<select
						class="ws-field"
						bind:value={printerId}
						data-testid="send-printer-select"
						disabled={sending}
					>
						{#each targets as p (p.id)}
							<option value={p.id}>{printerOptionLabel(p)}</option>
						{/each}
					</select>
				</label>
				{#if needsUrl}
					<!-- novalidate : « 192.168.1.20:7125 » (sans schéma) est accepté et
					     normalisé par normalizeMoonrakerUrl. -->
					<form
						novalidate
						class="flex flex-col gap-1 text-sm"
						onsubmit={(e) => {
							e.preventDefault();
							void saveUrl();
						}}
					>
						<label class="flex flex-col gap-1">
							<span class="text-content-muted">URL Moonraker de {selected?.name}</span>
							<input
								type="url"
								inputmode="url"
								class="ws-field"
								placeholder="http://192.168.1.20:7125"
								bind:value={urlDraft}
								disabled={savingUrl}
								data-testid="send-moonraker-url"
							/>
						</label>
						<div class="flex flex-col items-end gap-1">
							<span class="w-full text-xs text-content-subtle"
								>Enregistrée dans l'imprimante à l'envoi, réutilisée ensuite.</span
							>
							<button
								type="submit"
								class="ws-btn ws-btn-sm ws-btn-secondary shrink-0"
								disabled={savingUrl || !urlDraft.trim()}
								data-testid="send-save-url"
							>
								{savingUrl ? 'Enregistrement…' : "Enregistrer l'URL"}
							</button>
						</div>
						{#if urlError}
							<p class="text-xs text-danger" role="alert" data-testid="send-url-error">
								{urlError}
							</p>
						{/if}
					</form>
				{/if}
				<label class="flex items-center gap-2 text-sm">
					<input
						type="checkbox"
						bind:checked={startNow}
						data-testid="send-start-now"
						disabled={sending}
					/>
					Démarrer l'impression dès réception
				</label>
			{/if}

			{#if progressMessage}
				<p class="text-sm text-content-muted" role="status" data-testid="send-progress">
					{progressMessage}
				</p>
			{/if}
			{#if success}
				<p
					class="rounded-md bg-success-soft px-3 py-2 text-sm text-success-content"
					role="status"
					data-testid="send-success"
				>
					{success}
				</p>
			{/if}
			{#if error}
				<p
					class="rounded-md bg-danger-soft px-3 py-2 text-sm text-danger-content"
					role="alert"
					data-testid="send-error"
				>
					{error}
				</p>
			{/if}

			<div class="flex justify-end gap-2">
				<button
					type="button"
					class="ws-btn ws-btn-lg {success ? 'ws-btn-primary' : 'ws-btn-secondary'}"
					disabled={sending}
					onclick={() => (open = false)}
				>
					{success ? 'Fermer' : 'Annuler'}
				</button>
				<button
					type="button"
					class="ws-btn ws-btn-lg {success ? 'ws-btn-secondary' : 'ws-btn-primary'}"
					data-testid="send-confirm"
					disabled={!selected ||
						(needsUrl && !urlDraft.trim()) ||
						(scope === 'plate' && !gcodeId) ||
						sending ||
						savingUrl ||
						loading}
					onclick={send}
				>
					{sending ? 'Envoi…' : success ? 'Renvoyer' : 'Envoyer'}
				</button>
			</div>
		</div>
	</div>
{/if}
