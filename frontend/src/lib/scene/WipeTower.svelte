<script lang="ts">
	// Tour d'essuyage estimée sur le plateau (Préparer), comme le volume
	// « wipe tower » d'Orca (GLVolumeCollection::load_wipe_tower_preview) :
	// pavé semi-transparent + bordure au sol, déplaçable par glisser (bornée au
	// plateau). La position (coin min) remonte au parent à la fin du geste.
	import { T, useTask, useThrelte } from '@threlte/core';
	import { HTML, type IntersectionEvent } from '@threlte/extras';
	import * as THREE from 'three';
	import { sceneColors } from './scene-colors';
	import { clampTowerPosition } from './wipe-tower';

	interface Props {
		/** Coin min (repère plateau, mm). */
		x: number;
		y: number;
		size: [number, number, number];
		brim: number;
		/** wipe_tower_rotation_angle (degrés, autour du centre). */
		rotation?: number;
		bed: { width: number; depth: number; origin?: { x: number; y: number } };
		color?: string;
		onmove?: (x: number, y: number) => void;
		ondragchange?: (dragging: boolean) => void;
	}

	let { x, y, size, brim, rotation = 0, bed, color, onmove, ondragchange }: Props = $props();

	const { camera, renderer, invalidate } = useThrelte();
	// Position pendant le geste (sinon celle des props).
	let drag = $state<{ x: number; y: number } | null>(null);
	let grab: [number, number] = [0, 0];
	let hover = $state(false);
	const px = $derived(drag?.x ?? x);
	const py = $derived(drag?.y ?? y);
	const height = $derived(Math.max(size[2], 0.4));

	// Point écran (clientX,clientY) du sommet de la tour, mis à jour à chaque rendu.
	let grabPoint = $state('');
	const top = new THREE.Vector3();
	useTask(
		() => {
			top.set(px + size[0] / 2, py + size[1] / 2, height);
			top.project(camera.current);
			const rect = renderer.domElement.getBoundingClientRect();
			const next = `${Math.round(rect.left + ((top.x + 1) / 2) * rect.width)},${Math.round(rect.top + ((1 - top.y) / 2) * rect.height)}`;
			if (next !== grabPoint) grabPoint = next;
		},
		{ autoInvalidate: false }
	);

	const plane = new THREE.Plane(new THREE.Vector3(0, 0, 1), 0);
	const ray = new THREE.Raycaster();
	const ndc = new THREE.Vector2();
	const hit = new THREE.Vector3();
	function groundPoint(e: PointerEvent): [number, number] | null {
		const rect = renderer.domElement.getBoundingClientRect();
		ndc.set(
			((e.clientX - rect.left) / rect.width) * 2 - 1,
			-((e.clientY - rect.top) / rect.height) * 2 + 1
		);
		ray.setFromCamera(ndc, camera.current);
		return ray.ray.intersectPlane(plane, hit) ? [hit.x, hit.y] : null;
	}

	function onpointerdown(event: IntersectionEvent<PointerEvent>) {
		if (event.nativeEvent.button !== 0 || !onmove) return;
		event.stopPropagation();
		const p = groundPoint(event.nativeEvent);
		if (!p) return;
		grab = [p[0] - x, p[1] - y];
		drag = { x, y };
		ondragchange?.(true);
		window.addEventListener('pointermove', onmovewindow);
		window.addEventListener('pointerup', onup, { once: true });
	}
	function onmovewindow(e: PointerEvent) {
		const p = groundPoint(e);
		if (!p || !drag) return;
		const [cx, cy] = clampTowerPosition(p[0] - grab[0], p[1] - grab[1], size, bed, brim);
		drag = { x: cx, y: cy };
		invalidate();
	}
	function onup() {
		window.removeEventListener('pointermove', onmovewindow);
		const end = drag;
		drag = null;
		ondragchange?.(false);
		if (end && (Math.abs(end.x - x) > 1e-3 || Math.abs(end.y - y) > 1e-3))
			onmove?.(Math.round(end.x * 1000) / 1000, Math.round(end.y * 1000) / 1000);
	}
</script>

<T.Group
	position={[px + size[0] / 2, py + size[1] / 2, 0]}
	rotation={[0, 0, (rotation * Math.PI) / 180]}
	userData={{ wipeTower: true }}
>
	<!-- Bordure au sol. -->
	{#if brim > 0}
		<T.Mesh position={[0, 0, 0.05]} renderOrder={1}>
			<T.PlaneGeometry args={[size[0] + 2 * brim, size[1] + 2 * brim]} />
			<T.MeshBasicMaterial
				color={color ?? $sceneColors.volumeModifier}
				transparent
				opacity={0.18}
				depthWrite={false}
			/>
		</T.Mesh>
	{/if}
	<T.Mesh
		position={[0, 0, height / 2]}
		renderOrder={2}
		{onpointerdown}
		onpointerenter={() => (hover = true)}
		onpointerleave={() => (hover = false)}
	>
		<T.BoxGeometry args={[size[0], size[1], height]} />
		<!-- Repère écran (centre du sommet) : étiquette au survol / pendant le
		     glisser, et ancre stable pour les tests (pointer-events: none). -->
		<HTML position={[0, 0, height / 2 + 14]} center pointerEvents="none">
			<!-- Étiquette posée AU-DESSUS de la tour (le calque HTML intercepterait
			     sinon le pointeur) ; data-grab = point écran du sommet de la tour,
			     cible stable pour glisser au clavier/tests. -->
			<span
				class="ws-tooltip whitespace-nowrap transition-opacity {drag || hover
					? 'opacity-100'
					: 'opacity-0'}"
				data-testid="wipe-tower-handle"
				data-grab={grabPoint}
				data-dragging={drag ? 'true' : undefined}
				>Tour d'essuyage{drag ? ` · X ${px.toFixed(0)} Y ${py.toFixed(0)}` : ''}</span
			>
		</HTML>
		<T.MeshStandardMaterial
			color={color ?? $sceneColors.volumeModifier}
			transparent
			opacity={drag ? 0.85 : hover ? 0.75 : 0.6}
			emissive={drag || hover ? $sceneColors.emissiveActive : $sceneColors.emissiveNone}
			emissiveIntensity={drag ? 0.5 : hover ? 0.25 : 0}
			roughness={0.8}
		/>
	</T.Mesh>
</T.Group>
