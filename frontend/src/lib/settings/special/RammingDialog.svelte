<script lang="ts">
	// Éditeur de ramming (Orca : RammingDialog / RammingPanel + Chart) : courbe
	// du débit volumétrique pendant le ramming, points déplaçables verticalement
	// (souris, clavier), durée par pas de 0,25 s, largeur / espacement des
	// lignes. Débits échantillonnés et chaîne écrits comme Orca (ramming.ts) ;
	// sans changement de courbe, la chaîne d'origine est rendue telle quelle.
	import Dialog from '$lib/ui/Dialog.svelte';
	import {
		DEFAULT_RAMMING,
		RAMMING_MAX_SPEED,
		RAMMING_MAX_TIME,
		RAMMING_STEP,
		curveSamples,
		finishEdit,
		movePoint,
		parseRamming,
		rammingVolume,
		sampleSpeeds,
		setTime,
		startEdit,
		visiblePoints,
		type RammingEdit
	} from './ramming';

	interface Props {
		open: boolean;
		value: string;
		onclose: () => void;
		onapply: (value: string) => void;
	}

	let { open, value, onclose, onapply }: Props = $props();

	let edit = $state<RammingEdit>(startEdit(''));
	let uniform = $state(false);
	let lastOpen = false;
	$effect(() => {
		if (open && !lastOpen) edit = startEdit(value);
		lastOpen = open;
	});

	// Géométrie du tracé (SVG, unités = px du viewBox).
	const W = 520;
	const H = 300;
	const PAD = { l: 44, r: 12, t: 14, b: 34 };
	const PW = W - PAD.l - PAD.r;
	const PH = H - PAD.t - PAD.b;
	const axisTime = $derived(Math.max(edit.time, RAMMING_STEP));
	const sx = (t: number) => PAD.l + (t / axisTime) * PW;
	const sy = (v: number) => PAD.t + PH - (v / RAMMING_MAX_SPEED) * PH;

	const speeds = $derived(
		edit.curveChanged
			? (sampleSpeeds(edit.points, edit.time) ?? parseRamming(edit.original).speeds)
			: parseRamming(edit.original).speeds
	);
	const volume = $derived(Math.round(rammingVolume(edit.points, edit.time)));
	const curve = $derived(
		curveSamples(edit.points, edit.time)
			.map((p) => `${sx(p.t).toFixed(1)},${sy(p.speed).toFixed(1)}`)
			.join(' ')
	);
	const visible = $derived(visiblePoints(edit));
	const result = $derived(finishEdit(edit));
	const xTicks = $derived(
		Array.from({ length: Math.floor(axisTime / 0.5 + 1e-9) + 1 }, (_, i) => i * 0.5)
	);
	const yTicks = [0, 10, 20, 30, 40, 50, 60];

	let svg = $state<SVGSVGElement | null>(null);
	let dragging = $state<number | null>(null);
	function speedAt(event: PointerEvent): number {
		if (!svg) return 0;
		const pt = svg.createSVGPoint();
		pt.x = event.clientX;
		pt.y = event.clientY;
		const local = pt.matrixTransform(svg.getScreenCTM()?.inverse());
		return ((PAD.t + PH - local.y) / PH) * RAMMING_MAX_SPEED;
	}
	function onDown(event: PointerEvent, i: number) {
		dragging = i;
		(event.currentTarget as Element).setPointerCapture(event.pointerId);
		(event.currentTarget as SVGElement).focus();
	}
	function onMove(event: PointerEvent, i: number) {
		if (dragging !== i) return;
		edit = movePoint(edit, i, speedAt(event), uniform || event.ctrlKey || event.metaKey);
	}
	function onKey(event: KeyboardEvent, i: number) {
		const step = event.shiftKey ? 1 : 0.1;
		const d = event.key === 'ArrowUp' ? step : event.key === 'ArrowDown' ? -step : 0;
		if (!d) return;
		event.preventDefault();
		edit = movePoint(edit, i, edit.points[i][1] + d, uniform || event.ctrlKey || event.metaKey);
	}
	const fmt = (n: number, d = 1) => n.toLocaleString('fr-FR', { maximumFractionDigits: d });
</script>

<Dialog {open} label="Ramming" size="xl" {onclose} data-testid="ramming-dialog">
	{#snippet header()}
		<div class="flex min-w-0 flex-col gap-0.5">
			<h2 class="ws-dialog-title">Ramming</h2>
			<p class="text-xs text-content-subtle">
				Extrusion rapide juste avant un changement d'outil sur une imprimante multi-matériaux à
				extrudeur unique : elle donne à l'extrémité du filament déchargé une forme propre pour le
				recharger.
			</p>
		</div>
	{/snippet}
	<div class="flex flex-wrap gap-6">
		<figure class="m-0 w-[520px] max-w-full shrink-0">
			<svg
				bind:this={svg}
				viewBox="0 0 {W} {H}"
				width={W}
				height={H}
				class="block max-w-full rounded border border-border bg-surface-sunken select-none"
				data-testid="ramming-chart"
				role="group"
				aria-label="Débit volumétrique du ramming en fonction du temps"
			>
				{#each yTicks as v (v)}
					<line
						x1={PAD.l}
						x2={W - PAD.r}
						y1={sy(v)}
						y2={sy(v)}
						class="stroke-border"
						stroke-width="1"
					/>
					<text
						x={PAD.l - 6}
						y={sy(v) + 4}
						text-anchor="end"
						class="fill-content-subtle text-[10px]">{v}</text
					>
				{/each}
				{#each xTicks as tk (tk)}
					<line
						x1={sx(tk)}
						x2={sx(tk)}
						y1={PAD.t}
						y2={PAD.t + PH}
						class="stroke-border"
						stroke-width="1"
					/>
					<text
						x={sx(tk)}
						y={H - PAD.b + 14}
						text-anchor="middle"
						class="fill-content-subtle text-[10px]">{fmt(tk, 2)}</text
					>
				{/each}
				<text
					x={PAD.l + PW / 2}
					y={H - 4}
					text-anchor="middle"
					class="fill-content-muted text-[11px]">Temps (s)</text
				>
				<text
					x="10"
					y={PAD.t + PH / 2}
					text-anchor="middle"
					transform="rotate(-90 10 {PAD.t + PH / 2})"
					class="fill-content-muted text-[11px]">Débit (mm³/s)</text
				>
				<!-- Débit retenu par pas de 0,25 s (valeurs écrites dans la chaîne). -->
				{#each speeds as v, i (i)}
					<rect
						x={sx(i * RAMMING_STEP) + 1}
						y={sy(v)}
						width={Math.max(0, sx(RAMMING_STEP) - sx(0) - 2)}
						height={Math.max(0, PAD.t + PH - sy(v))}
						class="fill-primary/15"
						data-testid="ramming-step"
						data-speed={v}
					/>
				{/each}
				{#if curve}
					<polyline points={curve} fill="none" class="stroke-primary" stroke-width="2" />
				{/if}
				{#each visible as i (i)}
					{@const p = edit.points[i]}
					<g>
						<text
							x={sx(p[0])}
							y={sy(p[1]) - 10}
							text-anchor="middle"
							class="pointer-events-none fill-content text-[10px] tabular-nums">{fmt(p[1])}</text
						>
						<circle
							cx={sx(p[0])}
							cy={sy(p[1])}
							r="6"
							class="cursor-ns-resize fill-surface-raised stroke-primary outline-none focus-visible:stroke-accent"
							stroke-width={dragging === i ? 3 : 2}
							role="slider"
							tabindex="0"
							aria-label={`Débit à ${fmt(p[0], 2)} s`}
							aria-valuemin="0"
							aria-valuemax={RAMMING_MAX_SPEED}
							aria-valuenow={Math.round(p[1] * 10) / 10}
							data-testid="ramming-point"
							onpointerdown={(e) => onDown(e, i)}
							onpointermove={(e) => onMove(e, i)}
							onpointerup={() => (dragging = null)}
							onpointercancel={() => (dragging = null)}
							onkeydown={(e) => onKey(e, i)}
						/>
					</g>
				{/each}
			</svg>
			<figcaption class="mt-1 text-xs text-content-subtle">
				Glissez un point (ou flèches ↑/↓, Maj = pas de 1). Ctrl ou la case ci-dessous : débit
				constant, tous les points se déplacent ensemble.
			</figcaption>
		</figure>
		<div class="flex min-w-56 flex-1 flex-col gap-4 text-sm">
			<section class="flex flex-col gap-2">
				<h3 class="ws-section-title">Ramming total</h3>
				<label class="flex items-center justify-between gap-3">
					<span class="text-content-muted">Durée</span>
					<span class="flex items-center gap-1">
						<input
							type="number"
							class="ws-field ws-field-sm w-24 text-right"
							min="0"
							max={RAMMING_MAX_TIME * 1000}
							step="250"
							value={Math.round(edit.time * 1000)}
							data-testid="ramming-time"
							onchange={(e) => (edit = setTime(edit, Number(e.currentTarget.value) / 1000))}
						/>ms
					</span>
				</label>
				<div class="flex items-center justify-between gap-3">
					<span class="text-content-muted">Volume</span>
					<output class="tabular-nums" data-testid="ramming-volume">{volume} mm³</output>
				</div>
				<label class="flex items-center gap-2 text-content-muted">
					<input type="checkbox" bind:checked={uniform} data-testid="ramming-uniform" />
					Débit constant (déplacer tous les points)
				</label>
			</section>
			<section class="flex flex-col gap-2">
				<h3 class="ws-section-title">Ligne de ramming</h3>
				<label class="flex items-center justify-between gap-3">
					<span class="text-content-muted">Largeur</span>
					<span class="flex items-center gap-1">
						<input
							type="number"
							class="ws-field ws-field-sm w-24 text-right"
							min="10"
							max="300"
							step="1"
							bind:value={edit.lineWidth}
							data-testid="ramming-width"
						/>%
					</span>
				</label>
				<label class="flex items-center justify-between gap-3">
					<span class="text-content-muted">Espacement</span>
					<span class="flex items-center gap-1">
						<input
							type="number"
							class="ws-field ws-field-sm w-24 text-right"
							min="10"
							max="300"
							step="1"
							bind:value={edit.spacing}
							data-testid="ramming-spacing"
						/>%
					</span>
				</label>
			</section>
			<details class="text-xs text-content-subtle">
				<summary class="cursor-pointer">Chaîne Orca</summary>
				<code class="mt-1 block font-mono break-all" data-testid="ramming-string">{result}</code>
			</details>
		</div>
	</div>
	{#snippet footer()}
		<button
			type="button"
			class="ws-btn ws-btn-ghost mr-auto"
			data-testid="ramming-reset"
			onclick={() => (edit = startEdit(DEFAULT_RAMMING))}>Valeurs par défaut</button
		>
		<button type="button" class="ws-btn ws-btn-secondary" onclick={onclose}>Annuler</button>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			data-testid="ramming-apply"
			onclick={() => onapply(result)}>Appliquer</button
		>
	{/snippet}
</Dialog>
