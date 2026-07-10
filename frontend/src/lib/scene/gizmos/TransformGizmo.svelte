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

	interface Props {
		object: Object3D;
		mode?: GizmoMode;
		onchange?: (t: Transform) => void;
		oncommit?: () => void;
	}

	let { object, mode = 'translate', onchange, oncommit }: Props = $props();

	let controls = $state<ThreeTransformControls | undefined>();

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
		const onObjectChange = () => onchange?.(readTransform(object));
		const onDragging = (e: { value: unknown }) => {
			if (!e.value) oncommit?.();
		};
		c.addEventListener('objectChange', onObjectChange);
		c.addEventListener('dragging-changed', onDragging);
		return () => {
			c.removeEventListener('objectChange', onObjectChange);
			c.removeEventListener('dragging-changed', onDragging);
		};
	});
</script>

<TransformControls {object} {mode} bind:controls />
