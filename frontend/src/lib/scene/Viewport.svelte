<script lang="ts">
	// Contenu de la scène (T050) : caméra orbit cadrée sur le plateau, éclairage,
	// plateau et objets, gestion de la sélection par raycast. Monte le gizmo de
	// transformation (T103) sur l'objet unique sélectionné selon le mode actif.
	import { T, useTask, useThrelte } from '@threlte/core';
	import { HTML, interactivity } from '@threlte/extras';
	import CameraControls from './CameraControls.svelte';
	import { Box3, Object3D, Vector3 } from 'three';
	import type { ScreenBox, SectionPlane } from './screen-select';
	import Bed from './Bed.svelte';
	import type { BedShape } from './bed';
	import {
		frameBox,
		orthoZoom,
		viewPose,
		viewUp,
		zoomPose,
		type NamedView,
		type Projection
	} from './camera';
	import type { MeshPick, SceneObject } from './mesh';
	import ModelObject from './ModelObject.svelte';
	import TransformGizmo from './gizmos/TransformGizmo.svelte';
	import type { GizmoMode } from './gizmos/types';
	import type { Transform } from './transform';
	import { applyPick } from './selection';
	import { worldSize } from './transform-ops';
	import WipeTower from './WipeTower.svelte';
	import { volumeFromWorld, volumeWorldTransform } from './volume-frame';
	import { BufferAttribute, BufferGeometry, DoubleSide } from 'three';
	import { flattenPlanes, flattenRotation, type FlattenPlane } from './gizmos/flatten';
	import type { SceneVolume } from './volumes';
	import type { PaintOverlay } from './gizmos/painting/paint-overlay';
	import { sceneColors } from './scene-colors';
	import { guideRotation, markerWorldRadius, MARKER_PX, type SceneGuides } from './guides';

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
		/** Boîte à cadrer (sélection) ; null → vue nommée du plateau. */
		focus?: { min: [number, number, number]; max: [number, number, number] } | null;
		/** Affichage de la grille du plateau (menu Vue → Show Gridlines). */
		showGrid?: boolean;
		/** Projection caméra : perspective (défaut) ou orthogonale. */
		projection?: Projection;
		/** Seuil de surplomb (menu Vue › Show Overhang), null = masqué. */
		overhangNormalZ?: number | null;
		/** Étiquettes par objet (menu Vue › Show Labels), null = masquées. */
		labels?: Record<string, string> | null;
		/** Rang d'impression par objet (Orca, objet par objet : « Sequence #N »
		 *  sous le nom de l'étiquette), null = masqué. */
		printOrder?: Record<string, number> | null;
		/** Multi-filament : couleur par extrudeur et extrudeur effectif par objet. */
		extruderColors?: readonly string[] | null;
		objectExtruders?: Record<string, number> | null;
		/** Peinture visible par objet (supports/couture/fuzzy/MMU) ; absent = rien. */
		paintOverlays?: Record<string, PaintOverlay | null> | null;
		/** Tour d'essuyage estimée (coin min, taille, bordure) ; null = masquée. */
		wipeTower?: {
			x: number;
			y: number;
			size: [number, number, number];
			brim: number;
			rotation: number;
			color?: string;
		} | null;
		onmovewipetower?: (x: number, y: number) => void;
		/** Volume sélectionné de l'objet sélectionné (mis en évidence). */
		selectedVolumeId?: string | null;
		/** Clic sur un volume dans la vue 3D (objet, volume). */
		onpickvolume?: (objectId: string, volumeId: string) => void;
		/** Picking d'une facette pour les outils non destructifs. */
		onmeshpick?: (objectId: string, pick: MeshPick) => void;
		/** Peinture glissée (appui/déplacements/relâchement via `onmeshpick`). */
		paintstroke?: boolean;
		/** Caméra figée pendant un trait de peinture. */
		suspendcontrols?: boolean;
		/** Centre projeté de l'objet sélectionné (hook d'interaction e2e). */
		onobjectanchor?: (anchor: [number, number] | null) => void;
		/** Outil Flatten actif : faces candidates de l'objet sélectionné (Orca GLGizmoFlatten). */
		flattenMode?: boolean;
		/** Clic sur une face candidate : nouvelle rotation (Euler XYZ, degrés). */
		onflatten?: (objectId: string, rotation: [number, number, number]) => void;
		/** Centres projetés des faces candidates (px CSS du canvas), hook de test. */
		onflattenanchors?: (anchors: [number, number][] | null) => void;
		/** Remonte la transformation de l'objet manipulé (T103). */
		ontransform?: (id: string, transform: Transform) => void;
		/** Volume modifié par le gizmo, exprimé dans le repère de l'objet. */
		onvolumechange?: (objectId: string, volume: SceneVolume) => void;
		/** Début / fin d'un geste de gizmo (une étape d'historique par geste). */
		ondragchange?: (dragging: boolean) => void;
		/** Gizmo du volume : mode et centre projeté (px CSS du canvas), ou null. */
		onvolumegizmo?: (gizmo: { mode: GizmoMode; anchor: [number, number] | null } | null) => void;
		/**
		 * Emprises écran projetées (px CSS du canvas) : objets par id et pièces
		 * par « objet/volume ». Sert au rectangle de sélection et aux tests e2e.
		 */
		onscreenboxes?: (boxes: Record<string, ScreenBox>) => void;
		/** Plan de coupe appliqué à l'objet sélectionné (peinture), ou null. */
		clipPlane?: SectionPlane | null;
		/** Repères d'outil (plan de l'outil Coupe, points de l'outil Mesure). */
		guides?: SceneGuides | null;
		/** Direction de vue (avant caméra, unitaire), à chaque changement. */
		oncameradir?: (forward: [number, number, number]) => void;
		/** Clic simple sur un objet : sélection de l'objet entier (pas d'une pièce). */
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
		extruderColors = null,
		objectExtruders = null,
		paintOverlays = null,
		wipeTower = null,
		onmovewipetower,
		selectedVolumeId = null,
		onpickvolume,
		onmeshpick,
		paintstroke = false,
		suspendcontrols = false,
		onobjectanchor,
		flattenMode = false,
		onflatten,
		onflattenanchors,
		ontransform,
		onvolumechange,
		ondragchange,
		onvolumegizmo,
		onscreenboxes,
		clipPlane = null,
		guides = null,
		oncameradir,
		onpickobject
	}: Props = $props();

	// Segment de mesure (deux points) : géométrie de ligne recréée à chaque mesure.
	const measureLine = $derived.by(() => {
		const pts = guides?.points ?? [];
		if (pts.length !== 2) return null;
		const g = new BufferGeometry();
		g.setAttribute('position', new BufferAttribute(new Float32Array([...pts[0], ...pts[1]]), 3));
		return g;
	});
	$effect(() => {
		const g = measureLine;
		return () => g?.dispose();
	});

	// Active le raycasting Threlte (événements de pointeur sur les meshes).
	interactivity();

	// Cadrage de la sélection : même direction de vue que la vue nommée active,
	// recentrée et rapprochée sur la boîte (seule la distance change).
	const pose = $derived.by(() => {
		const base = viewPose(bed, view);
		if (!focus) return zoomPose(base, zoom);
		const boxPose = frameBox(focus.min, focus.max, 1.6);
		const dir = base.position.map((p, i) => p - base.target[i]);
		const len = Math.hypot(...dir) || 1;
		const dist = Math.hypot(...boxPose.position.map((p, i) => p - boxPose.target[i]));
		return zoomPose(
			{
				target: boxPose.target,
				position: boxPose.target.map((c, i) => c + (dir[i] / len) * dist) as [
					number,
					number,
					number
				]
			},
			zoom
		);
	});
	const up = $derived(viewUp(view));

	// Fin du dernier geste de gizmo : le « click » DOM qui suit un glisser n'est
	// pas une sélection (il ne doit pas désélectionner la pièce manipulée).
	let gestureEndedAt = 0;
	function pick(id: string | null, additive: boolean) {
		selection = applyPick(selection, id, additive);
		if (id && !additive && performance.now() - gestureEndedAt > 250) onpickobject?.(id);
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

	// — Gizmo des volumes : quand un volume de l'objet cible est sélectionné, le
	//   gizmo manipule un témoin invisible posé en repère monde sur le volume
	//   (axes non déformés par l'échelle de l'objet). Chaque modification est
	//   reconvertie dans le repère de l'objet à partir de l'état du début du
	//   geste (volume-frame.ts → applyVolumeTransform).
	const targetObject = $derived(targetId ? (objects.find((o) => o.id === targetId) ?? null) : null);
	const targetVolume = $derived(
		targetObject && selectedVolumeId
			? ((targetObject.volumes ?? []).find((v) => v.id === selectedVolumeId) ?? null)
			: null
	);
	const objectTransform = (o: SceneObject): Transform => ({
		position: o.position ?? [0, 0, 0],
		rotation: o.rotation ?? [0, 0, 0],
		scale: o.scale ?? [1, 1, 1]
	});
	const { camera, size, invalidate, renderer } = useThrelte();
	// Plans de coupe par matériau (section des gizmos de peinture).
	renderer.localClippingEnabled = true;
	const volumeProxy = new Object3D();
	let gesture: { objectId: string; base: SceneVolume; object: Transform } | null = null;

	// Pose du témoin hors geste (le geste en cours n'est jamais contrarié).
	$effect(() => {
		if (!targetObject || !targetVolume || dragging) return;
		const w = volumeWorldTransform(objectTransform(targetObject), targetVolume);
		const rad = Math.PI / 180;
		volumeProxy.position.set(...w.position);
		volumeProxy.rotation.set(w.rotation[0] * rad, w.rotation[1] * rad, w.rotation[2] * rad);
		volumeProxy.scale.set(1, 1, 1);
		volumeProxy.updateMatrixWorld();
		invalidate();
	});

	function startVolumeGesture() {
		if (!targetObject || !targetVolume) return;
		gesture = {
			objectId: targetObject.id,
			base: structuredClone($state.snapshot(targetVolume)) as SceneVolume,
			object: structuredClone($state.snapshot(objectTransform(targetObject))) as Transform
		};
		dragging = true;
		ondragchange?.(true);
	}

	function endGesture() {
		gesture = null;
		dragging = false;
		gestureEndedAt = performance.now();
		ondragchange?.(false);
	}

	// Ancre écran de l'objet sélectionné, pour viser une facette réelle.
	const objectProjected = new Vector3();
	let lastObjectAnchor = '';
	// Cible ou pose changée : l'ancre précédente est périmée (elle désignerait
	// l'objet sélectionné avant, ou l'ancienne position) — retirée tout de suite,
	// recalculée à la frame suivante. Sans cela, un clic visant l'ancre juste
	// après une sélection (liste d'objets) touchait l'objet précédent.
	$effect(() => {
		void target;
		void targetObject?.position;
		void targetObject?.rotation;
		void targetObject?.scale;
		lastObjectAnchor = '';
		onobjectanchor?.(null);
		if (target) invalidate();
	});
	useTask(
		() => {
			if (!target || !onobjectanchor) return;
			target.getWorldPosition(objectProjected).project(camera.current);
			const { width, height } = size.current;
			const anchor: [number, number] = [
				((objectProjected.x + 1) / 2) * width,
				((1 - objectProjected.y) / 2) * height
			];
			const key = anchor.map((n) => n.toFixed(1)).join(',');
			if (key === lastObjectAnchor) return;
			lastObjectAnchor = key;
			onobjectanchor(anchor);
		},
		{ autoInvalidate: false }
	);

	// Emprises écran de chaque objet (8 coins de sa boîte monde projetés) et
	// centres projetés de ses pièces, recalculés aux rendus.
	const worldBox = new Box3();
	const corner = new Vector3();
	let lastBoxes = '';
	function project(v: Vector3): [number, number] {
		const { width, height } = size.current;
		v.project(camera.current);
		return [((v.x + 1) / 2) * width, ((1 - v.y) / 2) * height];
	}
	useTask(
		() => {
			if (!onscreenboxes) return;
			const boxes: Record<string, ScreenBox> = {};
			for (const [id, object] of Object.entries(refs)) {
				worldBox.setFromObject(object);
				if (worldBox.isEmpty()) continue;
				const box = { x0: Infinity, y0: Infinity, x1: -Infinity, y1: -Infinity };
				for (let i = 0; i < 8; i++) {
					corner.set(
						i & 1 ? worldBox.max.x : worldBox.min.x,
						i & 2 ? worldBox.max.y : worldBox.min.y,
						i & 4 ? worldBox.max.z : worldBox.min.z
					);
					const [x, y] = project(corner);
					box.x0 = Math.min(box.x0, x);
					box.y0 = Math.min(box.y0, y);
					box.x1 = Math.max(box.x1, x);
					box.y1 = Math.max(box.y1, y);
				}
				boxes[id] = box;
				object.traverse((child) => {
					const volumeId = child.userData?.volumeId;
					if (typeof volumeId !== 'string') return;
					const [x, y] = project(child.getWorldPosition(corner));
					boxes[`${id}/${volumeId}`] = { x0: x, y0: y, x1: x, y1: y };
				});
			}
			const key = Object.entries(boxes)
				.map(([id, b]) => `${id}:${[b.x0, b.y0, b.x1, b.y1].map((n) => n.toFixed(0)).join(',')}`)
				.join(';');
			if (key === lastBoxes) return;
			lastBoxes = key;
			onscreenboxes(boxes);
		},
		{ autoInvalidate: false }
	);

	// Points de mesure à taille écran constante (rayon MARKER_PX) : l'échelle des
	// sphères (rayon 1) suit le zoom et la distance, recalculée à chaque rendu.
	const markerRefs: (import('three').Mesh | undefined)[] = $state([]);
	const markerPos = new Vector3();
	useTask(
		() => {
			const cam = camera.current as Object3D & {
				isOrthographicCamera?: boolean;
				fov?: number;
				top?: number;
				bottom?: number;
				zoom?: number;
			};
			for (const marker of markerRefs) {
				if (!marker) continue;
				const r = markerWorldRadius(
					MARKER_PX,
					cam.getWorldPosition(markerPos).distanceTo(marker.position),
					size.current.height,
					cam.isOrthographicCamera
						? { orthoHeight: (cam.top ?? 1) - (cam.bottom ?? -1), zoom: cam.zoom }
						: { fov: cam.fov }
				);
				marker.scale.setScalar(r);
			}
		},
		{ autoInvalidate: false }
	);

	// Direction de vue courante (normale du plan de coupe de peinture).
	const forward = new Vector3();
	let lastForward = '';
	useTask(
		() => {
			if (!oncameradir) return;
			camera.current.getWorldDirection(forward);
			const key = forward
				.toArray()
				.map((n) => n.toFixed(4))
				.join(',');
			if (key === lastForward) return;
			lastForward = key;
			oncameradir([forward.x, forward.y, forward.z]);
		},
		{ autoInvalidate: false }
	);

	// — Gizmo Flatten : faces candidates (enveloppe convexe), survol puis clic.
	const flatPlanes = $derived<FlattenPlane[]>(
		flattenMode && targetObject
			? flattenPlanes(
					targetObject.mesh,
					targetObject.rotation ?? [0, 0, 0],
					targetObject.scale ?? [1, 1, 1]
				)
			: []
	);
	let hoveredPlane = $state<number | null>(null);
	const planeGeometries = $derived(
		flatPlanes.map((plane) => {
			const lift = plane.normal.map((n) => n * 0.1);
			const ring = plane.polygon.map((v) => [v[0] + lift[0], v[1] + lift[1], v[2] + lift[2]]);
			const positions: number[] = [];
			for (let i = 1; i + 1 < ring.length; i++)
				positions.push(...ring[0], ...ring[i], ...ring[i + 1]);
			const g = new BufferGeometry();
			g.setAttribute('position', new BufferAttribute(new Float32Array(positions), 3));
			return g;
		})
	);
	$effect(() => {
		const list = planeGeometries;
		return () => list.forEach((g) => g.dispose());
	});
	const planeProjected = new Vector3();
	let lastPlaneAnchors = '';
	$effect(() => {
		if (flatPlanes.length === 0) {
			lastPlaneAnchors = '';
			hoveredPlane = null;
			onflattenanchors?.(null);
		}
	});
	useTask(
		() => {
			if (!onflattenanchors || flatPlanes.length === 0 || !targetObject) return;
			const { width, height } = size.current;
			const origin = targetObject.position ?? [0, 0, 0];
			const anchors = flatPlanes.map((plane): [number, number] => {
				planeProjected
					.set(
						origin[0] + plane.centroid[0] + plane.normal[0] * 0.1,
						origin[1] + plane.centroid[1] + plane.normal[1] * 0.1,
						origin[2] + plane.centroid[2] + plane.normal[2] * 0.1
					)
					.project(camera.current);
				return [((planeProjected.x + 1) / 2) * width, ((1 - planeProjected.y) / 2) * height];
			});
			const key = anchors.map((a) => a.map((n) => n.toFixed(1)).join(',')).join(';');
			if (key === lastPlaneAnchors) return;
			lastPlaneAnchors = key;
			onflattenanchors(anchors);
		},
		{ autoInvalidate: false }
	);

	// Ancre écran du gizmo de volume (hook de test), mise à jour aux rendus.
	const projected = new Vector3();
	let lastAnchor = '';
	const volumeGizmoMode = $derived(targetVolume && gizmoMode ? gizmoMode : null);
	$effect(() => {
		if (!volumeGizmoMode) {
			lastAnchor = '';
			onvolumegizmo?.(null);
		}
	});
	useTask(
		() => {
			if (!volumeGizmoMode || !onvolumegizmo) return;
			const { width, height } = size.current;
			projected.copy(volumeProxy.position).project(camera.current);
			const anchor: [number, number] = [
				((projected.x + 1) / 2) * width,
				((1 - projected.y) / 2) * height
			];
			const key = `${volumeGizmoMode}:${anchor[0].toFixed(1)},${anchor[1].toFixed(1)}`;
			if (key === lastAnchor) return;
			lastAnchor = key;
			onvolumegizmo({ mode: volumeGizmoMode, anchor });
		},
		{ autoInvalidate: false }
	);
</script>

{#if projection === 'orthographic'}
	<T.OrthographicCamera
		makeDefault
		position={pose.position}
		{up}
		zoom={orthoZoom(bed) * zoom}
		near={-20000}
		far={20000}
	>
		<CameraControls enabled={!dragging && !suspendcontrols} target={pose.target} {up} />
	</T.OrthographicCamera>
{:else}
	<T.PerspectiveCamera makeDefault position={pose.position} {up} fov={50} near={1} far={20000}>
		<CameraControls enabled={!dragging && !suspendcontrols} target={pose.target} {up} />
	</T.PerspectiveCamera>
{/if}

<T.AmbientLight intensity={0.6} />
<T.DirectionalLight
	position={[bed.center.x + 200, bed.center.y - 200, 400]}
	intensity={1.2}
	castShadow
/>

<Bed {bed} {showGrid} />

{#if wipeTower}
	<WipeTower {...wipeTower} {bed} onmove={onmovewipetower} ondragchange={(d) => (dragging = d)} />
{/if}

{#each objects as obj (obj.id)}
	<ModelObject
		{extruderColors}
		extruder={objectExtruders?.[obj.id] ?? 1}
		paintOverlay={paintOverlays?.[obj.id] ?? null}
		mesh={obj.mesh}
		position={obj.position}
		rotation={obj.rotation}
		scale={obj.scale}
		volumes={obj.volumes}
		volumeMeshes={obj.volumeMeshes}
		selectedVolumeId={selection.has(obj.id) ? selectedVolumeId : null}
		onpickvolume={onpickvolume ? (volumeId) => onpickvolume(obj.id, volumeId) : undefined}
		{overhangNormalZ}
		selected={selection.has(obj.id)}
		printable={obj.printable}
		clipPlane={selection.has(obj.id) ? clipPlane : null}
		onpick={(additive) => pick(obj.id, additive)}
		onmeshpick={onmeshpick ? (meshPick) => onmeshpick(obj.id, meshPick) : undefined}
		{paintstroke}
		onref={(o) => setRef(obj.id, o)}
	/>
	{#if labels?.[obj.id] && !printOrder?.[obj.id]}
		<HTML position={obj.position ?? [0, 0, 0]} center pointerEvents="none">
			<span
				class="rounded bg-inverse/90 px-1.5 py-0.5 text-xs whitespace-nowrap text-on-inverse"
				data-testid="object-label">{labels[obj.id]}</span
			>
		</HTML>
	{:else if printOrder?.[obj.id]}
		<!-- Objet par objet : étiquette posée au-dessus de l'objet, rang
		     d'impression (Orca « Sequence #N »), nom si les étiquettes sont
		     affichées. Décorative pour les lecteurs d'écran : le rang est aussi
		     annoncé dans la liste d'objets. -->
		{@const pos = obj.position ?? [0, 0, 0]}
		{@const top =
			pos[2] + worldSize(obj.mesh, obj.rotation ?? [0, 0, 0], obj.scale ?? [1, 1, 1])[2] / 2}
		<HTML position={[pos[0], pos[1], top + 2]} center pointerEvents="none">
			<span
				class="flex -translate-y-1/2 flex-col items-center overflow-hidden rounded-md border text-xs whitespace-nowrap shadow-sm {selection.has(
					obj.id
				)
					? 'border-primary bg-primary text-primary-content'
					: 'border-border-strong bg-surface-raised/95 text-content'}"
				aria-hidden="true"
				data-testid="object-sequence-label"
				data-order={printOrder[obj.id]}
			>
				{#if labels?.[obj.id]}
					<span class="px-2 pt-0.5" data-testid="object-label">{labels[obj.id]}</span>
					<span
						class="w-full border-t border-current/20 px-2 pb-0.5 text-center tabular-nums opacity-90"
						>Séquence n° {printOrder[obj.id]}</span
					>
				{:else}
					<span class="px-1.5 py-0.5 font-semibold tabular-nums">#{printOrder[obj.id]}</span>
				{/if}
			</span>
		</HTML>
	{/if}
{/each}

{#if targetObject && targetId && flatPlanes.length > 0}
	<T.Group position={targetObject.position ?? [0, 0, 0]}>
		{#each planeGeometries as geometry, i (i)}
			<T.Mesh
				{geometry}
				renderOrder={3}
				onpointerenter={() => (hoveredPlane = i)}
				onpointerleave={() => {
					if (hoveredPlane === i) hoveredPlane = null;
				}}
				onclick={(e: { stopPropagation: () => void }) => {
					e.stopPropagation();
					onflatten?.(
						targetId,
						flattenRotation(targetObject.rotation ?? [0, 0, 0], flatPlanes[i].normal)
					);
				}}
			>
				<T.MeshBasicMaterial
					color={hoveredPlane === i ? $sceneColors.sectionPlaneHover : $sceneColors.sectionPlane}
					transparent
					opacity={hoveredPlane === i ? 0.75 : 0.45}
					side={DoubleSide}
					depthWrite={false}
				/>
			</T.Mesh>
		{/each}
	</T.Group>
{/if}

{#if guides?.plane}
	<!-- Plan de l'outil Coupe (gizmo Cut d'Orca) : semi-transparent, non cliquable. -->
	<T.Mesh
		position={guides.plane.center}
		rotation={guideRotation(guides.plane.normal)}
		renderOrder={4}
		userData={{ guide: 'cut-plane' }}
	>
		<T.PlaneGeometry args={guides.plane.size} />
		<T.MeshBasicMaterial
			color={$sceneColors.sectionPlaneHover}
			transparent
			opacity={0.55}
			side={DoubleSide}
			depthWrite={false}
		/>
	</T.Mesh>
{/if}
{#each guides?.points ?? [] as point, i (i)}
	<!-- Points de l'outil Mesure : toujours visibles (pas de test de profondeur). -->
	<T.Mesh
		position={point}
		renderOrder={6}
		bind:ref={markerRefs[i]}
		userData={{ guide: 'measure-point' }}
	>
		<T.SphereGeometry args={[1, 16, 12]} />
		<T.MeshBasicMaterial
			color={i === 0 ? $sceneColors.measureFirst : $sceneColors.measureSecond}
			depthTest={false}
			transparent
		/>
	</T.Mesh>
{/each}
{#if measureLine}
	<T.Line geometry={measureLine} renderOrder={6}>
		<T.LineBasicMaterial color={$sceneColors.measureFirst} depthTest={false} transparent />
	</T.Line>
{/if}

{#if targetVolume && gizmoMode && targetId}
	<T is={volumeProxy} />
	<TransformGizmo
		object={volumeProxy}
		mode={gizmoMode}
		ondragstart={startVolumeGesture}
		onchange={(world) => {
			if (!gesture) return;
			onvolumechange?.(gesture.objectId, volumeFromWorld(gesture.base, gesture.object, world));
		}}
		oncommit={endGesture}
	/>
{:else if target && gizmoMode && targetId}
	<TransformGizmo
		object={target}
		mode={gizmoMode}
		ondragstart={() => {
			dragging = true;
			ondragchange?.(true);
		}}
		onchange={(transform) => {
			dragging = true;
			ontransform?.(targetId, transform);
		}}
		oncommit={endGesture}
	/>
{/if}
