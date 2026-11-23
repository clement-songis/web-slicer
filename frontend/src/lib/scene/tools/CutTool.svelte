<script lang="ts">
	// Outil Coupe (phase 28) : découpe complète d'Orca (GLGizmoCut3D) — plan
	// incliné (anneaux de rotation dans la vue, angles ici), connecteurs posés
	// d'un clic sur la face de coupe (vue 3D ou carte de section ci-dessous),
	// queue d'aronde, options après la coupe. État partagé : CutSession.
	import type { CutSession } from './cut-session.svelte';
	import { untrack } from 'svelte';
	import { t } from '$lib/i18n';
	import type { CutConnectorShape } from '../../../generated/api/CutConnectorShape';
	import type { CutConnectorType } from '../../../generated/api/CutConnectorType';

	interface Props {
		session: CutSession;
		oncut: () => void;
		oncancel?: () => void;
		busy?: boolean;
	}
	let { session: s, oncut, oncancel, busy = false }: Props = $props();

	// Audit 40 B2 : chaque changement de l'outil (plan, connecteurs, réglages)
	// alimente l'historique propre à l'outil ; Ctrl+Z / Ctrl+Y, outil ouvert,
	// n'annulent que ces étapes (Orca SnapshotType::GizmoAction). Oublié à la fermeture.
	$effect(() => {
		const key = s.historyKey();
		untrack(() => s.noteChange(key));
	});
	$effect(() => () => s.clearHistory());

	const round = (v: number) => Math.round(v * 100) / 100;
	const lo = $derived(round(Math.min(s.worldOf(s.range[0]), s.worldOf(s.range[1]))));
	const hi = $derived(round(Math.max(s.worldOf(s.range[0]), s.worldOf(s.range[1]))));
	const world = $derived(round(s.worldOf(s.offset)));
	const axisName = $derived(['X', 'Y', 'Z'][s.displayAxis]);
	function setWorld(v: number, clamp = true) {
		if (!Number.isFinite(v)) return;
		const w = clamp ? Math.min(Math.max(v, lo), hi) : v;
		s.offset = s.offsetOf(w);
	}
	function setRot(i: number, v: number) {
		if (!Number.isFinite(v)) return;
		const r = [...s.rotation] as [number, number, number];
		r[i] = v;
		s.setRotation(r);
	}
	const num = (e: Event) => Number((e.currentTarget as HTMLInputElement).value.replace(',', '.'));

	const TYPES: { id: CutConnectorType; label: string; tip: string }[] = [
		{ id: 'plug', label: 'Plug', tip: 'Plug printed on one half, hole in the other' },
		{ id: 'dowel', label: 'Dowel', tip: 'Hole in both halves + separately printed dowel' },
		{ id: 'snap', label: 'Snap', tip: 'Split clip that snaps in' }
	];
	const SHAPES: { id: CutConnectorShape; label: string }[] = [
		{ id: 'circle', label: 'Circle' },
		{ id: 'square', label: 'Square' },
		{ id: 'hexagon', label: 'Hexagon' },
		{ id: 'triangle', label: 'Triangle' }
	];

	// Carte de la face de coupe (repère du plan, v vers le haut).
	const pad = 3;
	const view = $derived.by(() => {
		const [u0, v0] = s.section.min;
		const [u1, v1] = s.section.max;
		const w = Math.max(u1 - u0, 1) + 2 * pad;
		const h = Math.max(v1 - v0, 1) + 2 * pad;
		return { x: u0 - pad, y: -v1 - pad, w, h };
	});
	const path = $derived.by(() => {
		const seg = s.section.segments;
		let d = '';
		for (let i = 0; i + 3 < seg.length; i += 4)
			d +=
				'M' +
				seg[i].toFixed(2) +
				' ' +
				(-seg[i + 1]).toFixed(2) +
				'L' +
				seg[i + 2].toFixed(2) +
				' ' +
				(-seg[i + 3]).toFixed(2);
		return d;
	});
	let svg = $state<SVGSVGElement | undefined>();
	let dragging: string | null = null;
	function toUv(e: PointerEvent): [number, number] | null {
		const m = svg?.getScreenCTM();
		if (!svg || !m) return null;
		const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(m.inverse());
		return [p.x, -p.y];
	}
	function onMapDown(e: PointerEvent) {
		const uv = toUv(e);
		if (!uv) return;
		s.pickUv(uv[0], uv[1]);
		dragging = s.selectedId;
		svg?.setPointerCapture?.(e.pointerId);
	}
	function onMapMove(e: PointerEvent) {
		if (!dragging) return;
		const uv = toUv(e);
		if (uv) s.moveConnector(dragging, uv[0], uv[1]);
	}
	function onKey(e: KeyboardEvent) {
		if ((e.key === 'Delete' || e.key === 'Backspace') && s.selectedId) {
			e.preventDefault();
			s.removeConnector();
		}
	}
	function polygon(cx: number, cy: number, r: number, n: number, rot: number): string {
		const pts: string[] = [];
		for (let i = 0; i < n; i++) {
			const a = ((rot + 90) * Math.PI) / 180 + (2 * Math.PI * i) / n;
			pts.push((cx + r * Math.cos(a)).toFixed(2) + ',' + (cy - r * Math.sin(a)).toFixed(2));
		}
		return pts.join(' ');
	}
	const canConnect = $derived(!s.asParts && s.mode === 'planar');
</script>

<div class="flex max-w-72 flex-col gap-2 text-sm" data-testid="cut-tool">
	<div class="ws-segmented flex gap-1" role="radiogroup" aria-label={$t('Cut mode')}>
		<button
			type="button"
			role="radio"
			aria-checked={s.mode === 'planar'}
			data-testid="cut-mode-planar"
			class="ws-chip {s.mode === 'planar' ? 'ws-chip-on' : ''}"
			onclick={() => s.setMode('planar')}
		>
			{$t('Planar||cut')}
		</button>
		<button
			type="button"
			role="radio"
			aria-checked={s.mode === 'groove'}
			data-testid="cut-mode-groove"
			class="ws-chip {s.mode === 'groove' ? 'ws-chip-on' : ''}"
			disabled={s.connectors.length > 0}
			title={s.connectors.length > 0
				? $t('Remove the connectors to switch to dovetail (Orca)')
				: $t('Dovetail: the two halves interlock')}
			onclick={() => s.setMode('groove')}
		>
			{$t('Dovetail')}
		</button>
	</div>

	<div class="flex items-center gap-2" role="radiogroup" aria-label={$t('Cut axis')}>
		<span class="text-content-muted">{$t('Axis')}</span>
		{#each ['x', 'y', 'z'] as const as a (a)}
			<button
				type="button"
				role="radio"
				aria-checked={s.axis === a}
				data-testid={'cut-axis-' + a}
				class="ws-chip {s.axis === a ? 'ws-chip-on' : ''}"
				onclick={() => s.setAxis(a)}
			>
				{a.toUpperCase()}
			</button>
		{/each}
		{#if !s.axis}<span class="text-xs text-primary" data-testid="cut-tilted">
				{$t('tilted')}
			</span>{/if}
		<span class="flex-1"></span>
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-ghost"
			data-testid="cut-flip-plane"
			title={$t('Flip cut plane (swap top and bottom)')}
			onclick={() => s.flipPlane()}
		>
			⇅
		</button>
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-ghost"
			data-testid="cut-reset"
			title={$t('Reset the cut plane and remove the connectors')}
			onclick={() => s.reset()}
		>
			↺
		</button>
	</div>

	<label class="flex items-center gap-2">
		<span class="w-20 text-content-muted">
			{s.displayAxis === 2 ? $t('Height') : $t('Position||cut')}
		</span>
		<input
			type="range"
			data-testid="cut-offset"
			min={lo}
			max={hi}
			step={0.5}
			value={world}
			oninput={(e) => setWorld(num(e))}
			class="flex-1"
			aria-label={s.displayAxis === 2
				? $t('Cut height (mm)')
				: $t('Cut position along {axis} (mm)', { axis: axisName })}
		/>
		<input
			type="number"
			data-testid="cut-offset-input"
			value={world}
			step={0.1}
			oninput={(e) => {
				if (e.currentTarget.value !== '') setWorld(num(e), false);
			}}
			onchange={(e) => setWorld(num(e))}
			class="ws-field ws-field-sm w-20 text-right"
			aria-label={s.displayAxis === 2
				? $t('Cut height (mm)')
				: $t('Cut position along {axis} (mm)', { axis: axisName })}
		/>
		<span class="text-content-subtle">mm</span>
	</label>
	<p class="text-xs text-content-subtle" data-testid="cut-range">
		{$t(
			s.displayAxis === 2
				? 'From the bed (plane center, {axis}): {lo} – {hi} mm'
				: 'Bed coordinate (plane center, {axis}): {lo} – {hi} mm',
			{ axis: axisName, lo, hi }
		)}
	</p>

	<fieldset class="flex flex-col gap-1">
		<legend class="text-content-muted">{$t('Plane tilt')}</legend>
		<div class="flex items-center gap-1">
			{#each ['X', 'Y', 'Z'] as a, i (a)}
				<label class="flex items-center gap-0.5 text-xs">
					<span class="text-content-subtle">{a}</span>
					<input
						type="number"
						step="5"
						data-testid={'cut-rot-' + a.toLowerCase()}
						value={s.rotation[i]}
						onchange={(e) => setRot(i, num(e))}
						class="ws-field ws-field-sm w-16 text-right"
						aria-label={$t('Plane rotation around {axis} (°)', { axis: a })}
					/>°
				</label>
			{/each}
		</div>
		<p class="text-xs text-content-subtle">
			{$t('Or rotate the rings around the plane in the 3D view.')}
		</p>
	</fieldset>

	{#if s.mode === 'planar'}
		<section class="flex flex-col gap-1 border-t border-border pt-2" aria-label={$t('Connectors')}>
			<div class="flex items-center gap-2">
				<span class="font-medium">{$t('Connectors')}</span>
				<span class="text-xs text-content-subtle" data-testid="cut-connector-count">
					{s.connectors.length}
				</span>
				<span class="flex-1"></span>
				<button
					type="button"
					class="ws-btn ws-btn-sm {s.editingConnectors ? 'ws-btn-primary' : 'ws-btn-secondary'}"
					data-testid="cut-connectors-edit"
					disabled={!canConnect}
					aria-pressed={s.editingConnectors}
					title={canConnect
						? $t('Then click the cut face to place a connector')
						: $t('Unavailable with “Cut to parts” (Orca)')}
					onclick={() => (s.editingConnectors = !s.editingConnectors)}
				>
					{s.editingConnectors
						? $t('Finish')
						: s.connectors.length
							? $t('Edit connectors')
							: $t('Add connectors')}
				</button>
			</div>
			<p class="text-xs text-content-subtle" data-testid="cut-connectors-note">
				{s.editingConnectors
					? $t(
							'Click the cut face (below or in the 3D view) to place a connector; drag it to move it, Delete to remove it.'
						)
					: $t('Plugs, dowels or snaps to reassemble the two halves.')}
			</p>
			{#if s.editingConnectors}
				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<svg
					bind:this={svg}
					viewBox="{view.x} {view.y} {view.w} {view.h}"
					class="h-40 w-full cursor-crosshair rounded border border-border bg-surface-sunken"
					data-testid="cut-section-map"
					role="application"
					aria-label={$t('Cut face: click to place a connector')}
					tabindex="0"
					onpointerdown={onMapDown}
					onpointermove={onMapMove}
					onpointerup={() => (dragging = null)}
					onkeydown={onKey}
				>
					<path
						d={path}
						stroke="currentColor"
						stroke-width={Math.max(view.w, view.h) / 150}
						fill="none"
						class="text-content-muted"
					/>
					{#each s.connectors as c (c.id)}
						{@const n = { triangle: 3, square: 4, hexagon: 6, circle: 0 }[c.shape]}
						{#if n}
							<polygon
								points={polygon(c.u, -c.v, c.size / 2, n, c.rotation)}
								class={c.id === s.selectedId ? 'fill-primary' : 'fill-warning'}
								data-testid="cut-connector"
							/>
						{:else}
							<circle
								cx={c.u}
								cy={-c.v}
								r={c.size / 2}
								class={c.id === s.selectedId ? 'fill-primary' : 'fill-warning'}
								data-testid="cut-connector"
							/>
						{/if}
					{/each}
				</svg>
				<div class="flex items-center gap-1" role="radiogroup" aria-label={$t('Connector type')}>
					<span class="w-16 text-xs text-content-muted">{$t('Type')}</span>
					{#each TYPES as ty (ty.id)}
						<button
							type="button"
							role="radio"
							aria-checked={s.params.type === ty.id}
							title={$t(ty.tip)}
							data-testid={'cut-type-' + ty.id}
							class="ws-chip {s.params.type === ty.id ? 'ws-chip-on' : ''}"
							onclick={() => s.setParams({ type: ty.id })}
						>
							{$t(ty.label)}
						</button>
					{/each}
				</div>
				<div class="flex items-center gap-1" role="radiogroup" aria-label={$t('Connector shape')}>
					<span class="w-16 text-xs text-content-muted">{$t('Shape')}</span>
					{#each SHAPES as sh (sh.id)}
						<button
							type="button"
							role="radio"
							aria-checked={s.params.shape === sh.id}
							disabled={s.params.type === 'snap'}
							data-testid={'cut-shape-' + sh.id}
							class="ws-chip ws-chip-sm {s.params.shape === sh.id ? 'ws-chip-on' : ''}"
							onclick={() => s.setParams({ shape: sh.id })}
						>
							{$t(sh.label)}
						</button>
					{/each}
				</div>
				<label class="flex items-center gap-1 text-xs">
					<span class="w-16 text-content-muted">{$t('Style')}</span>
					<select
						class="ws-field ws-field-sm"
						data-testid="cut-style"
						disabled={s.params.type !== 'plug'}
						value={s.params.style}
						onchange={(e) => s.setParams({ style: e.currentTarget.value as 'prism' | 'frustum' })}
					>
						<option value="prism">{$t('Prism')}</option>
						<option value="frustum">{$t('Frustum (cone)')}</option>
					</select>
				</label>
				{#each [['size', 'sizeTolerance', 'Size', 'Connector diameter (mm)', 'Diameter tolerance (mm)'], ['depth', 'depthTolerance', 'Depth', 'Connector depth (mm)', 'Depth tolerance (mm)']] as const as [k, tk, label, aria, tolAria] (k)}
					<div class="flex items-center gap-1 text-xs">
						<span class="w-16 text-content-muted">{$t(label)}</span>
						<input
							type="number"
							step="0.5"
							min="0.1"
							data-testid={'cut-' + k}
							value={s.params[k]}
							onchange={(e) => s.setParams({ [k]: num(e) })}
							class="ws-field ws-field-sm w-16 text-right"
							aria-label={$t(aria)}
						/>
						<span class="text-content-subtle">mm ± tol.</span>
						<input
							type="number"
							step="0.05"
							min="0"
							data-testid={'cut-' + k + '-tol'}
							value={s.params[tk]}
							onchange={(e) => s.setParams({ [tk]: num(e) })}
							class="ws-field ws-field-sm w-14 text-right"
							aria-label={$t(tolAria)}
						/>
					</div>
				{/each}
				<label class="flex items-center gap-1 text-xs">
					<span class="w-16 text-content-muted">{$t('Rotation')}</span>
					<input
						type="number"
						step="15"
						min="0"
						max="180"
						data-testid="cut-connector-rotation"
						value={s.params.rotation}
						onchange={(e) => s.setParams({ rotation: num(e) })}
						class="ws-field ws-field-sm w-16 text-right"
						aria-label={$t('Connector rotation (°)')}
					/>°
				</label>
				<div class="flex gap-2">
					<button
						type="button"
						class="ws-btn ws-btn-sm ws-btn-secondary"
						data-testid="cut-connector-remove"
						disabled={!s.selectedId}
						onclick={() => s.removeConnector()}
					>
						{$t('Remove')}
					</button>
					<button
						type="button"
						class="ws-btn ws-btn-sm ws-btn-secondary"
						data-testid="cut-connectors-clear"
						disabled={!s.connectors.length}
						onclick={() => s.clearConnectors()}
					>
						{$t('Remove connectors')}
					</button>
				</div>
				<p class="text-xs text-content-subtle">
					{s.selectedId
						? $t('Settings apply to the selected connector.')
						: $t('Settings apply to all connectors.')}
				</p>
			{/if}
		</section>
	{:else if s.groove}
		<section
			class="flex flex-col gap-1 border-t border-border pt-2 text-xs"
			aria-label={$t('Dovetail||section')}
			data-testid="cut-groove"
		>
			{#each [['depth', 'Depth', 'mm', 0.5], ['width', 'Width', 'mm', 0.5], ['depthTolerance', 'Depth tol.', 'mm', 0.05], ['widthTolerance', 'Width tol.', 'mm', 0.05], ['flapsAngle', 'Flaps angle', '° (30–120)', 5], ['angle', 'Groove Angle', '° (0–15)', 1], ['count', 'Count', '', 1], ['gap', 'Gap', 'mm', 1]] as const as [k, label, unit, step] (k)}
				<label class="flex items-center gap-1">
					<span class="w-32 text-content-muted">{$t(label)}</span>
					<input
						type="number"
						{step}
						data-testid={'cut-groove-' + k}
						value={s.groove[k]}
						onchange={(e) => {
							if (s.groove) s.groove = { ...s.groove, [k]: num(e) };
						}}
						disabled={k === 'gap' && s.groove.count <= 1}
						class="ws-field ws-field-sm w-16 text-right"
						aria-label={$t(label)}
					/>
					<span class="text-content-subtle">{unit}</span>
				</label>
			{/each}
		</section>
	{/if}

	<section class="flex flex-col gap-1 border-t border-border pt-2" aria-label={$t('After cut')}>
		<span class="font-medium">{$t('After cut')}</span>
		{#each [['upper', 'Upper part', 'bg-sky-500'], ['lower', 'Lower part', 'bg-amber-500']] as const as [k, label, dot] (k)}
			{@const part = s[k]}
			{@const locked = s.connectors.length > 0 || s.asParts}
			<div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
				<span class="flex w-28 items-center gap-1">
					<span class="h-2 w-2 rounded-full {dot}" aria-hidden="true"></span>{$t(label)}
				</span>
				<label
					class="flex items-center gap-1"
					title={locked ? $t('Always kept with connectors or as parts') : ''}
				>
					<input
						type="checkbox"
						data-testid={'cut-keep-' + k}
						checked={locked || part.keep}
						disabled={locked}
						onchange={(e) => (s[k] = { ...part, keep: e.currentTarget.checked })}
					/>{$t('Keep')}
				</label>
				<label class="flex items-center gap-1">
					<input
						type="checkbox"
						data-testid={'cut-place-' + k}
						checked={part.placeOnCut && !s.asParts}
						disabled={!part.keep || s.asParts}
						onchange={(e) => (s[k] = { ...part, placeOnCut: e.currentTarget.checked, flip: false })}
					/>{$t('Place on cut')}
				</label>
				<label class="flex items-center gap-1">
					<input
						type="checkbox"
						data-testid={'cut-flip-' + k}
						checked={part.flip && !s.asParts}
						disabled={!part.keep || s.asParts}
						onchange={(e) => (s[k] = { ...part, flip: e.currentTarget.checked, placeOnCut: false })}
					/>{$t('Flip')}
				</label>
			</div>
		{/each}
		<label
			class="flex items-center gap-1 text-xs"
			title={s.connectors.length || s.mode === 'groove'
				? $t('Unavailable with connectors or a dovetail (Orca)')
				: $t('A single object made of two parts')}
		>
			<input
				type="checkbox"
				data-testid="cut-as-parts"
				bind:checked={s.asParts}
				disabled={s.connectors.length > 0 || s.mode === 'groove'}
			/>{$t('Cut to parts (single object)')}
		</label>
	</section>

	{#if s.blocker}<p class="text-xs text-warning" data-testid="cut-blocker" role="status">
			{s.blocker}
		</p>{/if}
	<div class="flex gap-2">
		<button
			type="button"
			data-testid="cut-apply"
			class="ws-btn ws-btn-primary"
			disabled={busy || !!s.blocker}
			onclick={() => oncut()}
		>
			{busy ? $t('Cutting…') : $t('Cut')}
		</button>
		<button type="button" class="ws-btn ws-btn-secondary" onclick={() => oncancel?.()}>
			{$t('Cancel')}
		</button>
	</div>
</div>
