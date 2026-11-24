<script lang="ts">
	import { dialogFocus } from '#lib/ui/dialog-focus';
	// Envoi du G-code tranché vers une imprimante Moonraker (FR-061/FR-062) :
	// bouton + dialogue (cible, démarrage immédiat, succès/erreur lisibles).
	// Autonome : la page ne fournit que l'identifiant du G-code courant.
	import IconSend from '~icons/lucide/send';
	import { resolve } from '$app/paths';
	import { hasLiveControls, hostMeta, hostPrinterValue } from './hosts';
	import {
		getHostPrinters,
		getPrinterStorages,
		getUploadName,
		listPrinters,
		updatePrinter,
		uploadToPrinter,
		type UploadSlicedResponse
	} from '#lib/api/printers';
	import UploadQueue from './UploadQueue.svelte';
	import {
		joinUploadName,
		uploadNameErrorReason,
		uploadNameProblem,
		uploadNameReasonMessage,
		uploadNameStem
	} from './uploads';
	import type { PrinterResponse } from '#lib/api/types';
	import {
		normalizeMoonrakerUrl,
		printerOptionLabel,
		connectionHint,
		hostUrlError,
		isPrinterNotConnected,
		isPrinterIncompatible,
		presetMismatch,
		sendChoices,
		type MachinePresetRef,
		sendErrorMessage,
		sendSuccessMessage
	} from './send';
	import { t, tr } from '#lib/i18n';
	import { getResolvedPreset } from '#lib/api/presets';

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
			progress: (message: string) => void,
			queue?: boolean,
			/** Nom choisi (base commune des plateaux, lot 42 M2). */
			filename?: string
		) => Promise<UploadSlicedResponse>;
		/** Phase 46 (DEV1-a) : preset machine du projet (celui du tranchage). */
		machinePreset?: MachinePresetRef | null;
		/** Presets machine connus (résolution de l'héritage pour la comparaison). */
		machinePresets?: readonly MachinePresetRef[];
		/** Audit 46 BBL-a : « Export plate sliced file » (.gcode.3mf, carte SD),
		 *  proposé quand l'envoi direct est impossible (imprimantes Bambu Lab). */
		onexportsliced?: () => void;
	}

	let {
		gcodeId,
		openRequest = 0,
		plateCount = 1,
		sendAll,
		machinePreset = null,
		machinePresets = [],
		onexportsliced
	}: Props = $props();

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
	// `printer_model` résolu des presets (même règle que le 409 serveur du lot 46).
	// Cache de requêtes, non affiché.
	// eslint-disable-next-line svelte/prefer-svelte-reactivity
	const modelCache = new Map<string, Promise<string>>();
	function printerModel(id: string): Promise<string> {
		let entry = modelCache.get(id);
		if (!entry) {
			entry = getResolvedPreset(id)
				.then((r) => {
					const v = (r.values as Record<string, unknown>).printer_model;
					return typeof v === 'string' ? v : '';
				})
				.catch(() => '');
			modelCache.set(id, entry);
		}
		return entry;
	}
	let models = $state<{ key: string; sliced: string; target: string } | null>(null);
	$effect(() => {
		const a = machinePreset?.id;
		const b = selected?.machine_preset_id;
		if (!open || !a || !b || a === b) return;
		const key = `${a}|${b}`;
		if (models?.key === key) return;
		void Promise.all([printerModel(a), printerModel(b)]).then(([sliced, target]) => {
			if (machinePreset?.id === a && selected?.machine_preset_id === b)
				models = { key, sliced, target };
		});
	});
	/** G-code tranché pour une autre imprimante que celle choisie (Orca : envoi désactivé). */
	const mismatch = $derived.by(() => {
		const key = `${machinePreset?.id}|${selected?.machine_preset_id}`;
		// Modèles pas encore résolus : pas d'avertissement provisoire.
		if (models?.key !== key) return null;
		return presetMismatch(machinePreset, selected, machinePresets, models);
	});

	const declareHref = $derived(
		machinePreset
			? `${resolve('printers')}?preset=${encodeURIComponent(machinePreset.id)}`
			: resolve('printers')
	);

	// Imprimante choisie sans URL Moonraker (ex. celle créée à l'inscription) :
	// l'URL se saisit ici, sans quitter l'éditeur (Orca : bouton de connexion
	// à côté du choix d'imprimante, Plater.cpp `m_printer_connect`).
	// Audit 41 M1 : « Modifier l'URL » rouvre le champ pour l'imprimante choisie.
	let editingUrl = $state<string | null>(null);
	const needsUrl = $derived(
		!!selected && (!selected.moonraker_url?.trim() || editingUrl === selected.id)
	);
	// Destination sur l'hôte (Orca PrintHostSendDialog : stockage PrusaLink /
	// PrusaConnect, groupe Repetier), chargée à la sélection de l'imprimante.
	let storages = $state<{ kind: string; items: { path: string; name: string }[] } | null>(null);
	let storage = $state('');
	let storagesFor = '';
	$effect(() => {
		const p = selected;
		// Groupes Repetier : propres à l'imprimante du serveur (host_port).
		const key =
			p && p.moonraker_url && hostMeta(p.host_type).storages ? `${p.id}:${p.host_port ?? ''}` : '';
		if (key === storagesFor) return;
		storagesFor = key;
		storages = null;
		storage = '';
		if (!key) return;
		void getPrinterStorages(p!.id)
			.then((s) => {
				if (storagesFor !== key) return;
				storages = s;
				storage = s.items[0]?.path ?? '';
			})
			.catch(() => {
				if (storagesFor === key) storages = { kind: 'none', items: [] };
			});
	});
	// Imprimante du serveur Repetier (Orca : printhost_port), choisie ici sans
	// repasser par la page Imprimantes ; enregistrée dans l'imprimante au choix.
	const meta = $derived(hostMeta(selected?.host_type));
	let hostPrinters = $state<string[] | null>(null);
	let hostPrintersError = $state<string | null>(null);
	let hostPrintersFor = '';
	let savingSlug = $state(false);
	$effect(() => {
		const p = selected;
		const key = p && p.moonraker_url && hostMeta(p.host_type).printerSlug ? p.id : '';
		if (key === hostPrintersFor) return;
		hostPrintersFor = key;
		hostPrinters = null;
		hostPrintersError = null;
		if (!key) return;
		void getHostPrinters(key)
			.then((list) => {
				if (hostPrintersFor === key) hostPrinters = list;
			})
			.catch((e) => {
				if (hostPrintersFor === key)
					hostPrintersError = sendErrorMessage(e, p?.name) || tr('List unavailable.');
			});
	});
	async function chooseHostPrinter(slug: string) {
		const p = selected;
		if (!p || slug === (p.host_port ?? '')) return;
		savingSlug = true;
		error = null;
		try {
			const saved = await updatePrinter(p.id, {
				name: p.name,
				machine_preset_id: p.machine_preset_id,
				moonraker_url: p.moonraker_url,
				host_port: slug
			});
			targets = targets.map((t) => (t.id === saved.id ? saved : t));
		} catch (e) {
			error = sendErrorMessage(e, p.name);
		} finally {
			savingSlug = false;
		}
	}

	// Lot 42 M2 — nom du fichier envoyé (Orca PrintHostSendDialog : nom
	// proposé pré-rempli et modifiable, extension imposée affichée à part).
	// Pré-rempli par POST …/upload-name, validé en direct (contrôle local
	// immédiat puis serveur), transmis dans `filename`. Plusieurs plateaux :
	// base commune (suffixes _2, _3… côté serveur) ; vide = noms proposés.
	let nameStem = $state('');
	let nameExt = $state('.gcode');
	let nameEdited = $state(false);
	let nameServerReason = $state<string | null>(null);
	let nameFor = '';
	let nameTimer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		const p = selected;
		const g = gcodeId;
		const key = open && p && p.moonraker_url?.trim() && !needsUrl ? `${p.id}:${g ?? ''}` : '';
		if (key === nameFor) return;
		nameFor = key;
		nameServerReason = null;
		if (!key || !p || !g) return;
		void getUploadName(p.id, { gcode_id: g })
			.then((res) => {
				if (nameFor !== key) return;
				nameExt = res.extension || '.gcode';
				if (!nameEdited) nameStem = uploadNameStem(res.filename, nameExt);
			})
			.catch(() => {
				// Nom proposé indisponible : champ vide = nom proposé par le serveur.
			});
	});
	/** Raison de refus affichée (contrôle local d'abord, sinon verdict serveur). */
	const nameReason = $derived.by(() => {
		if (!nameEdited && !nameServerReason) return null;
		if (scope === 'all' && !nameStem.trim()) return null;
		return uploadNameProblem(nameStem, nameExt) ?? nameServerReason;
	});
	function onNameInput() {
		nameEdited = true;
		nameServerReason = null;
		clearTimeout(nameTimer);
		const p = selected;
		const g = gcodeId;
		if (!p || !g || uploadNameProblem(nameStem, nameExt)) return;
		const value = joinUploadName(nameStem, nameExt);
		nameTimer = setTimeout(() => {
			void getUploadName(p.id, { gcode_id: g, filename: value })
				.then(() => {
					if (joinUploadName(nameStem, nameExt) === value) nameServerReason = null;
				})
				.catch((e) => {
					const reason = uploadNameErrorReason(e);
					if (reason !== null && joinUploadName(nameStem, nameExt) === value)
						nameServerReason = reason;
				});
		}, 250);
	}
	/** Nom transmis (`undefined` = nom proposé par le serveur). */
	const chosenName = $derived(nameStem.trim() ? joinUploadName(nameStem, nameExt) : undefined);

	// Lot 42 M3 — file d'envoi (Orca « Print host upload queue »), ouverte
	// après un envoi avec la ou les lignes correspondantes en évidence.
	let queueOpen = $state(false);
	let queuePrinter = $state<PrinterResponse | null>(null);
	let lastUploadIds = $state<string[]>([]);
	function showQueue() {
		queuePrinter = selected ?? null;
		queueOpen = true;
	}
	/** 409 printer_not_connected : adresse enregistrée inutilisable → la redemander. */
	function askAddress(e: unknown) {
		if (!selected) return;
		editingUrl = selected.id;
		urlDraft = selected.moonraker_url ?? '';
		urlError = hostUrlError(e);
	}

	let urlDraft = $state('');
	let savingUrl = $state(false);
	let urlError = $state<string | null>(null);

	async function saveUrl() {
		if (!selected || savingUrl) return;
		const url = normalizeMoonrakerUrl(urlDraft);
		if (!url) {
			urlError = tr('Invalid URL: for example http://192.168.1.20:7125');
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
			editingUrl = null;
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
		nameEdited = false;
		nameServerReason = null;
		nameFor = '';
		lastUploadIds = [];
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

	/** Dernière action : « Envoyer » ou « Envoyer et mettre en file ». */
	let lastQueued = $state(false);

	// `queue` : « Envoyer et mettre en file » (Orca « Upload to Queue »,
	// PrusaConnect to_queue) — jamais avec le démarrage immédiat.
	async function send(queue = false) {
		if (!selected || sending) return;
		if (queue && !meta.queue) return;
		if (scope === 'plate' && !gcodeId) return;
		// URL saisie mais pas encore enregistrée : l'enregistrer d'abord (pas
		// d'étape cachée « Enregistrer l'URL » avant de pouvoir envoyer).
		if (needsUrl) {
			await saveUrl();
			if (needsUrl) return;
		}
		if (nameReason) return;
		sending = true;
		lastQueued = queue;
		success = null;
		error = null;
		lastUploadIds = [];
		const start = queue ? false : startNow;
		const filename = chosenName;
		try {
			if (scope === 'all' && sendAll) {
				const res = await sendAll(
					selected.id,
					start,
					(m) => (progressMessage = m),
					queue,
					filename
				);

				lastUploadIds = res.files.map((f) => f.upload_id).filter((id): id is string => !!id);
				success =
					res.files.length === 1
						? sendSuccessMessage(selected.name, res.files[0], queue)
						: tr(
								queue
									? '{n} files sent to {printer} and queued: {files}.'
									: '{n} files sent to {printer}: {files}.',
								{
									n: res.files.length,
									printer: selected.name,
									files: res.files.map((f) => f.path).join(', ')
								}
							);
			} else if (gcodeId) {
				const result = await uploadToPrinter(selected.id, {
					gcode_id: gcodeId,
					start_now: start,
					...(queue ? { queue: true } : {}),
					...(storage ? { storage } : {}),
					...(filename ? { filename } : {})
				});
				lastUploadIds = result.upload_id ? [result.upload_id] : [];
				success = sendSuccessMessage(selected.name, result, queue);
			}
		} catch (e) {
			const reason = uploadNameErrorReason(e);
			if (reason !== null) {
				nameEdited = true;
				nameServerReason = reason;
			}
			if (isPrinterNotConnected(e)) {
				// Lot 42 M1 : adresse enregistrée inutilisable → saisie proposée.
				askAddress(e);
				return;
			}
			error = isPrinterIncompatible(e)
				? tr('The selected printer is incompatible with the chosen printer presets.')
				: sendErrorMessage(e, selected.name);
		} finally {
			sending = false;
			progressMessage = null;
		}
	}

	// Libellés calculés hors gabarit : relus au changement de langue.
	function optionLabel(p: PrinterResponse): string {
		void $t;
		return printerOptionLabel(p);
	}
	function hintFor(p: PrinterResponse): string | null {
		void $t;
		return connectionHint(p);
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
	title={gcodeId || canSendAll ? $t('Send the G-code to the printer') : $t('Slice the plate first')}
	onclick={openDialog}
>
	<IconSend class="h-4 w-4" aria-hidden="true" />
	{$t('Send')}
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
			<h2 id="send-title" class="ws-dialog-title">{$t('Send to printer')}</h2>

			{#if canSendAll}
				<div class="ws-seg flex w-full" role="group" aria-label={$t('Plates to send')}>
					<button
						type="button"
						class="ws-seg-item flex-1 {scope === 'plate' ? 'ws-seg-on' : ''}"
						aria-pressed={scope === 'plate'}
						data-testid="send-scope-plate"
						disabled={sending || !gcodeId}
						title={gcodeId ? undefined : $t('Slice the active plate first')}
						onclick={() => (scope = 'plate')}
					>
						{$t('Sliced plate')}
					</button>

					<button
						type="button"
						class="ws-seg-item flex-1 {scope === 'all' ? 'ws-seg-on' : ''}"
						aria-pressed={scope === 'all'}
						data-testid="send-scope-all"
						disabled={sending}
						onclick={() => (scope = 'all')}
					>
						{$t('All plates ({n})', { n: plateCount })}
					</button>
				</div>
				{#if scope === 'all'}
					<p class="text-xs text-content-subtle">
						{$t(
							'All plates are sliced then sent: a single .gcode.3mf if the printer accepts it, otherwise one G-code per plate (immediate start impossible in that case).'
						)}
					</p>
				{/if}
			{/if}

			{#if loading}
				<p class="text-sm text-content-muted">{$t('Loading printers…')}</p>
			{:else if targets.length === 0 && !error}
				<p class="text-sm text-content-muted" data-testid="send-no-printer">
					{$t('No printer declared.')}
					<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- chemin résolu + requête -->
					<a class="text-primary underline" href={declareHref}>{$t('Declare a printer')}</a>
				</p>
			{:else if targets.length > 0}
				<label class="flex flex-col gap-1 text-sm">
					<span class="text-content-muted">{$t('Printer')}</span>
					<select
						class="ws-field"
						bind:value={printerId}
						data-testid="send-printer-select"
						disabled={sending}
					>
						{#each targets as p (p.id)}
							<option value={p.id}>{optionLabel(p)}</option>
						{/each}
					</select>
				</label>
				{#if selected && !needsUrl}
					<p class="-mt-2 text-xs text-content-subtle" data-testid="send-host-info">
						{meta.label} · {hasLiveControls(selected)
							? $t('print monitoring in the Device tab')
							: meta.id === 'flashair'
								? $t('file upload only')
								: $t('upload and start, without live monitoring')}
						·
						<button
							type="button"
							class="underline"
							data-testid="send-edit-url"
							onclick={() => {
								editingUrl = selected?.id ?? null;
								urlDraft = selected?.moonraker_url ?? '';
								urlError = null;
							}}
						>
							{$t('Change URL')}
						</button>
					</p>
				{/if}
				{#if mismatch}
					<p
						class="rounded-md bg-warning-soft px-3 py-2 text-xs text-warning-content"
						role="alert"
						data-testid="send-preset-mismatch"
					>
						{$t('The selected printer is incompatible with the chosen printer presets.')}
						{$t('The G-code was sliced for “{sliced}”; this printer uses “{target}”.', {
							sliced: mismatch.sliced,
							target: mismatch.target
						})}
					</p>
				{/if}
				{#if selected && !needsUrl && meta.printerSlug}
					<label class="flex flex-col gap-1 text-sm">
						<span class="text-content-muted">
							{meta.id === 'obico' ? $t('Obico printer') : $t('Repetier server printer')}
						</span>
						{#if hostPrinters?.length}
							<select
								class="ws-field"
								value={selected.host_port ?? ''}
								data-testid="send-host-printer"
								disabled={sending || savingSlug}
								onchange={(e) => chooseHostPrinter((e.currentTarget as HTMLSelectElement).value)}
							>
								{#if !selected.host_port}<option value="" disabled>{$t('Choose…')}</option>{/if}
								{#each hostPrinters as slug (slug)}
									<option value={hostPrinterValue(meta.id, slug)}>{slug}</option>
								{/each}
								{#if selected.host_port && !hostPrinters.some((s) => hostPrinterValue(meta.id, s) === selected.host_port)}
									<option value={selected.host_port}>
										{$t('{name} (not found)', { name: selected.host_port })}
									</option>
								{/if}
							</select>
						{:else if hostPrintersError}
							<span
								class="text-danger-text text-xs"
								role="alert"
								data-testid="send-host-printer-error"
							>
								{hostPrintersError}
							</span>
						{:else}
							<span class="text-xs text-content-subtle">{$t('Searching for printers…')}</span>
						{/if}
					</label>
				{/if}
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
						<!-- Audit 46 BBL-a : l'explication (réseau Bambu non pris en charge)
						     et l'alternative carte SD avant le champ d'adresse. -->
						{#if selected && hintFor(selected)}
							<div
								class="flex flex-col gap-2 rounded border border-warning/40 bg-warning-soft px-3 py-2"
								data-testid="send-connection-help"
							>
								<p class="text-warning-text text-xs" data-testid="send-connection-hint">
									{hintFor(selected)}
								</p>
								{#if onexportsliced}
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-secondary self-start"
										disabled={!gcodeId}
										data-testid="send-export-sliced"
										onclick={() => {
											open = false;
											onexportsliced?.();
										}}
									>
										{$t('Export plate sliced file (.gcode.3mf) for a microSD card')}
									</button>
								{/if}
							</div>
						{/if}
						<label class="flex flex-col gap-1">
							<span class="text-content-muted">
								{$t('{field} of {name}', {
									field:
										meta.id === 'moonraker'
											? $t('Moonraker URL')
											: $t('{host} address', { host: meta.label }),
									name: selected?.name ?? ''
								})}
							</span>
							<input
								type="url"
								inputmode="url"
								class="ws-field"
								placeholder={meta.id === 'moonraker'
									? 'http://192.168.1.20:7125'
									: meta.placeholder}
								bind:value={urlDraft}
								disabled={savingUrl}
								data-testid="send-moonraker-url"
							/>
						</label>
						<div class="flex flex-col items-end gap-1">
							<span class="w-full text-xs text-content-subtle">
								{$t(
									'Saved in the printer when sending, reused afterwards. Other connection type (OctoPrint, PrusaLink…):'
								)}
								<a class="underline" href={resolve('printers')} data-testid="send-host-settings">
									{$t('Printers page')}
								</a>.
							</span>
							<div class="flex gap-2">
								{#if editingUrl && editingUrl === selected?.id}
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-ghost shrink-0"
										data-testid="send-edit-url-cancel"
										onclick={() => {
											editingUrl = null;
											urlDraft = '';
											urlError = null;
										}}
									>
										{$t('Cancel')}
									</button>
								{/if}
								<button
									type="submit"
									class="ws-btn ws-btn-sm ws-btn-secondary shrink-0"
									disabled={savingUrl || !urlDraft.trim()}
									data-testid="send-save-url"
								>
									{savingUrl ? $t('Saving…') : $t('Save URL')}
								</button>
							</div>
						</div>
						{#if urlError}
							<p class="text-xs text-danger" role="alert" data-testid="send-url-error">
								{urlError}
							</p>
						{/if}
					</form>
				{/if}
				{#if !needsUrl && meta.storages && storages === null && selected?.moonraker_url}
					<p class="text-xs text-content-subtle" data-testid="send-storage-loading">
						{meta.id === 'repetier' ? $t('Reading groups…') : $t('Reading storages…')}
					</p>
				{/if}
				{#if storages && storages.items.length > 0}
					<label class="flex flex-col gap-1 text-sm">
						<span class="text-content-muted">
							{storages.kind === 'group' ? $t('Group (Repetier)') : $t('Destination storage')}
						</span>
						<select
							class="ws-field"
							bind:value={storage}
							data-testid="send-storage"
							disabled={sending || scope === 'all'}
						>
							{#each storages.items as s (s.path)}
								<option value={s.path}>{s.name}</option>
							{/each}
						</select>
					</label>
				{/if}
				{#if selected && !needsUrl}
					<!-- Lot 42 M2 : nom du fichier envoyé (Orca PrintHostSendDialog). -->
					<div class="flex flex-col gap-1 text-sm">
						<label class="text-content-muted" for="send-filename-input">
							{$t('Upload to Printer Host with the following filename:')}
						</label>
						<div class="flex items-stretch">
							<input
								id="send-filename-input"
								class="ws-field min-w-0 flex-1 rounded-r-none"
								bind:value={nameStem}
								oninput={onNameInput}
								disabled={sending}
								spellcheck="false"
								autocomplete="off"
								placeholder={scope === 'all' ? $t('Proposed names') : ''}
								aria-invalid={!!nameReason}
								aria-describedby={nameReason ? 'send-filename-error' : undefined}
								data-testid="send-filename"
							/>
							<span
								class="flex items-center rounded-r-md border border-l-0 border-border bg-surface-sunken px-2 text-xs text-content-muted"
								title={$t('Extension required by the printer host')}
								data-testid="send-filename-ext"
							>
								{nameExt}
							</span>
						</div>
						{#if nameReason}
							<p
								id="send-filename-error"
								class="text-danger-text text-xs"
								role="alert"
								data-testid="send-filename-error"
								data-reason={nameReason}
							>
								{uploadNameReasonMessage(nameReason)}
							</p>
						{:else if scope === 'all'}
							<p class="text-xs text-content-subtle" data-testid="send-filename-hint">
								{$t(
									'Common base name for all plates (suffixes _2, _3… when several files are sent); empty = proposed names.'
								)}
							</p>
						{/if}
					</div>
				{/if}
				<label class="flex items-center gap-2 text-sm">
					<input
						type="checkbox"
						bind:checked={startNow}
						data-testid="send-start-now"
						disabled={sending}
					/>
					{$t('Start printing on receipt')}
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
					<button
						type="button"
						class="ml-1 underline"
						data-testid="send-show-queue"
						onclick={showQueue}
					>
						{$t('Print host upload queue')}
					</button>
				</p>
			{/if}
			{#if error}
				<p
					class="rounded-md bg-danger-soft px-3 py-2 text-sm text-danger-content"
					role="alert"
					data-testid="send-error"
				>
					{error}
					{#if selected?.moonraker_url}
						<button
							type="button"
							class="ml-1 underline"
							data-testid="send-error-queue"
							onclick={showQueue}
						>
							{$t('Print host upload queue')}
						</button>
					{/if}
				</p>
			{/if}

			<div class="flex justify-end gap-2">
				<button
					type="button"
					class="ws-btn ws-btn-lg {success ? 'ws-btn-primary' : 'ws-btn-secondary'}"
					disabled={sending}
					onclick={() => (open = false)}
				>
					{success ? $t('Close') : $t('Cancel')}
				</button>

				{#if meta.queue && selected && !needsUrl}
					<!-- Orca PrintHostSendDialog « Upload to Queue » : hôtes dont
					     get_post_upload_actions contient QueuePrint (PrusaConnect). -->
					<button
						type="button"
						class="ws-btn ws-btn-lg ws-btn-secondary"
						data-testid="send-queue"
						title={$t('Upload then add to the printer print queue (without starting)')}
						disabled={(scope === 'plate' && !gcodeId) || sending || loading || !!mismatch}
						onclick={() => send(true)}
					>
						{sending && lastQueued ? $t('Sending…') : $t('Send and queue')}
					</button>
				{/if}
				<button
					type="button"
					class="ws-btn ws-btn-lg {success ? 'ws-btn-secondary' : 'ws-btn-primary'}"
					data-testid="send-confirm"
					disabled={!selected ||
						!!mismatch ||
						(needsUrl && !urlDraft.trim()) ||
						!!nameReason ||
						(scope === 'plate' && !gcodeId) ||
						sending ||
						savingUrl ||
						savingSlug ||
						(meta.printerSlug && !selected?.host_port && !needsUrl) ||
						loading}
					onclick={() => send()}
				>
					{sending && !lastQueued ? $t('Sending…') : success ? $t('Send again') : $t('Send')}
				</button>
			</div>
		</div>
	</div>
{/if}

<UploadQueue
	open={queueOpen}
	printerId={queuePrinter?.id ?? null}
	printerName={queuePrinter?.name ?? null}
	highlight={lastUploadIds}
	onclose={() => (queueOpen = false)}
/>
