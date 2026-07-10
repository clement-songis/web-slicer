<script lang="ts">
	// Un objet 3D de la scène (T050) : construit une BufferGeometry depuis le
	// maillage décodé, met en surbrillance la sélection, et remonte les clics
	// (raycast Three.js) au parent.
	import { T } from '@threlte/core';
	import type { IntersectionEvent } from '@threlte/extras';
	import * as THREE from 'three';
	import type { SceneMesh } from './mesh';

	interface Props {
		mesh: SceneMesh;
		position?: [number, number, number];
		selected?: boolean;
		onpick?: (additive: boolean) => void;
	}

	let { mesh, position = [0, 0, 0], selected = false, onpick }: Props = $props();

	const geometry = $derived.by(() => {
		const g = new THREE.BufferGeometry();
		g.setAttribute('position', new THREE.BufferAttribute(mesh.positions, 3));
		if (mesh.normals.length) {
			g.setAttribute('normal', new THREE.BufferAttribute(mesh.normals, 3));
		}
		g.setIndex(new THREE.BufferAttribute(mesh.indices, 1));
		if (!mesh.normals.length) g.computeVertexNormals();
		return g;
	});

	// Libère la géométrie précédente quand elle change / au démontage.
	$effect(() => {
		const g = geometry;
		return () => g.dispose();
	});

	function onclick(event: IntersectionEvent<MouseEvent>) {
		event.stopPropagation();
		const ne = event.nativeEvent;
		onpick?.(ne.shiftKey || ne.ctrlKey || ne.metaKey);
	}
</script>

<T.Mesh {geometry} {position} castShadow {onclick}>
	<T.MeshStandardMaterial
		color={selected ? '#f59e0b' : '#9aa7b4'}
		metalness={0.1}
		roughness={0.7}
	/>
</T.Mesh>
