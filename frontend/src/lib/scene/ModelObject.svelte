<script lang="ts">
	// Un objet 3D de la scène (T050) : construit une BufferGeometry depuis le
	// maillage décodé, met en surbrillance la sélection, et remonte les clics
	// (raycast Three.js) au parent.
	import { T } from '@threlte/core';
	import type { IntersectionEvent } from '@threlte/extras';
	import * as THREE from 'three';
	import type { SceneMesh } from './mesh';
	import { paintFaceColorData } from './gizmos/painting/colors';
	import type { PaintDocument } from './gizmos/painting/painting';

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
		/** Peinture au triangle active (un canal est sélectionné, T056). */
		paintActive?: boolean;
		/** Document de peinture de l'objet : colore les facettes peintes (T145). */
		painting?: PaintDocument;
		/** Remonte un coup de pinceau : triangle touché + point local (mm). */
		onpaint?: (faceIndex: number, local: [number, number, number]) => void;
	}

	let {
		mesh,
		position = [0, 0, 0],
		rotation = [0, 0, 0],
		scale = [1, 1, 1],
		selected = false,
		onpick,
		onref,
		paintActive = false,
		painting,
		onpaint
	}: Props = $props();

	// Données de couleur au triangle (non indexées) si l'objet porte de la
	// peinture ; `null` sinon → on garde la géométrie indexée normale.
	const painted = $derived(painting ? paintFaceColorData(mesh, painting) : null);

	// Coup de pinceau : le raycast Threlte donne le triangle (`faceIndex`) et le
	// point d'impact (monde) ; on repasse en repère local (celui de `mesh`) pour
	// le pinceau à rayon. Le glisser peint tant que le pointeur est enfoncé.
	let brushing = $state(false);
	function emitPaint(event: IntersectionEvent<PointerEvent>) {
		if (event.faceIndex == null || !ref) return;
		const local = ref.worldToLocal(event.point.clone());
		onpaint?.(event.faceIndex, [local.x, local.y, local.z]);
	}
	function onpointerdown(event: IntersectionEvent<PointerEvent>) {
		if (!paintActive) return;
		event.stopPropagation();
		brushing = true;
		emitPaint(event);
	}
	function onpointermove(event: IntersectionEvent<PointerEvent>) {
		if (!paintActive || !brushing) return;
		event.stopPropagation();
		emitPaint(event);
	}
	// Relâchement n'importe où (même hors du maillage) : fin du glisser.
	$effect(() => {
		const stop = () => (brushing = false);
		window.addEventListener('pointerup', stop);
		return () => window.removeEventListener('pointerup', stop);
	});

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
		if (painted) {
			// Géométrie développée (une facette = 3 sommets propres) portant la
			// couleur de peinture par sommet. L'ordre des facettes est préservé,
			// donc `faceIndex` reste l'index de triangle attendu par le pinceau.
			g.setAttribute('position', new THREE.BufferAttribute(painted.positions, 3));
			g.setAttribute('color', new THREE.BufferAttribute(painted.colors, 3));
			g.computeVertexNormals();
			return g;
		}
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

<T.Mesh
	bind:ref
	{geometry}
	{position}
	rotation={radRotation}
	{scale}
	castShadow
	{onclick}
	{onpointerdown}
	{onpointermove}
>
	<T.MeshStandardMaterial
		color={painted ? '#ffffff' : selected ? '#f59e0b' : '#9aa7b4'}
		vertexColors={!!painted}
		metalness={0.1}
		roughness={0.7}
	/>
</T.Mesh>
