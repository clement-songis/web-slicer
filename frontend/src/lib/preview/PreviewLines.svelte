<script lang="ts">
	// Rendu des segments de prévisualisation (T068) : assemble une
	// `THREE.BufferGeometry` (position + couleur par sommet) depuis la géométrie
	// pure et l'affiche en `LineSegments` à couleurs de sommet. Présentational :
	// la coloration/visibilité est décidée en amont (lib preview).
	import { T } from '@threlte/core';
	import * as THREE from 'three';
	import type { PreviewGeometry } from './geometry';

	interface Props {
		geometry: PreviewGeometry;
	}

	let { geometry: data }: Props = $props();

	const geometry = $derived.by(() => {
		const g = new THREE.BufferGeometry();
		g.setAttribute('position', new THREE.BufferAttribute(data.positions, 3));
		g.setAttribute('color', new THREE.BufferAttribute(data.colors, 3));
		return g;
	});

	// Libère la géométrie précédente au changement / démontage.
	$effect(() => {
		const g = geometry;
		return () => g.dispose();
	});
</script>

<T.LineSegments {geometry}>
	<T.LineBasicMaterial vertexColors />
</T.LineSegments>
