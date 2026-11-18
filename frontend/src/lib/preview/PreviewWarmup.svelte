<script lang="ts">
	// Précompilation des shaders de l'aperçu (gel du premier rendu) : le rendu
	// automatique est suspendu tant que `renderer.compileAsync` (three.js,
	// KHR_parallel_shader_compile) n'a pas compilé les programmes de la scène —
	// la compilation se fait sans bloquer le fil principal, puis la première
	// image est dessinée. Garde-fou : rendu repris après WARMUP_TIMEOUT_MS.
	import { useThrelte } from '@threlte/core';
	import { onMount } from 'svelte';
	import { warmupScene, WARMUP_TIMEOUT_MS } from './warmup';

	interface Props {
		/** Compilation terminée (durée en ms, `timedOut` si le garde-fou a joué). */
		onready?: (result: { ms: number; timedOut: boolean }) => void;
	}

	let { onready }: Props = $props();
	const { renderer, scene, camera, autoRender, invalidate } = useThrelte();
	// Avant la première image : pas de rendu synchrone (compilation bloquante).
	autoRender.set(false);

	onMount(() => {
		let alive = true;
		void warmupScene(() => renderer.compileAsync(scene, camera.current), WARMUP_TIMEOUT_MS).then(
			(result) => {
				if (!alive) return;
				autoRender.set(true);
				invalidate();
				onready?.(result);
			}
		);
		return () => {
			alive = false;
			autoRender.set(true);
		};
	});
</script>
