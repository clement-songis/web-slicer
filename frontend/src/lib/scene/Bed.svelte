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
		/** Plateau non actif (phase 30) : surface atténuée, sans repère d'origine. */
		inactive?: boolean;
		/** Clic sur la surface (plateau non actif → l'activer, comme Orca). */
		onselect?: () => void;
	}

	let { bed, showGrid = true, inactive = false, onselect }: Props = $props();

	// Clic franc uniquement : un glisser d'orbite qui finit sur le plateau
	// n'active rien (même seuil que le picking Threlte).
	function onclick(e: { delta: number; stopPropagation: () => void }) {
		if (e.delta > 4) return;
		e.stopPropagation();
		onselect?.();
	}

	const size = $derived(Math.max(bed.width, bed.depth));
</script>

<!-- Surface du plateau, légèrement sous z=0 pour ne pas z-fighter avec la grille. -->
{#if onselect}
	<T.Mesh
		position={[bed.center.x, bed.center.y, -0.05]}
		receiveShadow
		{onclick}
		userData={{ plateSurface: true }}
	>
		<T.PlaneGeometry args={[bed.width, bed.depth]} />
		<T.MeshStandardMaterial
			color={$sceneColors.bed}
			transparent
			opacity={$sceneColors.bedOpacity * (inactive ? 0.55 : 1)}
			roughness={0.9}
		/>
	</T.Mesh>
{:else}
	<T.Mesh position={[bed.center.x, bed.center.y, -0.05]} receiveShadow>
		<T.PlaneGeometry args={[bed.width, bed.depth]} />
		<T.MeshStandardMaterial
			color={$sceneColors.bed}
			transparent
			opacity={$sceneColors.bedOpacity * (inactive ? 0.55 : 1)}
			roughness={0.9}
		/>
	</T.Mesh>
{/if}

<!-- GridHelper repose dans le plan XZ : on le bascule dans le plan XY. -->
{#if showGrid}
	<T.GridHelper
		args={[size, gridDivisions(size), $sceneColors.gridMajor, $sceneColors.gridMinor]}
		rotation={[Math.PI / 2, 0, 0]}
		position={[bed.center.x, bed.center.y, 0]}
	/>
{/if}

<!-- Repère d'origine (0,0,0) du plateau (plateau actif seulement). -->
{#if !inactive}
	<T.AxesHelper args={[Math.min(bed.width, bed.depth) * 0.15]} />
{/if}
