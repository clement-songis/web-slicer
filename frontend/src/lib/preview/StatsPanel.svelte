<script lang="ts">
	// Panneau de statistiques de prévisualisation (T070, FR-043 ; parité
	// orca-preview.png) : estimation totale, répartition par type de ligne
	// (temps + %), consommation de filament par extrudeur (longueur/volume/
	// masse/coût) et changements d'outil. Présentational : le modèle de vue
	// est construit en amont (`buildPreviewStats`).
	import type { PreviewStats } from './stats';
	import { formatDuration } from './stats';
	import type { Rgb } from './colorations';

	interface Props {
		stats: PreviewStats;
	}

	let { stats }: Props = $props();

	/** Couleur normalisée (0–1) → `rgb()` CSS. */
	function css(color: Rgb): string {
		const [r, g, b] = color;
		return `rgb(${Math.round(r * 255)} ${Math.round(g * 255)} ${Math.round(b * 255)})`;
	}

	const pct = (v: number) => `${v.toFixed(1)}%`;
	const mm = (v: number) => `${v.toFixed(1)} mm`;
	const cm3 = (v: number) => `${v.toFixed(2)} cm³`;
	const grams = (v: number) => `${v.toFixed(2)} g`;
	const money = (v: number) => v.toFixed(2);
</script>

<section class="flex w-72 flex-col gap-4 p-3 text-sm text-slate-200" aria-label="Statistiques">
	<!-- Estimation globale -->
	<div class="flex flex-col gap-1">
		<h3 class="font-semibold text-slate-100">Estimation</h3>
		<dl class="grid grid-cols-2 gap-x-2 gap-y-0.5">
			<dt class="text-slate-400">Temps total</dt>
			<dd class="text-right tabular-nums">{stats.totalTimeText}</dd>
			<dt class="text-slate-400">Couches</dt>
			<dd class="text-right tabular-nums">{stats.layerCount}</dd>
			<dt class="text-slate-400">Changements d'outil</dt>
			<dd class="text-right tabular-nums">{stats.toolchanges}</dd>
		</dl>
	</div>

	<!-- Répartition par type de ligne -->
	{#if stats.types.length}
		<div class="flex flex-col gap-1">
			<h3 class="font-semibold text-slate-100">Par type de ligne</h3>
			<table class="w-full border-collapse">
				<thead>
					<tr class="text-xs text-slate-400">
						<th class="text-left font-normal">Type</th>
						<th class="text-right font-normal">Temps</th>
						<th class="text-right font-normal">%</th>
					</tr>
				</thead>
				<tbody>
					{#each stats.types as t (t.kind)}
						<tr>
							<td class="flex items-center gap-1.5 py-0.5">
								<span
									class="inline-block h-3 w-3 shrink-0 rounded-sm"
									style:background-color={css(t.color)}
								></span>
								<span class="truncate">{t.name}</span>
							</td>
							<td class="text-right tabular-nums">{formatDuration(t.timeSeconds)}</td>
							<td class="text-right tabular-nums text-slate-400">{pct(t.timePercent)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	<!-- Consommation de filament -->
	{#if stats.filaments.length}
		<div class="flex flex-col gap-1">
			<h3 class="font-semibold text-slate-100">Filament</h3>
			<table class="w-full border-collapse">
				<thead>
					<tr class="text-xs text-slate-400">
						<th class="text-left font-normal">#</th>
						<th class="text-right font-normal">Longueur</th>
						<th class="text-right font-normal">Volume</th>
						<th class="text-right font-normal">Masse</th>
						<th class="text-right font-normal">Coût</th>
					</tr>
				</thead>
				<tbody>
					{#each stats.filaments as f (f.extruder)}
						<tr class="tabular-nums">
							<td class="text-left text-slate-400">{f.extruder + 1}</td>
							<td class="text-right">{mm(f.lengthMm)}</td>
							<td class="text-right">{cm3(f.volumeCm3)}</td>
							<td class="text-right">{grams(f.massG)}</td>
							<td class="text-right">{money(f.cost)}</td>
						</tr>
					{/each}
				</tbody>
				<tfoot>
					<tr class="border-t border-slate-700 tabular-nums font-medium">
						<td class="text-left text-slate-400">Σ</td>
						<td></td>
						<td></td>
						<td class="text-right">{grams(stats.totalMassG)}</td>
						<td class="text-right">{money(stats.totalCost)}</td>
					</tr>
				</tfoot>
			</table>
		</div>
	{/if}
</section>
