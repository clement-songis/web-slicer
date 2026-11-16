<script lang="ts">
	// Panneau de statistiques de prévisualisation (T070, FR-043 ; parité
	// orca-preview.png) : estimation totale, répartition par type de ligne
	// (temps + %), consommation de filament par extrudeur (longueur/volume/
	// masse/coût) et changements d'outil. Présentational : le modèle de vue
	// est construit en amont (`buildPreviewStats`).
	import type { PreviewStats } from './stats';
	import { formatDuration } from './stats';
	import { formatMoney } from '$lib/settings/preferences';
	import type { Rgb } from './colorations';
	// Alias : la boucle des types nomme déjà sa variable `t`.
	import { t as label } from '$lib/i18n';

	interface Props {
		stats: PreviewStats;
		/** Types de ligne masqués (kind) — bascule de visibilité de la légende (T116). */
		hidden?: Set<number>;
		/** Bascule la visibilité d'un type de ligne. */
		ontoggletype?: (kind: number) => void;
		/** Devise des coûts (préférence ; filament_cost est en « money/kg »). */
		currency?: string;
	}

	let { stats, hidden, ontoggletype, currency = '€' }: Props = $props();

	/** Couleur normalisée (0–1) → `rgb()` CSS. */
	function css(color: Rgb): string {
		const [r, g, b] = color;
		return `rgb(${Math.round(r * 255)} ${Math.round(g * 255)} ${Math.round(b * 255)})`;
	}

	const pct = (v: number) => `${v.toFixed(1)}%`;
	const mm = (v: number) => `${v.toFixed(1)} mm`;
	const cm3 = (v: number) => `${v.toFixed(2)} cm³`;
	const grams = (v: number) => `${v.toFixed(2)} g`;
	const money = (v: number) => formatMoney(v, currency);
</script>

<section class="flex w-72 flex-col gap-4 p-3 text-sm text-content" aria-label="Statistiques">
	<!-- Estimation globale -->
	<div class="flex flex-col gap-1">
		<h3 class="ws-section-title">Estimation</h3>
		<dl class="grid grid-cols-[1fr_auto] gap-x-3 gap-y-0.5">
			<dt class="text-content-muted">Temps total</dt>
			<dd class="text-right whitespace-nowrap tabular-nums">{stats.totalTimeText}</dd>
			<dt class="text-content-muted">Couches</dt>
			<dd class="text-right whitespace-nowrap tabular-nums">{stats.layerCount}</dd>
			<dt class="text-content-muted">Changements d'outil</dt>
			<dd class="text-right whitespace-nowrap tabular-nums">{stats.toolchanges}</dd>
		</dl>
	</div>

	<!-- Répartition par type de ligne -->
	{#if stats.types.length}
		<div class="flex flex-col gap-1">
			<h3 class="ws-section-title">Par type de ligne</h3>
			<table class="w-full border-collapse">
				<thead>
					<tr class="text-xs text-content-muted">
						<th class="text-left font-normal">Type</th>
						<th class="text-right font-normal">Temps</th>
						<th class="text-right font-normal">%</th>
					</tr>
				</thead>
				<tbody>
					{#each stats.types as t (t.kind)}
						{@const off = hidden?.has(t.kind) ?? false}
						<tr class={off ? 'opacity-40' : ''}>
							<td class="py-0.5">
								{#if ontoggletype}
									<button
										type="button"
										class="flex items-center gap-1.5 text-left"
										aria-pressed={!off}
										title="Afficher/masquer ce type"
										onclick={() => ontoggletype(t.kind)}
									>
										<span
											class="inline-block h-3 w-3 shrink-0 rounded-sm"
											style:background-color={css(t.color)}
										></span>
										<span class="truncate">{$label(t.name)}</span>
									</button>
								{:else}
									<span class="flex items-center gap-1.5">
										<span
											class="inline-block h-3 w-3 shrink-0 rounded-sm"
											style:background-color={css(t.color)}
										></span>
										<span class="truncate">{$label(t.name)}</span>
									</span>
								{/if}
							</td>
							<td class="pl-2 text-right whitespace-nowrap tabular-nums"
								>{formatDuration(t.timeSeconds)}</td
							>
							<td class="pl-2 text-right whitespace-nowrap tabular-nums text-content-muted"
								>{pct(t.timePercent)}</td
							>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	<!-- Consommation de filament -->
	{#if stats.filaments.length}
		<div class="flex flex-col gap-1">
			<h3 class="ws-section-title">Filament</h3>
			<table class="w-full border-collapse">
				<thead>
					<tr class="text-xs text-content-muted">
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
							<td class="text-left text-content-muted">{f.extruder + 1}</td>
							<td class="pl-2 text-right whitespace-nowrap tabular-nums">{mm(f.lengthMm)}</td>
							<td class="pl-2 text-right whitespace-nowrap tabular-nums">{cm3(f.volumeCm3)}</td>
							<td class="pl-2 text-right whitespace-nowrap tabular-nums">{grams(f.massG)}</td>
							<td class="pl-2 text-right whitespace-nowrap tabular-nums">{money(f.cost)}</td>
						</tr>
					{/each}
				</tbody>
				<tfoot>
					<tr class="border-t border-border tabular-nums font-medium">
						<td class="text-left text-content-muted">Σ</td>
						<td></td>
						<td></td>
						<td class="pl-2 text-right whitespace-nowrap tabular-nums">{grams(stats.totalMassG)}</td
						>
						<td class="pl-2 text-right whitespace-nowrap tabular-nums">{money(stats.totalCost)}</td>
					</tr>
				</tfoot>
			</table>
		</div>
	{/if}
</section>
