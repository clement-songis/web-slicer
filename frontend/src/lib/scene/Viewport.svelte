<script lang="ts">
	// Contenu de la scène (T050) : caméra orbit cadrée sur le plateau, éclairage,
	// plateau et objets, gestion de la sélection par raycast. Monte le gizmo de
	// transformation (T103) sur l'objet unique sélectionné selon le mode actif.
	import { T } from '@threlte/core';
	import { OrbitControls, interactivity } from '@threlte/extras';
	import type { Object3D } from 'three';
	import Bed from './Bed.svelte';
	import type { BedShape } from './bed';
	import { viewPose, viewUp, type NamedView } from './camera';
	import type { SceneObject } from './mesh';
	import ModelObject from './ModelObject.svelte';
	import TransformGizmo from './gizmos/TransformGizmo.svelte';
	import type { GizmoMode } from './gizmos/types';
	import type { Transform } from './transform';
	import { applyPick } from './selection';

	interface Props {
		bed: BedShape;
		objects?: SceneObject[];
		selection?: Set<string>;
		/** Mode du gizmo de transformation, ou null pour le masquer (T103). */
		gizmoMode?: GizmoMode | null;
		/** Vue caméra nommée du menu Vue (T109). */
		view?: NamedView;
		/** Affichage de la grille du plateau (menu Vue → Show Gridlines). */
		showGrid?: boolean;
		/** Remonte la transformation de l'objet manipulé (T103). */
		ontransform?: (id: string, transform: Transform) => void;
	}

	let {
		bed,
		objects = [],
		selection = $bindable(new Set<string>()),
		gizmoMode = null,
		view = 'default',
		showGrid = true,
		ontransform
	}: Props = $props();

	// Active le raycasting Threlte (événements de pointeur sur les meshes).
	interactivity();

	const pose = $derived(viewPose(bed, view));
	const up = $derived(viewUp(view));

	function pick(id: string | null, additive: boolean) {
		selection = applyPick(selection, id, additive);
	}

	// Références Three.js des objets rendus (pour attacher le gizmo).
	let refs = $state<Record<string, Object3D>>({});
	function setRef(id: string, object: Object3D | null) {
		if (object) refs[id] = object;
		else delete refs[id];
	}

	// Objet cible du gizmo : l'unique objet sélectionné (sinon aucun).
	const targetId = $derived(selection.size === 1 ? [...selection][0] : null);
	const target = $derived(targetId ? (refs[targetId] ?? null) : null);

	// Désactive l'orbite pendant un glisser de gizmo (évite le conflit de contrôles).
	let dragging = $state(false);
</script>

<T.PerspectiveCamera makeDefault position={pose.position} {up} fov={50} near={1} far={20000}>
	<OrbitControls enableDamping enabled={!dragging} target={pose.target} />
</T.PerspectiveCamera>

<T.AmbientLight intensity={0.6} />
<T.DirectionalLight
	position={[bed.center.x + 200, bed.center.y - 200, 400]}
	intensity={1.2}
	castShadow
/>

<Bed {bed} {showGrid} />

{#each objects as obj (obj.id)}
	<ModelObject
		mesh={obj.mesh}
		position={obj.position}
		rotation={obj.rotation}
		scale={obj.scale}
		selected={selection.has(obj.id)}
		onpick={(additive) => pick(obj.id, additive)}
		onref={(o) => setRef(obj.id, o)}
	/>
{/each}

{#if target && gizmoMode && targetId}
	<TransformGizmo
		object={target}
		mode={gizmoMode}
		onchange={(transform) => {
			dragging = true;
			ontransform?.(targetId, transform);
		}}
		oncommit={() => (dragging = false)}
	/>
{/if}
