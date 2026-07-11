<script lang="ts">
	// Hôte Canvas de l'aperçu G-code (T088) : caméra cadrée sur le plateau,
	// éclairage neutre et lignes d'extrusion colorées (PreviewLines). Pendant du
	// `Scene.svelte` de préparation, réutilisé par le workspace éditeur.
	import { Canvas, T } from '@threlte/core';
	import { OrbitControls } from '@threlte/extras';
	import { frameBed, type BedShape } from '$lib/scene';
	import PreviewLines from './PreviewLines.svelte';
	import type { PreviewGeometry } from './geometry';

	interface Props {
		geometry: PreviewGeometry;
		bed: BedShape;
	}

	let { geometry, bed }: Props = $props();

	const pose = $derived(frameBed(bed));
</script>

<div class="h-full w-full">
	<Canvas>
		<T.PerspectiveCamera
			makeDefault
			position={pose.position}
			up={[0, 0, 1]}
			fov={50}
			near={1}
			far={20000}
		>
			<OrbitControls enableDamping target={pose.target} />
		</T.PerspectiveCamera>
		<T.AmbientLight intensity={0.9} />
		<PreviewLines {geometry} />
	</Canvas>
</div>
