<script lang="ts">
	// Outil oreilles de bord (T057, BrimEars) : liste les points d'ancrage posés
	// sur le modèle et permet de les retirer. Présentationnel — le gizmo place les
	// points par raycast ; les paramètres (`brim_ears_*`) vivent dans les réglages.
	import type { BrimEarPoint } from './brim-ears';
	import { t } from '$lib/i18n';
	import IconX from '~icons/lucide/x';

	interface Props {
		points: readonly BrimEarPoint[];
		onremove: (index: number) => void;
		onclear?: () => void;
	}

	let { points, onremove, onclear }: Props = $props();
</script>

<div class="flex flex-col gap-2 text-sm">
	{#if points.length === 0}
		<p class="text-content-subtle">{$t('Click the model to place brim ears.')}</p>
	{:else}
		<ul class="flex flex-col gap-1">
			{#each points as p, i (i)}
				<li class="flex items-center justify-between rounded bg-surface-sunken px-2 py-0.5">
					<span>{p.x.toFixed(1)}, {p.y.toFixed(1)}</span>
					<button
						type="button"
						class="ws-icon-btn ws-icon-btn-xs"
						aria-label={$t('Remove ear {n}', { n: i + 1 })}
						onclick={() => onremove(i)}><IconX class="h-3.5 w-3.5" aria-hidden="true" /></button
					>
				</li>
			{/each}
		</ul>
		<button type="button" class="ws-btn ws-btn-secondary self-start" onclick={() => onclear?.()}
			>{$t('Clear all')}</button
		>
	{/if}
</div>
