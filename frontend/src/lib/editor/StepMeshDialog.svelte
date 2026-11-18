<script lang="ts">
	// Maillage d'un STEP importé (Orca : StepMeshDialog) : déflexions linéaire et
	// angulaire, découpe des compounds. Chaque essai remaille côté serveur et
	// remplace l'objet affiché ; le nombre de triangles obtenu est affiché.
	import Dialog from '$lib/ui/Dialog.svelte';
	import { ApiError } from '$lib/api/client';
	import {
		ANGLE_RANGE,
		LINEAR_RANGE,
		STEP_DEFAULTS,
		formatTriangles,
		normalizeStepOptions,
		remeshStep,
		sameStepOptions,
		type StepMeshOptions,
		type StepMeshResult
	} from './step-mesh';

	interface Props {
		open: boolean;
		filename: string;
		modelId: string;
		/** Triangles du maillage d'import (réglages par défaut). */
		initialTriangles?: number | null;
		/** Maillage remplacé côté serveur : recharger l'objet affiché. */
		onremeshed: (result: StepMeshResult) => Promise<void> | void;
		onclose: () => void;
	}

	let { open, filename, modelId, initialTriangles = null, onremeshed, onclose }: Props = $props();

	let options = $state<StepMeshOptions>({ ...STEP_DEFAULTS });
	let applied = $state<StepMeshOptions>({ ...STEP_DEFAULTS });
	let result = $state<StepMeshResult | null>(null);
	let busy = $state(false);
	let error = $state<string | null>(null);
	const dirty = $derived(!sameStepOptions(normalizeStepOptions(options), applied));
	const triangles = $derived(result?.triangles ?? initialTriangles);

	async function apply() {
		const next = normalizeStepOptions(options);
		busy = true;
		error = null;
		try {
			result = await remeshStep(modelId, next);
			applied = next;
			options = { ...next };
			await onremeshed(result);
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Remaillage impossible.';
		} finally {
			busy = false;
		}
	}
	const fmt = (n: number, d: number) =>
		n.toLocaleString('fr-FR', { minimumFractionDigits: d, maximumFractionDigits: d });
</script>

<Dialog
	{open}
	label="Maillage du STEP"
	size="md"
	onclose={() => !busy && onclose()}
	data-testid="step-mesh-dialog"
>
	{#snippet header()}
		<div class="flex min-w-0 flex-col gap-0.5">
			<h2 class="ws-dialog-title">Maillage du STEP</h2>
			<p class="text-xs text-content-subtle">
				« {filename} » : une précision plus fine donne des surfaces plus lisses mais plus de triangles.
			</p>
		</div>
	{/snippet}
	<div class="flex flex-col gap-4 text-sm">
		<label class="flex flex-col gap-1">
			<span class="flex justify-between text-content-muted">
				<span>Déflexion linéaire</span>
				<span class="tabular-nums text-content">{fmt(options.linear_deflection, 3)} mm</span>
			</span>
			<input
				type="range"
				min={LINEAR_RANGE.min}
				max={LINEAR_RANGE.max}
				step={LINEAR_RANGE.step}
				bind:value={options.linear_deflection}
				disabled={busy}
				data-testid="step-linear"
			/>
			<span class="flex justify-between text-2xs text-content-subtle"
				><span>Plus fin</span><span>Plus grossier</span></span
			>
		</label>
		<label class="flex flex-col gap-1">
			<span class="flex justify-between text-content-muted">
				<span>Déflexion angulaire</span>
				<span class="tabular-nums text-content">{fmt(options.angle_deflection, 2)} rad</span>
			</span>
			<input
				type="range"
				min={ANGLE_RANGE.min}
				max={ANGLE_RANGE.max}
				step={ANGLE_RANGE.step}
				bind:value={options.angle_deflection}
				disabled={busy}
				data-testid="step-angle"
			/>
		</label>
		<label class="flex items-center gap-2">
			<input
				type="checkbox"
				bind:checked={options.split_compound}
				disabled={busy}
				data-testid="step-split"
			/>
			<span>Découper les compounds (une pièce par solide)</span>
		</label>
		<div class="flex items-center justify-between rounded-md bg-surface-sunken px-3 py-2">
			<span class="text-content-muted">Triangles</span>
			<output class="tabular-nums font-medium" data-testid="step-triangles"
				>{triangles != null ? formatTriangles(triangles) : '—'}</output
			>
		</div>
		{#if result && result.objects.length > 1}
			<p class="text-xs text-content-subtle" data-testid="step-pieces">
				{result.objects.length} pièces : {result.objects.map((o) => o.name).join(', ')}
			</p>
		{/if}
		{#if error}<p class="text-xs text-danger-text" role="alert">{error}</p>{/if}
	</div>
	{#snippet footer()}
		<button
			type="button"
			class="ws-btn ws-btn-ghost mr-auto"
			disabled={busy}
			onclick={() => (options = { ...STEP_DEFAULTS })}>Valeurs par défaut</button
		>
		<button
			type="button"
			class="ws-btn ws-btn-secondary"
			data-testid="step-apply"
			disabled={busy || !dirty}
			onclick={apply}>{busy ? 'Remaillage…' : 'Remailler'}</button
		>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			data-testid="step-done"
			disabled={busy}
			onclick={onclose}>Terminé</button
		>
	{/snippet}
</Dialog>
