<script lang="ts">
	// Outil texte/SVG en relief (T057, Emboss & SVG). Texte : maillage généré par
	// le moteur (POST /api/projects/{id}/emboss) ; SVG : importé comme modèle
	// (profils extrudés par libslic3r, Format/svg.cpp).
	import { untrack } from 'svelte';
	import { defaultEmbossParams, validateEmboss, type EmbossParams } from './emboss';

	interface Props {
		onsubmit: (params: EmbossParams) => void;
		/** Vrai quand l'endpoint de création de volume moteur est disponible (P5). */
		available?: boolean;
		/** Source initiale (outil « Forme SVG » → 'svg'). */
		initialSource?: EmbossParams['source'];
		/** Polices proposées (GET /api/emboss/fonts). */
		fonts?: readonly { id: string; label: string }[];
		busy?: boolean;
		/** Un objet est sélectionné : le mode « en creux » est disponible. */
		canEngrave?: boolean;
		/** Phase 28 — ré-édition d'un relief existant (paramètres emboss relus). */
		initial?: EmbossParams | null;
		/** Ré-édition : « Annuler » referme sans rien changer. */
		oncancel?: () => void;
	}

	let {
		onsubmit,
		available = false,
		initialSource = 'text',
		fonts = [],
		busy = false,
		canEngrave = false,
		initial = null,
		oncancel
	}: Props = $props();

	// Source initiale lue une fois (outil Texte ou SVG) : l'utilisateur bascule ensuite.
	const startSource = untrack(() => initialSource);
	const editing = untrack(() => !!initial);
	let params = $state<EmbossParams>(
		untrack(() => (initial ? { ...initial } : { ...defaultEmbossParams(), source: startSource }))
	);
	// Sans objet sélectionné, seule la création en relief est possible.
	$effect(() => {
		if (!canEngrave && !editing) params.embossed = true;
	});
	$effect(() => {
		if (fonts.length > 0 && !fonts.some((f) => f.id === params.font)) params.font = fonts[0].id;
	});
	const error = $derived(validateEmboss(params));

	// Sélecteur SVG (phase 26) : bouton stylé au lieu du « Choose File » natif
	// (anglais dans l'UI française) + aperçu de la forme ; la source brute reste
	// accessible (repliée) pour coller un SVG.
	let svgInput = $state<HTMLInputElement | null>(null);
	let svgFileName = $state<string | null>(null);
	const svgPreview = $derived.by(() => {
		const src = params.svg.trim();
		if (!src.startsWith('<') || !/<svg[\s>]/i.test(src)) return null;
		// <img> : le SVG n'est jamais exécuté (pas de script), simple aperçu.
		return 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(src);
	});
</script>

<div class="flex flex-col gap-2 text-sm">
	<div class="flex items-center gap-2">
		<span class="text-content-muted">Source</span>
		<button
			type="button"
			class="ws-chip {params.source === 'text' ? 'ws-chip-on' : ''}"
			onclick={() => (params.source = 'text')}>Texte</button
		>
		<button
			type="button"
			class="ws-chip {params.source === 'svg' ? 'ws-chip-on' : ''}"
			onclick={() => (params.source = 'svg')}>SVG</button
		>
	</div>

	{#if params.source === 'text'}
		<label class="flex flex-col gap-1">
			<span class="text-content-muted">Texte</span>
			<input bind:value={params.text} data-testid="emboss-text" class="ws-field ws-field-sm" />
		</label>
		<label class="flex items-center justify-between gap-2">
			<span class="text-content-muted">Police</span>
			<select bind:value={params.font} data-testid="emboss-font" class="ws-field ws-field-sm">
				{#each fonts as f (f.id)}
					<option value={f.id}>{f.label}</option>
				{/each}
			</select>
		</label>
		<label class="flex items-center justify-between gap-2">
			<span class="text-content-muted">Graisse (mm)</span>
			<input
				type="number"
				min={-2}
				max={5}
				step={0.1}
				bind:value={params.boldness}
				data-testid="emboss-boldness"
				class="ws-field ws-field-sm w-20 text-right"
			/>
		</label>
		<label class="flex items-center justify-between gap-2">
			<span class="text-content-muted">Taille de police (mm)</span>
			<input
				type="number"
				min={0.1}
				step={0.5}
				bind:value={params.fontSize}
				class="ws-field ws-field-sm w-20 text-right"
			/>
		</label>
	{:else}
		<div class="flex flex-col gap-1">
			<span class="text-content-muted">Forme SVG</span>
			<input
				bind:this={svgInput}
				type="file"
				accept=".svg,image/svg+xml"
				data-testid="emboss-svg-file"
				class="sr-only"
				tabindex="-1"
				aria-hidden="true"
				onchange={async (e) => {
					const file = e.currentTarget.files?.[0];
					if (file) {
						params.svg = await file.text();
						svgFileName = file.name;
					}
				}}
			/>
			<div class="flex items-center gap-2">
				<button
					type="button"
					class="ws-btn ws-btn-secondary"
					data-testid="emboss-svg-pick"
					onclick={() => svgInput?.click()}>Choisir un fichier SVG…</button
				>
				<span class="min-w-0 truncate text-xs text-content-subtle" data-testid="emboss-svg-name"
					>{svgFileName ?? (params.svg.trim() ? 'SVG collé' : 'Aucun fichier')}</span
				>
			</div>
			{#if svgPreview}
				<img
					src={svgPreview}
					alt="Aperçu de la forme SVG"
					data-testid="emboss-svg-preview"
					class="h-24 w-full rounded border border-border bg-surface-sunken object-contain p-1"
				/>
			{/if}
			<details class="text-xs">
				<summary class="cursor-pointer text-content-subtle">Source SVG (coller)</summary>
				<textarea
					bind:value={params.svg}
					data-testid="emboss-svg"
					rows="3"
					aria-label="Source SVG"
					class="ws-field ws-field-sm mt-1 w-full font-mono"></textarea>
			</details>
		</div>
	{/if}

	{#if params.source === 'svg'}
		<label class="flex items-center justify-between gap-2">
			<span class="text-content-muted">Largeur (mm)</span>
			<input
				type="number"
				min={1}
				step={1}
				bind:value={params.svgWidth}
				data-testid="emboss-svg-width"
				class="ws-field ws-field-sm w-20 text-right"
			/>
		</label>
	{/if}

	<label class="flex items-center justify-between gap-2">
		<span class="text-content-muted">Profondeur (mm)</span>
		<input
			type="number"
			min={0.1}
			step={0.1}
			bind:value={params.depth}
			data-testid="emboss-panel-depth"
			class="ws-field ws-field-sm w-20 text-right"
		/>
	</label>

	{#if editing}
		<p class="text-xs text-content-subtle" data-testid="emboss-editing">
			Modification du relief existant : il est régénéré à la même place.
		</p>
	{:else}
		<label class="flex items-center gap-2">
			<input
				type="checkbox"
				bind:checked={params.embossed}
				disabled={!canEngrave}
				data-testid="emboss-raised"
			/>
			<span class="text-content-muted"
				>En relief (décoché : gravé en creux dans l’objet sélectionné)</span
			>
		</label>

		{#if canEngrave && params.embossed}
			<label class="flex items-center gap-2">
				<input type="checkbox" bind:checked={params.onObject} data-testid="emboss-on-object" />
				<span class="text-content-muted">Sur l’objet sélectionné (pièce posée sur le dessus)</span>
			</label>
		{/if}
	{/if}

	{#if error}
		<p class="text-warning">{error}</p>
	{/if}

	<button
		type="button"
		class="ws-btn ws-btn-primary self-start"
		data-testid="emboss-submit"
		disabled={!available || busy || error !== null}
		onclick={() => onsubmit($state.snapshot(params))}
		>{busy
			? editing
				? 'Mise à jour…'
				: 'Création…'
			: editing
				? 'Appliquer'
				: 'Créer l’objet'}</button
	>
	{#if editing}
		<button
			type="button"
			class="ws-btn ws-btn-secondary self-start"
			data-testid="emboss-cancel"
			onclick={() => oncancel?.()}>Annuler</button
		>
	{/if}
	{#if !available}
		<p class="text-content-subtle">La création de volume nécessite le moteur (à venir).</p>
	{/if}
</div>
