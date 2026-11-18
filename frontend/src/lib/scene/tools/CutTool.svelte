<script lang="ts">
	// Outil de coupe (T055) : plan positionnable par axe + position. Présentationnel :
	// émet le plan calculé au parent (qui appelle la coupe serveur et historise).
	// Phase 26 : la position est affichée en coordonnées du plateau (hauteur
	// depuis le plateau en Z, comme le gizmo Cut d'Orca) et le plan est dessiné
	// dans la vue 3D via `onpreview` ; les connecteurs (tenons, goujons) ne sont
	// pas encore pris en charge par la coupe serveur : on le dit avant la coupe.
	import type { CutPlane } from './cut';
	import { cutOffset, cutWorld, type Axis, type ObjectFrame } from '../guides';

	interface Props {
		/** Emprise du modèle (mm, repère recentré) pour borner le curseur. */
		min?: [number, number, number];
		max?: [number, number, number];
		/** Repère monde de l'objet (origine du maillage recentré, échelle). */
		frame?: ObjectFrame;
		oncut: (
			plane: CutPlane,
			connectorSpacing: number,
			keep: { upper: boolean; lower: boolean }
		) => void;
		oncancel?: () => void;
		/** Plan courant (axe + décalage local) pour l'aperçu 3D ; null à la fermeture. */
		onpreview?: (preview: { axis: Axis; offset: number } | null) => void;
		/** Coupe serveur en cours : bouton désactivé. */
		busy?: boolean;
	}

	let {
		min = [-100, -100, -100],
		max = [100, 100, 100],
		frame = { origin: [0, 0, 0], scale: [1, 1, 1] },
		oncut,
		oncancel,
		onpreview,
		busy = false
	}: Props = $props();

	let axis = $state<Axis>('z');
	/** Décalage dans le repère recentré du maillage (0 = milieu de l'objet). */
	let offset = $state(0);
	// Moitiés conservées (attributs KeepUpper / KeepLower de `Cut` d'Orca).
	let keepUpper = $state(true);
	let keepLower = $state(true);

	const axisIndex: Record<Axis, number> = { x: 0, y: 1, z: 2 };
	const round = (v: number) => Math.round(v * 100) / 100;
	// Bornes et valeur en coordonnées du plateau (mm).
	const lo = $derived(round(cutWorld(min[axisIndex[axis]], axis, frame)));
	const hi = $derived(round(cutWorld(max[axisIndex[axis]], axis, frame)));
	const world = $derived(round(cutWorld(offset, axis, frame)));
	function setWorld(v: number, clamp = true) {
		if (!Number.isFinite(v)) return;
		const clamped = clamp ? Math.min(Math.max(v, Math.min(lo, hi)), Math.max(lo, hi)) : v;
		offset = cutOffset(clamped, axis, frame);
	}
	function setAxis(a: Axis) {
		axis = a;
		offset = 0;
	}

	$effect(() => {
		onpreview?.({ axis, offset });
	});
	$effect(() => () => onpreview?.(null));

	function plane(): CutPlane {
		const normal: [number, number, number] = [0, 0, 0];
		normal[axisIndex[axis]] = 1;
		const point: [number, number, number] = [0, 0, 0];
		point[axisIndex[axis]] = offset;
		return { point, normal };
	}
</script>

<div class="flex flex-col gap-2 text-sm">
	<div class="flex items-center gap-2" role="radiogroup" aria-label="Axe de coupe">
		<span class="text-content-muted">Axe</span>
		{#each ['x', 'y', 'z'] as const as a (a)}
			<button
				type="button"
				role="radio"
				aria-checked={axis === a}
				data-testid={`cut-axis-${a}`}
				class="ws-chip {axis === a ? 'ws-chip-on' : ''}"
				onclick={() => setAxis(a)}>{a.toUpperCase()}</button
			>
		{/each}
	</div>

	<label class="flex items-center gap-2">
		<span class="w-20 text-content-muted">{axis === 'z' ? 'Hauteur' : 'Position'}</span>
		<input
			type="range"
			data-testid="cut-offset"
			min={Math.min(lo, hi)}
			max={Math.max(lo, hi)}
			step={0.5}
			value={world}
			oninput={(e) => setWorld(Number(e.currentTarget.value))}
			class="flex-1"
			aria-label={axis === 'z'
				? 'Hauteur de coupe (mm)'
				: `Position de coupe en ${axis.toUpperCase()} (mm)`}
		/>
		<input
			type="number"
			data-testid="cut-offset-input"
			value={world}
			step={0.1}
			oninput={(e) => {
				// Saisie libre (pas de bornage pendant la frappe), bornée à la validation.
				if (e.currentTarget.value !== '') setWorld(Number(e.currentTarget.value), false);
			}}
			onchange={(e) => setWorld(Number(e.currentTarget.value))}
			class="ws-field ws-field-sm w-20 text-right"
			aria-label={axis === 'z'
				? 'Hauteur de coupe (mm)'
				: `Position de coupe en ${axis.toUpperCase()} (mm)`}
		/>
		<span class="text-content-subtle">mm</span>
	</label>
	<p class="text-xs text-content-subtle" data-testid="cut-range">
		{axis === 'z' ? 'Depuis le plateau' : 'Coordonnée du plateau'} : {Math.min(lo, hi)} – {Math.max(
			lo,
			hi
		)} mm
	</p>

	<div class="flex items-center gap-3">
		<label class="flex items-center gap-1">
			<input type="checkbox" data-testid="cut-keep-upper" bind:checked={keepUpper} />
			<span>Garder le haut</span>
		</label>
		<label class="flex items-center gap-1">
			<input type="checkbox" data-testid="cut-keep-lower" bind:checked={keepLower} />
			<span>Garder le bas</span>
		</label>
	</div>

	<p class="text-xs text-content-subtle" data-testid="cut-connectors-note">
		Coupe plane uniquement : les connecteurs (tenons, goujons, clips) et la queue d'aronde d'Orca ne
		sont pas encore disponibles.
	</p>

	<div class="flex gap-2">
		<button
			type="button"
			data-testid="cut-apply"
			class="ws-btn ws-btn-primary"
			disabled={busy || (!keepUpper && !keepLower)}
			onclick={() => oncut(plane(), 0, { upper: keepUpper, lower: keepLower })}>Couper</button
		>
		<button type="button" class="ws-btn ws-btn-secondary" onclick={() => oncancel?.()}
			>Annuler</button
		>
	</div>
</div>
