<script lang="ts">
	// Remonte la direction de vue de la caméra active (navigateur 3D de l'aperçu,
	// phase 38) — même calcul que le Viewport de préparation.
	import { useTask, useThrelte } from '@threlte/core';
	import { Vector3 } from 'three';

	interface Props {
		oncameradir: (forward: [number, number, number]) => void;
	}

	let { oncameradir }: Props = $props();
	const { camera } = useThrelte();
	const forward = new Vector3();
	let last = '';
	useTask(
		() => {
			camera.current.getWorldDirection(forward);
			const key = forward
				.toArray()
				.map((n) => n.toFixed(4))
				.join(',');
			if (key === last) return;
			last = key;
			oncameradir([forward.x, forward.y, forward.z]);
		},
		{ autoInvalidate: false }
	);
</script>
