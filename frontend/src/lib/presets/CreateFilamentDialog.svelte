<script lang="ts">
	// Créer un filament personnalisé (Orca CreateFilamentPresetDialog) :
	// fournisseur, type, série, à partir d'un filament existant (réglages hérités).
	import { untrack } from 'svelte';
	import Dialog from '$lib/ui/Dialog.svelte';
	import { ApiError } from '$lib/api/client';
	import { createPreset, getCreateOptions } from '$lib/api/presets';
	import type { PresetDetail, PresetSummary } from '$lib/api/types';
	import {
		FILAMENT_TYPES,
		FILAMENT_VENDORS,
		filamentPresetName,
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
	$effect(() => {
		if (!open) return;
		void getCreateOptions().then((o) => {
			if (o?.filament.vendors.length) vendorList = o.filament.vendors;
			if (o?.filament.types.length) typeList = o.filament.types;
		});
	});
	let vendor = $state('');
	let type = $state('PLA');
	let serial = $state('');
	let base = $state<string | null>(null);
	let busy = $state(false);
	let error = $state<string | null>(null);
	// Réinitialisé à l'ouverture seulement (la liste des presets peut être
	// rechargée pendant la saisie).
	// Liste arrivée après l'ouverture : reprend le filament courant comme base.
	$effect(() => {
		if (open && presets.length && !presets.some((p) => p.id === base)) base = baseId;
	});
	$effect(() => {
		if (!open) return;
		untrack(() => {
			vendor = '';
			serial = '';
			error = null;
			base = baseId;
			const b = presets.find((p) => p.id === baseId);
			type = FILAMENT_TYPES.find((t) => b?.name.includes(t)) ?? 'PLA';
		});
	});
	const draft = $derived({ vendor, type, serial });
	const name = $derived(filamentPresetName(draft));

	async function submit() {
		error = validateFilament(
			draft,
			presets.map((p) => p.name)
		);
		const parent = presets.find((p) => p.id === base);
		if (!error && !parent) error = 'Choisissez le filament de base.';
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
			error = e instanceof ApiError ? e.message : 'création impossible';
		} finally {
			busy = false;
		}
	}
</script>

<Dialog
	{open}
	label="Créer un filament"
	title="Créer un filament"
	size="md"
	{onclose}
	data-testid="create-filament-dialog"
>
	<form class="flex flex-col gap-3 text-sm" onsubmit={(e) => (e.preventDefault(), submit())}>
		<label class="flex flex-col gap-1">
			<span class="text-content-muted">Fournisseur</span>
			<input
				class="ws-field"
				list="filament-vendors"
				bind:value={vendor}
				data-testid="create-filament-vendor"
				placeholder="ex. Polymaker"
			/>
			<datalist id="filament-vendors">
				{#each vendorList as v (v)}<option value={v}></option>{/each}
			</datalist>
		</label>
		<div class="grid grid-cols-2 gap-3">
			<label class="flex flex-col gap-1">
				<span class="text-content-muted">Type</span>
				<select class="ws-field" bind:value={type} data-testid="create-filament-type">
					{#each typeList as t (t)}<option value={t}>{t}</option>{/each}
				</select>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-content-muted">Série</span>
				<input
					class="ws-field"
					bind:value={serial}
					data-testid="create-filament-serial"
					placeholder="ex. Matte"
				/>
			</label>
		</div>
		<label class="flex flex-col gap-1">
			<span class="text-content-muted">Réglages de départ (filament existant)</span>
			<select
				class="ws-field"
				bind:value={base}
				data-testid="create-filament-base"
				disabled={presets.length === 0}
			>
				{#each presets as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
			</select>
		</label>
		<p class="text-xs text-content-subtle">
			Nom du preset : <strong class="text-content" data-testid="create-filament-name"
				>{name || '—'}</strong
			>. Les températures, débits et refroidissement sont repris du filament de départ ; ajustez-les
			ensuite dans ses réglages.
		</p>
		{#if error}<p class="text-sm text-danger" role="alert">{error}</p>{/if}
		<button type="submit" class="hidden" aria-hidden="true" tabindex="-1"></button>
	</form>
	{#snippet footer()}
		<button type="button" class="ws-btn ws-btn-secondary" onclick={onclose}>Annuler</button>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			disabled={busy || presets.length === 0}
			title={presets.length === 0 ? 'Chargement des filaments…' : undefined}
			data-testid="create-filament-submit"
			onclick={submit}>Créer</button
		>
	{/snippet}
</Dialog>
