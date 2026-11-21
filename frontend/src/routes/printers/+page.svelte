<script lang="ts">
	import { dialogFocus } from '$lib/ui/dialog-focus';
	import { t, tr } from '$lib/i18n';
	// Page imprimantes (T077/US8) : déclaration + test de connexion, panneau
	// d'état en direct (progression, températures) via WebSocket `printer.status`,
	// et contrôles pause/reprise/annulation. La logique d'état est pure
	// (`lib/printers/printers.ts`) ; cette page orchestre chargement, formulaire,
	// abonnement temps réel et actions.
	import { onMount } from 'svelte';
	import { ApiError } from '$lib/api/client';
	import {
		cancelPrinter,
		createPrinter,
		deletePrinter,
		getPrinterStatus,
		listPrinters,
		pausePrinter,
		resumePrinter,
		testPrinter,
		updatePrinter
	} from '$lib/api/printers';
	import { createPreset, getPreset, updatePreset } from '$lib/api/presets';
	import type { PresetSummary, PrinterCatalogModel, PrinterResponse } from '$lib/api/types';
	import PrinterCatalog from '$lib/printers/PrinterCatalog.svelte';
	import CreatePrinterDialog from '$lib/presets/CreatePrinterDialog.svelte';
	import { PresetSettingsDialog } from '$lib/settings';
	import type { DisplayMode } from '$lib/settings/filter';
	import { subscribeEvents } from '$lib/queue/events';
	import {
		applyPrinterStatus,
		canCancel,
		canPause,
		canResume,
		formatTemp,
		fromStatusResponse,
		progressPercent,
		stateMeta,
		type StatusMap
	} from '$lib/printers/printers';
	import type { PageData } from './$types';
	import AppSidebar from '$lib/nav/AppSidebar.svelte';
	import Dialog from '$lib/ui/Dialog.svelte';
	import PrinterHostFields from '$lib/printers/PrinterHostFields.svelte';
	import UploadQueue from '$lib/printers/UploadQueue.svelte';
	import { hostUrlError } from '$lib/printers/send';
	import {
		emptyHostForm,
		hasLiveControls,
		hostFormError,
		hostFormOf,
		hostMeta,
		credentialSummary,
		hostRequest,
		testFailure,
		testOutcome,
		type HostForm,
		type TestOutcome,
		type ProfileHost
	} from '$lib/printers/hosts';
	import IconOk from '~icons/lucide/circle-check';
	import IconFail from '~icons/lucide/circle-alert';
	import IconPlus from '~icons/lucide/plus';
	import IconChevron from '~icons/lucide/chevron-right';
	import IconRefresh from '~icons/lucide/refresh-cw';
	import IconPrinter from '~icons/lucide/printer';

	let { data }: { data: PageData } = $props();

	let printers = $state<PrinterResponse[]>([]);
	const machines = $derived(data.machines as PresetSummary[]);
	let showCreatePrinter = $state(false);

	let statuses = $state<StatusMap>({});
	let error = $state<string | null>(null);
	let testResult = $state<Record<string, TestOutcome>>({});
	let busy = $state<Record<string, boolean>>({});

	// Formulaire de déclaration.
	let name = $state('');
	let hostForm = $state<HostForm>(emptyHostForm());
	let hostProfile = $state<ProfileHost>({});
	const hostError = $derived(hostFormError(hostForm, hostProfile));

	// Connexion d'une imprimante déclarée (Orca : PhysicalPrinterDialog).
	let connecting = $state<PrinterResponse | null>(null);
	let connectForm = $state<HostForm>(emptyHostForm());
	let connectMessage = $state<TestOutcome | null>(null);
	let connectError = $state<string | null>(null);
	let connectBusy = $state(false);
	let connectProfile = $state<ProfileHost>({});
	const connectFormError = $derived(hostFormError(connectForm, connectProfile));
	function openConnection(printer: PrinterResponse) {
		connecting = printer;
		connectForm = hostFormOf(printer);
		connectMessage = null;
		connectError = null;
	}
	/** Enregistre la connexion ; renvoie l'id (null si échec). */
	async function saveConnection(): Promise<string | null> {
		if (!connecting || connectFormError) return null;
		connectError = null;
		try {
			const saved = await updatePrinter(connecting.id, {
				name: connecting.name,
				machine_preset_id: connecting.machine_preset_id,
				...hostRequest(connectForm)
			});
			printers = printers.map((p) => (p.id === saved.id ? saved : p));
			connecting = saved;
			// Secrets enregistrés : champs vidés (conservés côté serveur).
			connectForm = { ...connectForm, apiKey: '', password: '' };
			if (!hasLiveControls(saved)) {
				const { [saved.id]: _drop, ...rest } = statuses;
				void _drop;
				statuses = rest;
			}
			return saved.id;
		} catch (e) {
			// Lot 42 M1 : refus de l'adresse par le serveur (422 details.reason) traduit.
			connectError = hostUrlError(e) ?? (e instanceof ApiError ? e.message : tr('Could not save'));
			return null;
		}
	}
	async function saveAndClose() {
		connectBusy = true;
		const id = await saveConnection();
		connectBusy = false;
		if (id) connecting = null;
	}
	async function saveAndTest() {
		connectBusy = true;
		connectMessage = null;
		const id = await saveConnection();
		if (id && connecting) {
			try {
				const r = await testPrinter(id);
				connectMessage = testOutcome(connecting.host_type, r);
				testResult = { ...testResult, [id]: connectMessage };
			} catch (e) {
				connectMessage = testFailure(
					connecting.host_type,
					e instanceof ApiError ? e.message : null
				);
				testResult = { ...testResult, [id]: connectMessage };
			}
		}
		connectBusy = false;
	}
	let machinePreset = $state('');
	/** Formulaire de déclaration manuelle replié (le catalogue est l'entrée principale). */
	let showManual = $state(false);
	let creating = $state(false);

	// Ajout par le catalogue (grille façon OrcaSlicer, Phase 14).
	let showCatalog = $state(false);

	// Édition des réglages d'une imprimante (dialogue réutilisé de l'éditeur,
	// Phase 14 refonte). On édite toujours un preset **utilisateur** : si
	// l'imprimante pointe encore sur un preset système, on le dérive puis on
	// repointe l'imprimante dessus (la clé API est préservée côté serveur).
	let editing = $state<null | { printerId: string; presetId: string }>(null);
	let editMode = $state<DisplayMode>('simple');
	let editValues = $state<Record<string, unknown>>({});

	async function editSettings(printer: PrinterResponse) {
		error = null;
		busy = { ...busy, [printer.id]: true };
		try {
			const detail = await getPreset(printer.machine_preset_id);
			let presetId = printer.machine_preset_id;
			if (detail.origin !== 'user') {
				// Dérive une copie utilisateur (même nom → même modèle/buse) et repointe.
				const derived = await createPreset({
					kind: 'machine',
					name: detail.name,
					// Orca : `inherits` = nom du parent (le backend exclut la copie
					// homonyme elle-même et préfère le preset système).
					inherits: detail.name,
					values: {}
				});
				await updatePrinter(printer.id, {
					name: printer.name,
					moonraker_url: printer.moonraker_url || undefined,
					machine_preset_id: derived.id
				});
				await reload();
				presetId = derived.id;
				editValues = {};
			} else {
				editValues = (detail.values as Record<string, unknown>) ?? {};
			}
			editing = { printerId: printer.id, presetId };
		} catch (e) {
			fail(e, tr('Could not edit'));
		} finally {
			busy = { ...busy, [printer.id]: false };
		}
	}

	async function saveEditSettings() {
		if (!editing) return;
		try {
			await updatePreset(editing.presetId, { values: editValues });
			editing = null;
		} catch (e) {
			fail(e, tr('Could not save'));
		}
	}

	async function addFromCatalog(models: PrinterCatalogModel[]) {
		error = null;
		try {
			// Une imprimante par modèle : nom = modèle, buse par défaut (0.4 mm),
			// sans connexion réseau (ajoutable ensuite).
			for (const m of models) {
				await createPrinter({ name: m.model, machine_preset_id: m.default_machine_preset_id });
			}
			await reload();
			showCatalog = false;
		} catch (e) {
			fail(e, tr('Could not add'));
		}
	}

	onMount(() => {
		printers = data.printers;
		if (machines.length > 0) machinePreset = machines[0].id;
		// Suivi en direct : chaque `printer.status` patche la carte d'état.
		const sub = subscribeEvents({
			onEvent(event) {
				statuses = applyPrinterStatus(statuses, event);
			}
		});
		return () => sub.close();
	});

	function fail(e: unknown, fallback: string) {
		error = hostUrlError(e) ?? (e instanceof ApiError ? e.message : fallback);
	}

	// Lot 42 M3 : file d'envoi (Orca « Print host upload queue »), toutes les
	// imprimantes ou une seule.
	let queueFor = $state<PrinterResponse | 'all' | null>(null);

	/** Nom par défaut : hôte de l'URL Moonraker, sinon libellé générique. */
	function defaultName(rawUrl: string): string {
		try {
			return new URL(rawUrl).hostname || tr('Printer');
		} catch {
			return rawUrl.trim() ? rawUrl.trim() : tr('Printer');
		}
	}

	async function declare(event: SubmitEvent) {
		event.preventDefault();
		creating = true;
		error = null;
		try {
			const host = hostRequest(hostForm);
			const printer = await createPrinter({
				name: name.trim() || defaultName(host.moonraker_url ?? ''),
				machine_preset_id: machinePreset,
				...host
			});
			printers = [...printers, printer];
			name = '';
			hostForm = emptyHostForm(hostForm.type);
		} catch (e) {
			fail(e, tr('Could not add the printer'));
		} finally {
			creating = false;
		}
	}

	async function runTest(id: string) {
		busy = { ...busy, [id]: true };
		error = null;
		const type = printers.find((p) => p.id === id)?.host_type ?? 'moonraker';
		try {
			const result = await testPrinter(id);
			testResult = { ...testResult, [id]: testOutcome(type, result) };
		} catch (e) {
			// Cause affichée sur la carte (clé refusée, hôte injoignable…), pas en tête de page.
			testResult = {
				...testResult,
				[id]: testFailure(type, e instanceof ApiError ? e.message : null)
			};
		} finally {
			busy = { ...busy, [id]: false };
		}
	}

	async function refreshStatus(id: string) {
		busy = { ...busy, [id]: true };
		error = null;
		try {
			// Amorce aussi le relais WebSocket côté serveur (T076).
			const snapshot = await getPrinterStatus(id);
			statuses = { ...statuses, [id]: fromStatusResponse(snapshot) };
		} catch (e) {
			fail(e, tr('Status unavailable'));
		} finally {
			busy = { ...busy, [id]: false };
		}
	}

	async function control(id: string, action: (id: string) => Promise<void>) {
		busy = { ...busy, [id]: true };
		error = null;
		try {
			await action(id);
			await refreshStatus(id);
		} catch (e) {
			fail(e, tr('Command rejected'));
		} finally {
			busy = { ...busy, [id]: false };
		}
	}

	async function remove(id: string) {
		busy = { ...busy, [id]: true };
		error = null;
		try {
			await deletePrinter(id);
			printers = printers.filter((p) => p.id !== id);
		} catch (e) {
			fail(e, tr('Could not delete'));
		} finally {
			busy = { ...busy, [id]: false };
		}
	}

	async function reload() {
		printers = await listPrinters();
	}
</script>

<svelte:head><title>{$t('Printers')}</title></svelte:head>

<div class="flex h-screen bg-surface text-content">
	<AppSidebar email={data.user.email} active="printers" />
	<div class="flex-1 overflow-auto">
		<!-- Même gouttière que la bibliothèque (p-8, alignement à gauche). -->
		<main class="flex max-w-3xl flex-col gap-6 p-8 text-content">
			<h1 class="ws-page-title">{$t('Printers')}</h1>

			{#if error}
				<p class="rounded bg-danger-soft px-3 py-2 text-danger-content" role="alert">{error}</p>
			{/if}

			<!-- Ajout par le catalogue (grille façon OrcaSlicer) -->
			<button
				type="button"
				class="ws-btn ws-btn-lg ws-btn-primary self-start"
				onclick={() => (showCatalog = true)}
			>
				<IconPlus class="h-4 w-4" aria-hidden="true" />
				{$t('Add printers')}
			</button>
			<button
				type="button"
				class="ws-btn ws-btn-secondary self-start"
				data-testid="printer-create-custom"
				onclick={() => (showCreatePrinter = true)}
			>
				{$t('Custom printer…')}
			</button>
			<CreatePrinterDialog
				open={showCreatePrinter}
				catalog={data.catalog}
				existing={machines.map((m) => m.name)}
				onclose={() => (showCreatePrinter = false)}
				oncreated={(printer) => {
					printers = [...printers, printer];
					void reload();
				}}
			/>

			<!-- Déclaration manuelle (connexion Moonraker facultative) -->
			<section
				class="flex flex-col gap-3 rounded-lg border border-border bg-surface-raised p-4"
				aria-label={$t('Add a printer')}
			>
				<button
					type="button"
					class="flex items-center gap-1.5 self-start text-sm font-semibold text-content-muted hover:text-content"
					data-testid="printer-add"
					aria-expanded={showManual}
					onclick={() => (showManual = !showManual)}
				>
					<IconChevron
						class="h-4 w-4 transition-transform {showManual ? 'rotate-90' : ''}"
						aria-hidden="true"
					/>
					{$t('Add manually (Moonraker, OctoPrint, PrusaLink…)')}
				</button>
				{#if showManual}
					<form class="flex flex-col gap-3" onsubmit={declare}>
						<label class="flex flex-col gap-1 text-sm">
							<span class="text-content-muted">{$t('Name')}</span>
							<input
								class="ws-field"
								bind:value={name}
								placeholder={$t('Living room printer (default: URL host)')}
							/>
						</label>
						<PrinterHostFields
							bind:form={hostForm}
							bind:profile={hostProfile}
							machinePresetId={machinePreset || null}
						/>
						<label class="flex flex-col gap-1 text-sm">
							<span class="text-content-muted">{$t('Printer preset')}</span>
							<select class="ws-field" bind:value={machinePreset} required>
								{#each machines as preset (preset.id)}
									<option value={preset.id}>{preset.name}</option>
								{/each}
							</select>
						</label>
						<button
							type="submit"
							class="ws-btn ws-btn-primary self-start"
							data-testid="printer-save"
							disabled={creating || machines.length === 0 || !!hostError}
						>
							{$t('Add printer||declare')}
						</button>
						{#if hostError && hostForm.url.trim()}
							<p class="text-xs text-danger-text" role="alert">{hostError}</p>
						{/if}
						{#if machines.length === 0}
							<p class="text-xs text-warning">{$t('No printer preset available.')}</p>
						{/if}
					</form>
				{/if}
			</section>

			<!-- Liste -->
			<section class="flex flex-col gap-3" aria-label={$t('Added printers')}>
				<div class="flex items-center justify-between">
					<h2 class="ws-section-title">{$t('Added ({n})', { n: printers.length })}</h2>
					<div class="flex gap-1">
						<button
							type="button"
							class="ws-btn ws-btn-sm ws-btn-ghost"
							data-testid="printers-upload-queue"
							onclick={() => (queueFor = 'all')}
						>
							{$t('Print host upload queue')}
						</button>
						<button type="button" class="ws-btn ws-btn-sm ws-btn-ghost" onclick={reload}>
							<IconRefresh class="h-3.5 w-3.5" aria-hidden="true" />
							{$t('Refresh')}
						</button>
					</div>
				</div>

				{#if creating}
					<!-- Déclaration en cours : la liste n'est réaffichée qu'une fois
					     l'imprimante enregistrée (aucune ligne « déjà là » qui laisserait
					     croire que l'ajout est fait — et quitter la page trop tôt
					     annulerait la requête). -->
					<p
						class="ws-empty rounded-lg border border-dashed border-border"
						role="status"
						data-testid="printer-saving"
					>
						{$t('Saving printer…')}
					</p>
				{:else if printers.length === 0}
					<div
						class="ws-empty rounded-lg border border-dashed border-border"
						data-testid="printers-empty"
					>
						<IconPrinter class="h-8 w-8 text-content-subtle" aria-hidden="true" />
						<p class="font-medium text-content">{$t('No printers added')}</p>
						<p>{$t('Pick your models from the catalog, or add a Moonraker printer.')}</p>
						<button
							type="button"
							class="ws-btn ws-btn-sm ws-btn-primary"
							onclick={() => (showCatalog = true)}>{$t('Open catalog')}</button
						>
					</div>
				{:else}
					{#each printers as printer (printer.id)}
						{@const st = statuses[printer.id]}
						<article
							class="flex flex-col gap-3 rounded-lg border border-border bg-surface-raised p-4"
							data-testid="printer-row"
							data-printer-id={printer.id}
						>
							<div class="flex items-start justify-between gap-3">
								<div class="flex min-w-0 flex-col">
									<span class="truncate font-medium text-content">{printer.name}</span>
									<span class="truncate text-xs text-content-subtle">
										{machines.find((m) => m.id === printer.machine_preset_id)?.name ?? ''}
									</span>
									<span class="flex min-w-0 items-center gap-1.5 text-xs text-content-subtle">
										{#if printer.moonraker_url}
											<span class="ws-badge shrink-0" data-testid="printer-host-type"
												>{hostMeta(printer.host_type).label}</span
											>
										{/if}
										<span class="truncate"
											>{printer.moonraker_url ?? $t('Not connected||printer')}</span
										>
									</span>
									{#if credentialSummary(printer).length}
										<span class="mt-1 flex flex-wrap gap-1.5" data-testid="printer-credentials">
											{#each credentialSummary(printer) as c (c.text)}
												<span
													class="rounded px-1.5 py-0.5 text-2xs {c.missing
														? 'bg-warning-soft text-warning-text'
														: 'bg-surface-sunken text-content-muted'}">{c.text}</span
												>
											{/each}
										</span>
									{/if}
								</div>
								{#if st}
									<span
										class="shrink-0 rounded-full px-2 py-0.5 text-xs {stateMeta(st.state).badge}"
									>
										{stateMeta(st.state).label}
									</span>
								{/if}
							</div>

							{#if st}
								<div class="flex flex-col gap-1">
									<div class="flex justify-between text-xs text-content-muted">
										<span class="truncate">{st.filename ?? '—'}</span>
										<span class="tabular-nums">{progressPercent(st.progress)}%</span>
									</div>
									<div class="h-1.5 overflow-hidden rounded bg-overlay">
										<div
											class="h-full bg-primary"
											style:width="{progressPercent(st.progress)}%"
										></div>
									</div>
									<div class="mt-1 flex gap-4 text-xs text-content-muted">
										<span
											>{$t('Nozzle {temp}', {
												temp: formatTemp(st.extruderTemp, st.extruderTarget)
											})}</span
										>
										<span>{$t('Bed {temp}', { temp: formatTemp(st.bedTemp, st.bedTarget) })}</span>
									</div>
								</div>
							{/if}

							{#if testResult[printer.id]}
								{@const res = testResult[printer.id]}
								<p
									class="flex items-center gap-1.5 text-xs {res.ok
										? 'text-success-text'
										: 'text-danger-text'}"
									role="status"
									data-testid="printer-test-result"
									data-ok={res.ok}
								>
									{#if res.ok}<IconOk
											class="h-3.5 w-3.5 shrink-0"
											aria-hidden="true"
										/>{:else}<IconFail class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />{/if}
									<span class="min-w-0 break-words">{res.message}</span>
								</p>
							{/if}

							<div class="flex flex-wrap gap-2">
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-secondary"
									disabled={busy[printer.id]}
									onclick={() => editSettings(printer)}
								>
									{$t('Settings')}
								</button>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-secondary"
									data-testid="printer-connection"
									disabled={busy[printer.id]}
									onclick={() => openConnection(printer)}
								>
									{$t('Connection…')}
								</button>
								{#if printer.moonraker_url}
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-secondary"
										data-testid="printer-test"
										disabled={busy[printer.id]}
										onclick={() => runTest(printer.id)}
									>
										{busy[printer.id] ? $t('Testing…') : $t('Test')}
									</button>
									{#if hasLiveControls(printer)}
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-secondary"
											data-testid="printer-status"
											disabled={busy[printer.id]}
											onclick={() => refreshStatus(printer.id)}
										>
											{$t('Status')}
										</button>
									{/if}
								{:else}
									<span class="self-center text-xs text-content-subtle"
										>{$t('No Moonraker URL or other host: settings and slicing only.')}</span
									>
								{/if}
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-ghost"
									data-testid="printer-uploads"
									onclick={() => (queueFor = printer)}
								>
									{$t('Uploads')}
								</button>
								<!-- Commandes d'impression : seulement quand l'état est connu
								     (au lieu de trois boutons grisés en permanence). -->
								{#if st && hasLiveControls(printer)}
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-secondary"
										disabled={busy[printer.id] || !st || !canPause(st.state)}
										onclick={() => control(printer.id, pausePrinter)}
									>
										{$t('Pause')}
									</button>
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-secondary"
										disabled={busy[printer.id] || !st || !canResume(st.state)}
										onclick={() => control(printer.id, resumePrinter)}
									>
										{$t('Resume')}
									</button>
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-secondary"
										disabled={busy[printer.id] || !st || !canCancel(st.state)}
										onclick={() => control(printer.id, cancelPrinter)}
									>
										{$t('Cancel')}
									</button>
								{/if}
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-ghost ml-auto text-danger hover:bg-danger-soft hover:text-danger-content"
									disabled={busy[printer.id]}
									onclick={() => remove(printer.id)}
								>
									{$t('Delete')}
								</button>
							</div>
						</article>
					{/each}
				{/if}
			</section>
		</main>
	</div>
</div>

{#if showCatalog}
	<!-- Grille de sélection façon OrcaSlicer pour ajouter des imprimantes possédées. -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4"
		role="presentation"
		onclick={() => (showCatalog = false)}
	>
		<div
			class="ws-dialog flex h-[85vh] w-full max-w-4xl flex-col overflow-hidden"
			role="dialog"
			aria-modal="true"
			aria-label={$t('Add printers')}
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			tabindex="-1"
			use:dialogFocus={() => (showCatalog = false)}
		>
			<PrinterCatalog
				vendors={data.catalog}
				onconfirm={addFromCatalog}
				oncancel={() => (showCatalog = false)}
				confirmLabel={$t('Add')}
			/>
		</div>
	</div>
{/if}

<!-- Connexion d'une imprimante (Orca : PhysicalPrinterDialog). -->
<Dialog
	open={connecting !== null}
	label={$t('Printer connection')}
	title={connecting ? $t('Connection — {name}', { name: connecting.name }) : ''}
	size="md"
	onclose={() => !connectBusy && (connecting = null)}
	data-testid="printer-connection-dialog"
>
	{#if connecting}
		<div class="flex flex-col gap-3">
			<PrinterHostFields
				bind:form={connectForm}
				bind:profile={connectProfile}
				machinePresetId={connecting.machine_preset_id}
				printerId={connecting.id}
				hasSecret={connecting.has_api_key}
				hasPassword={connecting.has_host_password}
				beforeListPrinters={saveConnection}
			/>
			{#if connectFormError && connectForm.url.trim()}
				<p class="text-xs text-danger-text" role="alert">{connectFormError}</p>
			{/if}
			{#if connectError}
				<p class="text-xs text-danger-text" role="alert" data-testid="connection-error">
					{connectError}
				</p>
			{:else if connectMessage}
				<p
					class="flex items-center gap-1.5 rounded-md px-3 py-2 text-xs {connectMessage.ok
						? 'bg-success-soft text-success-content'
						: 'bg-danger-soft text-danger-content'}"
					role="status"
					data-testid="connection-result"
					data-ok={connectMessage.ok}
				>
					{#if connectMessage.ok}<IconOk
							class="h-4 w-4 shrink-0"
							aria-hidden="true"
						/>{:else}<IconFail class="h-4 w-4 shrink-0" aria-hidden="true" />{/if}
					<span class="min-w-0 break-words">{connectMessage.message}</span>
				</p>
			{/if}
		</div>
	{/if}
	{#snippet footer()}
		<button
			type="button"
			class="ws-btn ws-btn-secondary mr-auto"
			data-testid="connection-test"
			disabled={connectBusy || !!connectFormError || !connectForm.url.trim()}
			onclick={saveAndTest}>{$t('Test connection')}</button
		>
		<button type="button" class="ws-btn ws-btn-ghost" onclick={() => (connecting = null)}
			>{$t('Cancel')}</button
		>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			data-testid="connection-save"
			disabled={connectBusy || !!connectFormError}
			onclick={saveAndClose}>{$t('Save')}</button
		>
	{/snippet}
</Dialog>

<UploadQueue
	open={queueFor !== null}
	printerId={queueFor && queueFor !== 'all' ? queueFor.id : null}
	printerName={queueFor && queueFor !== 'all' ? queueFor.name : null}
	onclose={() => (queueFor = null)}
/>

<!-- Réglages complets d'une imprimante (dialogue réutilisé de l'éditeur). -->
<PresetSettingsDialog
	open={editing !== null}
	title={$t('Printer settings')}
	kind="machine"
	bind:mode={editMode}
	bind:values={editValues}
	onClose={() => (editing = null)}
	onsave={saveEditSettings}
/>
