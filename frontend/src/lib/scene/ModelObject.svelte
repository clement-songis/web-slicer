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
		/** Rotation d'Euler en degrés (T103). */
		rotation?: [number, number, number];
		/** Échelle par axe (T103). */
		scale?: [number, number, number];
		selected?: boolean;
		onpick?: (additive: boolean) => void;
		/** Remonte l'objet Three.js créé (pour y attacher le gizmo, T103). */
		onref?: (object: THREE.Object3D | null) => void;
	}

	let {
		mesh,
		position = [0, 0, 0],
		rotation = [0, 0, 0],
		scale = [1, 1, 1],
		selected = false,
		onpick,
		onref
	}: Props = $props();

	const DEG = Math.PI / 180;
	const radRotation = $derived<[number, number, number]>([
		rotation[0] * DEG,
		rotation[1] * DEG,
		rotation[2] * DEG
	]);

	// Référence à l'objet Three.js, remontée au parent (Viewport) pour le gizmo.
	let ref = $state<THREE.Mesh>();
	$effect(() => {
		onref?.(ref ?? null);
		return () => onref?.(null);
	});

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

<T.Mesh bind:ref {geometry} {position} rotation={radRotation} {scale} castShadow {onclick}>
	<T.MeshStandardMaterial
		color={selected ? '#f59e0b' : '#9aa7b4'}
		metalness={0.1}
		roughness={0.7}
	/>
</T.Mesh>
