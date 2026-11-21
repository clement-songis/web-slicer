<script lang="ts">
	// Wizard d'onboarding : la grille de sélection façon OrcaSlicer en plein
	// écran. Confirmer crée une imprimante par modèle et buse cochée (défaut 0.4) puis
	// entre dans la bibliothèque. Aucune sortie sans imprimante (Confirmer inactif
	// à 0) : la garde `requireOnboardedUser` ramène ici tant qu'aucune n'existe.
	import { goto } from '$app/navigation';
	import { t, tr } from '$lib/i18n';
	import { resolve } from '$app/paths';
	import { ApiError } from '$lib/api/client';
	import { createPrinter } from '$lib/api/printers';
	import { getResolvedPreset, listPresets } from '$lib/api/presets';
	import { currentPreferences, flushPreferences, savePreferences } from '$lib/settings/preferences';
	import {
		defaultSelection,
		filamentChoices,
		filterChoices,
		orderChoices,
		type FilamentChoice
	} from '$lib/printers/wizard-filaments';
	import type { PresetSummary } from '$lib/api/types';
	import AppSidebar from '$lib/nav/AppSidebar.svelte';
	import PrinterCatalog from '$lib/printers/PrinterCatalog.svelte';
	import type { PrinterCatalogModel } from '$lib/api/types';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	let busy = $state(false);
	let error = $state<string | null>(null);

	// — Page « Filaments » de l'assistant d'Orca (phase 25) ———————————————————
	// Filaments compatibles des imprimantes cochées ; le filament par défaut de
	// chaque imprimante (`default_filament_profile`) est pré-coché, plus ceux d'un
	// passage précédent. Les filaments cochés deviennent le défaut des nouveaux
	// projets (préférence `wizardFilaments`, section « filaments » d'AppConfig).
	let pickedIds = $state<string[]>([]);
	let choices = $state<FilamentChoice[]>([]);
	let chosen = $state<string[]>([]);
	let touched = false;
	let loadingFilaments = $state(false);
	let filamentQuery = $state('');
	let filamentType = $state('');
	let onlyChosen = $state(false);
	const types = $derived([...new Set(choices.map((c) => c.type))].sort());
	const shown = $derived(
		filterChoices(choices, {
			query: filamentQuery,
			type: filamentType || undefined,
			only: onlyChosen ? chosen : null
		})
	);
	let machineNames: Promise<Map<string, string>> | null = null;
	// Cache de requêtes, non affiché : pas besoin de réactivité.
	// eslint-disable-next-line svelte/prefer-svelte-reactivity
	const perMachine = new Map<string, Promise<{ list: PresetSummary[]; def: unknown }>>();
	function machineData(id: string) {
		let entry = perMachine.get(id);
		if (!entry) {
			machineNames ??= listPresets('machine').then((l) => new Map(l.map((m) => [m.id, m.name])));
			const names = machineNames;
			entry = (async () => {
				const name = (await names).get(id);
				const [list, resolved] = await Promise.all([
					listPresets('filament', name),
					getResolvedPreset(id)
				]);
				return { list, def: resolved.values.default_filament_profile };
			})();
			perMachine.set(id, entry);
		}
		return entry;
	}
	let pickSeq = 0;
	async function onpick(picked: { model: PrinterCatalogModel; ids: string[] }[]) {
		const ids = [...new Set(picked.flatMap((p) => p.ids))];
		if (ids.join('|') === pickedIds.join('|')) return;
		pickedIds = ids;
		const seq = ++pickSeq;
		if (ids.length === 0) {
			choices = [];
			return;
		}
		loadingFilaments = true;
		try {
			const data = await Promise.all(ids.map(machineData));
			if (seq !== pickSeq) return;
			const next = filamentChoices(data.map((d) => d.list));
			const known = new Set(next.map((c) => c.name));
			const defaults = defaultSelection(next, [
				...data.map((d) => d.def),
				...(currentPreferences().wizardFilaments ?? [])
			]);
			choices = orderChoices(next, defaults);
			chosen = touched
				? [...new Set([...chosen.filter((n) => known.has(n)), ...defaults])]
				: defaults;
		} catch {
			// Liste de confort : l'assistant reste utilisable sans elle.
		} finally {
			if (seq === pickSeq) loadingFilaments = false;
		}
	}
	function toggleFilament(name: string) {
		touched = true;
		chosen = chosen.includes(name) ? chosen.filter((n) => n !== name) : [...chosen, name];
	}

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
					const name =
						ids.length > 1 && nozzle
							? tr('{model} ({nozzle} mm nozzle)', { model: m.model, nozzle })
							: m.model;
					await createPrinter({ name, machine_preset_id: id });
				}
			}
			if (choices.length > 0) {
				savePreferences({ ...currentPreferences(), wizardFilaments: [...chosen] });
				await flushPreferences();
			}
			await goto(resolve('/library'));
		} catch (e) {
			error = e instanceof ApiError ? e.message : tr('Unexpected error');
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head><title>{$t('Printer selection')} — Web-Slicer</title></svelte:head>

<div class="flex h-screen bg-surface text-content">
	<AppSidebar email={data.user.email} />

	<main class="flex min-w-0 flex-1 flex-col p-4">
		<div class="mx-auto flex min-h-0 w-full max-w-6xl flex-1 flex-col">
			<header class="mb-3 text-center">
				<h1 class="ws-page-title">{$t('Printer selection')}</h1>
				<p class="text-sm text-content-muted">
					{$t('Choose the printer(s) you own, then your filaments, to get started.')}
				</p>
			</header>

			{#if error}
				<p class="mb-2 text-center text-sm text-danger" role="alert">{error}</p>
			{/if}

			<div class="flex min-h-0 flex-1 gap-3">
				<div
					class="min-h-0 min-w-0 flex-1 overflow-hidden rounded border border-border bg-surface-raised"
				>
					<PrinterCatalog
						vendors={data.catalog}
						onconfirm={confirm}
						confirmLabel={$t('Confirm')}
						chooseNozzle
						{onpick}
						{busy}
					/>
				</div>
				<!-- Étape 2 de l'assistant d'Orca : filaments des imprimantes cochées. -->
				<aside
					class="flex min-h-0 w-80 shrink-0 flex-col rounded border border-border bg-surface-raised"
					data-testid="wizard-filaments"
					aria-label={$t('Filaments')}
				>
					<div class="border-b border-border p-3">
						<h2 class="ws-section-title">{$t('Filaments')}</h2>
						<p class="text-xs text-content-muted">
							{#if pickedIds.length === 0}
								{$t('Check a printer: its compatible filaments are shown here.')}
							{:else}
								{$t(
									'Filaments you own; the first compatible one becomes the filament of new projects.'
								)}
							{/if}
						</p>
						{#if choices.length > 0}
							<div class="mt-2 flex gap-1.5">
								<input
									class="ws-field ws-field-sm min-w-0 flex-1"
									placeholder={$t('Search filaments…')}
									aria-label={$t('Search filaments')}
									data-testid="wizard-filament-search"
									bind:value={filamentQuery}
								/>
								<select
									class="ws-field ws-field-sm w-24"
									aria-label={$t('Filament type')}
									data-testid="wizard-filament-type"
									bind:value={filamentType}
								>
									<option value="">{$t('All')}</option>
									{#each types as ty (ty)}<option value={ty}>{ty}</option>{/each}
								</select>
							</div>
							<label class="mt-1.5 flex items-center gap-1.5 text-xs text-content-muted">
								<input type="checkbox" bind:checked={onlyChosen} />
								{$t('Checked only')}
								<span class="ml-auto" data-testid="wizard-filament-count"
									>{chosen.length} / {choices.length}</span
								>
							</label>
						{/if}
					</div>
					<div class="min-h-0 flex-1 overflow-y-auto p-1.5">
						{#if loadingFilaments && choices.length === 0}
							<p class="p-2 text-xs text-content-muted">{$t('Loading filaments…')}</p>
						{/if}
						{#each shown as c (c.name)}
							<label
								class="flex items-center gap-2 rounded px-1.5 py-1 text-sm hover:bg-overlay"
								data-testid="wizard-filament"
								data-name={c.name}
							>
								<input
									type="checkbox"
									checked={chosen.includes(c.name)}
									onchange={() => toggleFilament(c.name)}
								/>
								<span class="min-w-0 flex-1 truncate" title={c.name}>{c.name}</span>
								<span class="text-xs text-content-subtle">{c.type}</span>
							</label>
						{/each}
					</div>
				</aside>
			</div>
		</div>
	</main>
</div>
