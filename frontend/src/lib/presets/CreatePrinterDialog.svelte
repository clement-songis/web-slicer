<script lang="ts">
	// Créer une imprimante personnalisée (Orca CreatePrinterPresetDialog) :
	// fournisseur / modèle, buse, volume d'impression, à partir d'un modèle
	// existant du catalogue (G-code, cinématique… hérités).
	import { untrack } from 'svelte';
	import Dialog from '$lib/ui/Dialog.svelte';
	import { t, tr } from '$lib/i18n';
	import { ApiError } from '$lib/api/client';
	import { createPreset, getCreateOptions, getPreset, type CreateOptions } from '$lib/api/presets';
	import { createPrinter } from '$lib/api/printers';
	import type { PrinterCatalogVendor, PrinterResponse } from '$lib/api/types';
	import { printerPresetName, printerValues, validatePrinter } from './create-custom';

	interface Props {
		open: boolean;
		catalog: PrinterCatalogVendor[];
		/** Noms de presets machine existants (doublons refusés). */
		existing: string[];
		onclose: () => void;
		oncreated: (printer: PrinterResponse) => void;
	}
	let { open, catalog, existing, onclose, oncreated }: Props = $props();

	// Listes du serveur (Orca CreatePrinterPresetDialog), sinon locales.
	let options = $state<CreateOptions | null>(null);
	$effect(() => {
		if (open) void getCreateOptions().then((o) => (options = o));
	});
	const NOZZLES = $derived(
		options?.printer.nozzle_diameters.length
			? [...new Set(options.printer.nozzle_diameters.map(Number))]
					.filter((n) => n > 0)
					.sort((a, b) => a - b)
			: [0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.8, 1.0]
	);
	const models = $derived(catalog.flatMap((v) => v.models));
	let vendor = $state('');
	let model = $state('');
	const vendorModels = $derived(options?.printer.models[vendor.trim()] ?? []);
	let nozzle = $state(0.4);
	let width = $state(220);
	let depth = $state(220);
	let height = $state(250);
	let centered = $state(false);
	let baseId = $state('');
	let busy = $state(false);
	let error = $state<string | null>(null);
	$effect(() => {
		if (!open) return;
		untrack(() => {
			vendor = '';
			model = '';
			error = null;
			baseId = models[0]?.default_machine_preset_id ?? '';
		});
	});
	const draft = $derived({ vendor, model, nozzle, width, depth, height, centered });
	const name = $derived(printerPresetName(draft));

	async function submit() {
		error = validatePrinter(draft, existing);
		if (!error && !baseId) error = tr('Choose a starting model.');
		if (error) return;
		busy = true;
		try {
			const base = await getPreset(baseId);
			const preset = await createPreset({
				kind: 'machine',
				name,
				inherits: base.origin === 'user' ? (base.inherits ?? base.name) : base.name,
				values: printerValues(draft)
			});
			const printer = await createPrinter({
				name: `${vendor.trim()} ${model.trim()}`,
				machine_preset_id: preset.id
			});
			oncreated(printer);
			onclose();
		} catch (e) {
			error = e instanceof ApiError ? e.message : tr('creation failed');
		} finally {
			busy = false;
		}
	}
</script>

<Dialog
	{open}
	label={$t('Create printer')}
	title={$t('Create custom printer')}
	size="md"
	{onclose}
	data-testid="create-printer-dialog"
>
	<form class="flex flex-col gap-3 text-sm" onsubmit={(e) => (e.preventDefault(), submit())}>
		<div class="grid grid-cols-2 gap-3">
			<label class="flex flex-col gap-1">
				<span class="text-content-muted">{$t('Vendor')}</span>
				<input
					class="ws-field"
					list="printer-vendors"
					bind:value={vendor}
					data-testid="create-printer-vendor"
					placeholder={$t('e.g. {example}', { example: 'Atelier' })}
				/>
				<datalist id="printer-vendors">
					{#each options?.printer.vendors ?? [] as v (v)}<option value={v}></option>{/each}
				</datalist>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-content-muted">{$t('Model')}</span>
				<input
					class="ws-field"
					list="printer-models"
					bind:value={model}
					data-testid="create-printer-model"
					placeholder={$t('e.g. {example}', { example: 'Cube 300' })}
				/>
				<datalist id="printer-models">
					{#each vendorModels as m (m)}<option value={m}></option>{/each}
				</datalist>
			</label>
		</div>
		<label class="flex flex-col gap-1">
			<span class="text-content-muted"
				>{$t('Start from the model (G-code, kinematics, limits inherited)')}</span
			>
			<select class="ws-field" bind:value={baseId} data-testid="create-printer-base">
				{#each catalog as v (v.vendor)}
					<optgroup label={v.vendor}>
						{#each v.models as m (m.default_machine_preset_id)}<option
								value={m.default_machine_preset_id}>{m.model}</option
							>{/each}
					</optgroup>
				{/each}
			</select>
		</label>
		<fieldset class="grid grid-cols-4 items-end gap-2">
			<legend class="mb-1 text-content-muted">{$t('Print volume (mm) and nozzle')}</legend>
			<label class="flex flex-col gap-1 text-xs"
				>{$t('Width X')}<input
					type="number"
					min="1"
					class="ws-field"
					bind:value={width}
					data-testid="create-printer-width"
				/></label
			>
			<label class="flex flex-col gap-1 text-xs"
				>{$t('Depth Y')}<input
					type="number"
					min="1"
					class="ws-field"
					bind:value={depth}
					data-testid="create-printer-depth"
				/></label
			>
			<label class="flex flex-col gap-1 text-xs"
				>{$t('Height Z')}<input
					type="number"
					min="1"
					class="ws-field"
					bind:value={height}
					data-testid="create-printer-height"
				/></label
			>
			<label class="flex flex-col gap-1 text-xs"
				>{$t('Nozzle')}<select
					class="ws-field"
					bind:value={nozzle}
					data-testid="create-printer-nozzle"
				>
					{#each NOZZLES as n (n)}<option value={n}>{n} mm</option>{/each}
				</select></label
			>
		</fieldset>
		<label class="flex items-center gap-2 text-xs text-content-muted">
			<input type="checkbox" bind:checked={centered} />
			{$t('Origin at the plate center (delta, round bed)')}
		</label>
		<p class="text-xs text-content-subtle">
			{$t('Machine preset:')}
			<strong class="text-content" data-testid="create-printer-name">{name}</strong>
		</p>
		{#if error}<p class="text-sm text-danger" role="alert">{error}</p>{/if}
		<button type="submit" class="hidden" aria-hidden="true" tabindex="-1"></button>
	</form>
	{#snippet footer()}
		<button type="button" class="ws-btn ws-btn-secondary" onclick={onclose}>{$t('Cancel')}</button>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			disabled={busy}
			data-testid="create-printer-submit"
			onclick={submit}>{$t('Create')}</button
		>
	{/snippet}
</Dialog>
