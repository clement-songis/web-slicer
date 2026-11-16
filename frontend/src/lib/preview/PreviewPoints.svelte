<script lang="ts">
	// Marqueurs de l'aperçu (rétractions, réamorçages, coutures, changements
	// d'outil…) : points de taille fixe à l'écran, couleurs de sommet
	// (DEFAULT_OPTIONS_COLORS d'Orca). Présentationnel.
	import { T } from '@threlte/core';
	import * as THREE from 'three';
	import type { PreviewGeometry } from './geometry';

	interface Props {
		geometry: PreviewGeometry;
		size?: number;
	}

	let { geometry: data, size = 6 }: Props = $props();

	const geometry = $derived.by(() => {
		const g = new THREE.BufferGeometry();
		g.setAttribute('position', new THREE.BufferAttribute(data.positions, 3));
		g.setAttribute('color', new THREE.BufferAttribute(data.colors, 3));
		return g;
	});

	$effect(() => {
		const g = geometry;
		return () => g.dispose();
	});
</script>

<T.Points {geometry}>
	<T.PointsMaterial vertexColors {size} sizeAttenuation={false} depthTest={false} />
</T.Points>
