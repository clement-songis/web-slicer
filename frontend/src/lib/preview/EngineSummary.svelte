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
	import { t as translate } from '$lib/i18n';
	import { formatMoney } from '$lib/settings/preferences';
	import { formatDuration, type PreviewStats } from './stats';
	import { summaryView, type EngineMeta } from './engine-track';
	import { OPTION_COLORS, SEGMENT_ROLE_COLORS, type Rgb } from './colorations';

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
	}

	let { meta, fallback, mode = 0, onmode, currency = '€', hidden, ontoggletype }: Props = $props();

	// Noms des rôles Orca (ExtrusionRole, mêmes ids que SEGMENT_ROLE_COLORS).
	const ORCA_ROLES: Record<number, string> = {
		0: 'Aucun',
		1: 'Paroi intérieure',
		2: 'Paroi extérieure',
		3: 'Paroi en surplomb',
		4: 'Remplissage interne',
		5: 'Remplissage plein',
		6: 'Surface supérieure',
		7: 'Surface inférieure',
		8: 'Lissage',
		9: 'Pont',
		10: 'Pont interne',
		11: 'Remplissage d’espace',
		12: 'Jupe',
		13: 'Bordure',
		14: 'Support',
		15: 'Interface de support',
		16: 'Transition de support',
		17: 'Tour d’essuyage',
		18: 'Personnalisé',
		19: 'Mixte'
	};
	const CUSTOM_LABELS: Record<string, string> = {
		color_change: 'Changement de couleur',
		pause_print: 'Pause',
		tool_change: 'Changement d’outil',
		template: 'Modèle',
		custom: 'G-code personnalisé',
		unknown: 'Inconnu'
	};

	const view = $derived(meta ? summaryView(meta, ORCA_ROLES, mode) : null);

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
					label: ORCA_ROLES[role] ?? `Rôle ${role}`,
					color: SEGMENT_ROLE_COLORS[role] ?? [0.5, 0.5, 0.5],
					time: t[mode],
					percent: (100 * t[mode]) / total,
					kind: role
				});
		});
		const extra: [string, string, Rgb, [number, number]][] = [
			['travel', 'Déplacements', OPTION_COLORS.travel, meta.travel_time],
			['wipe', 'Essuyages', OPTION_COLORS.wipe, meta.wipe_time]
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
			<h3 class="ws-section-title">Estimation</h3>
			{#if view?.hasStealth && onmode}
				<div class="ws-seg" role="group" aria-label="Mode d’estimation">
					{#each ['Normal', 'Silencieux'] as label, i (label)}
						<button
							type="button"
							class="ws-seg-item {mode === i ? 'ws-seg-on' : ''}"
							aria-pressed={mode === i}
							data-testid={`engine-mode-${i}`}
							onclick={() => onmode?.(i)}>{label}</button
						>
					{/each}
				</div>
			{/if}
		</div>
		<dl class="grid grid-cols-[1fr_auto] gap-x-3 gap-y-0.5">
			<dt class="text-content-muted">Temps total</dt>
			<dd
				class="text-right font-medium whitespace-nowrap tabular-nums"
				data-testid="engine-total-time"
			>
				{view ? formatDuration(view.total[mode]) : fallback.totalTimeText}
			</dd>
			{#if view}
				{#if view.hasStealth}
					<dt class="text-content-muted">{mode === 0 ? 'Mode silencieux' : 'Mode normal'}</dt>
					<dd class="text-right whitespace-nowrap tabular-nums">
						{formatDuration(view.total[mode === 0 ? 1 : 0])}
					</dd>
				{/if}
				<dt class="text-content-muted">Première couche</dt>
				<dd class="text-right whitespace-nowrap tabular-nums" data-testid="engine-first-layer">
					{formatDuration(view.firstLayer[mode])}
				</dd>
				<dt
					class="text-content-muted"
					data-tip="Temps de préparation écrit par Orca (chauffe, mise à l’origine…)"
					data-tip-side="right"
				>
					Préparation
				</dt>
				<dd class="text-right whitespace-nowrap tabular-nums" data-testid="engine-prepare">
					{formatDuration(view.prepare[mode])}
				</dd>
			{/if}
			<dt class="text-content-muted">Couches</dt>
			<dd class="text-right whitespace-nowrap tabular-nums">
				{meta?.layer_count || fallback.layerCount}
			</dd>
			<dt class="text-content-muted">Changements de filament</dt>
			<dd class="text-right whitespace-nowrap tabular-nums" data-testid="summary-filament-changes">
				{view ? view.filamentChanges : fallback.toolchanges}
			</dd>
			{#if view && view.extruderChanges > 0}
				<dt class="text-content-muted">Changements d’extrudeur</dt>
				<dd
					class="text-right whitespace-nowrap tabular-nums"
					data-testid="summary-extruder-changes"
				>
					{view.extruderChanges}
				</dd>
			{/if}
			{#if view && view.travelDistance > 0}
				<dt class="text-content-muted">Distance de déplacement</dt>
				<dd class="text-right whitespace-nowrap tabular-nums">
					{(view.travelDistance / 1000).toFixed(2)} m
				</dd>
			{/if}
		</dl>
		{#if !meta}
			<p class="text-xs text-content-subtle" data-testid="summary-approx">
				Estimation approchée (relecture du G-code) — la simulation Orca n’est pas disponible.
			</p>
		{/if}
	</div>

	<!-- Par type de ligne (une seule liste, déplacements et essuyages compris) -->
	{#if rows.length}
		<div class="flex flex-col gap-1">
			<h3 class="ws-section-title">Par type de ligne</h3>
			<table class="w-full border-collapse" data-testid="engine-role-times">
				<thead>
					<tr class="text-xs text-content-muted">
						<th class="text-left font-normal">Type</th>
						<th class="text-right font-normal">Temps</th>
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
										data-tip="Afficher/masquer ce type"
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
							<td class="pl-2 text-right whitespace-nowrap tabular-nums"
								>{formatDuration(row.time)}</td
							>
							<td class="pl-2 text-right whitespace-nowrap tabular-nums text-content-muted">
								{row.percent.toFixed(1)} %
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
			<h3 class="ws-section-title">Filament</h3>
			<table class="w-full border-collapse" data-testid="engine-extruders">
				<thead>
					<tr class="text-xs text-content-muted">
						<th class="text-left font-normal">#</th>
						<th class="text-right font-normal">Longueur</th>
						<th class="text-right font-normal">Volume</th>
						<th class="text-right font-normal">Masse</th>
						<th class="text-right font-normal">Coût</th>
						{#if multi && meta}<th class="text-right font-normal">Temps</th>{/if}
					</tr>
				</thead>
				<tbody>
					{#each filaments as f (f.extruder)}
						<tr class="tabular-nums">
							<td class="text-left text-content-muted">{f.extruder + 1}</td>
							<td class="pl-2 text-right whitespace-nowrap">{f.meters.toFixed(2)} m</td>
							<td class="pl-2 text-right whitespace-nowrap">{f.cm3.toFixed(2)} cm³</td>
							<td class="pl-2 text-right whitespace-nowrap">{f.grams.toFixed(2)} g</td>
							<td class="pl-2 text-right whitespace-nowrap">{money(f.cost)}</td>
							{#if multi && meta}<td class="pl-2 text-right whitespace-nowrap"
									>{formatDuration(f.time ?? 0)}</td
								>{/if}
						</tr>
					{/each}
				</tbody>
				{#if multi}
					<tfoot>
						<tr class="border-t border-border font-medium tabular-nums">
							<td class="text-left text-content-muted">Σ</td>
							<td></td>
							<td></td>
							<td class="pl-2 text-right whitespace-nowrap">{totalGrams.toFixed(2)} g</td>
							<td class="pl-2 text-right whitespace-nowrap">{money(totalCost)}</td>
							{#if meta}<td></td>{/if}
						</tr>
					</tfoot>
				{/if}
			</table>
		</div>
	{/if}

	<!-- G-code personnalisés, pauses, changements de couleur -->
	{#if view && (view.customGcodes.length || view.colorChangeTimes.length)}
		<div class="flex flex-col gap-1">
			<h3 class="ws-section-title">Événements</h3>
			{#if view.customGcodes.length}
				<ul class="flex flex-col gap-0.5" data-testid="engine-custom-gcodes">
					{#each view.customGcodes as g, i (i)}
						<li class="flex justify-between gap-2">
							<span>{CUSTOM_LABELS[g.type] ?? g.type}{g.extra ? ' — ' + g.extra : ''}</span>
							<span class="tabular-nums text-content-muted">Z {g.print_z.toFixed(2)}</span>
						</li>
					{/each}
				</ul>
			{/if}
			{#each view.colorChangeTimes as c, i (i)}
				<div class="flex justify-between gap-2 text-content-muted">
					<span>{CUSTOM_LABELS[c.type] ?? c.type} à</span>
					<span class="tabular-nums"
						>{formatDuration(c.time)} (reste {formatDuration(c.remaining)})</span
					>
				</div>
			{/each}
		</div>
	{/if}
</div>
