<script lang="ts">
	import { dialogFocus } from '$lib/ui/dialog-focus';
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

	let { data }: { data: PageData } = $props();

	let printers = $state<PrinterResponse[]>([]);
	const machines = $derived(data.machines as PresetSummary[]);

	let statuses = $state<StatusMap>({});
	let error = $state<string | null>(null);
	let testResult = $state<Record<string, string>>({});
	let busy = $state<Record<string, boolean>>({});

	// Formulaire de déclaration.
	let name = $state('');
	let url = $state('');
	let apiKey = $state('');
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
			fail(e, 'Édition impossible');
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
			fail(e, 'Enregistrement impossible');
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
			fail(e, 'Ajout impossible');
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
		error = e instanceof ApiError ? e.message : fallback;
	}

	/** Nom par défaut : hôte de l'URL Moonraker, sinon libellé générique. */
	function defaultName(rawUrl: string): string {
		try {
			return new URL(rawUrl).hostname || 'Imprimante';
		} catch {
			return rawUrl.trim() ? rawUrl.trim() : 'Imprimante';
		}
	}

	async function declare(event: SubmitEvent) {
		event.preventDefault();
		creating = true;
		error = null;
		try {
			const printer = await createPrinter({
				name: name.trim() || defaultName(url),
				moonraker_url: url || undefined,
				api_key: apiKey || undefined,
				machine_preset_id: machinePreset
			});
			printers = [...printers, printer];
			name = '';
			url = '';
			apiKey = '';
		} catch (e) {
			fail(e, 'Déclaration impossible');
		} finally {
			creating = false;
		}
	}

	async function runTest(id: string) {
		busy = { ...busy, [id]: true };
		error = null;
		try {
			const result = await testPrinter(id);
			testResult = {
				...testResult,
				[id]: result.connected
					? `Connecté (Klipper ${result.klippy_state}, Moonraker ${result.moonraker_version})`
					: `Injoignable (${result.klippy_state})`
			};
		} catch (e) {
			testResult = { ...testResult, [id]: 'Échec de connexion' };
			fail(e, 'Test impossible');
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
			fail(e, 'État indisponible');
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
			fail(e, 'Commande refusée');
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
			fail(e, 'Suppression impossible');
		} finally {
			busy = { ...busy, [id]: false };
		}
	}

	async function reload() {
		printers = await listPrinters();
	}
</script>

<svelte:head><title>Imprimantes</title></svelte:head>

<div class="flex h-screen bg-surface text-content">
	<AppSidebar email={data.user.email} active="printers" />
	<div class="flex-1 overflow-auto">
		<main class="mx-auto flex max-w-3xl flex-col gap-6 p-6 text-content">
			<h1 class="ws-page-title">Imprimantes</h1>

			{#if error}
				<p class="rounded bg-danger-soft px-3 py-2 text-danger-content" role="alert">{error}</p>
			{/if}

			<!-- Ajout par le catalogue (grille façon OrcaSlicer) -->
			<button
				type="button"
				class="ws-btn ws-btn-lg ws-btn-primary self-start"
				onclick={() => (showCatalog = true)}
			>
				＋ Ajouter des imprimantes
			</button>

			<!-- Déclaration manuelle (connexion Moonraker facultative) -->
			<section
				class="flex flex-col gap-3 rounded border border-border p-4"
				aria-label="Déclarer une imprimante"
			>
				<button
					type="button"
					class="self-start text-sm font-semibold text-content-muted hover:text-content"
					data-testid="printer-add"
					aria-expanded={showManual}
					onclick={() => (showManual = !showManual)}
				>
					{showManual ? '▾' : '▸'} Déclarer manuellement (Moonraker)
				</button>
				{#if showManual}
					<form class="flex flex-col gap-3" onsubmit={declare}>
						<label class="flex flex-col gap-1 text-sm">
							<span class="text-content-muted">Nom</span>
							<input
								class="ws-field"
								bind:value={name}
								placeholder="Imprimante du salon (défaut : hôte de l'URL)"
							/>
						</label>
						<label class="flex flex-col gap-1 text-sm">
							<span class="text-content-muted">URL Moonraker (facultatif)</span>
							<input
								class="ws-field"
								bind:value={url}
								type="url"
								data-testid="printer-moonraker-url"
								placeholder="http://klipper.local:7125"
							/>
						</label>
						<label class="flex flex-col gap-1 text-sm">
							<span class="text-content-muted">Clé API (facultatif)</span>
							<input
								class="ws-field"
								bind:value={apiKey}
								type="password"
								autocomplete="off"
								placeholder="X-Api-Key"
							/>
						</label>
						<label class="flex flex-col gap-1 text-sm">
							<span class="text-content-muted">Préréglage machine</span>
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
							disabled={creating || machines.length === 0}
						>
							Déclarer
						</button>
						{#if machines.length === 0}
							<p class="text-xs text-warning">Aucun preset machine disponible.</p>
						{/if}
					</form>
				{/if}
			</section>

			<!-- Liste -->
			<section class="flex flex-col gap-3" aria-label="Imprimantes déclarées">
				<div class="flex items-center justify-between">
					<h2 class="ws-section-title">Déclarées ({printers.length})</h2>
					<button type="button" class="text-xs text-content-muted hover:underline" onclick={reload}>
						Rafraîchir
					</button>
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
						Enregistrement de l'imprimante…
					</p>
				{:else if printers.length === 0}
					<p class="ws-empty rounded-lg border border-dashed border-border">
						Aucune imprimante déclarée.
					</p>
				{:else}
					{#each printers as printer (printer.id)}
						{@const st = statuses[printer.id]}
						<article
							class="flex flex-col gap-3 rounded border border-border p-4"
							data-testid="printer-row"
							data-printer-id={printer.id}
						>
							<div class="flex items-start justify-between gap-3">
								<div class="flex min-w-0 flex-col">
									<span class="truncate font-medium text-content">{printer.name}</span>
									<span class="truncate text-xs text-content-subtle">
										{printer.moonraker_url ?? 'Non connectée'}
									</span>
								</div>
								{#if st}
									<span class="rounded px-2 py-0.5 text-xs {stateMeta(st.state).badge}">
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
										<span>Buse {formatTemp(st.extruderTemp, st.extruderTarget)}</span>
										<span>Plateau {formatTemp(st.bedTemp, st.bedTarget)}</span>
									</div>
								</div>
							{/if}

							{#if testResult[printer.id]}
								<p class="text-xs text-content-muted" role="status">{testResult[printer.id]}</p>
							{/if}

							<div class="flex flex-wrap gap-2">
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-secondary"
									disabled={busy[printer.id]}
									onclick={() => editSettings(printer)}
								>
									Réglages
								</button>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-secondary"
									disabled={busy[printer.id] || !printer.moonraker_url}
									onclick={() => runTest(printer.id)}
								>
									Tester
								</button>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-secondary"
									disabled={busy[printer.id] || !printer.moonraker_url}
									onclick={() => refreshStatus(printer.id)}
								>
									État
								</button>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-secondary"
									disabled={busy[printer.id] || !st || !canPause(st.state)}
									onclick={() => control(printer.id, pausePrinter)}
								>
									Pause
								</button>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-secondary"
									disabled={busy[printer.id] || !st || !canResume(st.state)}
									onclick={() => control(printer.id, resumePrinter)}
								>
									Reprendre
								</button>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-secondary"
									disabled={busy[printer.id] || !st || !canCancel(st.state)}
									onclick={() => control(printer.id, cancelPrinter)}
								>
									Annuler
								</button>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-danger ml-auto"
									disabled={busy[printer.id]}
									onclick={() => remove(printer.id)}
								>
									Supprimer
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
			aria-label="Ajouter des imprimantes"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			tabindex="-1"
			use:dialogFocus={() => (showCatalog = false)}
		>
			<PrinterCatalog
				vendors={data.catalog}
				onconfirm={addFromCatalog}
				oncancel={() => (showCatalog = false)}
				confirmLabel="Ajouter"
			/>
		</div>
	</div>
{/if}

<!-- Réglages complets d'une imprimante (dialogue réutilisé de l'éditeur). -->
<PresetSettingsDialog
	open={editing !== null}
	title="Paramètres de l'imprimante"
	kind="machine"
	bind:mode={editMode}
	bind:values={editValues}
	onClose={() => (editing = null)}
	onsave={saveEditSettings}
/>
