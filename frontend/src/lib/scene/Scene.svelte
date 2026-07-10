<script lang="ts">
	// Racine de la scène 3D de préparation (T050) : un Canvas Threlte qui héberge
	// le viewport. Conforme à references/orca-prepare.png (plateau + grille +
	// caméra trois-quarts).
	import { Canvas } from '@threlte/core';
	import type { BedShape } from './bed';
	import type { SceneObject } from './mesh';
	import Viewport from './Viewport.svelte';

	interface Props {
		bed: BedShape;
		objects?: SceneObject[];
		selection?: Set<string>;
	}

	let { bed, objects = [], selection = $bindable(new Set<string>()) }: Props = $props();
</script>

<div class="h-full w-full">
	<Canvas>
		<Viewport {bed} {objects} bind:selection />
	</Canvas>
</div>
