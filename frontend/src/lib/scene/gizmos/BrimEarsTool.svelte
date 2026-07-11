<script lang="ts">
	// Outil oreilles de bord (T057, BrimEars) : liste les points d'ancrage posés
	// sur le modèle et permet de les retirer. Présentationnel — le gizmo place les
	// points par raycast ; les paramètres (`brim_ears_*`) vivent dans les réglages.
	import type { BrimEarPoint } from './brim-ears';

	interface Props {
		points: readonly BrimEarPoint[];
		onremove: (index: number) => void;
		onclear?: () => void;
	}

	let { points, onremove, onclear }: Props = $props();
</script>

<div class="flex flex-col gap-2 text-sm">
	{#if points.length === 0}
		<p class="text-content-subtle">Cliquez sur le modèle pour poser des oreilles de bord.</p>
	{:else}
		<ul class="flex flex-col gap-1">
			{#each points as p, i (i)}
				<li class="flex items-center justify-between rounded bg-surface-sunken px-2 py-0.5">
					<span>{p.x.toFixed(1)}, {p.y.toFixed(1)}</span>
					<button
						type="button"
						class="px-1 text-xs"
						aria-label={`Retirer l'oreille ${i + 1}`}
						onclick={() => onremove(i)}>✕</button
					>
				</li>
			{/each}
		</ul>
		<button
			type="button"
			class="self-start rounded border border-border-strong px-3 py-1 hover:bg-overlay"
			onclick={() => onclear?.()}>Tout effacer</button
		>
	{/if}
</div>
