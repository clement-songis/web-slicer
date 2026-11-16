<script lang="ts">
	import { sceneColors } from './scene-colors';
	// Plateau d'impression (T050) : surface, grille (pas 10 mm) et repère
	// d'origine, dans le plan XY (repère Z-up comme Orca).
	import { T } from '@threlte/core';
	import { gridDivisions, type BedShape } from './bed';

	interface Props {
		bed: BedShape;
		/** Affiche la grille du plateau (menu Vue → Show Gridlines, T109). */
		showGrid?: boolean;
	}

	let { bed, showGrid = true }: Props = $props();

	const size = $derived(Math.max(bed.width, bed.depth));
</script>

<!-- Surface du plateau, légèrement sous z=0 pour ne pas z-fighter avec la grille. -->
<T.Mesh position={[bed.center.x, bed.center.y, -0.05]} receiveShadow>
	<T.PlaneGeometry args={[bed.width, bed.depth]} />
	<T.MeshStandardMaterial
		color={$sceneColors.bed}
		transparent
		opacity={$sceneColors.bedOpacity}
		roughness={0.9}
	/>
</T.Mesh>

<!-- GridHelper repose dans le plan XZ : on le bascule dans le plan XY. -->
{#if showGrid}
	<T.GridHelper
		args={[size, gridDivisions(size), $sceneColors.gridMajor, $sceneColors.gridMinor]}
		rotation={[Math.PI / 2, 0, 0]}
		position={[bed.center.x, bed.center.y, 0]}
	/>
{/if}

<!-- Repère d'origine (0,0,0) du plateau. -->
<T.AxesHelper args={[Math.min(bed.width, bed.depth) * 0.15]} />
