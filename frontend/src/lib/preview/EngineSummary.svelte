<script lang="ts">
	// Résumé du tranchage (colonne de gauche de l'aperçu) — un seul bloc, comme
	// le panneau « Résumé » d'Orca (GCodeViewer::render_legend) :
	// - source principale = simulation du moteur (GCodeProcessor d'Orca :
	//   temps Normal / Silencieux, première couche, préparation, temps par rôle,
	//   déplacements et essuyages, filament par extrudeur, G-code personnalisés) ;
	// - repli = relecture texte du G-code (`PreviewStats`) tant que la méta moteur
	//   n'est pas disponible, signalé « estimation approchée ».
	// Les données viennent de `engine-track.ts` (summaryView) et de `stats.ts` ;
	// ce composant ne fait que la présentation.
	import { fmtNum, locale, t as translate } from '#lib/i18n';
	import { formatMoney } from '#lib/settings/preferences';
	import { formatDuration, type PreviewStats } from './stats';
	import { summaryView, type EngineMeta } from './engine-track';
	import { OPTION_COLORS, SEGMENT_ROLE_COLORS, type Rgb } from './colorations';
	import { hasFilamentBreakdown } from './engine-data';
	import FilamentTable from './FilamentTable.svelte';
	import { hasEvents, partialTimes, type PartialKind } from './partial-times';

	interface Props {
		/** Méta du moteur (simulation Orca) ; null → repli sur `fallback`. */
		meta: EngineMeta | null;
		/** Statistiques de relecture texte (repli). */
		fallback: PreviewStats;
		/** Mode d'estimation affiché (0 Normal, 1 Silencieux). */
		mode?: number;
		onmode?: (mode: number) => void;
		/** Devise des coûts (préférence). */
		currency?: string;
		/** Types de ligne masqués et bascule (mode « type de ligne »). */
		hidden?: Set<number>;
		ontoggletype?: (kind: number) => void;
		/** G-code externe : temps annoncé par son pied (affiché s'il diffère). */
		declared?: string | null;
	}

	let {
		meta,
		fallback,
		mode = 0,
		onmode,
		currency = '€',
		hidden,
		ontoggletype,
		declared = null
	}: Props = $props();

	// Noms des rôles Orca (ExtrusionRole, mêmes ids que SEGMENT_ROLE_COLORS).
	// Clés i18n anglaises, traduites au rendu (réactif à la langue).
	const ORCA_ROLE_KEYS: Record<number, string> = {
		0: 'None',
		1: 'Inner wall',
		2: 'Outer wall',
		3: 'Overhang wall',
		4: 'Sparse infill||role',
		5: 'Internal solid infill||role',
		6: 'Top surface',
		7: 'Bottom surface',
		8: 'Ironing',
		9: 'Bridge',
		10: 'Internal Bridge',
		11: 'Gap infill||role',
		12: 'Skirt',
		13: 'Brim',
		14: 'Support||role',
		15: 'Support interface',
		16: 'Support transition',
		17: 'Prime tower||role',
		18: 'Custom',
		19: 'Multiple||role'
	};
	const ORCA_ROLES = $derived(
		Object.fromEntries(
			Object.entries(ORCA_ROLE_KEYS).map(([k, v]) => [k, $translate(v)])
		) as Record<number, string>
	);
	const CUSTOM_LABELS: Record<string, string> = {
		color_change: 'Color Change',
		pause_print: 'Pause',
		tool_change: 'Tool change',
		template: 'Template',
		custom: 'Custom G-code',
		unknown: 'Unknown'
	};

	const view = $derived(meta ? summaryView(meta, ORCA_ROLES, mode) : null);
	// Tronçons d'impression entre pauses / changements de couleur (Orca
	// generate_partial_times) : pas d'« événement » pour le tronçon final.
	const partials = $derived(view ? partialTimes(view.colorChangeTimes, view.customGcodes) : []);
	const PARTIAL_LABELS: Record<PartialKind, string> = {
		print: 'Printing',
		pause: 'Pause',
		color_change: 'Color Change'
	};

	type Row = {
		key: string;
		label: string;
		color: Rgb;
		time: number;
		percent: number;
		kind?: number;
	};

	// Une seule liste par type : rôles + déplacements + essuyages (comme Orca).
	const rows = $derived.by((): Row[] => {
		if (!meta) {
			return fallback.types.map((t) => ({
				key: 'k' + t.kind,
				label: $translate(t.name),
				color: t.color,
				time: t.timeSeconds,
				percent: t.timePercent,
				kind: t.kind
			}));
		}
		const total = meta.summary.modes[mode]?.time || 1;
		const out: Row[] = [];
		meta.role_times.forEach((t, role) => {
			if (t[mode] > 0)
				out.push({
					key: 'r' + role,
					label: ORCA_ROLES[role] ?? $translate('Role {n}', { n: role }),
					color: SEGMENT_ROLE_COLORS[role] ?? [0.5, 0.5, 0.5],
					time: t[mode],
					percent: (100 * t[mode]) / total,
					kind: role
				});
		});
		const extra: [string, string, Rgb, [number, number]][] = [
			['travel', $translate('Travel||summary'), OPTION_COLORS.travel, meta.travel_time],
			['wipe', $translate('Wipes'), OPTION_COLORS.wipe, meta.wipe_time]
		];
		for (const [key, label, color, t] of extra)
			if (t[mode] > 0)
				out.push({ key, label, color, time: t[mode], percent: (100 * t[mode]) / total });
		return out.sort((a, b) => b.time - a.time);
	});

	type Fil = {
		extruder: number;
		meters: number;
		cm3: number;
		grams: number;
		cost: number;
		time?: number;
	};
	const filaments = $derived.by((): Fil[] => {
		if (!view || !meta) {
			return fallback.filaments.map((f) => ({
				extruder: f.extruder,
				meters: f.lengthMm / 1000,
				cm3: f.volumeCm3,
				grams: f.massG,
				cost: f.cost
			}));
		}
		return view.extruders.map((e) => ({
			extruder: e.extruder,
			meters: e.meters,
			cm3: e.volumeMm3 / 1000,
			grams: e.grams,
			// filament_cost d'Orca : monnaie / kg.
			cost: (e.grams / 1000) * (meta.summary.filament_costs[e.extruder] ?? 0),
			time: e.time
		}));
	});
	const totalGrams = $derived(filaments.reduce((s, f) => s + f.grams, 0));
	const totalCost = $derived(meta ? filaments.reduce((s, f) => s + f.cost, 0) : fallback.totalCost);
	const multi = $derived(filaments.length > 1);

	const css = ([r, g, b]: Rgb) =>
		`rgb(${Math.round(r * 255)} ${Math.round(g * 255)} ${Math.round(b * 255)})`;
	const money = (v: number) => formatMoney(v, currency);
</script>

<div
	class="flex flex-col gap-4 text-sm text-content"
	data-testid="engine-summary"
	data-source={meta ? 'engine' : 'text'}
>
	<!-- Estimation -->
	<div class="flex flex-col gap-1">
		<div class="flex items-center justify-between gap-2">
			<h3 class="ws-section-title">{$translate('Estimate')}</h3>
			{#if view?.hasStealth && onmode}
				<div class="ws-seg" role="group" aria-label={$translate('Estimation mode')}>
					{#each ['Normal', 'Silent'] as label, i (label)}
						<button
							type="button"
							class="ws-seg-item {mode === i ? 'ws-seg-on' : ''}"
							aria-pressed={mode === i}
							data-testid={`engine-mode-${i}`}
							onclick={() => onmode?.(i)}
						>
							{$translate(label)}
						</button>
					{/each}
				</div>
			{/if}
		</div>
		<dl class="grid grid-cols-[1fr_auto] gap-x-3 gap-y-0.5">
			<dt class="text-content-muted">{$translate('Total time')}</dt>
			<dd
				class="text-right font-medium whitespace-nowrap tabular-nums"
				data-testid="engine-total-time"
			>
				{view ? formatDuration(view.total[mode]) : fallback.totalTimeText}
			</dd>
			{#if view && declared && declared !== formatDuration(view.total[mode])}
				<dt
					class="text-content-muted"
					data-tip={$translate('Duration written in the file footer by the original slicer')}
					data-tip-side="right"
				>
					{$translate('Declared by the G-code')}
				</dt>
				<dd class="text-right whitespace-nowrap tabular-nums" data-testid="engine-declared-time">
					{declared}
				</dd>
			{/if}
			{#if view}
				{#if view.hasStealth}
					<dt class="text-content-muted">
						{mode === 0 ? $translate('Silent mode') : $translate('Normal mode')}
					</dt>
					<dd class="text-right whitespace-nowrap tabular-nums">
						{formatDuration(view.total[mode === 0 ? 1 : 0])}
					</dd>
				{/if}
				<dt class="text-content-muted">{$translate('First layer||summary')}</dt>
				<dd class="text-right whitespace-nowrap tabular-nums" data-testid="engine-first-layer">
					{formatDuration(view.firstLayer[mode])}
				</dd>
				<dt
					class="text-content-muted"
					data-tip={$translate('Preparation time written by Orca (heating, homing…)')}
					data-tip-side="right"
				>
					{$translate('Preparation')}
				</dt>
				<dd class="text-right whitespace-nowrap tabular-nums" data-testid="engine-prepare">
					{formatDuration(view.prepare[mode])}
				</dd>
			{/if}
			<dt class="text-content-muted">{$translate('Layers')}</dt>
			<dd class="text-right whitespace-nowrap tabular-nums">
				{meta?.layer_count || fallback.layerCount}
			</dd>
			<dt class="text-content-muted">{$translate('Filament changes')}</dt>
			<dd class="text-right whitespace-nowrap tabular-nums" data-testid="summary-filament-changes">
				{view ? view.filamentChanges : fallback.toolchanges}
			</dd>
			{#if view && view.extruderChanges > 0}
				<dt class="text-content-muted">{$translate('Extruder changes')}</dt>
				<dd
					class="text-right whitespace-nowrap tabular-nums"
					data-testid="summary-extruder-changes"
				>
					{view.extruderChanges}
				</dd>
			{/if}
			{#if view && view.travelDistance > 0}
				<dt class="text-content-muted">{$translate('Travel distance')}</dt>
				<dd class="text-right whitespace-nowrap tabular-nums">
					{fmtNum(view.travelDistance / 1000, 2, $locale)} m
				</dd>
			{/if}
		</dl>
		{#if !meta}
			<p class="text-xs text-content-subtle" data-testid="summary-approx">
				{$translate(
					'Approximate estimate (G-code re-read) — the Orca simulation is not available.'
				)}
			</p>
		{/if}
	</div>

	<!-- Par type de ligne (une seule liste, déplacements et essuyages compris) -->
	{#if rows.length}
		<div class="flex flex-col gap-1">
			<h3 class="ws-section-title">{$translate('By line type')}</h3>
			<table class="w-full border-collapse" data-testid="engine-role-times">
				<thead>
					<tr class="text-xs text-content-muted">
						<th class="text-left font-normal">{$translate('Type')}</th>
						<th class="text-right font-normal">{$translate('Time')}</th>
						<th class="text-right font-normal">%</th>
					</tr>
				</thead>
				<tbody>
					{#each rows as row (row.key)}
						{@const off = row.kind !== undefined && (hidden?.has(row.kind) ?? false)}
						<tr class={off ? 'opacity-40' : ''}>
							<td class="py-0.5">
								{#if ontoggletype && row.kind !== undefined}
									<button
										type="button"
										class="flex items-center gap-1.5 text-left"
										aria-pressed={!off}
										data-tip={$translate('Show/hide this type')}
										data-tip-side="right"
										onclick={() => row.kind !== undefined && ontoggletype(row.kind)}
									>
										<span
											class="inline-block h-3 w-3 shrink-0 rounded-sm"
											style:background-color={css(row.color)}
										></span>
										<span class="truncate">{row.label}</span>
									</button>
								{:else}
									<span class="flex items-center gap-1.5">
										<span
											class="inline-block h-3 w-3 shrink-0 rounded-sm"
											style:background-color={css(row.color)}
										></span>
										<span class="truncate">{row.label}</span>
									</span>
								{/if}
							</td>
							<td class="pl-2 text-right whitespace-nowrap tabular-nums">
								{formatDuration(row.time)}
							</td>
							<td class="pl-2 text-right whitespace-nowrap text-content-muted tabular-nums">
								{fmtNum(row.percent, 1, $locale)} %
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	<!-- Filament (un seul tableau) -->
	{#if filaments.length}
		<div class="flex flex-col gap-1">
			<h3 class="ws-section-title">{$translate('Filament')}</h3>
			<table class="w-full border-collapse" data-testid="engine-extruders">
				<thead>
					<tr class="text-xs text-content-muted">
						<th class="text-left font-normal">#</th>
						<th class="text-right font-normal">{$translate('Length')}</th>
						<th class="text-right font-normal">{$translate('Volume')}</th>
						<th class="text-right font-normal">{$translate('Mass')}</th>
						<th class="text-right font-normal">{$translate('Cost')}</th>
						{#if multi && meta}<th class="text-right font-normal">{$translate('Time')}</th>{/if}
					</tr>
				</thead>
				<tbody>
					{#each filaments as f (f.extruder)}
						<tr class="tabular-nums">
							<td class="text-left text-content-muted">{f.extruder + 1}</td>
							<td class="pl-2 text-right whitespace-nowrap">{fmtNum(f.meters, 2, $locale)} m</td>
							<td class="pl-2 text-right whitespace-nowrap">{fmtNum(f.cm3, 2, $locale)} cm³</td>
							<td class="pl-2 text-right whitespace-nowrap">{fmtNum(f.grams, 2, $locale)} g</td>
							<td class="pl-2 text-right whitespace-nowrap">{money(f.cost)}</td>
							{#if multi && meta}<td class="pl-2 text-right whitespace-nowrap">
									{formatDuration(f.time ?? 0)}
								</td>{/if}
						</tr>
					{/each}
				</tbody>
				{#if multi}
					<tfoot>
						<tr class="border-t border-border font-medium tabular-nums">
							<td class="text-left text-content-muted">Σ</td>
							<td></td>
							<td></td>
							<td class="pl-2 text-right whitespace-nowrap">{fmtNum(totalGrams, 2, $locale)} g</td>
							<td class="pl-2 text-right whitespace-nowrap">{money(totalCost)}</td>
							{#if meta}<td></td>{/if}
						</tr>
					</tfoot>
				{/if}
			</table>
		</div>
	{/if}

	<!-- Répartition par usage (lot 31 : tableau « Filament » d'Orca, modèle /
	     support / purge / tour) — seulement s'il y a plus que le modèle. -->
	{#if meta?.filament_table && hasFilamentBreakdown(meta.filament_table)}
		<div class="flex flex-col gap-1">
			<h3 class="ws-section-title">{$translate('Filament by usage')}</h3>
			<FilamentTable table={meta.filament_table} testid="engine-filament-table" />
		</div>
	{/if}

	<!-- G-code personnalisés, pauses, changements de couleur -->
	{#if view && (view.customGcodes.length || hasEvents(partials))}
		<div class="flex flex-col gap-1">
			<h3 class="ws-section-title">{$translate('Events')}</h3>
			{#if view.customGcodes.length}
				<ul class="flex flex-col gap-0.5" data-testid="engine-custom-gcodes">
					{#each view.customGcodes as g, i (i)}
						<li class="flex justify-between gap-2">
							<span>
								{$translate(CUSTOM_LABELS[g.type] ?? g.type)}{g.extra ? ' — ' + g.extra : ''}
							</span>
							<span class="text-content-muted tabular-nums">Z {fmtNum(g.print_z, 2, $locale)}</span>
						</li>
					{/each}
				</ul>
			{/if}
			{#if hasEvents(partials)}
				<table class="w-full text-xs" data-testid="engine-partial-times">
					<thead>
						<tr class="text-content-subtle">
							<th class="text-left font-normal">{$translate('Step')}</th>
							<th class="text-right font-normal">{$translate('Remaining')}</th>
							<th class="text-right font-normal">{$translate('Duration')}</th>
						</tr>
					</thead>
					<tbody>
						{#each partials as p, i (i)}
							<tr class={p.kind === 'print' ? 'text-content-muted' : 'text-content'}>
								<td>
									<span class="inline-flex items-center gap-1">
										{#if p.kind === 'color_change' && p.color}
											<span
												class="inline-block size-2.5 rounded-sm border border-border-strong"
												style:background-color={p.color}
												aria-hidden="true"
											></span>
										{/if}
										{$translate(PARTIAL_LABELS[p.kind])}
									</span>
								</td>
								<td class="text-right tabular-nums">
									{p.kind === 'print' ? formatDuration(p.remaining) : ''}
								</td>
								<td class="text-right tabular-nums">
									{p.kind === 'print' ? formatDuration(p.duration) : ''}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</div>
	{/if}
</div>
