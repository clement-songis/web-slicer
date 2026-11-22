<script lang="ts">
	// Barre des gizmos de peinture (T056, lot 38) : canal, un seul sélecteur
	// d'outil (outils d'Orca propres au canal, cf. paint-tools.ts), état peint
	// (forcer / bloquer, ou filament MMU), puis le seul paramètre de l'outil
	// actif : rayon, hauteur de plage, angle (+ « Détection des arêtes » au MMU)
	// ou aire de comblement avec son bouton. Contrainte de souris « Verticale /
	// Horizontale » et remappage des filaments (MMU) en fin de barre.
	// Ctrl + molette : paramètre de l'outil actif ; Alt + molette remonte à la
	// page (plan de coupe). Présentationnel : la page envoie les messages moteur.
	import { CHANNELS, type PaintChannel } from './painting';
	import { ENFORCER, BLOCKER } from './facets';
	import { CHANNEL_LABELS } from './types';
	import { fmtNum, locale, t } from '$lib/i18n';
	import {
		FILL_ANGLE_MAX,
		FILL_ANGLE_MIN,
		GAP_AREA_MAX,
		GAP_AREA_MIN,
		GAP_AREA_STEP,
		HEIGHT_MAX,
		HEIGHT_MIN,
		PAINT_TOOL_HINTS,
		PAINT_TOOL_LABELS,
		OVERHANG_ANGLE_MAX,
		OVERHANG_ANGLE_MIN,
		RADIUS_MAX,
		RADIUS_MIN,
		clampOverhangAngle,
		constraintsFor,
		remapMessageMap,
		toolsFor,
		wheelParams,
		type PaintTool,
		type PointerConstraint
	} from './paint-tools';

	interface Props {
		channel?: PaintChannel;
		tool?: PaintTool;
		radius?: number;
		/** État peint appliqué au clic (ENFORCER, BLOCKER, ou index d'extrudeur MMU). */
		state?: number;
		/** Couleurs des filaments (slot i ↔ extrudeur i + 1) : pastilles du canal
		 *  multi-matériaux (Orca GLGizmoMmuSegmentation : un bouton coloré par filament). */
		filamentColours?: readonly string[];
		/** Plage de hauteur (mm). */
		height?: number;
		/** Angle d'arrêt du remplissage (°). */
		fillAngle?: number;
		/** Pot MMU : « Détection des arêtes » (décochée → tout le volume connexe). */
		edgeDetection?: boolean;
		/** Aire maximale comblée (mm²). */
		gapArea?: number;
		/** Contrainte de souris pendant le trait. */
		constraint?: PointerConstraint;
		/** Bouton « Combler » (Orca « Perform »). */
		ongapfill?: () => void;
		/** Remappage MMU : table 0-based (table[src] = dst). */
		onremap?: (table: number[]) => void;
		/** Requête moteur en cours (boutons désactivés). */
		busy?: boolean;
		/** Phase 39 — fil de fer (« Toggle Wireframe » d'Orca, Alt+Maj+Entrée). Orca
		 *  ne le dessine qu'en peinture multi-matériaux (set_wireframe_needed). */
		wireframe?: boolean;
		/** Supports — « Seulement sur les surplombs en évidence » (Orca
		 *  `m_paint_on_overhangs_only`). */
		overhangOnly?: boolean;
		/** Supports — « Mettre en évidence les zones en surplomb » : angle (°)
		 *  0–90, 0 = rien en évidence (`m_highlight_by_angle_threshold_deg`). */
		highlightAngle?: number;
		/** Seuil des supports automatiques (°), 0 = pas de support auto
		 *  (infobulle du curseur d'Orca). */
		supportThresholdAngle?: number;
		/** Surface irrégulière effective de l'objet = « Désactivé » : avertissement
		 *  d'Orca (GLGizmoFuzzySkin.cpp:331-357). */
		fuzzyDisabled?: boolean;
		/** Lien « Activer la surface irrégulière peinte pour cet objet ». */
		onenablefuzzy?: () => void;
	}

	let {
		channel = $bindable('supports'),
		tool = $bindable('brush'),
		radius = $bindable(2),
		state: painted = $bindable(ENFORCER),
		filamentColours = [],
		height = $bindable(0.2),
		fillAngle = $bindable(30),
		edgeDetection = $bindable(true),
		gapArea = $bindable(0),
		constraint = $bindable(null),
		ongapfill,
		onremap,
		busy = false,
		wireframe = $bindable(false),
		overhangOnly = $bindable(false),
		highlightAngle = $bindable(0),
		supportThresholdAngle = 0,
		fuzzyDisabled = false,
		onenablefuzzy
	}: Props = $props();
	// Audit 47 FUZZ-b : la surface irrégulière n'a qu'un état peint
	// (GLGizmoFuzzySkin.hpp:29 get_left_button_state_type = FUZZY_SKIN) — pas de
	// choix Forcer / Bloquer ; un état « Bloquer » hérité des supports est levé.
	$effect(() => {
		if (channel === 'fuzzy' && painted !== ENFORCER) painted = ENFORCER;
	});
	const angleTip = $derived(
		supportThresholdAngle > 0
			? $t('Auto support threshold angle: {n}', { n: supportThresholdAngle })
			: $t('No auto support')
	);

	const tools = $derived(toolsFor(channel));
	const constraints = $derived(constraintsFor(channel, tool));
	// Contrainte non proposée par l'outil courant : levée (Orca : cases propres
	// à chaque outil).
	$effect(() => {
		if (constraint && !constraints.includes(constraint)) constraint = null;
	});

	// Chiffre lisible sur la pastille (clair sur foncé, foncé sur clair).
	function contrastText(hex: string): string {
		const m = /^#?([0-9a-f]{6})$/i.exec(hex ?? '');
		if (!m) return 'inherit';
		const n = parseInt(m[1], 16);
		const lum = 0.299 * (n >> 16) + 0.587 * ((n >> 8) & 255) + 0.114 * (n & 255);
		return lum > 150 ? '#111827' : '#ffffff';
	}

	// Ctrl + molette : paramètre de l'outil actif (raccourci groupe Gizmo d'Orca) ;
	// Alt + molette remonte jusqu'à la page (plan de coupe).
	function onwheel(e: WheelEvent) {
		if (!e.ctrlKey) return;
		e.preventDefault();
		e.stopPropagation();
		const next = wheelParams(
			tool,
			{ radius, height, fillAngle, edgeDetection, gapArea },
			e.deltaY < 0
		);
		radius = next.radius;
		height = next.height;
		fillAngle = next.fillAngle;
		gapArea = next.gapArea;
	}

	// — Remappage des filaments (MMU) : tableau source → destination ——————
	let remapOpen = $state(false);
	let remapTable = $state<number[]>([]);
	$effect(() => {
		const n = filamentColours.length;
		if (remapTable.length !== n) remapTable = Array.from({ length: n }, (_, i) => i);
	});
	const remapChanged = $derived(remapMessageMap(remapTable) !== null);
	function applyRemap() {
		if (!remapChanged) return;
		onremap?.([...remapTable]);
		remapTable = remapTable.map((_, i) => i);
	}

	const CONSTRAINT_LABELS: Record<'vertical' | 'horizontal', string> = {
		vertical: 'Vertical',
		horizontal: 'Horizontal'
	};
</script>

<div class="flex flex-col gap-2 text-sm" role="toolbar" tabindex="0" {onwheel}>
	<label class="flex items-center gap-2">
		<span class="w-16 text-content-muted">{$t('Channel')}</span>
		<select bind:value={channel} class="ws-field ws-field-sm">
			{#each CHANNELS as c (c)}
				<option value={c}>{$t(CHANNEL_LABELS[c])}</option>
			{/each}
		</select>
	</label>

	<div class="flex items-start gap-2">
		<span class="w-16 shrink-0 pt-1 text-content-muted">{$t('Tool')}</span>
		<div class="flex flex-wrap gap-1" role="radiogroup" aria-label={$t('Paint tool')}>
			{#each tools as tl (tl)}
				<button
					type="button"
					role="radio"
					class="ws-chip {tool === tl ? 'ws-chip-on' : ''}"
					aria-checked={tool === tl}
					title={$t(PAINT_TOOL_HINTS[tl])}
					data-testid={`paint-tool-${tl}`}
					onclick={() => (tool = tl)}>{$t(PAINT_TOOL_LABELS[tl])}</button
				>
			{/each}
		</div>
	</div>
	<p class="pl-18 text-xs text-content-subtle" data-testid="paint-tool-hint">
		{$t(PAINT_TOOL_HINTS[tool])}
	</p>
	<!-- Audit 42 S2 : rappel des boutons de la souris (Orca : « Left mouse
	     button / Right mouse button / Shift + Left mouse button »). -->
	<p class="pl-18 text-xs text-content-subtle" data-testid="paint-mouse-hint">
		{channel === 'mmu'
			? $t('Left button: paint the selected filament · Shift: erase')
			: channel === 'fuzzy'
				? $t('Left button: paint · Right button or Shift: erase')
				: $t('Left button: enforce · Right button: block · Shift: erase')}
	</p>

	{#if channel === 'fuzzy'}
		<!-- Orca : clic gauche « Add fuzzy skin », Maj + clic « Remove fuzzy skin »
		     (GLGizmoFuzzySkin.cpp:49-54) — aucun choix d'état. -->
	{:else if channel !== 'mmu'}
		<div class="flex items-center gap-2" data-testid="paint-state">
			<span class="w-16 text-content-muted">{$t('Paint||state')}</span>
			<button
				type="button"
				class="ws-chip {painted === ENFORCER
					? 'bg-success text-primary-content border-transparent'
					: ''}"
				onclick={() => (painted = ENFORCER)}>{$t('Enforce')}</button
			>
			<button
				type="button"
				class="ws-chip {painted === BLOCKER
					? 'bg-danger text-primary-content border-transparent'
					: ''}"
				onclick={() => (painted = BLOCKER)}>{$t('Block')}</button
			>
		</div>
	{:else if filamentColours.length > 0}
		<div class="flex items-center gap-2">
			<span class="w-16 text-content-muted">{$t('Filament')}</span>
			<div class="flex flex-wrap gap-1" role="radiogroup" aria-label={$t('Painted filament')}>
				{#each filamentColours as colour, i (i)}
					<button
						type="button"
						role="radio"
						aria-checked={painted === i + 1}
						aria-label={$t('Filament {n}', { n: i + 1 })}
						title={$t('Filament {n}', { n: i + 1 })}
						class="flex size-7 items-center justify-center rounded border text-xs font-semibold {painted ===
						i + 1
							? 'border-accent ring-2 ring-accent'
							: 'border-border-strong'}"
						style:background-color={colour || undefined}
						style:color={contrastText(colour)}
						data-testid={`paint-filament-${i + 1}`}
						onclick={() => (painted = i + 1)}>{i + 1}</button
					>
				{/each}
			</div>
		</div>
	{:else}
		<label class="flex items-center gap-2">
			<span class="w-16 text-content-muted">{$t('Extruder')}</span>
			<input
				type="number"
				min={1}
				step={1}
				bind:value={painted}
				class="ws-field ws-field-sm w-16 text-right"
				aria-label={$t('Extruder index')}
			/>
		</label>
	{/if}

	{#if tool === 'brush' || tool === 'sphere'}
		<label class="flex items-center gap-2">
			<span class="w-16 text-content-muted">{$t('Radius')}</span>
			<input
				type="range"
				min={RADIUS_MIN}
				max={RADIUS_MAX}
				step={0.1}
				bind:value={radius}
				class="flex-1"
				data-testid="paint-radius"
			/>
			<span class="w-16 text-right whitespace-nowrap text-content-muted tabular-nums"
				>{fmtNum(radius, 1, $locale)} mm</span
			>
		</label>
	{:else if tool === 'height'}
		<label class="flex items-center gap-2">
			<span class="w-16 text-content-muted">{$t('Height')}</span>
			<input
				type="range"
				min={HEIGHT_MIN}
				max={HEIGHT_MAX}
				step={0.1}
				bind:value={height}
				class="flex-1"
				data-testid="paint-height"
			/>
			<span class="w-16 text-right whitespace-nowrap text-content-muted tabular-nums"
				>{fmtNum(height, 1, $locale)} mm</span
			>
		</label>
	{:else if tool === 'fill'}
		{#if channel === 'mmu'}
			<label class="flex items-center gap-2 pl-18 text-xs">
				<input type="checkbox" bind:checked={edgeDetection} data-testid="paint-edge-detection" />
				{$t('Stop at edges')}
			</label>
		{/if}
		{#if channel !== 'mmu' || edgeDetection}
			<label class="flex items-center gap-2">
				<span class="w-16 text-content-muted">{$t('Angle')}</span>
				<input
					type="range"
					min={FILL_ANGLE_MIN}
					max={FILL_ANGLE_MAX}
					step={1}
					bind:value={fillAngle}
					class="flex-1"
					data-testid="paint-fill-angle"
				/>
				<span class="w-16 text-right whitespace-nowrap text-content-muted tabular-nums"
					>{fillAngle}°</span
				>
			</label>
		{:else}
			<p class="pl-18 text-xs text-content-subtle">
				{$t('Fills the whole connected part of the object.')}
			</p>
		{/if}
	{:else if tool === 'gap'}
		<label class="flex items-center gap-2">
			<span class="w-16 text-content-muted">{$t('Max area')}</span>
			<input
				type="range"
				min={GAP_AREA_MIN}
				max={GAP_AREA_MAX}
				step={GAP_AREA_STEP}
				bind:value={gapArea}
				class="flex-1"
				data-testid="paint-gap-area"
			/>
			<span class="w-16 text-right whitespace-nowrap text-content-muted tabular-nums"
				>{fmtNum(gapArea, 1, $locale)} mm²</span
			>
		</label>
		<div class="flex items-center justify-between gap-2 pl-18">
			<span class="text-xs text-content-subtle">
				{gapArea > 0 ? $t('Smaller areas absorbed by their neighbor') : $t('Zero area: no effect')}
			</span>
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-primary"
				disabled={busy || gapArea <= 0}
				data-testid="paint-gap-apply"
				onclick={() => ongapfill?.()}>{$t('Fill gaps')}</button
			>
		</div>
	{/if}

	{#if channel === 'supports'}
		<!-- Audit 46 SUPP-a : GLGizmoFdmSupports (« On highlighted overhangs only »,
		     masquée pour le comblement ; « Highlight overhang areas » 0–90°). -->
		<div class="flex flex-col gap-1 border-t border-border pt-2" data-testid="paint-overhang">
			{#if tool !== 'gap'}
				<label
					class="flex items-center gap-2 text-xs"
					title={$t('Allows painting only on facets selected by: "{name}"', {
						name: $t('Highlight overhang areas')
					})}
				>
					<input type="checkbox" bind:checked={overhangOnly} data-testid="paint-overhang-only" />
					{$t('On highlighted overhangs only')}
				</label>
			{/if}
			<span class="text-xs text-content-muted">{$t('Highlight overhang areas')}</span>
			<div class="flex items-center gap-2" title={angleTip}>
				<input
					type="range"
					min={OVERHANG_ANGLE_MIN}
					max={OVERHANG_ANGLE_MAX}
					step={1}
					bind:value={highlightAngle}
					class="flex-1"
					aria-label={$t('Highlight overhang areas')}
					title={$t('Highlight faces according to overhang angle.')}
					data-testid="paint-overhang-angle"
				/>
				<input
					type="number"
					min={OVERHANG_ANGLE_MIN}
					max={OVERHANG_ANGLE_MAX}
					step={0.05}
					value={highlightAngle}
					onchange={(e) => (highlightAngle = clampOverhangAngle(e.currentTarget.valueAsNumber))}
					class="ws-field ws-field-sm w-16 text-right tabular-nums"
					aria-label={$t('Highlight overhang areas')}
					data-testid="paint-overhang-angle-input"
				/>
				<span class="text-content-muted">°</span>
			</div>
		</div>
	{/if}

	{#if constraints.length}
		<div class="flex items-center gap-2">
			<span class="w-16 text-content-muted">{$t('Guide')}</span>
			{#each constraints as c (c)}
				{#if c}
					<label class="flex items-center gap-1 text-xs">
						<input
							type="checkbox"
							checked={constraint === c}
							data-testid={`paint-constraint-${c}`}
							onchange={(e) => (constraint = e.currentTarget.checked ? c : null)}
						/>
						{$t(CONSTRAINT_LABELS[c])}
					</label>
				{/if}
			{/each}
		</div>
	{/if}

	{#if tool !== 'gap' && tool !== 'pointer'}
		<p class="text-xs text-content-subtle">
			{$t('Ctrl + wheel: {param} · Alt + wheel: cut plane', {
				param: $t(
					tool === 'height' ? 'height||wheel' : tool === 'fill' ? 'angle||wheel' : 'radius||wheel'
				)
			})}
		</p>
	{/if}

	{#if channel === 'mmu'}
		<label class="flex items-center gap-2 text-xs" title={$t('Alt + Shift + Enter')}>
			<input type="checkbox" bind:checked={wireframe} data-testid="paint-wireframe" />
			<span>{$t('Show wireframe')}</span>
			<kbd class="ml-auto text-[10px] text-content-subtle">{$t('Alt+Shift+Enter')}</kbd>
		</label>
	{/if}

	{#if channel === 'fuzzy' && fuzzyDisabled}
		<div
			class="flex flex-col gap-1 border-t border-border pt-2 text-xs text-warning"
			role="status"
			data-testid="paint-fuzzy-disabled"
		>
			<p>
				{$t('Warning: Fuzzy skin is disabled, painted fuzzy skin will not take effect!')}
			</p>
			<button
				type="button"
				class="self-start font-semibold text-primary underline"
				data-testid="paint-fuzzy-enable"
				onclick={() => onenablefuzzy?.()}>{$t('Enable painted fuzzy skin for this object')}</button
			>
		</div>
	{/if}

	{#if channel === 'mmu' && filamentColours.length > 1}
		<div class="border-t border-border pt-2">
			<button
				type="button"
				class="flex w-full items-center justify-between text-xs text-content-muted hover:text-content"
				aria-expanded={remapOpen}
				data-testid="paint-remap-open"
				onclick={() => (remapOpen = !remapOpen)}
			>
				<span>{$t('Remap filaments')}</span>
				<span aria-hidden="true">{remapOpen ? '▾' : '▸'}</span>
			</button>
			{#if remapOpen}
				<div class="mt-2 flex flex-col gap-1" data-testid="paint-remap">
					{#each filamentColours as colour, src (src)}
						{@const dst = remapTable[src] ?? src}
						<div class="flex items-center gap-2 text-xs">
							<span
								class="flex size-6 items-center justify-center rounded border border-border-strong font-semibold"
								style:background-color={colour || undefined}
								style:color={contrastText(colour)}>{src + 1}</span
							>
							<span aria-hidden="true" class="text-content-subtle">→</span>
							<span
								class="flex size-6 items-center justify-center rounded border border-border-strong font-semibold"
								style:background-color={filamentColours[dst] || undefined}
								style:color={contrastText(filamentColours[dst])}>{dst + 1}</span
							>
							<select
								class="ws-field ws-field-sm flex-1"
								aria-label={$t('Filament {n} becomes', { n: src + 1 })}
								data-testid={`paint-remap-${src + 1}`}
								value={dst}
								onchange={(e) => {
									const next = [...remapTable];
									next[src] = Number(e.currentTarget.value);
									remapTable = next;
								}}
							>
								{#each filamentColours, j (j)}
									<option value={j}>{$t('Filament {n}', { n: j + 1 })}</option>
								{/each}
							</select>
						</div>
					{/each}
					<div class="mt-1 flex items-center justify-between gap-2">
						<span class="text-xs text-content-subtle"
							>{$t('Painting and base filament of the object')}</span
						>
						<button
							type="button"
							class="ws-btn ws-btn-sm ws-btn-primary"
							disabled={busy || !remapChanged}
							data-testid="paint-remap-apply"
							onclick={applyRemap}>{$t('Apply')}</button
						>
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>
