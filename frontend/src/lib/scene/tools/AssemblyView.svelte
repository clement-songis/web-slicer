<script lang="ts">
	// Vue d'assemblage (T058, `assembly_view` + gizmo Assembly) : bascule
	// l'éclaté des pièces et règle le facteur d'écartement. Présentationnel —
	// le parent applique `explode` aux positions de rendu (sans altérer les
	// transformations enregistrées).
	// Phase 39 : contrôles de la vue d'assemblage d'Orca 2.4.1
	// (GLCanvas3D::_render_assemble_control / _render_assemble_info) : « Vue en
	// coupe » (0–1, « Réinitialiser la direction »), « Mode de sélection »
	// Objet / Pièce et « Informations sur l'assemblage » (Volume, Taille).
	import { fmtNum, locale, t } from '$lib/i18n';

	interface Props {
		exploded?: boolean;
		factor?: number;
		/** Mode Orca `GLGizmoAssembly` : vue éclatée, face à face, point à point. */
		mode?: 'explode' | 'face' | 'point';
		/** Nombre d'entités déjà choisies (0 ou 1) en mode face/point. */
		picks?: number;
		onchange?: (exploded: boolean, factor: number) => void;
		/** Vue en coupe : position du plan (0 = sans coupe, 1 = tout coupé). */
		section?: number;
		/** « Réinitialiser la direction » : reprend la direction de vue courante. */
		onresetsection?: () => void;
		/** Mode de sélection d'Orca : objet entier ou pièce (volume) au clic. */
		selectionMode?: 'object' | 'part';
		/** Infos de la sélection (null = rien de sélectionné). */
		info?: { volume: string; size: string } | null;
	}

	let {
		exploded = $bindable(false),
		factor = $bindable(0.5),
		mode = $bindable('explode'),
		picks = 0,
		onchange,
		section = $bindable(0),
		onresetsection,
		selectionMode = $bindable('object'),
		info = null
	}: Props = $props();

	function emit() {
		onchange?.(exploded, factor);
	}
	function setSection(v: number) {
		section = Math.min(1, Math.max(0, Math.round(v * 100) / 100));
	}
</script>

<div class="flex flex-col gap-2 text-sm">
	<label class="flex items-center gap-2">
		<span class="text-content-muted">{$t('Mode')}</span>
		<select bind:value={mode} data-testid="assembly-mode" class="ws-field ws-field-sm">
			<option value="explode">{$t('Exploded view')}</option>
			<option value="face">{$t('Face and face assembly')}</option>
			<option value="point">{$t('Point and point assembly')}</option>
		</select>
	</label>
	{#if mode !== 'explode'}
		<p class="text-content-subtle" data-testid="assembly-hint" data-picks={picks}>
			{picks === 0
				? mode === 'face'
					? $t('Click a face of the fixed object.')
					: $t('Click a point of the fixed object.')
				: mode === 'face'
					? $t('Click a face of the object to move.')
					: $t('Click a point of the object to move.')}
		</p>
	{/if}
	<label class="flex items-center gap-2">
		<input type="checkbox" bind:checked={exploded} onchange={emit} />
		<span class="text-content-muted">{$t('Exploded view')}</span>
	</label>

	{#if exploded}
		<label class="flex items-center gap-2">
			<span class="w-20 text-content-muted">{$t('Spacing||explode')}</span>
			<input
				type="range"
				min={0}
				max={3}
				step={0.1}
				bind:value={factor}
				oninput={emit}
				class="flex-1"
			/>
			<span class="w-8 text-right text-content-muted">{fmtNum(factor, 1, $locale)}</span>
		</label>
	{/if}

	<div class="flex flex-col gap-1" data-testid="assembly-section" data-section={section.toFixed(2)}>
		<label class="flex items-center gap-2">
			<span class="w-20 shrink-0 text-content-muted">{$t('Section view')}</span>
			<input
				type="range"
				min={0}
				max={1}
				step={0.01}
				value={section}
				oninput={(e) => setSection(Number(e.currentTarget.value))}
				class="min-w-0 flex-1"
				aria-label={$t('Section view')}
			/>
			<input
				type="number"
				min={0}
				max={1}
				step={0.05}
				value={section.toFixed(2)}
				onchange={(e) => setSection(Number(e.currentTarget.value) || 0)}
				class="ws-field ws-field-sm w-16 text-right"
				aria-label={$t('Section position')}
			/>
		</label>
		{#if section > 0}
			<div class="flex items-center justify-between gap-2">
				<span class="text-xs text-content-subtle"
					>{$t('Section facing the view; 0 removes it.')}</span
				>
				<button
					type="button"
					class="ws-btn ws-btn-sm"
					data-testid="assembly-section-reset"
					onclick={() => onresetsection?.()}>{$t('Reset direction')}</button
				>
			</div>
		{/if}
	</div>

	<div class="flex items-center gap-2" role="radiogroup" aria-label={$t('Selection Mode')}>
		<span class="w-20 shrink-0 text-content-muted">{$t('Selection')}</span>
		<div class="ws-seg" data-testid="assembly-select-mode">
			{#each [['object', 'Object'], ['part', 'Part']] as const as [value, label] (value)}
				<button
					type="button"
					role="radio"
					aria-checked={selectionMode === value}
					data-testid="assembly-select-{value}"
					class="ws-seg-item {selectionMode === value ? 'ws-seg-on' : ''}"
					onclick={() => (selectionMode = value)}>{$t(label)}</button
				>
			{/each}
		</div>
	</div>
	{#if selectionMode === 'part'}
		<p class="text-xs text-content-subtle">{$t('A click selects the part under the cursor.')}</p>
	{/if}

	<section
		class="rounded border border-border bg-surface-sunken p-2"
		aria-label={$t('Assembly Info')}
		data-testid="assembly-info"
	>
		<h3 class="mb-1 text-xs font-semibold text-content-muted">{$t('Assembly Info')}</h3>
		{#if info}
			<dl class="grid grid-cols-[auto_1fr] gap-x-3 text-xs tabular-nums">
				<dt class="text-content-muted" title={$t('Volume of the selection’s bounding box')}>
					{$t('Volume:')}
				</dt>
				<dd data-testid="assembly-info-volume">{info.volume} mm³</dd>
				<dt class="text-content-muted">{$t('Size:')}</dt>
				<dd data-testid="assembly-info-size">{info.size} mm</dd>
			</dl>
		{:else}
			<p class="text-xs text-content-subtle">{$t('Select an object or a part.')}</p>
		{/if}
	</section>
</div>
