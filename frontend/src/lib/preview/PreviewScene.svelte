<script lang="ts">
	// Hôte Canvas de l'aperçu G-code (T088) : caméra cadrée sur le plateau,
	// éclairage neutre et lignes d'extrusion colorées (PreviewLines). Pendant du
	// `Scene.svelte` de préparation, réutilisé par le workspace éditeur.
	import { Canvas, T } from '@threlte/core';
	import CameraControls from '$lib/scene/CameraControls.svelte';
	import Bed from '$lib/scene/Bed.svelte';
	import { frameBed, frameBox, type BedShape } from '$lib/scene';
	import PreviewLines from './PreviewLines.svelte';
	import PreviewPoints from './PreviewPoints.svelte';
	import PreviewExtrusions from './PreviewExtrusions.svelte';
	import PreviewWarmup from './PreviewWarmup.svelte';
	import FpsCounter from '$lib/scene/FpsCounter.svelte';
	import { FrameMeter, rendererFactory } from '$lib/scene/render-settings';
	import { appPreferences } from '$lib/settings/preferences';
	import type { PreviewGeometry, PreviewOverlay } from './geometry';

	interface Props {
		geometry: PreviewGeometry;
		bed: BedShape;
		/** Options d'affichage : déplacements, essuyages, marqueurs. */
		overlay?: PreviewOverlay | null;
		/** Boîte des objets imprimés (cadrage « Objets ») ; null → plateau. */
		frame?: { min: [number, number, number]; max: [number, number, number] } | null;
		/** Cadrage demandé ; `frameKey` change à chaque demande (re-cadre). */
		fit?: 'objects' | 'bed';
		frameKey?: number;
		/** Rendu des extrusions : volumes éclairés (défaut) ou lignes. */
		renderMode?: 'volumes' | 'lines';
		/** Marqueur de l'outil (sommet courant du curseur horizontal, Orca ToolMarker). */
		tool?: [number, number, number] | null;
	}

	let {
		geometry,
		bed,
		overlay = null,
		frame = null,
		fit = 'objects',
		frameKey = 0,
		renderMode = 'volumes',
		tool = null
	}: Props = $props();
	// Marqueur : cône pointe en bas posé sur le sommet (comme le ToolMarker de
	// libvgcode : blanc translucide, toujours visible au-dessus des parcours).
	const MARKER_H = 4;

	// Précompilation des shaders avant la première image de chaque mode de
	// rendu (volumes éclairés / lignes) : état exposé pour les tests et la mesure.
	let compiled = $state<{ mode: string; ms: number; timedOut: boolean } | null>(null);
	// Interrupteur de diagnostic (mesure A/B) : localStorage « web-slicer.preview-warmup » = « off ».
	const warmup = (() => {
		try {
			return localStorage.getItem('web-slicer.preview-warmup') !== 'off';
		} catch {
			return true;
		}
	})();

	const pose = $derived(
		fit === 'objects' && frame ? frameBox(frame.min, frame.max) : frameBed(bed)
	);
	// Phase 34 : graphismes (MSAA, plafond et compteur d'images/s).
	const frameMeter = new FrameMeter();
</script>

<div
	class="relative h-full w-full"
	data-testid="preview-scene"
	data-preview-ready={compiled?.mode === renderMode ? 'true' : 'false'}
	data-preview-bed={`${bed.width}x${bed.depth}`}
	data-preview-compile-ms={compiled?.ms}
	data-preview-compile-timeout={compiled?.timedOut ? 'true' : undefined}
	data-tool-marker={tool ? tool.map((v) => v.toFixed(2)).join(',') : undefined}
>
	{#key $appPreferences.graphics.antialias}
		<Canvas createRenderer={rendererFactory($appPreferences.graphics.antialias, frameMeter)}>
			{#key frameKey}
				<T.PerspectiveCamera
					makeDefault
					position={pose.position}
					up={[0, 0, 1]}
					fov={50}
					near={1}
					far={20000}
				>
					<CameraControls target={pose.target} />
				</T.PerspectiveCamera>
			{/key}
			<!-- Plateau sous les parcours (Orca GCodeViewer : le lit reste affiché en aperçu). -->
			<Bed {bed} />
			{#if renderMode === 'volumes'}
				<!-- Éclairage des volumes : ambiant + lumière principale + contre-jour doux. -->
				<T.AmbientLight intensity={0.85} />
				<T.DirectionalLight position={[-0.5, -1, 1.6]} intensity={1.25} />
				<T.DirectionalLight position={[1, 0.8, 0.4]} intensity={0.6} />
				<PreviewExtrusions {geometry} />
			{:else}
				<T.AmbientLight intensity={0.9} />
				<PreviewLines {geometry} />
			{/if}
			{#if overlay && overlay.lines.visibleCount > 0}
				<PreviewLines geometry={overlay.lines} />
			{/if}
			{#if overlay && overlay.points.visibleCount > 0}
				<PreviewPoints geometry={overlay.points} />
			{/if}
			{#if tool}
				<T.Mesh
					position={[tool[0], tool[1], tool[2] + MARKER_H / 2]}
					rotation={[-Math.PI / 2, 0, 0]}
					renderOrder={10}
				>
					<T.ConeGeometry args={[1.2, MARKER_H, 20]} />
					<T.MeshBasicMaterial
						color="#ffffff"
						transparent
						opacity={0.65}
						depthTest={false}
						depthWrite={false}
					/>
				</T.Mesh>
			{/if}
			{#if warmup}
				{#key renderMode}
					<PreviewWarmup onready={(r) => (compiled = { mode: renderMode, ...r })} />
				{/key}
			{/if}
		</Canvas>
	{/key}
	<FpsCounter meter={frameMeter} />
</div>
