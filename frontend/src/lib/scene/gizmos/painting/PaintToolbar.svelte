<script lang="ts">
	// Barre du gizmo de peinture (T056) : canal, outil (pinceau/sphère/
	// remplissage), état peint (enforcer/blocker) et rayon ajustable. Raccourci
	// groupe Gizmo : Ctrl + molette pour le rayon (Alt + molette = plan de coupe, géré par la page). Présentationnel — le
	// viewport raycaste et applique via `trianglesInRadius` + `TrianglePainting`.
	import { CHANNELS, type PaintChannel } from './painting';
	import { ENFORCER, BLOCKER } from './facets';
	import { PAINT_TOOL_LABELS, CHANNEL_LABELS, type PaintTool } from './types';

	interface Props {
		channel?: PaintChannel;
		tool?: PaintTool;
		radius?: number;
		/** État peint appliqué au clic (ENFORCER, BLOCKER, ou index d'extrudeur MMU). */
		state?: number;
	}

	let {
		channel = $bindable('supports'),
		tool = $bindable('brush'),
		radius = $bindable(2),
		state = $bindable(ENFORCER)
	}: Props = $props();

	const tools: PaintTool[] = ['brush', 'sphere', 'fill'];
	const MIN_RADIUS = 0.2;
	const MAX_RADIUS = 20;

	// Ctrl + molette ajuste le rayon (raccourci groupe Gizmo d'Orca) ; Alt +
	// molette remonte jusqu'à la page (plan de coupe).
	function onwheel(e: WheelEvent) {
		if (!e.ctrlKey) return;
		e.preventDefault();
		e.stopPropagation();
		const step = e.deltaY < 0 ? 0.2 : -0.2;
		radius = Math.min(MAX_RADIUS, Math.max(MIN_RADIUS, Math.round((radius + step) * 10) / 10));
	}
</script>

<div class="flex flex-col gap-2 text-sm" role="toolbar" tabindex="0" {onwheel}>
	<label class="flex items-center gap-2">
		<span class="w-16 text-content-muted">Canal</span>
		<select bind:value={channel} class="ws-field ws-field-sm">
			{#each CHANNELS as c (c)}
				<option value={c}>{CHANNEL_LABELS[c]}</option>
			{/each}
		</select>
	</label>

	<div class="flex items-center gap-2">
		<span class="w-16 text-content-muted">Outil</span>
		{#each tools as t (t)}
			<button
				type="button"
				class="ws-chip {tool === t ? 'ws-chip-on' : ''}"
				aria-pressed={tool === t}
				data-testid={`paint-tool-${t}`}
				onclick={() => (tool = t)}>{PAINT_TOOL_LABELS[t]}</button
			>
		{/each}
	</div>

	{#if channel !== 'mmu'}
		<div class="flex items-center gap-2">
			<span class="w-16 text-content-muted">Peindre</span>
			<button
				type="button"
				class="ws-chip {state === ENFORCER
					? 'bg-success text-primary-content border-transparent'
					: ''}"
				onclick={() => (state = ENFORCER)}>Forcer</button
			>
			<button
				type="button"
				class="ws-chip {state === BLOCKER
					? 'bg-danger text-primary-content border-transparent'
					: ''}"
				onclick={() => (state = BLOCKER)}>Bloquer</button
			>
		</div>
	{:else}
		<label class="flex items-center gap-2">
			<span class="w-16 text-content-muted">Extrudeur</span>
			<input
				type="number"
				min={1}
				step={1}
				bind:value={state}
				class="ws-field ws-field-sm w-16 text-right"
				aria-label="Index d'extrudeur"
			/>
		</label>
	{/if}

	{#if tool !== 'fill'}
		<label class="flex items-center gap-2">
			<span class="w-16 text-content-muted">Rayon</span>
			<input
				type="range"
				min={MIN_RADIUS}
				max={MAX_RADIUS}
				step={0.1}
				bind:value={radius}
				class="flex-1"
			/>
			<span class="w-10 text-right text-content-muted">{radius.toFixed(1)}</span>
		</label>
		<p class="text-xs text-content-subtle">
			Ctrl + molette : rayon · Alt + molette : plan de coupe
		</p>
	{/if}
</div>
