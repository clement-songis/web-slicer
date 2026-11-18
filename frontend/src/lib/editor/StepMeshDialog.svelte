<script lang="ts">
	// Maillage d'un STEP importé (Orca : StepMeshDialog) : déflexions linéaire et
	// angulaire (curseur + saisie, bornes d'Orca), découpe des compounds. Chaque
	// essai remaille côté serveur et remplace l'objet affiché ; le nombre de
	// triangles obtenu, l'écart avec l'import et les pièces sont affichés. Plus
	// direct qu'Orca : préréglages de qualité, réglages mémorisés pour le prochain
	// import (app_config d'Orca), « Ne plus afficher ».
	import Dialog from '$lib/ui/Dialog.svelte';
	import { ApiError } from '$lib/api/client';
	import {
		ANGLE_RANGE,
		LINEAR_RANGE,
		STEP_DEFAULTS,
		STEP_PRESETS,
		formatTriangles,
		loadStepPrefs,
		normalizeStepOptions,
		parseDeflection,
		presetOf,
		remeshStep,
		sameStepOptions,
		saveStepPrefs,
		triangleDelta,
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

	// Réglages du dernier import (Orca les reprend à l'ouverture du dialogue).
	const prefs = loadStepPrefs();
	const remembered = !sameStepOptions(prefs.options, STEP_DEFAULTS);
	let options = $state<StepMeshOptions>({ ...prefs.options });
	/** Réglages du maillage affiché (l'import utilise les défauts). */
	let applied = $state<StepMeshOptions>({ ...STEP_DEFAULTS });
	let result = $state<StepMeshResult | null>(null);
	let busy = $state(false);
	let error = $state<string | null>(null);
	let dontAsk = $state(!prefs.ask);
	const dirty = $derived(!sameStepOptions(normalizeStepOptions(options), applied));
	const triangles = $derived(result?.triangles ?? initialTriangles);
	const delta = $derived(result ? triangleDelta(initialTriangles, result.triangles) : null);
	const preset = $derived(presetOf(options));

	// Saisie numérique à côté des curseurs (Orca : wxTextCtrl + contrôle des bornes).
	let linearText = $state(fmt(prefs.options.linear_deflection, 3));
	let angleText = $state(fmt(prefs.options.angle_deflection, 2));
	let linearError = $state<string | null>(null);
	let angleError = $state<string | null>(null);
	function syncTexts() {
		linearText = fmt(options.linear_deflection, 3);
		angleText = fmt(options.angle_deflection, 2);
		linearError = angleError = null;
	}
	function commitText(kind: 'linear' | 'angle') {
		const r = parseDeflection(kind === 'linear' ? linearText : angleText, kind);
		if ('error' in r) {
			if (kind === 'linear') linearError = r.error;
			else angleError = r.error;
			return;
		}
		if (kind === 'linear') {
			options.linear_deflection = r.value;
			linearError = null;
		} else {
			options.angle_deflection = r.value;
			angleError = null;
		}
		options = normalizeStepOptions(options);
		syncTexts();
	}
	function choosePreset(id: string) {
		const p = STEP_PRESETS.find((x) => x.id === id);
		if (!p) return;
		options = {
			...options,
			linear_deflection: p.linear_deflection,
			angle_deflection: p.angle_deflection
		};
		syncTexts();
	}

	async function apply(): Promise<boolean> {
		const next = normalizeStepOptions(options);
		busy = true;
		error = null;
		try {
			result = await remeshStep(modelId, next);
			applied = next;
			options = { ...next };
			syncTexts();
			await onremeshed(result);
			return true;
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Remaillage impossible.';
			return false;
		} finally {
			busy = false;
		}
	}
	/** « Terminé » / « Appliquer et fermer » : mémorise les réglages comme le OK d'Orca. */
	async function finish() {
		if (dirty && !(await apply())) return;
		saveStepPrefs({ options: applied, ask: !dontAsk });
		onclose();
	}
	function fmt(n: number, d: number) {
		return n.toLocaleString('fr-FR', { minimumFractionDigits: d, maximumFractionDigits: d });
	}
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
			<p class="truncate text-xs text-content-subtle" title={filename}>« {filename} »</p>
		</div>
	{/snippet}
	<div class="flex flex-col gap-4 text-sm">
		<div class="flex flex-col gap-1.5">
			<span class="text-content-muted">Qualité</span>
			<div class="ws-seg flex w-full" role="radiogroup" aria-label="Qualité du maillage">
				{#each STEP_PRESETS as p (p.id)}
					<button
						type="button"
						role="radio"
						aria-checked={preset === p.id}
						class="ws-seg-item flex h-auto flex-1 flex-col items-center py-1 leading-tight {preset ===
						p.id
							? 'ws-seg-on'
							: ''}"
						disabled={busy}
						data-testid={'step-preset-' + p.id}
						onclick={() => choosePreset(p.id)}
						><span>{p.label}</span><span class="text-2xs opacity-75">{p.hint}</span></button
					>
				{/each}
			</div>
			{#if preset === null}
				<span class="text-2xs text-content-subtle" data-testid="step-preset-custom"
					>Réglages personnalisés</span
				>
			{/if}
			{#if remembered && !result}
				<span class="text-2xs text-content-subtle" data-testid="step-remembered"
					>Réglages de votre dernier import STEP — appliqués avec « Appliquer et fermer ».</span
				>
			{/if}
		</div>

		<div class="grid grid-cols-[1fr_auto] items-center gap-x-3 gap-y-1">
			<label for="step-linear" class="col-span-2 flex justify-between text-content-muted">
				<span>Déflexion linéaire</span>
				<span class="text-2xs text-content-subtle">écart corde / surface</span>
			</label>
			<input
				id="step-linear"
				type="range"
				min={LINEAR_RANGE.min}
				max={LINEAR_RANGE.max}
				step={LINEAR_RANGE.step}
				bind:value={options.linear_deflection}
				oninput={syncTexts}
				disabled={busy}
				data-testid="step-linear"
			/>
			<span class="flex items-center gap-1">
				<input
					class="ws-field ws-field-sm w-20 text-right tabular-nums"
					inputmode="decimal"
					aria-label="Déflexion linéaire (mm)"
					aria-invalid={!!linearError}
					bind:value={linearText}
					onchange={() => commitText('linear')}
					onkeydown={(e) => e.key === 'Enter' && commitText('linear')}
					disabled={busy}
					data-testid="step-linear-input"
				/>
				<span class="w-7 text-xs text-content-subtle">mm</span>
			</span>
			<span class="flex justify-between text-2xs text-content-subtle"
				><span>Plus fin · plus de triangles</span><span>Plus grossier</span></span
			>
			<span></span>
			{#if linearError}<span class="col-span-2 text-xs text-danger-text" role="alert"
					>{linearError}</span
				>{/if}

			<label for="step-angle" class="col-span-2 mt-2 flex justify-between text-content-muted">
				<span>Déflexion angulaire</span>
				<span class="text-2xs text-content-subtle">angle entre facettes voisines</span>
			</label>
			<input
				id="step-angle"
				type="range"
				min={ANGLE_RANGE.min}
				max={ANGLE_RANGE.max}
				step={ANGLE_RANGE.step}
				bind:value={options.angle_deflection}
				oninput={syncTexts}
				disabled={busy}
				data-testid="step-angle"
			/>
			<span class="flex items-center gap-1">
				<input
					class="ws-field ws-field-sm w-20 text-right tabular-nums"
					inputmode="decimal"
					aria-label="Déflexion angulaire (rad)"
					aria-invalid={!!angleError}
					bind:value={angleText}
					onchange={() => commitText('angle')}
					onkeydown={(e) => e.key === 'Enter' && commitText('angle')}
					disabled={busy}
					data-testid="step-angle-input"
				/>
				<span class="w-7 text-xs text-content-subtle">rad</span>
			</span>
			{#if angleError}<span class="col-span-2 text-xs text-danger-text" role="alert"
					>{angleError}</span
				>{/if}
		</div>

		<label class="flex items-start gap-2">
			<input
				type="checkbox"
				class="mt-0.5"
				bind:checked={options.split_compound}
				disabled={busy}
				data-testid="step-split"
			/>
			<span
				>Découper les compounds en plusieurs pièces
				<span class="block text-2xs text-content-subtle"
					>une pièce par solide (« nom-SOLID-n »)</span
				></span
			>
		</label>

		<div class="flex flex-col gap-2 rounded-md bg-surface-sunken px-3 py-2.5" aria-live="polite">
			<div class="flex items-baseline justify-between gap-2">
				<span class="text-content-muted">Triangles</span>
				<span class="flex items-baseline gap-2">
					{#if delta}
						<span class="text-xs text-content-subtle" data-testid="step-delta"
							>{delta} vs import</span
						>
					{/if}
					<output class="text-base font-medium tabular-nums" data-testid="step-triangles"
						>{triangles != null ? formatTriangles(triangles) : '—'}</output
					>
				</span>
			</div>
			<span
				class="text-2xs {dirty ? 'text-warning-text' : 'text-content-subtle'}"
				data-testid="step-state"
				data-dirty={dirty}
			>
				{#if busy}Remaillage en cours…{:else if dirty}Réglages modifiés : « Remailler » pour voir le
					résultat{:else if result}Maillage affiché à jour{:else}Maillage de l’import (réglages par
					défaut){/if}
			</span>
			{#if result && result.objects.length > 1}
				<div data-testid="step-pieces">
					<p class="text-xs text-content-muted">{result.objects.length} pièces</p>
					<ul class="mt-1 flex max-h-28 flex-col gap-0.5 overflow-auto text-xs">
						{#each result.objects as o, i (i)}
							<li class="flex justify-between gap-2">
								<span class="truncate">{o.name}</span>
								<span class="shrink-0 tabular-nums text-content-subtle"
									>{formatTriangles(o.triangles)}</span
								>
							</li>
						{/each}
					</ul>
				</div>
			{/if}
		</div>
		<p class="text-2xs text-content-subtle">
			Des déflexions plus faibles donnent un maillage plus fidèle mais un traitement plus long.
			<a
				class="text-primary-text underline"
				href="https://www.orcaslicer.com/wiki/import_export#step"
				target="_blank"
				rel="noopener noreferrer">Guide (wiki Orca)</a
			>
		</p>
		{#if error}<p class="text-xs text-danger-text" role="alert">{error}</p>{/if}
	</div>
	{#snippet footer()}
		<label class="mr-auto flex items-center gap-2 text-xs text-content-muted">
			<input type="checkbox" bind:checked={dontAsk} disabled={busy} data-testid="step-dont-ask" />
			Ne plus afficher
		</label>
		<button
			type="button"
			class="ws-btn ws-btn-ghost"
			disabled={busy}
			data-testid="step-defaults"
			onclick={() => {
				options = { ...STEP_DEFAULTS };
				syncTexts();
			}}>Défauts</button
		>
		<button
			type="button"
			class="ws-btn ws-btn-secondary"
			data-testid="step-apply"
			disabled={busy || !dirty || !!linearError || !!angleError}
			onclick={apply}>{busy ? 'Remaillage…' : 'Remailler'}</button
		>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			data-testid="step-done"
			disabled={busy || !!linearError || !!angleError}
			onclick={finish}>{dirty ? 'Appliquer et fermer' : 'Terminé'}</button
		>
	{/snippet}
</Dialog>
