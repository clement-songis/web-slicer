<script lang="ts">
	// Racine de la scène 3D de préparation (T050) : un Canvas Threlte qui héberge
	// le viewport. Conforme à references/orca-prepare.png (plateau + grille +
	// caméra trois-quarts).
	import { Canvas } from '@threlte/core';
	import type { BedShape } from './bed';
	import type { MeshPick, SceneObject } from './mesh';
	import type { GizmoMode } from './gizmos/types';
	import type { NamedView, Projection } from './camera';
	import type { Transform } from './transform';
	import type { SceneVolume } from './volumes';
	import Viewport from './Viewport.svelte';
	import { overhangIndices } from './overhang';
	import {
		pointInBox,
		rectFromDrag,
		rectHits,
		selectByRect,
		type ScreenBox,
		type ScreenRect,
		type SectionPlane
	} from './screen-select';

	interface Props {
		bed: BedShape;
		objects?: SceneObject[];
		selection?: Set<string>;
		/** Mode du gizmo de transformation, ou null pour le masquer (T103). */
		gizmoMode?: GizmoMode | null;
		/** Vue caméra nommée du menu Vue (T109). */
		view?: NamedView;
		/** Zoom I/O : 1 = cadrage nominal. */
		zoom?: number;
		/** Boîte à cadrer (sélection), transmise au Viewport. */
		focus?: { min: [number, number, number]; max: [number, number, number] } | null;
		/** Affichage de la grille du plateau (menu Vue → Show Gridlines). */
		showGrid?: boolean;
		/** Projection caméra (menu Vue → Use Perspective / Orthogonal View). */
		projection?: Projection;
		overhangNormalZ?: number | null;
		labels?: Record<string, string> | null;
		/** Rang d'impression par objet (objet par objet), null = masqué. */
		printOrder?: Record<string, number> | null;
		/** Volume sélectionné de l'objet sélectionné (mis en évidence). */
		selectedVolumeId?: string | null;
		/** Clic sur un volume dans la vue 3D (objet, volume). */
		onpickvolume?: (objectId: string, volumeId: string) => void;
		onmeshpick?: (objectId: string, pick: MeshPick) => void;
		/** Peinture glissée (appui/déplacements/relâchement via `onmeshpick`). */
		paintstroke?: boolean;
		/** Caméra figée pendant un trait de peinture (pas de rotation au glisser). */
		suspendcontrols?: boolean;
		/** Outil Flatten : faces candidates cliquables. */
		flattenMode?: boolean;
		onflatten?: (objectId: string, rotation: [number, number, number]) => void;
		/** Remonte la transformation de l'objet manipulé (T103). */
		ontransform?: (id: string, transform: Transform) => void;
		/** Volume modifié par le gizmo (repère de l'objet). */
		onvolumechange?: (objectId: string, volume: SceneVolume) => void;
		/** Début / fin d'un geste de gizmo (une étape d'historique par geste). */
		ondragchange?: (dragging: boolean) => void;
		/** Plan de coupe de peinture (Alt+molette) : ratio 0..1 et hauteur (mm). */
		sectionRatio?: number;
		clipPlane?: SectionPlane | null;
		oncameradir?: (forward: [number, number, number]) => void;
		onpickobject?: (id: string) => void;
	}

	let {
		bed,
		objects = [],
		selection = $bindable(new Set<string>()),
		gizmoMode = null,
		view = 'default',
		zoom = 1,
		focus = null,
		showGrid = true,
		projection = 'perspective',
		overhangNormalZ = null,
		labels = null,
		printOrder = null,
		selectedVolumeId = null,
		onpickvolume,
		onmeshpick,
		paintstroke = false,
		suspendcontrols = false,
		flattenMode = false,
		onflatten,
		ontransform,
		onvolumechange,
		ondragchange,
		sectionRatio = 0,
		clipPlane = null,
		oncameradir,
		onpickobject
	}: Props = $props();

	// Emprises écran projetées (objets, et pièces « objet/volume »).
	let screenBoxes = $state<Record<string, ScreenBox>>({});
	const objectBoxes = $derived(
		Object.fromEntries(Object.entries(screenBoxes).filter(([id]) => !id.includes('/')))
	);
	const anchors = $derived(
		Object.entries(screenBoxes)
			.map(([id, b]) => `${id}:${Math.round((b.x0 + b.x1) / 2)},${Math.round((b.y0 + b.y1) / 2)}`)
			.join(';')
	);

	// Maj+glisser dans le vide : rectangle de sélection (GLSelectionRectangle
	// d'Orca). Les événements sont interceptés en capture pour que ni l'orbite
	// (Maj = panoramique) ni le picking ne réagissent pendant le tracé.
	let container: HTMLDivElement;
	let dragStart: [number, number] | null = null;
	let dragRect = $state<ScreenRect | null>(null);
	function local(e: PointerEvent): [number, number] {
		const r = container.getBoundingClientRect();
		return [e.clientX - r.left, e.clientY - r.top];
	}
	// Sur un objet (ou le gizmo de l'objet sélectionné), Maj garde son rôle :
	// sélection additive au clic, pas de 1 mm pendant le glisser du gizmo.
	function overObject(p: [number, number]): boolean {
		for (const [id, box] of Object.entries(objectBoxes)) {
			const margin = gizmoMode && selection.size === 1 && selection.has(id) ? 120 : 8;
			if (pointInBox(p, box, margin)) return true;
		}
		return false;
	}
	function onRectDown(e: PointerEvent) {
		if (!e.shiftKey || e.button !== 0) return;
		const p = local(e);
		if (overObject(p)) return;
		e.stopPropagation();
		e.preventDefault();
		dragStart = p;
		dragRect = rectFromDrag(p, p);
		container.setPointerCapture?.(e.pointerId);
	}
	function onRectMove(e: PointerEvent) {
		if (!dragStart) return;
		e.stopPropagation();
		dragRect = rectFromDrag(dragStart, local(e));
	}
	function onRectUp(e: PointerEvent) {
		if (!dragStart) return;
		e.stopPropagation();
		const rect = rectFromDrag(dragStart, local(e));
		dragStart = null;
		dragRect = null;
		container.releasePointerCapture?.(e.pointerId);
		selection = selectByRect(selection, rectHits(rect, objectBoxes));
	}

	// Gizmo attaché au volume sélectionné : mode et ancre écran (hooks de test).
	let volumeGizmo = $state<{ mode: GizmoMode; anchor: [number, number] | null } | null>(null);
	let objectAnchor = $state<[number, number] | null>(null);
	let flattenAnchors = $state<[number, number][] | null>(null);

	// Nombre de facettes en surplomb (attribut de test, même calcul que le rendu).
	const overhangFaces = $derived(
		overhangNormalZ === null
			? 0
			: objects.reduce(
					(n, o) =>
						n +
						overhangIndices(o.mesh, overhangNormalZ, o.rotation, o.scale, o.position).length / 3,
					0
				)
	);
</script>

<div
	class="relative h-full w-full"
	bind:this={container}
	onpointerdowncapture={onRectDown}
	onpointermovecapture={onRectMove}
	onpointerupcapture={onRectUp}
	data-object-anchors={anchors}
	data-section-ratio={sectionRatio.toFixed(2)}
	data-section-normal={clipPlane?.normal.map((n) => n.toFixed(3)).join(',')}
	data-section-offset={clipPlane?.offset.toFixed(2)}
	data-testid="scene-canvas"
	data-projection={projection}
	data-camera-view={view}
	data-camera-zoom={zoom.toFixed(3)}
	data-overhang-faces={overhangNormalZ === null ? undefined : overhangFaces}
	data-labels={labels ? Object.keys(labels).length : undefined}
	data-print-order={printOrder ? Object.keys(printOrder).length : undefined}
	data-selected-volume={selectedVolumeId ?? undefined}
	data-flatten-planes={flattenAnchors ? flattenAnchors.length : undefined}
	data-flatten-anchors={flattenAnchors
		? flattenAnchors.map((a) => `${Math.round(a[0])},${Math.round(a[1])}`).join(';')
		: undefined}
	data-object-anchor={objectAnchor
		? `${Math.round(objectAnchor[0])},${Math.round(objectAnchor[1])}`
		: undefined}
	data-object-positions={objects
		.map((o) => `${o.id}:${(o.position ?? [0, 0, 0]).map((n) => n.toFixed(3)).join(',')}`)
		.join(';')}
	data-volume-gizmo={volumeGizmo?.mode}
	data-volume-gizmo-anchor={volumeGizmo?.anchor
		? `${Math.round(volumeGizmo.anchor[0])},${Math.round(volumeGizmo.anchor[1])}`
		: undefined}
>
	<Canvas>
		<Viewport
			{bed}
			{objects}
			bind:selection
			{gizmoMode}
			{view}
			{showGrid}
			{zoom}
			{focus}
			{projection}
			{overhangNormalZ}
			{labels}
			{printOrder}
			{selectedVolumeId}
			{onpickvolume}
			{onmeshpick}
			{paintstroke}
			{suspendcontrols}
			{ontransform}
			{onvolumechange}
			{ondragchange}
			onvolumegizmo={(g) => (volumeGizmo = g)}
			onobjectanchor={(anchor) => (objectAnchor = anchor)}
			{flattenMode}
			{onflatten}
			onflattenanchors={(anchors) => (flattenAnchors = anchors)}
			onscreenboxes={(boxes) => (screenBoxes = boxes)}
			{clipPlane}
			{oncameradir}
			{onpickobject}
		/>
	</Canvas>
	{#if dragRect}
		<div
			class="pointer-events-none absolute border border-dashed border-accent bg-accent/10"
			data-testid="selection-rectangle"
			style="left: {dragRect.x0}px; top: {dragRect.y0}px; width: {dragRect.x1 -
				dragRect.x0}px; height: {dragRect.y1 - dragRect.y0}px"
		></div>
	{/if}
</div>
