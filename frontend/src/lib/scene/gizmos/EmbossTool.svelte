<script lang="ts">
	// Outil texte/SVG en relief (T057, Emboss & SVG). Texte : maillage généré par
	// le moteur (POST /api/projects/{id}/emboss) ; SVG : importé comme modèle
	// (profils extrudés par libslic3r, Format/svg.cpp).
	import { untrack } from 'svelte';
	import { defaultEmbossParams, validateEmboss, type EmbossParams } from './emboss';
	import { t } from '#lib/i18n';

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
	const error = $derived.by(() => {
		void $t; // message traduit : se réévalue au changement de langue
		return validateEmboss(params);
	});

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
		<span class="text-content-muted">{$t('Source')}</span>
		<button
			type="button"
			class="ws-chip {params.source === 'text' ? 'ws-chip-on' : ''}"
			onclick={() => (params.source = 'text')}
		>
			{$t('Text')}
		</button>
		<button
			type="button"
			class="ws-chip {params.source === 'svg' ? 'ws-chip-on' : ''}"
			onclick={() => (params.source = 'svg')}
		>
			SVG
		</button>
	</div>

	{#if params.source === 'text'}
		<label class="flex flex-col gap-1">
			<span class="text-content-muted">{$t('Text')}</span>
			<input bind:value={params.text} data-testid="emboss-text" class="ws-field ws-field-sm" />
		</label>
		<label class="flex items-center justify-between gap-2">
			<span class="text-content-muted">{$t('Font')}</span>
			<select bind:value={params.font} data-testid="emboss-font" class="ws-field ws-field-sm">
				{#each fonts as f (f.id)}
					<option value={f.id}>{f.label}</option>
				{/each}
			</select>
		</label>
		<label class="flex items-center justify-between gap-2">
			<span class="text-content-muted">{$t('Boldness (mm)')}</span>
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
			<span class="text-content-muted">{$t('Font size (mm)')}</span>
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
			<span class="text-content-muted">{$t('SVG shape')}</span>
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
						params.svgFileName = file.name;
					}
				}}
			/>
			<div class="flex items-center gap-2">
				<button
					type="button"
					class="ws-btn ws-btn-secondary"
					data-testid="emboss-svg-pick"
					onclick={() => svgInput?.click()}
				>
					{$t('Choose an SVG file…')}
				</button>
				<span class="min-w-0 truncate text-xs text-content-subtle" data-testid="emboss-svg-name">
					{svgFileName ?? (params.svg.trim() ? $t('Pasted SVG') : $t('No file'))}
				</span>
			</div>
			{#if svgPreview}
				<img
					src={svgPreview}
					alt={$t('SVG shape preview')}
					data-testid="emboss-svg-preview"
					class="h-24 w-full rounded border border-border bg-surface-sunken object-contain p-1"
				/>
			{/if}
			<details class="text-xs">
				<summary class="cursor-pointer text-content-subtle">{$t('SVG source (paste)')}</summary>
				<textarea
					bind:value={params.svg}
					data-testid="emboss-svg"
					rows="3"
					aria-label={$t('SVG source')}
					class="ws-field ws-field-sm mt-1 w-full font-mono"></textarea>
			</details>
		</div>
	{/if}

	{#if params.source === 'svg'}
		<label class="flex items-center justify-between gap-2">
			<span class="text-content-muted">{$t('Width (mm)')}</span>
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
		<span class="text-content-muted">{$t('Depth (mm)')}</span>
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
			{$t('Editing the existing emboss: it is regenerated in the same place.')}
		</p>
	{:else}
		<label class="flex items-center gap-2">
			<input
				type="checkbox"
				bind:checked={params.embossed}
				disabled={!canEngrave}
				data-testid="emboss-raised"
			/>
			<span class="text-content-muted">
				{$t('Embossed (unchecked: engraved into the selected object)')}
			</span>
		</label>

		{#if canEngrave && params.embossed}
			<label class="flex items-center gap-2">
				<input type="checkbox" bind:checked={params.onObject} data-testid="emboss-on-object" />
				<span class="text-content-muted">{$t('On the selected object (part placed on top)')}</span>
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
	>
		{busy
			? editing
				? $t('Updating…')
				: $t('Creating…')
			: editing
				? $t('Apply')
				: $t('Create object')}
	</button>
	{#if editing}
		<button
			type="button"
			class="ws-btn ws-btn-secondary self-start"
			data-testid="emboss-cancel"
			onclick={() => oncancel?.()}
		>
			{$t('Cancel')}
		</button>
	{/if}
	{#if !available}
		<p class="text-content-subtle">
			{$t('Volume creation requires the engine (coming soon).')}
		</p>
	{/if}
</div>
