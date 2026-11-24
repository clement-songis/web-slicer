<script lang="ts">
	// Légende de l'aperçu (parité GCodeViewer::render_legend d'Orca) :
	// - mode « type » : une entrée par rôle présent, pastille de couleur, clic =
	//   bascule de visibilité (comme cliquer une ligne de la légende d'Orca) ;
	// - modes continus : dégradé d'Orca et une valeur par arrêt (11, ou 1 si
	//   l'échelle est plate), échelle logarithmique signalée ;
	// - options d'affichage (EOptionType) : déplacements, essuyages,
	//   rétractions, réamorçages, coutures, changements d'outil / de couleur,
	//   pauses, avec leur couleur Orca et leur nombre dans les couches visibles.
	import { locale, numberLocale, t } from '#lib/i18n';
	import { OPTION_COLORS, SEGMENT_ROLE_COLORS, type Legend, type Rgb } from './colorations';
	import type { PreviewOption } from './geometry';

	interface Props {
		legend: Legend;
		/** Mode « type de ligne » : entrées par rôle, cliquables. */
		showTypes: boolean;
		/** Rôles présents (mode type) : id + nom Orca (clé i18n, data-role). */
		types: { id: number; name: string }[];
		hiddenTypes: number[];
		ontoggletype?: (kind: number) => void;
		/** Options disponibles et leur nombre dans les couches visibles. */
		options: { id: PreviewOption; count: number }[];
		enabled: ReadonlySet<PreviewOption>;
		ontoggleoption: (id: PreviewOption) => void;
		/** Piste G-code en cours de chargement (options pas encore connues). */
		loading?: boolean;
	}

	let {
		legend,
		showTypes,
		types,
		hiddenTypes,
		ontoggletype,
		options,
		enabled,
		ontoggleoption,
		loading = false
	}: Props = $props();

	const css = ([r, g, b]: Rgb) =>
		`rgb(${Math.round(r * 255)} ${Math.round(g * 255)} ${Math.round(b * 255)})`;

	const OPTION_LABELS: Record<PreviewOption, string> = {
		// Clés i18n anglaises, traduites au rendu.
		travel: 'Travel||summary',
		wipe: 'Wipes',
		retract: 'Retractions',
		unretract: 'Deretractions',
		seam: 'Seams',
		toolchange: 'Tool changes',
		colorchange: 'Color changes',
		pause: 'Pauses'
	};

	function fmt(v: number, unit: string): string {
		const digits = Math.abs(v) >= 100 ? 0 : Math.abs(v) >= 10 ? 1 : 2;
		return `${v.toLocaleString(numberLocale($locale), { maximumFractionDigits: digits })}${unit ? ' ' + unit : ''}`;
	}
</script>

<div class="flex flex-col gap-2 px-3 py-2" data-testid="preview-legend">
	{#if showTypes && types.length}
		<div class="flex flex-wrap gap-1.5" role="group" aria-label={$t('Line types')}>
			{#each types as type (type.id)}
				{@const off = hiddenTypes.includes(type.id)}
				<button
					type="button"
					class="ws-chip h-6 gap-1.5 font-normal {off ? 'opacity-50' : ''}"
					data-testid="legend-item"
					data-role={type.name}
					aria-pressed={!off}
					disabled={!ontoggletype}
					onclick={() => ontoggletype?.(type.id)}
				>
					<span
						class="inline-block h-2.5 w-2.5 shrink-0 rounded-full border border-border-strong"
						style:background-color={css(SEGMENT_ROLE_COLORS[type.id] ?? [0.5, 0.5, 0.5])}
						aria-hidden="true"
					></span>
					<span class={off ? 'line-through' : ''}>{$t(type.name)}</span>
				</button>
			{/each}
		</div>
	{:else if legend.kind === 'list'}
		<div class="flex flex-wrap gap-1.5" data-testid="legend-list">
			{#each legend.entries as entry (entry.label)}
				<span
					class="ws-badge gap-1.5 text-xs"
					data-testid="legend-entry"
					data-color={css(entry.color)}
				>
					<span
						class="inline-block h-2.5 w-2.5 shrink-0 rounded-full"
						style:background-color={css(entry.color)}
						aria-hidden="true"
					></span>
					{entry.label}
				</span>
			{/each}
		</div>
	{:else}
		{@const values = legend.values ?? [legend.min, legend.max]}
		<div
			class="flex flex-col gap-1"
			data-testid="legend-scale"
			data-log={legend.log ? 'true' : 'false'}
		>
			<div
				class="h-2.5 rounded-full border border-border"
				style:background-image={`linear-gradient(to right, ${legend.stops.map(css).join(', ')})`}
				aria-hidden="true"
			></div>
			<div class="flex justify-between gap-1 text-2xs text-content-muted tabular-nums">
				{#each values as v, i (i)}
					{#if values.length <= 3 || i % 2 === 0 || i === values.length - 1}
						<span>{fmt(v, i === values.length - 1 ? legend.unit : '')}</span>
					{/if}
				{/each}
			</div>
			{#if legend.log}<span class="text-2xs text-content-subtle">
					{$t('Logarithmic scale')}
				</span>{/if}
		</div>
	{/if}

	<div class="flex flex-wrap items-center gap-1.5" role="group" aria-label={$t('Display options')}>
		<span class="ws-section-title mr-1 text-2xs">{$t('Show||legend')}</span>
		{#if loading}
			<span class="text-xs text-content-subtle" aria-busy="true">{$t('Reading the G-code…')}</span>
		{:else}
			{#each options as opt (opt.id)}
				<button
					type="button"
					class="ws-chip h-6 gap-1.5 font-normal {enabled.has(opt.id) ? 'ws-chip-on' : ''}"
					aria-pressed={enabled.has(opt.id)}
					data-testid={`preview-option-${opt.id}`}
					onclick={() => ontoggleoption(opt.id)}
				>
					<span
						class="inline-block h-2.5 w-2.5 shrink-0 rounded-full"
						style:background-color={css(OPTION_COLORS[opt.id])}
						aria-hidden="true"
					></span>
					{$t(OPTION_LABELS[opt.id])}
					<span class="tabular-nums {enabled.has(opt.id) ? '' : 'text-content-muted'}">
						{opt.count}
					</span>
				</button>
			{/each}
		{/if}
	</div>
</div>
