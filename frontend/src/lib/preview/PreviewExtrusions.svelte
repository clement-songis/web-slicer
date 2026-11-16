<script lang="ts">
	// Extrusions en volumes pleins éclairés (parité Orca) : un InstancedMesh
	// (prisme à section ovale, largeur × hauteur réelles), couleurs d'instance.
	// Géométrie et matériau partagés, recréés seulement quand les données changent.
	import { T } from '@threlte/core';
	import * as THREE from 'three';
	import type { PreviewGeometry } from './geometry';
	import { instanceColors, instanceMatrices, sectionPrism } from './extrusions';

	interface Props {
		geometry: PreviewGeometry;
	}

	let { geometry: data }: Props = $props();

	const prism = sectionPrism();
	const base = new THREE.BufferGeometry();
	base.setAttribute('position', new THREE.BufferAttribute(prism.positions, 3));
	base.setAttribute('normal', new THREE.BufferAttribute(prism.normals, 3));
	const material = new THREE.MeshLambertMaterial();

	const mesh = $derived.by(() => {
		const count = data.visibleCount;
		const m = new THREE.InstancedMesh(base, material, Math.max(1, count));
		m.count = count;
		if (count > 0 && data.sizes) {
			m.instanceMatrix = new THREE.InstancedBufferAttribute(
				instanceMatrices(data.positions, data.sizes, count),
				16
			);
			m.instanceColor = new THREE.InstancedBufferAttribute(instanceColors(data.colors, count), 3);
		}
		m.frustumCulled = false;
		return m;
	});

	$effect(() => {
		const m = mesh;
		return () => m.dispose();
	});
	$effect(() => () => {
		base.dispose();
		material.dispose();
	});
</script>

<T is={mesh} />
