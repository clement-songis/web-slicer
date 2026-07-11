<script lang="ts">
	// Racine de la scène 3D de préparation (T050) : un Canvas Threlte qui héberge
	// le viewport. Conforme à references/orca-prepare.png (plateau + grille +
	// caméra trois-quarts).
	import { Canvas } from '@threlte/core';
	import type { BedShape } from './bed';
	import type { SceneObject } from './mesh';
	import type { GizmoMode } from './gizmos/types';
	import type { Transform } from './transform';
	import Viewport from './Viewport.svelte';

	interface Props {
		bed: BedShape;
		objects?: SceneObject[];
		selection?: Set<string>;
		/** Mode du gizmo de transformation, ou null pour le masquer (T103). */
		gizmoMode?: GizmoMode | null;
		/** Remonte la transformation de l'objet manipulé (T103). */
		ontransform?: (id: string, transform: Transform) => void;
	}

	let {
		bed,
		objects = [],
		selection = $bindable(new Set<string>()),
		gizmoMode = null,
		ontransform
	}: Props = $props();
</script>

<div class="h-full w-full">
	<Canvas>
		<Viewport {bed} {objects} bind:selection {gizmoMode} {ontransform} />
	</Canvas>
</div>
