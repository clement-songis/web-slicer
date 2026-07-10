<script lang="ts">
	// Contenu de la scène (T050) : caméra orbit cadrée sur le plateau, éclairage,
	// plateau et objets, gestion de la sélection par raycast.
	import { T } from '@threlte/core';
	import { OrbitControls, interactivity } from '@threlte/extras';
	import Bed from './Bed.svelte';
	import type { BedShape } from './bed';
	import { frameBed } from './camera';
	import type { SceneObject } from './mesh';
	import ModelObject from './ModelObject.svelte';
	import { applyPick } from './selection';

	interface Props {
		bed: BedShape;
		objects?: SceneObject[];
		selection?: Set<string>;
	}

	let { bed, objects = [], selection = $bindable(new Set<string>()) }: Props = $props();

	// Active le raycasting Threlte (événements de pointeur sur les meshes).
	interactivity();

	const pose = $derived(frameBed(bed));

	function pick(id: string | null, additive: boolean) {
		selection = applyPick(selection, id, additive);
	}
</script>

<T.PerspectiveCamera
	makeDefault
	position={pose.position}
	up={[0, 0, 1]}
	fov={50}
	near={1}
	far={20000}
>
	<OrbitControls enableDamping target={pose.target} />
</T.PerspectiveCamera>

<T.AmbientLight intensity={0.6} />
<T.DirectionalLight
	position={[bed.center.x + 200, bed.center.y - 200, 400]}
	intensity={1.2}
	castShadow
/>

<Bed {bed} />

{#each objects as obj (obj.id)}
	<ModelObject
		mesh={obj.mesh}
		position={obj.position}
		selected={selection.has(obj.id)}
		onpick={(additive) => pick(obj.id, additive)}
	/>
{/each}
