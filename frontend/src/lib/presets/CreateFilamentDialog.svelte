<script lang="ts">
	// Créer un filament personnalisé (Orca CreateFilamentPresetDialog) :
	// fournisseur, type, série, à partir d'un filament existant (réglages hérités).
	import { untrack } from 'svelte';
	import Dialog from '$lib/ui/Dialog.svelte';
	import { t, tr } from '$lib/i18n';
	import { ApiError } from '$lib/api/client';
	import { createPreset, getCreateOptions } from '$lib/api/presets';
	import type { PresetDetail, PresetSummary } from '$lib/api/types';
	import {
		FILAMENT_TYPES,
		FILAMENT_VENDORS,
		baseFilamentChoices,
		defaultBaseFilament,
		filamentPresetName,
		nameType,
		filamentValues,
		validateFilament
	} from './create-custom';

	interface Props {
		open: boolean;
		presets: PresetSummary[];
		baseId: string | null;
		onclose: () => void;
		oncreated: (preset: PresetDetail) => void;
	}
	let { open, presets, baseId, onclose, oncreated }: Props = $props();

	// Listes du serveur (Orca + filaments système installés), sinon locales.
	let vendorList = $state<string[]>(FILAMENT_VENDORS);
	let typeList = $state<string[]>([...FILAMENT_TYPES]);
	// Type de chaque filament système (gabarits serveur, `filament_type` hérité).
	let typeById = $state<Map<string, string>>(new Map());
	$effect(() => {
		if (!open) return;
		void getCreateOptions().then((o) => {
			if (o?.filament.vendors.length) vendorList = o.filament.vendors;
			if (o?.filament.types.length) typeList = o.filament.types;
			if (o?.filament.templates.length)
				typeById = new Map(
					o.filament.templates.flatMap((t) => (t.type ? [[t.id, t.type] as [string, string]] : []))
				);
		});
	});
	const typeOf = (p: PresetSummary): string | null => typeById.get(p.id) ?? nameType(p.name);
	/** Type d'un filament (preset utilisateur : celui de son parent système). */
	function filamentTypeOf(id: string | null): string | null {
		const p = presets.find((x) => x.id === id);
		if (!p) return null;
		const parent = p.origin === 'user' ? presets.find((x) => x.name === p.inherits) : null;
		return typeOf(parent ?? p);
	}
	let vendor = $state('');
	let type = $state('PLA');
	let serial = $state('');
	let base = $state<string | null>(null);
	let busy = $state(false);
	let error = $state<string | null>(null);
	// Réinitialisé à l'ouverture seulement (la liste des presets peut être
	// rechargée pendant la saisie).
	// Phase 46 (PR1-c) : filaments de départ = presets système du type choisi
	// (Orca CreateFilamentPresetDialog) ; au changement de type (ou à l'arrivée
	// des listes), le courant est gardé s'il convient, sinon « Generic <type> ».
	const choices = $derived(baseFilamentChoices(presets, type, typeOf));
	$effect(() => {
		if (!open) return;
		if (!choices.some((p) => p.id === base))
			base = defaultBaseFilament(
				choices,
				untrack(() => baseId),
				type
			);
	});
	$effect(() => {
		if (!open) return;
		untrack(() => {
			vendor = '';
			serial = '';
			error = null;
			base = baseId;
			type = filamentTypeOf(baseId) ?? 'PLA';
		});
	});
	const draft = $derived({ vendor, type, serial });
	const name = $derived(filamentPresetName(draft));

	async function submit() {
		error = validateFilament(
			draft,
			presets.map((p) => p.name)
		);
		const parent = choices.find((p) => p.id === base);
		if (!error && !parent) error = tr('Choose the base filament.');
		if (error || !parent) return;
		busy = true;
		try {
			const created = await createPreset({
				kind: 'filament',
				name,
				inherits: parent.origin === 'user' ? (parent.inherits ?? parent.name) : parent.name,
				values: filamentValues(draft)
			});
			oncreated(created);
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
	label={$t('Create filament')}
	title={$t('Create filament')}
	size="md"
	{onclose}
	data-testid="create-filament-dialog"
>
	<form class="flex flex-col gap-3 text-sm" onsubmit={(e) => (e.preventDefault(), submit())}>
		<label class="flex flex-col gap-1">
			<span class="text-content-muted">{$t('Vendor')}</span>
			<input
				class="ws-field"
				list="filament-vendors"
				bind:value={vendor}
				data-testid="create-filament-vendor"
				placeholder={$t('e.g. {example}', { example: 'Polymaker' })}
			/>
			<datalist id="filament-vendors">
				{#each vendorList as v (v)}<option value={v}></option>{/each}
			</datalist>
		</label>
		<div class="grid grid-cols-2 gap-3">
			<label class="flex flex-col gap-1">
				<span class="text-content-muted">{$t('Type')}</span>
				<select class="ws-field" bind:value={type} data-testid="create-filament-type">
					{#each typeList as ty (ty)}<option value={ty}>{ty}</option>{/each}
				</select>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-content-muted">{$t('Serial')}</span>
				<input
					class="ws-field"
					bind:value={serial}
					data-testid="create-filament-serial"
					placeholder={$t('e.g. {example}', { example: 'Matte' })}
				/>
			</label>
		</div>
		<label class="flex flex-col gap-1">
			<span class="text-content-muted">{$t('Starting settings (existing filament)')}</span>
			<select
				class="ws-field"
				bind:value={base}
				data-testid="create-filament-base"
				disabled={choices.length === 0}
			>
				{#each choices as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
			</select>
		</label>
		{#if presets.length > 0 && choices.length === 0}
			<p class="text-xs text-warning-text" data-testid="create-filament-no-base">
				{$t('No system filament of type {type} for this printer.', { type })}
			</p>
		{/if}
		<p class="text-xs text-content-subtle">
			{$t('Preset name:')}
			<strong class="text-content" data-testid="create-filament-name">{name || '—'}</strong>{$t(
				'. Temperatures, flows and cooling are taken from the starting filament; adjust them afterwards in its settings.'
			)}
		</p>
		{#if error}<p class="text-sm text-danger" role="alert">{error}</p>{/if}
		<button type="submit" class="hidden" aria-hidden="true" tabindex="-1"></button>
	</form>
	{#snippet footer()}
		<button type="button" class="ws-btn ws-btn-secondary" onclick={onclose}>{$t('Cancel')}</button>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			disabled={busy || choices.length === 0}
			title={presets.length === 0 ? $t('Loading filaments…') : undefined}
			data-testid="create-filament-submit"
			onclick={submit}>{$t('Create')}</button
		>
	{/snippet}
</Dialog>
