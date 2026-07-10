<script lang="ts">
	// Plateau d'impression (T050) : surface, grille (pas 10 mm) et repère
	// d'origine, dans le plan XY (repère Z-up comme Orca).
	import { T } from '@threlte/core';
	import { gridDivisions, type BedShape } from './bed';

	interface Props {
		bed: BedShape;
	}

	let { bed }: Props = $props();

	const size = $derived(Math.max(bed.width, bed.depth));
</script>

<!-- Surface du plateau, légèrement sous z=0 pour ne pas z-fighter avec la grille. -->
<T.Mesh position={[bed.center.x, bed.center.y, -0.05]} receiveShadow>
	<T.PlaneGeometry args={[bed.width, bed.depth]} />
	<T.MeshStandardMaterial color="#1e293b" transparent opacity={0.55} roughness={0.9} />
</T.Mesh>

<!-- GridHelper repose dans le plan XZ : on le bascule dans le plan XY. -->
<T.GridHelper
	args={[size, gridDivisions(size), 0x64748b, 0x334155]}
	rotation={[Math.PI / 2, 0, 0]}
	position={[bed.center.x, bed.center.y, 0]}
/>

<!-- Repère d'origine (0,0,0) du plateau. -->
<T.AxesHelper args={[Math.min(bed.width, bed.depth) * 0.15]} />
