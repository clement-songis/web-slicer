<script lang="ts">
	// Gizmo de transformation interactif (T052) : enveloppe TransformControls de
	// Three.js sur l'objet sélectionné. Émet la transformation courante à chaque
	// modification (`onchange`) et un `oncommit` à la fin d'un glisser (point
	// d'entrée d'une étape d'historique undo/redo).
	import { TransformControls } from '@threlte/extras';
	import type { Object3D } from 'three';
	import { TransformControls as ThreeTransformControls } from 'three/examples/jsm/controls/TransformControls.js';
	import type { GizmoMode } from './types';
	import type { Transform } from '../transform';
	import { MOVE_SNAP_MM, snapPosition } from '../screen-select';

	interface Props {
		object: Object3D;
		mode?: GizmoMode;
		onchange?: (t: Transform) => void;
		oncommit?: () => void;
		/** Début d'un glisser (capture de l'état initial du geste). */
		ondragstart?: () => void;
	}

	let { object, mode = 'translate', onchange, oncommit, ondragstart }: Props = $props();

	let controls = $state<ThreeTransformControls | undefined>();
	let shiftSnap = $state(false);
	const snap = (event: KeyboardEvent, value: boolean) => {
		if (event.key === 'Shift') shiftSnap = value;
	};

	function readTransform(o: Object3D): Transform {
		const deg = 180 / Math.PI;
		return {
			position: [o.position.x, o.position.y, o.position.z],
			rotation: [o.rotation.x * deg, o.rotation.y * deg, o.rotation.z * deg],
			scale: [o.scale.x, o.scale.y, o.scale.z]
		};
	}

	// Abonne les événements du contrôleur : suivi continu + validation en fin de glisser.
	$effect(() => {
		const c = controls;
		if (!c) return;
		// Maj : pas de 1 mm sur la position émise (et persistée), comme Orca.
		const onObjectChange = () => {
			const t = readTransform(object);
			onchange?.(
				shiftSnap && mode === 'translate' ? { ...t, position: snapPosition(t.position) } : t
			);
		};
		const onDragging = (e: { value: unknown }) => {
			if (e.value) ondragstart?.();
			else oncommit?.();
		};
		c.addEventListener('objectChange', onObjectChange);
		c.addEventListener('dragging-changed', onDragging);
		return () => {
			c.removeEventListener('objectChange', onObjectChange);
			c.removeEventListener('dragging-changed', onDragging);
		};
	});
</script>

<svelte:window onkeydown={(event) => snap(event, true)} onkeyup={(event) => snap(event, false)} />

<TransformControls
	{object}
	{mode}
	bind:controls
	translationSnap={shiftSnap ? MOVE_SNAP_MM : undefined}
/>
