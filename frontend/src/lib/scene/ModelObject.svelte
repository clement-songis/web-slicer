<script lang="ts">
	// Un objet 3D de la scène (T050) : construit une BufferGeometry depuis le
	// maillage décodé, met en surbrillance la sélection, et remonte les clics
	// (raycast Three.js) au parent.
	import { T, useThrelte } from '@threlte/core';
	import type { IntersectionEvent } from '@threlte/extras';
	import * as THREE from 'three';
	import type { MeshPick, SceneMesh } from './mesh';
	import type { SceneVolume, VolumeType } from './volumes';
	import { overhangIndices } from './overhang';
	import { sceneColors, wireframeColor } from './scene-colors';
	import type { SectionPlane } from './screen-select';
	import type { PaintOverlay } from './gizmos/painting/paint-overlay';
	import {
		constrainPointer,
		type HoverPreview,
		type PaintAssist,
		type PointerConstraint
	} from './gizmos/painting/paint-tools';
	import { creasedGeometry, type RealisticOptions } from './realistic';

	interface Props {
		mesh: SceneMesh;
		position?: [number, number, number];
		/** Rotation d'Euler en degrés (T103). */
		rotation?: [number, number, number];
		/** Échelle par axe (T103). */
		scale?: [number, number, number];
		selected?: boolean;
		/** Non imprimable (V) : grisé, mais toujours visible dans l'éditeur. */
		printable?: boolean;
		onpick?: (additive: boolean) => void;
		/** Picking complet pour les gizmos de peinture, mesure et oreilles. */
		onmeshpick?: (pick: MeshPick) => void;
		/** Remonte l'objet Three.js créé (pour y attacher le gizmo, T103). */
		onref?: (object: THREE.Object3D | null) => void;
		/** Volumes de l'objet, rendus dans son repère (semi-transparents, comme Orca). */
		volumes?: readonly SceneVolume[];
		/** Maillages (recentrés) des volumes `mesh`, par `modelId`. */
		volumeMeshes?: Record<string, SceneMesh>;
		/** Seuil `slope.normal_z` d'Orca : surplombs teintés en rouge (null = off). */
		overhangNormalZ?: number | null;
		/** Volume sélectionné (mis en évidence). */
		selectedVolumeId?: string | null;
		/** Alt+clic sur une pièce (volume) dans la vue 3D. */
		onpickvolume?: (volumeId: string) => void;
		/** Plan de coupe des gizmos de peinture (orienté caméra) : masque n·p > offset. */
		clipPlane?: SectionPlane | null;
		/** Peinture glissée : `onmeshpick` reçoit appui / déplacements / relâchement
		 *  (phase) au lieu du seul clic, comme le pinceau de GLGizmoPainterBase. */
		paintstroke?: boolean;
		/** Multi-filament : couleur de chaque extrudeur (Orca teinte les objets
		 *  à la couleur de leur filament) ; null = rendu neutre. */
		extruderColors?: readonly string[] | null;
		/** Extrudeur effectif de l'objet (1-based). */
		extruder?: number;
		/** Peinture visible (supports/couture/fuzzy/MMU), repère du maillage. */
		paintOverlay?: PaintOverlay | null;
		/** Objet désigné par la validation (phase 29) : lueur rouge (erreur) ou
		 *  orange (avertissement), visible même sélectionné ou teinté filament. */
		issue?: 'error' | 'warning' | null;
		/** Survol / contrainte du pinceau (gizmos de peinture). */
		paintAssist?: PaintAssist | null;
		/** Triangles que le clic peindrait (remplissage, triangle). */
		hoverPreview?: HoverPreview | null;
		/** Vue réaliste (Phong / Gouraud, normales lissées, ombres), null = rendu standard. */
		realistic?: RealisticOptions | null;
		/** Phase 39 — mode « Pièce » de la vue d'assemblage : le clic simple choisit
		 *  la pièce sous le curseur (comme Alt+clic). */
		partSelect?: boolean;
		/** Phase 39 — fil de fer (arêtes des triangles, peinture comprise) :
		 *  « Toggle Wireframe » d'Orca, Alt+Maj+Entrée en peinture MMU. */
		wireframe?: boolean;
	}

	// Couleurs par rôle, proches du rendu Orca (GLVolume) : pièce opaque,
	// négatif gris, modificateur jaune, bloqueur rouge, forceur vert.
	const VOLUME_STYLE: Record<VolumeType, { color: string; opacity: number }> = $derived({
		part: { color: $sceneColors.part, opacity: 1 },
		negative: { color: $sceneColors.volumeNegative, opacity: 0.35 },
		modifier: { color: $sceneColors.volumeModifier, opacity: 0.35 },
		blocker: { color: $sceneColors.volumeBlocker, opacity: 0.35 },
		enforcer: { color: $sceneColors.volumeEnforcer, opacity: 0.35 }
	});

	let {
		mesh,
		position = [0, 0, 0],
		rotation = [0, 0, 0],
		scale = [1, 1, 1],
		selected = false,
		printable = true,
		onpick,
		onmeshpick,
		onref,
		volumes = [],
		volumeMeshes = {},
		overhangNormalZ = null,
		selectedVolumeId = null,
		onpickvolume,
		clipPlane = null,
		paintstroke = false,
		extruderColors = null,
		extruder = 1,
		paintOverlay = null,
		issue = null,
		paintAssist = null,
		hoverPreview = null,
		realistic = null,
		partSelect = false,
		wireframe = false
	}: Props = $props();

	// Aperçu au survol (Orca : triangles « is_selected_by_seed_fill »).
	const previewGeometry = $derived.by(() => {
		if (!hoverPreview || hoverPreview.positions.length < 9) return null;
		const g = new THREE.BufferGeometry();
		g.setAttribute('position', new THREE.BufferAttribute(hoverPreview.positions, 3));
		return g;
	});
	$effect(() => {
		const g = previewGeometry;
		return () => g?.dispose();
	});

	// Peinture : triangles feuilles à couleur par sommet, posés sur la surface.
	const paintGeometry = $derived.by(() => {
		if (!paintOverlay) return null;
		const g = new THREE.BufferGeometry();
		g.setAttribute('position', new THREE.BufferAttribute(paintOverlay.positions, 3));
		g.setAttribute('color', new THREE.BufferAttribute(paintOverlay.colors, 3));
		g.computeVertexNormals();
		return g;
	});
	$effect(() => {
		const g = paintGeometry;
		return () => g?.dispose();
	});

	const tint = $derived(
		extruderColors ? (extruderColors[Math.max(1, extruder) - 1] ?? null) : null
	);
	/** Couleur d'une pièce : son extrudeur propre, sinon celui de l'objet. */
	const partTint = (v: SceneVolume) =>
		extruderColors && v.extruder ? (extruderColors[v.extruder - 1] ?? tint) : tint;

	// Coupe (Alt+molette en peinture) : Three garde -n·p + offset >= 0.
	const clippingPlanes = $derived(
		clipPlane === null
			? []
			: [new THREE.Plane(new THREE.Vector3(...clipPlane.normal).negate(), clipPlane.offset)]
	);

	// Facettes en surplomb (repère monde), recalculées avec la transformation.
	const overhangGeometry = $derived.by(() => {
		if (overhangNormalZ === null) return null;
		const idx = overhangIndices(mesh, overhangNormalZ, rotation, scale, position);
		if (idx.length === 0) return null;
		const g = new THREE.BufferGeometry();
		g.setAttribute('position', new THREE.BufferAttribute(mesh.positions, 3));
		g.setIndex(new THREE.BufferAttribute(idx, 1));
		return g;
	});
	$effect(() => {
		const g = overhangGeometry;
		return () => g?.dispose();
	});

	const DEG = Math.PI / 180;
	const radRotation = $derived<[number, number, number]>([
		rotation[0] * DEG,
		rotation[1] * DEG,
		rotation[2] * DEG
	]);

	// Référence à l'objet Three.js, remontée au parent (Viewport) pour le gizmo.
	let ref = $state<THREE.Mesh>();
	$effect(() => {
		onref?.(ref ?? null);
		return () => onref?.(null);
	});

	const baseGeometry = $derived.by(() => {
		const g = new THREE.BufferGeometry();
		g.setAttribute('position', new THREE.BufferAttribute(mesh.positions, 3));
		if (mesh.normals.length) {
			g.setAttribute('normal', new THREE.BufferAttribute(mesh.normals, 3));
		}
		g.setIndex(new THREE.BufferAttribute(mesh.indices, 1));
		if (!mesh.normals.length) g.computeVertexNormals();
		return g;
	});
	// Vue réaliste + normales lissées : per_corner_normals(5°) d'Orca. Géométrie
	// non indexée, même ordre de triangles (faceIndex du picking inchangé).
	const smooth = $derived(!!realistic?.smoothNormals);
	const geometry = $derived(smooth ? creasedGeometry(baseGeometry) : baseGeometry);
	$effect(() => {
		const g = geometry;
		const base = baseGeometry;
		return () => {
			if (g !== base) g.dispose();
		};
	});
	// Matériau commun (standard, Phong ou Gouraud selon la vue).
	const materialProps = $derived({
		color: printable
			? tint
				? tint
				: selected
					? $sceneColors.partSelected
					: $sceneColors.part
			: $sceneColors.partUnprintable,
		emissive: issue
			? issue === 'error'
				? $sceneColors.issueError
				: $sceneColors.issueWarning
			: tint && selected && printable
				? $sceneColors.emissiveActive
				: $sceneColors.emissiveNone,
		emissiveIntensity: issue
			? issue === 'error'
				? 0.55
				: 0.4
			: tint && selected && printable
				? 0.35
				: 0,
		transparent: !printable,
		opacity: printable ? 1 : 0.45,
		clippingPlanes,
		side: clipPlane === null ? THREE.FrontSide : THREE.DoubleSide
	});

	// Audit 40 E2 : matériau de surface géré ici plutôt que par trois branches
	// <T.Mesh…Material> : en basculant la vue réaliste à chaud, Threlte détachait
	// l'ancien matériau APRÈS avoir attaché le nouveau et rendait au maillage son
	// matériau par défaut (MeshBasicMaterial blanc, non éclairé) → objets gris clair
	// uniformes en Phong (mesuré : correct après rechargement, faux après bascule).
	const { invalidate } = useThrelte();
	const surfaceKind = $derived(realistic?.phong ? 'phong' : realistic ? 'gouraud' : 'standard');
	const surface = $derived.by((): THREE.Material => {
		if (surfaceKind === 'phong')
			// Vue réaliste, Phong (shader « phong » d'Orca).
			return new THREE.MeshPhongMaterial({ shininess: 40, specular: new THREE.Color('#3a3a3a') });
		if (surfaceKind === 'gouraud')
			// Vue réaliste sans Phong (« gouraud » d'Orca) : ombrage mat, sans reflets.
			return new THREE.MeshLambertMaterial();
		return new THREE.MeshStandardMaterial({ metalness: 0.1, roughness: 0.7 });
	});
	$effect(() => {
		const m = surface;
		return () => m.dispose();
	});
	$effect(() => {
		const m = surface as THREE.MeshStandardMaterial;
		const p = materialProps;
		const recompile =
			m.transparent !== p.transparent ||
			m.side !== p.side ||
			(m.clippingPlanes?.length ?? 0) !== (p.clippingPlanes?.length ?? 0);
		m.color.set(p.color);
		m.emissive.set(p.emissive);
		m.emissiveIntensity = p.emissiveIntensity;
		m.transparent = p.transparent;
		m.opacity = p.opacity;
		m.clippingPlanes = p.clippingPlanes ?? null;
		m.side = p.side;
		if (recompile) m.needsUpdate = true;
		invalidate();
	});

	// Fil de fer : arêtes claires ou sombres selon la teinte (getWireframeColor d'Orca).
	const edgeColor = $derived(wireframeColor(materialProps.color));

	// Libère la géométrie précédente quand elle change / au démontage.
	$effect(() => {
		const g = baseGeometry;
		return () => g.dispose();
	});

	// Géométries des volumes maillés, une par maillage (cache non réactif).
	const volumeGeometries = new WeakMap<SceneMesh, THREE.BufferGeometry>();
	function volumeGeometry(m: SceneMesh): THREE.BufferGeometry {
		let g = volumeGeometries.get(m);
		if (!g) {
			g = new THREE.BufferGeometry();
			g.setAttribute('position', new THREE.BufferAttribute(m.positions, 3));
			g.setIndex(new THREE.BufferAttribute(m.indices, 1));
			g.computeVertexNormals();
			volumeGeometries.set(m, g);
		}
		return g;
	}

	function volumeRotation(v: SceneVolume): [number, number, number] {
		const r = v.rotation ?? [0, 0, 0];
		return [r[0] * DEG, r[1] * DEG, r[2] * DEG];
	}

	// Clic sur un volume : comme Orca, le clic simple sélectionne l'objet et
	// Alt+clic sélectionne la pièce (volume) sous le curseur.
	function onvolumeclick(volumeId: string, event: IntersectionEvent<MouseEvent>) {
		event.stopPropagation();
		const ne = event.nativeEvent;
		if ((ne.altKey || partSelect) && onpickvolume) onpickvolume(volumeId);
		else onpick?.(ne.shiftKey || ne.ctrlKey || ne.metaKey);
	}

	function meshHit(event: IntersectionEvent<MouseEvent | PointerEvent>): MeshPick | null {
		if (!ref) return null;
		const local = ref.worldToLocal(event.point.clone());
		// Origine du rayon = caméra (perspective) ; repère maillage pour le
		// curseur cercle du TriangleSelector (GLGizmoPainterBase : trafo⁻¹·caméra).
		const camera = event.ray ? ref.worldToLocal(event.ray.origin.clone()) : null;
		return {
			point: [event.point.x, event.point.y, event.point.z],
			localPoint: [local.x, local.y, local.z],
			triangle: event.faceIndex ?? null,
			localCamera: camera ? [camera.x, camera.y, camera.z] : undefined
		};
	}

	// Trait glissé : chaque `pointermove` sur l'objet pendant l'appui est un
	// échantillon ; le relâchement (même hors de l'objet) clôt le trait.
	let stroking = false;
	// Trait contraint (« Verticale / Horizontale ») : le glisser est suivi sur
	// la fenêtre et re-lancé depuis la caméra au point écran contraint.
	let constrained: {
		camera: THREE.Camera;
		element: Element;
		first: [number, number];
		mode: PointerConstraint;
	} | null = null;
	const raycaster = new THREE.Raycaster();
	function constrainedMove(e: PointerEvent) {
		const c = constrained;
		if (!c || !ref || !onmeshpick) return;
		const rect = c.element.getBoundingClientRect();
		const ndc = constrainPointer(
			c.first,
			[
				((e.clientX - rect.left) / rect.width) * 2 - 1,
				-((e.clientY - rect.top) / rect.height) * 2 + 1
			],
			c.mode
		);
		raycaster.setFromCamera(new THREE.Vector2(ndc[0], ndc[1]), c.camera);
		const [hit] = raycaster.intersectObject(ref, false);
		if (!hit) return;
		const local = ref.worldToLocal(hit.point.clone());
		const cam = ref.worldToLocal(raycaster.ray.origin.clone());
		onmeshpick({
			point: [hit.point.x, hit.point.y, hit.point.z],
			localPoint: [local.x, local.y, local.z],
			triangle: hit.faceIndex ?? null,
			localCamera: [cam.x, cam.y, cam.z],
			phase: 'move'
		});
	}
	function endStroke() {
		if (constrained) window.removeEventListener('pointermove', constrainedMove);
		constrained = null;
		if (!stroking) return;
		stroking = false;
		onmeshpick?.({ point: [0, 0, 0], localPoint: [0, 0, 0], triangle: null, phase: 'up' });
	}
	function onpointerdown(event: IntersectionEvent<PointerEvent>) {
		if (!paintstroke || !onmeshpick || event.nativeEvent.button !== 0) return;
		const hit = meshHit(event);
		if (!hit) return;
		event.stopPropagation();
		stroking = true;
		paintAssist?.onhover?.(null);
		onpick?.(false);
		onmeshpick({ ...hit, phase: 'down' });
		const mode = paintAssist?.constraint ?? null;
		const element = event.nativeEvent.target;
		if (mode && element instanceof Element) {
			constrained = {
				camera: event.camera,
				element,
				first: [event.pointer.x, event.pointer.y],
				mode
			};
			window.addEventListener('pointermove', constrainedMove);
		}
		window.addEventListener('pointerup', endStroke, { once: true });
	}
	function onpointermove(event: IntersectionEvent<PointerEvent>) {
		if (!stroking) {
			// Survol (aperçu des remplissages / du triangle).
			if (paintstroke && paintAssist?.onhover) {
				const hit = meshHit(event);
				paintAssist.onhover(hit);
			}
			return;
		}
		if (!onmeshpick || constrained) return;
		const hit = meshHit(event);
		if (hit) onmeshpick({ ...hit, phase: 'move' });
	}
	function onpointerleave() {
		if (!stroking) paintAssist?.onhover?.(null);
	}
	$effect(() => () => {
		window.removeEventListener('pointermove', constrainedMove);
	});

	function onclick(event: IntersectionEvent<MouseEvent>) {
		event.stopPropagation();
		const ne = event.nativeEvent;
		if ((ne.altKey || partSelect) && onpickvolume) {
			// Alt+clic : pièce la plus proche sous le curseur (même si la surface
			// de l'objet la recouvre), sinon l'objet lui-même.
			const hit = (event.intersections ?? []).find((i) => i.object.userData?.volumeId);
			if (hit) onpickvolume(hit.object.userData.volumeId as string);
			else onpick?.(false);
			return;
		}
		onpick?.(ne.shiftKey || ne.ctrlKey || ne.metaKey);
		// En peinture glissée, le trait (appui → relâchement) a déjà tout appliqué.
		if (onmeshpick && !paintstroke) {
			const hit = meshHit(event);
			if (hit) onmeshpick(hit);
		}
	}
</script>

<T.Mesh
	bind:ref
	{geometry}
	{position}
	rotation={radRotation}
	{scale}
	castShadow
	material={surface}
	{onclick}
	{onpointerdown}
	{onpointermove}
	{onpointerleave}
>
	{#if wireframe}
		<!-- Fil de fer (shader mm_gouraud d'Orca, show_wireframe) : arêtes des
		     triangles du maillage, non pickable. -->
		<T.Mesh {geometry} renderOrder={5} raycast={() => {}}>
			<T.MeshBasicMaterial
				wireframe
				color={edgeColor}
				transparent
				opacity={0.55}
				depthWrite={false}
				polygonOffset
				polygonOffsetFactor={-4}
				polygonOffsetUnits={-4}
				{clippingPlanes}
			/>
		</T.Mesh>
		{#if paintGeometry}
			<T.Mesh geometry={paintGeometry} renderOrder={5} raycast={() => {}}>
				<T.MeshBasicMaterial
					wireframe
					color={edgeColor}
					transparent
					opacity={0.55}
					depthWrite={false}
					polygonOffset
					polygonOffsetFactor={-4}
					polygonOffsetUnits={-4}
					{clippingPlanes}
				/>
			</T.Mesh>
		{/if}
	{/if}
	{#if overhangGeometry}
		<!-- Surplombs : teinte rouge par-dessus (gouraud.fs : 50 % LightRed). -->
		<T.Mesh geometry={overhangGeometry} renderOrder={2}>
			<T.MeshBasicMaterial
				color={$sceneColors.overhang}
				transparent
				opacity={0.6}
				polygonOffset
				polygonOffsetFactor={-1}
			/>
		</T.Mesh>
	{/if}
	{#if paintGeometry}
		<!-- Peinture (Orca TriangleSelectorGUI) : non pickable, le trait vise l'objet. -->
		<T.Mesh geometry={paintGeometry} renderOrder={3} raycast={() => {}}>
			<T.MeshLambertMaterial
				vertexColors
				polygonOffset
				polygonOffsetFactor={-2}
				polygonOffsetUnits={-2}
				{clippingPlanes}
			/>
		</T.Mesh>
	{/if}
	{#if previewGeometry && hoverPreview}
		<!-- Aperçu au survol : zone que le clic peindrait (non pickable). -->
		<T.Mesh geometry={previewGeometry} renderOrder={4} raycast={() => {}}>
			<T.MeshBasicMaterial
				color={hoverPreview.color}
				transparent
				opacity={0.55}
				depthWrite={false}
				side={THREE.DoubleSide}
				polygonOffset
				polygonOffsetFactor={-3}
				polygonOffsetUnits={-3}
				{clippingPlanes}
			/>
		</T.Mesh>
	{/if}
	{#each volumes as volume (volume.id)}
		{@const style = VOLUME_STYLE[volume.type]}
		{@const active = volume.id === selectedVolumeId}
		{@const partMesh =
			volume.shape === 'mesh' && volume.modelId ? volumeMeshes[volume.modelId] : undefined}
		<T.Mesh
			position={volume.offset}
			rotation={volumeRotation(volume)}
			renderOrder={style.opacity < 1 ? 1 : 0}
			userData={{ volumeId: volume.id }}
			geometry={partMesh ? volumeGeometry(partMesh) : undefined}
			onclick={(e: IntersectionEvent<MouseEvent>) => onvolumeclick(volume.id, e)}
		>
			{#if !partMesh}
				<T.BoxGeometry args={volume.size} />
			{/if}
			<T.MeshStandardMaterial
				color={volume.type === 'part' ? (partTint(volume) ?? style.color) : style.color}
				emissive={active ? $sceneColors.emissiveActive : $sceneColors.emissiveNone}
				emissiveIntensity={active ? 0.6 : 0}
				transparent={style.opacity < 1}
				opacity={active ? Math.min(1, style.opacity + 0.25) : style.opacity}
				depthWrite={style.opacity >= 1}
				metalness={0.1}
				roughness={0.7}
				{clippingPlanes}
				side={clipPlane === null ? THREE.FrontSide : THREE.DoubleSide}
			/>
		</T.Mesh>
	{/each}
</T.Mesh>
