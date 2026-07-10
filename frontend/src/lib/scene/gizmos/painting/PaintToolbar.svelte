<script lang="ts">
	// Barre du gizmo de peinture (T056) : canal, outil (pinceau/sphère/
	// remplissage), état peint (enforcer/blocker) et rayon ajustable. Raccourci
	// groupe Gizmo : Ctrl/Alt + molette pour le rayon. Présentationnel — le
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

	// Ctrl/Alt + molette ajuste le rayon (raccourci groupe Gizmo d'Orca).
	function onwheel(e: WheelEvent) {
		if (!e.ctrlKey && !e.altKey) return;
		e.preventDefault();
		const step = e.deltaY < 0 ? 0.2 : -0.2;
		radius = Math.min(MAX_RADIUS, Math.max(MIN_RADIUS, Math.round((radius + step) * 10) / 10));
	}
</script>

<div class="flex flex-col gap-2 text-sm" role="toolbar" tabindex="0" {onwheel}>
	<label class="flex items-center gap-2">
		<span class="w-16 text-slate-300">Canal</span>
		<select bind:value={channel} class="rounded border border-slate-600 bg-slate-900 px-1 py-0.5">
			{#each CHANNELS as c (c)}
				<option value={c}>{CHANNEL_LABELS[c]}</option>
			{/each}
		</select>
	</label>

	<div class="flex items-center gap-2">
		<span class="w-16 text-slate-300">Outil</span>
		{#each tools as t (t)}
			<button
				type="button"
				class="rounded border border-slate-600 px-2 py-0.5 {tool === t
					? 'bg-slate-600 text-white'
					: 'bg-slate-800'}"
				aria-pressed={tool === t}
				onclick={() => (tool = t)}>{PAINT_TOOL_LABELS[t]}</button
			>
		{/each}
	</div>

	{#if channel !== 'mmu'}
		<div class="flex items-center gap-2">
			<span class="w-16 text-slate-300">Peindre</span>
			<button
				type="button"
				class="rounded border border-slate-600 px-2 py-0.5 {state === ENFORCER
					? 'bg-emerald-600 text-white'
					: 'bg-slate-800'}"
				onclick={() => (state = ENFORCER)}>Forcer</button
			>
			<button
				type="button"
				class="rounded border border-slate-600 px-2 py-0.5 {state === BLOCKER
					? 'bg-red-600 text-white'
					: 'bg-slate-800'}"
				onclick={() => (state = BLOCKER)}>Bloquer</button
			>
		</div>
	{:else}
		<label class="flex items-center gap-2">
			<span class="w-16 text-slate-300">Extrudeur</span>
			<input
				type="number"
				min={1}
				step={1}
				bind:value={state}
				class="w-16 rounded border border-slate-600 bg-slate-900 px-1 text-right"
				aria-label="Index d'extrudeur"
			/>
		</label>
	{/if}

	{#if tool !== 'fill'}
		<label class="flex items-center gap-2">
			<span class="w-16 text-slate-300">Rayon</span>
			<input
				type="range"
				min={MIN_RADIUS}
				max={MAX_RADIUS}
				step={0.1}
				bind:value={radius}
				class="flex-1"
			/>
			<span class="w-10 text-right text-slate-400">{radius.toFixed(1)}</span>
		</label>
		<p class="text-xs text-slate-500">Ctrl/Alt + molette : ajuster le rayon</p>
	{/if}
</div>
