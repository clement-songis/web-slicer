<script lang="ts">
	// Panneau « G-code Window » de l'aperçu : lignes du fichier autour du début
	// de la couche haute visible (ligne courante surlignée), comme Orca.
	import { gcodeWindowFor, layerStartLines } from './gcode-window';

	interface Props {
		lines: readonly string[];
		layer: number;
	}

	let { lines, layer }: Props = $props();

	const starts = $derived(layerStartLines(lines));
	const view = $derived(gcodeWindowFor(lines, starts, layer));
</script>

<div
	class="pointer-events-auto max-h-[60vh] w-80 overflow-auto rounded bg-surface-raised/90 p-1 font-mono text-2xs text-content-muted"
	data-testid="gcode-window"
	data-current-line={view.current + 1}
	data-layer={layer}
>
	{#each view.lines as text, i (view.first + i)}
		{@const n = view.first + i}
		<div class="flex gap-2 {n === view.current ? 'bg-primary/30 text-content' : ''}">
			<span class="w-12 shrink-0 text-right text-content-subtle">{n + 1}</span>
			<span class="whitespace-pre">{text}</span>
		</div>
	{/each}
</div>
