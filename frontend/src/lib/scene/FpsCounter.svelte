<script lang="ts">
	// Compteur d'images/s (Orca Preferences › « Show FPS ») en coin de vue 3D.
	import { appPreferences } from '#lib/settings/preferences';
	import type { FrameMeter } from './render-settings';

	let { meter }: { meter: FrameMeter } = $props();
	let fps = $state<number | null>(null);
	const shown = $derived($appPreferences.graphics.showFps);

	$effect(() => {
		if (!shown) return;
		meter.sample(performance.now());
		const timer = setInterval(() => {
			const value = meter.sample(performance.now());
			if (value !== null) fps = value;
		}, 500);
		return () => clearInterval(timer);
	});
</script>

{#if shown}
	<div
		class="pointer-events-none absolute top-2 right-2 z-10 rounded bg-surface-raised/80 px-1.5 py-0.5 font-mono text-2xs text-content-muted tabular-nums"
		data-testid="fps-counter"
		data-fps={fps ?? undefined}
		aria-live="off"
	>
		{fps ?? '–'} img/s
	</div>
{/if}
