<script lang="ts">
	// Lumière de la vue réaliste (Orca : lumière fixe dans le repère caméra,
	// direction œil (-0,4575, 0,4575, 0,7625) ; ombres projetées sur le plateau,
	// GLCanvas3D « basic plate shadows »). Suit la caméra à chaque image ; le
	// cadre d'ombre couvre le plateau.
	import { T, useTask, useThrelte } from '@threlte/core';
	import * as THREE from 'three';
	import type { BedShape } from './bed';
	import { worldLightDirection } from './realistic';

	interface Props {
		bed: BedShape;
		/** Ombres portées sur le plateau. */
		shadows?: boolean;
	}

	let { bed, shadows = false }: Props = $props();

	const { camera, invalidate } = useThrelte();
	let light = $state<THREE.DirectionalLight>();
	const target = new THREE.Object3D();
	const span = $derived(Math.max(bed.width, bed.depth) * 0.9 + 50);

	$effect(() => {
		const l = light;
		if (!l) return;
		l.target = target;
		const cam = l.shadow.camera;
		cam.left = -span;
		cam.right = span;
		cam.top = span;
		cam.bottom = -span;
		cam.near = 1;
		cam.far = span * 6;
		cam.updateProjectionMatrix();
		l.shadow.mapSize.set(2048, 2048);
		// Biais d'Orca (0,01) ramené à l'échelle de la carte d'ombre.
		l.shadow.bias = -0.0005;
		l.shadow.normalBias = 0.02;
		invalidate();
	});

	// Sans rendu forcé : la lumière suit la caméra dans l'image que son
	// mouvement provoque (vue au repos = aucune image, cf. compteur d'images/s).
	useTask(
		() => {
			const l = light;
			const cam = camera.current;
			if (!l || !cam) return;
			const dir = worldLightDirection(cam.quaternion);
			target.position.set(bed.center.x, bed.center.y, 0);
			target.updateMatrixWorld();
			l.position.copy(target.position).addScaledVector(dir, span * 3);
		},
		{ autoInvalidate: false }
	);
</script>

<T.DirectionalLight bind:ref={light} intensity={1.1} castShadow={shadows} />
