<script lang="ts">
	// Contrôles caméra selon les préférences (Orca « Camera ») : orbite contrainte
	// (OrbitControls, Z vertical) ou caméra libre (TrackballControls), boutons de
	// souris, zoom vers le pointeur, molette inversée, vitesse d'orbite, style
	// pavé tactile (Alt/Maj + déplacement). À placer comme enfant de la caméra.
	import { useThrelte } from '@threlte/core';
	import { OrbitControls, TrackballControls } from '@threlte/extras';
	import type { OrbitControls as ThreeOrbit } from 'three/examples/jsm/controls/OrbitControls.js';
	import { appPreferences } from '#lib/settings/preferences';
	import { orbitConfig, touchpadGesture, trackballConfig } from './camera-controls';

	interface Props {
		target: [number, number, number];
		enabled?: boolean;
		/** Haut de la caméra rétabli au retour en orbite (la boule le fait tourner). */
		up?: [number, number, number];
	}

	let { target, enabled = true, up = [0, 0, 1] }: Props = $props();

	const { dom, camera, invalidate } = useThrelte();
	const cam = $derived($appPreferences.camera);
	const orbit = $derived(orbitConfig(cam));
	const trackball = $derived(trackballConfig(cam));
	let orbitRef = $state<ThreeOrbit | undefined>();

	// Retour en orbite après la caméra libre : Z redevient vertical.
	$effect(() => {
		if (cam.freeCamera) return;
		camera.current.up.set(...up);
		orbitRef?.update();
		invalidate();
	});

	// Pavé tactile (GLCanvas3D::is_camera_rotate/pan, style 1).
	$effect(() => {
		if (!orbit.touchpad || cam.freeCamera) return;
		let last: [number, number] | null = null;
		const onMove = (e: PointerEvent) => {
			const gesture = touchpadGesture(e, enabled);
			const prev = last;
			last = gesture ? [e.clientX, e.clientY] : null;
			if (!gesture || !prev || !orbitRef) return;
			const dx = e.clientX - prev[0];
			const dy = e.clientY - prev[1];
			if (gesture === 'rotate') {
				const h = dom.clientHeight || 1;
				orbitRef.rotateLeft((2 * Math.PI * dx * orbit.rotateSpeed) / h);
				orbitRef.rotateUp((2 * Math.PI * dy * orbit.rotateSpeed) / h);
			} else orbitRef.pan(dx, dy);
			invalidate();
		};
		dom.addEventListener('pointermove', onMove);
		return () => dom.removeEventListener('pointermove', onMove);
	});
</script>

{#if cam.freeCamera}
	<TrackballControls
		{enabled}
		{target}
		mouseButtons={trackball.mouseButtons}
		rotateSpeed={trackball.rotateSpeed}
		zoomSpeed={trackball.zoomSpeed}
		panSpeed={trackball.panSpeed}
	/>
{:else}
	<OrbitControls
		bind:ref={orbitRef}
		enableDamping
		{enabled}
		{target}
		mouseButtons={orbit.mouseButtons}
		zoomToCursor={orbit.zoomToCursor}
		zoomSpeed={orbit.zoomSpeed}
		rotateSpeed={orbit.rotateSpeed}
	/>
{/if}
