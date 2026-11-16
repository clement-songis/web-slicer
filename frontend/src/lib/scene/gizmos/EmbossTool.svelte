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
	}

	let {
		onsubmit,
		available = false,
		initialSource = 'text',
		fonts = [],
		busy = false,
		canEngrave = false
	}: Props = $props();

	// Source initiale lue une fois (outil Texte ou SVG) : l'utilisateur bascule ensuite.
	const startSource = untrack(() => initialSource);
	let params = $state<EmbossParams>({ ...defaultEmbossParams(), source: startSource });
	// Sans objet sélectionné, seule la création en relief est possible.
	$effect(() => {
		if (!canEngrave) params.embossed = true;
	});
	$effect(() => {
		if (fonts.length > 0 && !fonts.some((f) => f.id === params.font)) params.font = fonts[0].id;
	});
	const error = $derived(validateEmboss(params));
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
		<label class="flex flex-col gap-1">
			<span class="text-content-muted">SVG</span>
			<input
				type="file"
				accept=".svg,image/svg+xml"
				data-testid="emboss-svg-file"
				class="text-xs"
				onchange={async (e) => {
					const file = e.currentTarget.files?.[0];
					if (file) params.svg = await file.text();
				}}
			/>
			<textarea
				bind:value={params.svg}
				data-testid="emboss-svg"
				rows="3"
				class="ws-field ws-field-sm font-mono"></textarea>
		</label>
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

	{#if error}
		<p class="text-warning">{error}</p>
	{/if}

	<button
		type="button"
		class="ws-btn ws-btn-primary self-start"
		data-testid="emboss-submit"
		disabled={!available || busy || error !== null}
		onclick={() => onsubmit($state.snapshot(params))}>{busy ? 'Création…' : 'Créer l’objet'}</button
	>
	{#if !available}
		<p class="text-content-subtle">La création de volume nécessite le moteur (à venir).</p>
	{/if}
</div>
